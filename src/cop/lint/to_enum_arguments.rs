/// Lint/ToEnumArguments
///
/// Ensures that `to_enum`/`enum_for`, called for the current method,
/// has correct arguments. The cop checks that each argument passed to
/// `to_enum`/`enum_for` matches the enclosing method's parameters by
/// source text (not just count).
///
/// ## Investigation findings
///
/// The original implementation used a simple count-based check
/// (`provided_args < param_count`), which missed several categories of FN:
///
/// 1. **Argument value mismatch**: `def combination(n)` →
///    `enum_for(:combination, num)` passes `num` but the param is `n`.
///    RuboCop checks source text equality, not just count.
///
/// 2. **Swapped arguments**: `def m(x, y)` → `to_enum(:m, y, x)` has
///    matching count but wrong order.
///
/// 3. **Keyword argument mismatch**: `def m(required:)` →
///    `to_enum(:m, required: something_else)` passes the wrong value
///    for the keyword argument.
///
/// 4. **Missing optional/keyword/rest args**: count-based check didn't
///    properly handle keyword args since they're packed into a single
///    KeywordHashNode in the call arguments.
///
/// The fix rewrites the cop to use RuboCop's `arguments_match?` approach:
/// iterate over each def parameter (excluding block), advance a positional
/// index for required/optional/rest params, and verify source text matches.
/// Keyword params check the KeywordHashNode for matching `key: key` pairs.
///
/// 5. **Ruby 3.1 shorthand hash syntax**: `enum_for(__method__, prefix:)`
///    uses value omission where `prefix:` means `prefix: prefix`. Prism
///    wraps the value in an `ImplicitNode` containing a `LocalVariableReadNode`.
///    Fix: unwrap `ImplicitNode` in `assoc_value_matches_param` before checking
///    the local variable name.
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ToEnumArguments;

impl Cop for ToEnumArguments {
    fn name(&self) -> &'static str {
        "Lint/ToEnumArguments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = ToEnumVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            method_stack: Vec::new(),
            corrections: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(c) = corrections.as_deref_mut() {
            c.extend(visitor.corrections);
        }
    }
}

/// Describes a single method parameter for argument matching.
#[derive(Debug)]
enum ParamKind {
    /// Required positional: `def m(x)` — source must match `x`
    Required(Vec<u8>),
    /// Optional positional: `def m(x = 1)` — source must match `x` (just name, not default)
    Optional(Vec<u8>),
    /// Rest positional: `def m(*args)` — source must match `*args`
    Rest(Vec<u8>),
    /// Required keyword: `def m(key:)` — call arg hash must contain `key: key`
    Keyword(Vec<u8>),
    /// Optional keyword: `def m(key: val)` — call arg hash must contain `key: key`
    OptionalKeyword(Vec<u8>),
    /// Keyword rest: `def m(**opts)` — call arg must contain `**opts`
    KeywordRest(Vec<u8>),
    /// Forwarding: `def m(...)` — call arg must be `...`
    ForwardArg,
}

struct MethodInfo {
    name: Vec<u8>,
    params: Vec<ParamKind>,
}

struct ToEnumVisitor<'a, 'src> {
    cop: &'a ToEnumArguments,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    method_stack: Vec<MethodInfo>,
    corrections: Vec<crate::correction::Correction>,
}

fn node_source<'a>(node: &ruby_prism::Node<'a>) -> &'a [u8] {
    node.location().as_slice()
}

/// Extract parameters from a DefNode into ParamKind entries (excluding block arg).
fn extract_params(params: &ruby_prism::ParametersNode<'_>) -> Vec<ParamKind> {
    let mut result = Vec::new();

    for req in params.requireds().iter() {
        // RequiredParameterNode has a name; source is just the name
        if let Some(rp) = req.as_required_parameter_node() {
            result.push(ParamKind::Required(rp.name().as_slice().to_vec()));
        } else {
            // MultiTargetNode (destructuring) — use source text
            result.push(ParamKind::Required(node_source(&req).to_vec()));
        }
    }

    for opt in params.optionals().iter() {
        if let Some(op) = opt.as_optional_parameter_node() {
            // For optional params, we only match on the name (not the default value)
            result.push(ParamKind::Optional(op.name().as_slice().to_vec()));
        }
    }

    if let Some(rest) = params.rest() {
        // RestParameterNode — source is `*args` or just `*`
        let source = rest.location().as_slice().to_vec();
        result.push(ParamKind::Rest(source));
    }

    for kw in params.keywords().iter() {
        if let Some(rkp) = kw.as_required_keyword_parameter_node() {
            result.push(ParamKind::Keyword(rkp.name().as_slice().to_vec()));
        } else if let Some(okp) = kw.as_optional_keyword_parameter_node() {
            result.push(ParamKind::OptionalKeyword(okp.name().as_slice().to_vec()));
        }
    }

    if let Some(kw_rest) = params.keyword_rest() {
        if kw_rest.as_forwarding_parameter_node().is_some() {
            result.push(ParamKind::ForwardArg);
        } else {
            // KeywordRestParameterNode — source is `**opts`
            let source = kw_rest.location().as_slice().to_vec();
            result.push(ParamKind::KeywordRest(source));
        }
    }

    // Block arg is excluded (matching RuboCop behavior)

    result
}

