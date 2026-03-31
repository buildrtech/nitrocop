use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE,
    HASH_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE,
    REGULAR_EXPRESSION_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SelectByRegexp;

impl SelectByRegexp {
    fn is_regexp(node: &ruby_prism::Node<'_>) -> bool {
        node.as_regular_expression_node().is_some()
            || node.as_interpolated_regular_expression_node().is_some()
    }

    fn is_local_var_named(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
        if let Some(lvar) = node.as_local_variable_read_node() {
            return lvar.name().as_slice() == name;
        }
        false
    }

    fn regexp_match_arg(
        body: &ruby_prism::Node<'_>,
        block_arg_name: &[u8],
    ) -> Option<(usize, usize, bool)> {
        let call = body.as_call_node()?;
        let name_bytes = call.name().as_slice();
        if !matches!(name_bytes, b"match?" | b"=~" | b"!~") {
            return None;
        }

        let receiver = call.receiver()?;
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }

        let recv_is_var = Self::is_local_var_named(&receiver, block_arg_name);
        let arg_is_var = Self::is_local_var_named(&arg_list[0], block_arg_name);
        let recv_is_re = Self::is_regexp(&receiver);
        let arg_is_re = Self::is_regexp(&arg_list[0]);

        if recv_is_var && arg_is_re {
            let loc = arg_list[0].location();
            return Some((loc.start_offset(), loc.end_offset(), name_bytes == b"!~"));
        }

        if recv_is_re && arg_is_var {
            let loc = receiver.location();
            return Some((loc.start_offset(), loc.end_offset(), name_bytes == b"!~"));
        }

        None
    }

    fn is_hash_receiver(node: &ruby_prism::Node<'_>) -> bool {
        if node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some() {
            return true;
        }
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            if matches!(name_bytes, b"to_h" | b"to_hash") {
                return true;
            }
            if matches!(name_bytes, b"new" | b"[]") {
                if let Some(recv) = call.receiver() {
                    if let Some(cr) = recv.as_constant_read_node() {
                        if cr.name().as_slice() == b"Hash" {
                            return true;
                        }
                    }
                    if let Some(cp) = recv.as_constant_path_node() {
                        if cp.location().as_slice().ends_with(b"Hash") {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"ENV" {
                return true;
            }
        }
        if let Some(cp) = node.as_constant_path_node() {
            if cp.location().as_slice().ends_with(b"ENV") {
                return true;
            }
        }
        false
    }
}

impl Cop for SelectByRegexp {
    fn name(&self) -> &'static str {
        "Style/SelectByRegexp"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            KEYWORD_HASH_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REGULAR_EXPRESSION_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // We check the CallNode; its block() gives us the BlockNode
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be select, filter, find_all, or reject
        if !matches!(
            method_bytes,
            b"select" | b"filter" | b"find_all" | b"reject"
        ) {
            return;
        }

        // Must not be called on a hash-like receiver
        if let Some(receiver) = call.receiver() {
            if Self::is_hash_receiver(&receiver) {
                return;
            }
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Get block parameters - must have exactly one
        let block_params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params_node = match block_params.as_block_parameters_node() {
            Some(p) => p,
            None => return,
        };

        let inner_params = match block_params_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let requireds: Vec<_> = inner_params.requireds().into_iter().collect();
        if requireds.len() != 1 {
            return;
        }

        let block_arg_name = match requireds[0].as_required_parameter_node() {
            Some(req) => req.name().as_slice().to_vec(),
            None => return,
        };

        // Block body must be a single expression that matches regexp
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let (regexp_start, regexp_end, negated_match) = if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                return;
            }

            match Self::regexp_match_arg(&body_nodes[0], &block_arg_name) {
                Some(result) => result,
                None => return,
            }
        } else {
            return;
        };

        let replacement = match method_bytes {
            b"select" | b"filter" | b"find_all" => {
                if negated_match {
                    "grep_v"
                } else {
                    "grep"
                }
            }
            b"reject" => {
                if negated_match {
                    "grep"
                } else {
                    "grep_v"
                }
            }
            _ => return,
        };

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("select");
        // Report on the whole call including block
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!(
                "Prefer `{}` to `{}` with a regexp match.",
                replacement, method_str
            ),
        );

        if let Some(corrs) = corrections.as_mut() {
            let regexp_source = source.byte_slice(regexp_start, regexp_end, "");
            let replacement_source = if let Some(receiver) = call.receiver() {
                let recv_loc = receiver.location();
                let recv_source = source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
                let op = if let Some(op_loc) = call.call_operator_loc() {
                    source.byte_slice(op_loc.start_offset(), op_loc.end_offset(), "")
                } else {
                    "."
                };
                format!("{recv_source}{op}{replacement}({regexp_source})")
            } else {
                format!("{replacement}({regexp_source})")
            };

            corrs.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: replacement_source,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelectByRegexp, "cops/style/select_by_regexp");
    crate::cop_autocorrect_fixture_tests!(SelectByRegexp, "cops/style/select_by_regexp");
}
