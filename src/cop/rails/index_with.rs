use ruby_prism::Visit;

use crate::cop::node_type::{
    ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE,
    LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IndexWith;

fn contains_nested_block(node: &ruby_prism::Node<'_>) -> bool {
    struct Finder {
        found: bool,
    }

    impl<'pr> Visit<'pr> for Finder {
        fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {
            self.found = true;
        }
    }

    let mut f = Finder { found: false };
    f.visit(node);
    f.found
}

/// Check if the block body is an array literal `[block_param, value_expr]`
/// where the first element is a local variable reference matching the block parameter name.
fn is_index_with_block(block_node: &ruby_prism::BlockNode<'_>) -> bool {
    let params = match block_node.parameters() {
        Some(p) => p,
        None => return false,
    };
    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };
    let param_list = match block_params.parameters() {
        Some(pl) => pl,
        None => return false,
    };
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 1 {
        return false;
    }
    // Ensure there are no extra parameters (rest, optional, keyword, etc.)
    if param_list.rest().is_some()
        || !param_list.optionals().is_empty()
        || !param_list.posts().is_empty()
        || !param_list.keywords().is_empty()
        || param_list.keyword_rest().is_some()
    {
        return false;
    }
    let param_node = match requireds[0].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let param_name = param_node.name().as_slice();

    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }

    let array = match body_nodes[0].as_array_node() {
        Some(a) => a,
        None => return false,
    };
    let elements: Vec<_> = array.elements().iter().collect();
    if elements.len() != 2 {
        return false;
    }

    // First element must be a local variable read matching the block param
    let first = match elements[0].as_local_variable_read_node() {
        Some(lv) => lv,
        None => return false,
    };
    if first.name().as_slice() != param_name {
        return false;
    }
    // Second element (value) must be derived from the element (a method call),
    // not the element itself.
    if let Some(second_lvar) = elements[1].as_local_variable_read_node() {
        if second_lvar.name().as_slice() == param_name {
            return false;
        }
    }
    true
}

/// Check if the block is `each_with_object({}) { |el, memo| memo[el] = value }`
fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    source
        .byte_slice(
            node.location().start_offset(),
            node.location().end_offset(),
            "",
        )
        .to_string()
}

fn index_with_parts(block_node: &ruby_prism::BlockNode<'_>, source: &SourceFile) -> Option<(String, String)> {
    let params = block_node.parameters()?.as_block_parameters_node()?;
    let param_list = params.parameters()?;
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 1 {
        return None;
    }
    let param = requireds[0].as_required_parameter_node()?;
    let param_name = String::from_utf8_lossy(param.name().as_slice()).to_string();

    let body = block_node.body()?.as_statements_node()?;
    let nodes: Vec<_> = body.body().iter().collect();
    if nodes.len() != 1 {
        return None;
    }
    let arr = nodes[0].as_array_node()?;
    let elems: Vec<_> = arr.elements().iter().collect();
    if elems.len() != 2 {
        return None;
    }
    let first = elems[0].as_local_variable_read_node()?;
    if first.name().as_slice() != param.name().as_slice() {
        return None;
    }
    let value_src = node_source(source, &elems[1]);
    Some((param_name, value_src))
}

fn each_with_object_parts(
    block_node: &ruby_prism::BlockNode<'_>,
    source: &SourceFile,
) -> Option<(String, String)> {
    let params = block_node.parameters()?.as_block_parameters_node()?;
    let param_list = params.parameters()?;
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 2 {
        return None;
    }
    let el = requireds[0].as_required_parameter_node()?;
    let el_name = String::from_utf8_lossy(el.name().as_slice()).to_string();

    let body = block_node.body()?.as_statements_node()?;
    let nodes: Vec<_> = body.body().iter().collect();
    if nodes.len() != 1 {
        return None;
    }
    let assign = nodes[0].as_call_node()?;
    let args = assign.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 2 {
        return None;
    }
    let value_src = node_source(source, &arg_list[1]);
    Some((el_name, value_src))
}

fn is_each_with_object_index_with(
    call: &ruby_prism::CallNode<'_>,
    block_node: &ruby_prism::BlockNode<'_>,
) -> bool {
    if call.name().as_slice() != b"each_with_object" {
        return false;
    }
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return false;
        }
        let is_empty_hash = if let Some(hash) = arg_list[0].as_hash_node() {
            hash.elements().iter().count() == 0
        } else if let Some(kw_hash) = arg_list[0].as_keyword_hash_node() {
            kw_hash.elements().iter().count() == 0
        } else {
            false
        };
        if !is_empty_hash {
            return false;
        }
    } else {
        return false;
    }

    let params = match block_node.parameters() {
        Some(p) => p,
        None => return false,
    };
    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };
    let param_list = match block_params.parameters() {
        Some(pl) => pl,
        None => return false,
    };
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 2 {
        return false;
    }
    let el_param = match requireds[0].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let memo_param = match requireds[1].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let el_name = el_param.name().as_slice();
    let memo_name = memo_param.name().as_slice();

    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }
    let assign = match body_nodes[0].as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if assign.name().as_slice() != b"[]=" {
        return false;
    }
    // Receiver must be memo
    let recv = match assign.receiver() {
        Some(r) => r,
        None => return false,
    };
    let recv_lvar = match recv.as_local_variable_read_node() {
        Some(lv) => lv,
        None => return false,
    };
    if recv_lvar.name().as_slice() != memo_name {
        return false;
    }
    // Arguments: [key, value] where key is el
    if let Some(args) = assign.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 2 {
            return false;
        }
        // Key must be the element
        let key = match arg_list[0].as_local_variable_read_node() {
            Some(lv) => lv,
            None => return false,
        };
        if key.name().as_slice() != el_name {
            return false;
        }
        // Value must NOT be the element itself
        let value = &arg_list[1];
        if let Some(val_lvar) = value.as_local_variable_read_node() {
            if val_lvar.name().as_slice() == el_name {
                return false;
            }
        }
        // RuboCop does not flag complex RHS expressions that contain nested blocks.
        // Example from app: value built with `.map do ... end`.
        if contains_nested_block(value) {
            return false;
        }
        true
    } else {
        false
    }
}