/// Check if the call arguments match the method parameters.
/// Mirrors RuboCop's `arguments_match?` logic.
fn arguments_match(call_args: &[ruby_prism::Node<'_>], params: &[ParamKind]) -> bool {
    let mut index: usize = 0;

    for param in params {
        match param {
            ParamKind::Required(name) => {
                let send_arg = call_args.get(index);
                index += 1;
                match send_arg {
                    Some(arg) => {
                        if node_source(arg) != name.as_slice() {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            ParamKind::Optional(name) => {
                let send_arg = call_args.get(index);
                index += 1;
                match send_arg {
                    Some(arg) => {
                        if node_source(arg) != name.as_slice() {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            ParamKind::Rest(source) => {
                let send_arg = call_args.get(index);
                index += 1;
                match send_arg {
                    Some(arg) => {
                        // For rest, match source exactly (e.g., `*args`)
                        if node_source(arg) != source.as_slice() {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            ParamKind::Keyword(name) => {
                // Keyword args: look for a hash arg containing `name: name`
                if !any_call_arg_has_keyword_pair(call_args, name) {
                    return false;
                }
            }
            ParamKind::OptionalKeyword(name) => {
                // Same check for optional keywords
                if !any_call_arg_has_keyword_pair(call_args, name) {
                    return false;
                }
            }
            ParamKind::KeywordRest(source) => {
                // Look for **kwargs splat in call args
                if !any_call_arg_has_kwsplat(call_args, source) {
                    return false;
                }
            }
            ParamKind::ForwardArg => {
                // Look for ... in call args
                let send_arg = call_args.get(index);
                match send_arg {
                    Some(arg) => {
                        if arg.as_forwarding_arguments_node().is_none() {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }
    }

    true
}

fn param_to_to_enum_arg(param: &ParamKind) -> Option<String> {
    match param {
        ParamKind::Required(name) | ParamKind::Optional(name) => {
            Some(String::from_utf8_lossy(name).to_string())
        }
        ParamKind::Rest(source) | ParamKind::KeywordRest(source) => {
            Some(String::from_utf8_lossy(source).to_string())
        }
        ParamKind::Keyword(name) | ParamKind::OptionalKeyword(name) => {
            let n = String::from_utf8_lossy(name);
            Some(format!("{n}: {n}"))
        }
        ParamKind::ForwardArg => Some("...".to_string()),
    }
}

fn autocorrect_rewrite_for_to_enum_call(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    method_arg: &ruby_prism::Node<'_>,
    params: &[ParamKind],
) -> Option<String> {
    let method_name = std::str::from_utf8(call.name().as_slice()).ok()?;
    let method_arg_loc = method_arg.location();
    let method_arg_src = source.byte_slice(
        method_arg_loc.start_offset(),
        method_arg_loc.end_offset(),
        "",
    );

    let receiver_prefix = if let Some(recv) = call.receiver() {
        let recv_loc = recv.location();
        let recv_src = source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
        let op = call
            .call_operator_loc()
            .map(|op| source.byte_slice(op.start_offset(), op.end_offset(), "."))
            .unwrap_or(".");
        format!("{recv_src}{op}")
    } else {
        String::new()
    };

    let call_args: Vec<String> = params.iter().filter_map(param_to_to_enum_arg).collect();
    if call_args.is_empty() {
        Some(format!("{receiver_prefix}{method_name}({method_arg_src})"))
    } else {
        Some(format!(
            "{receiver_prefix}{method_name}({method_arg_src}, {})",
            call_args.join(", ")
        ))
    }
}

/// Check if the value of an AssocNode is a local variable read matching `param_name`.
/// Handles both explicit `key: key` and Ruby 3.1+ shorthand `key:` (ImplicitNode wrapper).
fn assoc_value_matches_param(value: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    // Direct local variable read: `key: key`
    if let Some(lvar) = value.as_local_variable_read_node() {
        return lvar.name().as_slice() == param_name;
    }
    // Ruby 3.1+ shorthand `key:` — Prism wraps the value in an ImplicitNode
    if let Some(implicit) = value.as_implicit_node() {
        if let Some(lvar) = implicit.value().as_local_variable_read_node() {
            return lvar.name().as_slice() == param_name;
        }
    }
    false
}

/// Check if any call argument is a hash containing a `name: name` pair.
fn any_call_arg_has_keyword_pair(call_args: &[ruby_prism::Node<'_>], param_name: &[u8]) -> bool {
    for arg in call_args {
        if let Some(kh) = arg.as_keyword_hash_node() {
            for elem in kh.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    // Check key is a symbol matching param_name
                    if let Some(sym_key) = assoc.key().as_symbol_node() {
                        if sym_key.unescaped() == param_name
                            && assoc_value_matches_param(&assoc.value(), param_name)
                        {
                            return true;
                        }
                    }
                }
            }
        }
        // Also check HashNode (explicit `{}`)
        if let Some(h) = arg.as_hash_node() {
            for elem in h.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym_key) = assoc.key().as_symbol_node() {
                        if sym_key.unescaped() == param_name
                            && assoc_value_matches_param(&assoc.value(), param_name)
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if any call argument contains a kwsplat matching the source (e.g., `**kwargs`).
fn any_call_arg_has_kwsplat(call_args: &[ruby_prism::Node<'_>], source: &[u8]) -> bool {
    for arg in call_args {
        if let Some(kh) = arg.as_keyword_hash_node() {
            for elem in kh.elements().iter() {
                if let Some(splat) = elem.as_assoc_splat_node() {
                    if splat.location().as_slice() == source {
                        return true;
                    }
                }
            }
        }
        // Also check HashNode
        if let Some(h) = arg.as_hash_node() {
            for elem in h.elements().iter() {
                if let Some(splat) = elem.as_assoc_splat_node() {
                    if splat.location().as_slice() == source {
                        return true;
                    }
                }
            }
        }
    }
    false
}

impl<'pr> Visit<'pr> for ToEnumVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice().to_vec();

        let params = if let Some(parameters) = node.parameters() {
            extract_params(&parameters)
        } else {
            Vec::new()
        };

        self.method_stack.push(MethodInfo { name, params });
        ruby_prism::visit_def_node(self, node);
        self.method_stack.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        if (method_name == b"to_enum" || method_name == b"enum_for")
            && (node.receiver().is_none()
                || node
                    .receiver()
                    .as_ref()
                    .is_some_and(|r| r.as_self_node().is_some()))
        {
            if let Some(current_method) = self.method_stack.last() {
                if let Some(args) = node.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();

                    if !arg_list.is_empty() {
                        // First arg should be the method name
                        let first = &arg_list[0];
                        let refers_to_current = is_method_ref(first, &current_method.name);

                        if refers_to_current && !current_method.params.is_empty() {
                            // Remaining args (after method name) should match the method params
                            let call_args = &arg_list[1..];
                            if !arguments_match(call_args, &current_method.params) {
                                let loc = node.location();
                                let (line, column) =
                                    self.source.offset_to_line_col(loc.start_offset());
                                let mut diagnostic = self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    "Ensure you correctly provided all the arguments.".to_string(),
                                );

                                if let Some(replacement) = autocorrect_rewrite_for_to_enum_call(
                                    self.source,
                                    node,
                                    first,
                                    &current_method.params,
                                ) {
                                    self.corrections.push(crate::correction::Correction {
                                        start: loc.start_offset(),
                                        end: loc.end_offset(),
                                        replacement,
                                        cop_name: self.cop.name(),
                                        cop_index: 0,
                                    });
                                    diagnostic.corrected = true;
                                }

                                self.diagnostics.push(diagnostic);
                            }
                        }
                    }
                }
            }
        }

        // Visit children normally
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    // Note: we intentionally do NOT override visit_class_node / visit_module_node
    // to skip them. Methods inside classes/modules should be visited — the
    // method_stack tracks the enclosing def scope correctly.
}

fn is_method_ref(node: &ruby_prism::Node<'_>, method_name: &[u8]) -> bool {
    // Check for :method_name (symbol)
    if let Some(sym) = node.as_symbol_node() {
        return sym.unescaped() == method_name;
    }

    // Check for __method__ or __callee__
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"__method__" || name == b"__callee__") && call.receiver().is_none() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ToEnumArguments, "cops/lint/to_enum_arguments");

    #[test]
    fn supports_autocorrect() {
        assert!(ToEnumArguments.supports_autocorrect());
    }

    #[test]
    fn autocorrect_fills_missing_positional_args() {
        crate::testutil::assert_cop_autocorrect(
            &ToEnumArguments,
            b"def foo(x, y = 1)\n  to_enum(__method__, x)\nend\n",
            b"def foo(x, y = 1)\n  to_enum(__method__, x, y)\nend\n",
        );
    }

    #[test]
    fn autocorrect_fills_missing_keyword_and_kwrest_args() {
        crate::testutil::assert_cop_autocorrect(
            &ToEnumArguments,
            b"def setup(x, required:, **kwargs)\n  to_enum(:setup, x)\nend\n",
            b"def setup(x, required:, **kwargs)\n  to_enum(:setup, x, required: required, **kwargs)\nend\n",
        );
    }
}