impl Cop for IndexWith {
    fn name(&self) -> &'static str {
        "Rails/IndexWith"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // minimum_target_rails_version 6.0
        if !config.rails_version_at_least(6.0) {
            return;
        }

        // Pattern 1: items.map { |e| [e, value] }.to_h
        if let Some(chain) = util::as_method_chain(node) {
            if chain.outer_method == b"to_h"
                && (chain.inner_method == b"map" || chain.inner_method == b"collect")
            {
                // `.to_h` must have no arguments — `.to_h(arg)` is a different semantic.
                let outer_call = node.as_call_node().unwrap();
                if outer_call.arguments().is_none() {
                    if let Some(block) = chain.inner_call.block() {
                        if let Some(block_node) = block.as_block_node() {
                            if is_index_with_block(&block_node) {
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                let mut diagnostic = self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Use `index_with` instead of `map { ... }.to_h`.".to_string(),
                                );

                                if let Some((param, value_src)) = index_with_parts(&block_node, source)
                                    && let Some(base) = chain.inner_call.receiver()
                                    && let Some(ref mut corr) = corrections
                                {
                                    let base_src = node_source(source, &base);
                                    let replacement =
                                        format!("{base_src}.index_with {{ |{param}| {value_src} }}");
                                    corr.push(crate::correction::Correction {
                                        start: loc.start_offset(),
                                        end: loc.end_offset(),
                                        replacement,
                                        cop_name: self.name(),
                                        cop_index: 0,
                                    });
                                    diagnostic.corrected = true;
                                }

                                diagnostics.push(diagnostic);
                            }
                        }
                    }
                }
            }
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Pattern 2: items.to_h { |e| [e, value] }
        if call.name().as_slice() == b"to_h" {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if is_index_with_block(&block_node) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `index_with` instead of `to_h { ... }`.".to_string(),
                        );

                        if let Some((param, value_src)) = index_with_parts(&block_node, source)
                            && let Some(ref mut corr) = corrections
                        {
                            let replacement = if let Some(base) = call.receiver() {
                                let base_src = node_source(source, &base);
                                format!("{base_src}.index_with {{ |{param}| {value_src} }}")
                            } else {
                                format!("index_with {{ |{param}| {value_src} }}")
                            };
                            corr.push(crate::correction::Correction {
                                start: loc.start_offset(),
                                end: loc.end_offset(),
                                replacement,
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // Pattern 3: items.each_with_object({}) { |el, memo| memo[el] = value }
        if call.name().as_slice() == b"each_with_object" {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if is_each_with_object_index_with(&call, &block_node) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `index_with` instead of `each_with_object`.".to_string(),
                        );

                        if let Some((param, value_src)) = each_with_object_parts(&block_node, source)
                            && let Some(base) = call.receiver()
                            && let Some(ref mut corr) = corrections
                        {
                            let base_src = node_source(source, &base);
                            let replacement =
                                format!("{base_src}.index_with {{ |{param}| {value_src} }}");
                            corr.push(crate::correction::Correction {
                                start: loc.start_offset(),
                                end: loc.end_offset(),
                                replacement,
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // Pattern 4: Hash[items.map { |e| [e, value] }]
        if call.name().as_slice() == b"[]" {
            if let Some(recv) = call.receiver() {
                if util::constant_name(&recv) == Some(b"Hash") {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            if let Some(inner_call) = arg_list[0].as_call_node() {
                                let name = inner_call.name().as_slice();
                                if name == b"map" || name == b"collect" {
                                    if let Some(block) = inner_call.block() {
                                        if let Some(block_node) = block.as_block_node() {
                                            if is_index_with_block(&block_node) {
                                                let loc = node.location();
                                                let (line, column) =
                                                    source.offset_to_line_col(loc.start_offset());
                                                diagnostics.push(self.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Use `index_with` instead of `Hash[map { ... }]`.".to_string(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::run_cop_full_internal;
    use std::collections::HashMap;

    crate::cop_rails_fixture_tests!(IndexWith, "cops/rails/index_with", 6.0);

    #[test]
    fn supports_autocorrect() {
        assert!(IndexWith.supports_autocorrect());
    }

    #[test]
    fn autocorrects_direct_map_to_h_pattern() {
        let input = b"[1, 2, 3].map { |el| [el, foo(el)] }.to_h\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect_with_config(
            &IndexWith,
            input,
            config_with_rails(7.1),
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"[1, 2, 3].index_with { |el| foo(el) }\n");
    }

    fn config_with_rails(version: f64) -> CopConfig {
        CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(version)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn each_with_object_with_nested_block_rhs_is_not_flagged() {
        let source = b"SUPPORTED_COUNTRIES.each_with_object({}) do |code, subdivisions|\n  subdivisions[code] = ISO3166::Country[code].subdivisions.values.map do |subdivision|\n    [subdivision.name, subdivision.code]\n  end\nend\n";
        let diags = run_cop_full_internal(&IndexWith, source, config_with_rails(7.1), "test.rb");
        assert!(
            diags.is_empty(),
            "complex each_with_object RHS with nested block should not be flagged"
        );
    }
}
