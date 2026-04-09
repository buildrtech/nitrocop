use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-22)
///
/// Extended corpus reported FP=0, FN=7.
///
/// FN=7: All caused by `.downcase()` and `.upcase()` with explicit empty
/// parentheses not being detected. `is_case_method` and
/// `is_valid_casecmp_operand` checked `call.opening_loc().is_none()` which
/// rejected calls with explicit parens like `.downcase()`. In Ruby,
/// `.downcase()` and `.downcase` are identical — parens are optional for
/// 0-arg methods. Fixed by removing the `opening_loc().is_none()` check.
/// The `call.arguments().is_none()` guard already ensures no arguments.
pub struct Casecmp;

fn is_valid_casecmp_operand(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_string_node().is_some() {
        return true;
    }
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"downcase" || name == b"upcase")
            && call.receiver().is_some()
            && call.arguments().is_none()
            && !has_safe_navigation(&call)
        {
            return true;
        }
    }
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if body_nodes.len() == 1 {
                    let inner = &body_nodes[0];
                    if inner.as_string_node().is_some() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn has_safe_navigation(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(op) = call.call_operator_loc() {
        return op.as_slice() == b"&.";
    }
    false
}

fn is_case_method(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    (name == b"downcase" || name == b"upcase")
        && call.receiver().is_some()
        && call.arguments().is_none()
        && !has_safe_navigation(call)
}

fn build_casecmp_replacement(
    source: &SourceFile,
    variable: &ruby_prism::Node<'_>,
    arg: &ruby_prism::Node<'_>,
    negated: bool,
) -> String {
    let variable_loc = variable.location();
    let variable_source = source.byte_slice(variable_loc.start_offset(), variable_loc.end_offset(), "");
    let arg_loc = arg.location();
    let arg_source = source.byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "");

    let mut replacement = String::new();
    if negated {
        replacement.push('!');
    }
    replacement.push_str(variable_source);

    if arg.as_call_node().is_some() || arg.as_parentheses_node().is_none() {
        replacement.push_str(&format!(".casecmp({arg_source}).zero?"));
    } else {
        replacement.push_str(&format!(".casecmp{arg_source}.zero?"));
    }

    replacement
}

impl Cop for Casecmp {
    fn name(&self) -> &'static str {
        "Performance/Casecmp"
    }

    fn uses_node_check(&self) -> bool {
        true
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = outer_call.name().as_slice();

        let mut message = None::<String>;
        let mut replacement = None::<String>;

        if method == b"==" || method == b"!=" {
            let receiver = match outer_call.receiver() {
                Some(r) => r,
                None => return,
            };

            let args: Vec<_> = match outer_call.arguments() {
                Some(a) => a.arguments().iter().collect(),
                None => return,
            };
            if args.len() != 1 {
                return;
            }
            let rhs = &args[0];

            if let Some(recv_call) = receiver.as_call_node() {
                if is_case_method(&recv_call) && is_valid_casecmp_operand(rhs) {
                    let case_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("downcase");
                    let op = std::str::from_utf8(method).unwrap_or("==");
                    message = Some(format!("Use `casecmp` instead of `{case_method} {op}`."));

                    let variable = match recv_call.receiver() {
                        Some(v) => v,
                        None => return,
                    };
                    replacement = Some(build_casecmp_replacement(
                        source,
                        &variable,
                        rhs,
                        method == b"!=",
                    ));
                }
            }

            if replacement.is_none() {
                if let Some(rhs_call) = rhs.as_call_node() {
                    if is_case_method(&rhs_call) && is_valid_casecmp_operand(&receiver) {
                        let case_method =
                            std::str::from_utf8(rhs_call.name().as_slice()).unwrap_or("downcase");
                        let op = std::str::from_utf8(method).unwrap_or("==");
                        message = Some(format!("Use `casecmp` instead of `{op} {case_method}`."));

                        let variable = match rhs_call.receiver() {
                            Some(v) => v,
                            None => return,
                        };
                        replacement = Some(build_casecmp_replacement(
                            source,
                            &variable,
                            &receiver,
                            method == b"!=",
                        ));
                    }
                }
            }
        }

        if method == b"eql?" && replacement.is_none() {
            let receiver = match outer_call.receiver() {
                Some(r) => r,
                None => return,
            };

            let recv_call = match receiver.as_call_node() {
                Some(c) => c,
                None => return,
            };

            if !is_case_method(&recv_call) {
                return;
            }

            let args: Vec<_> = match outer_call.arguments() {
                Some(a) => a.arguments().iter().collect(),
                None => return,
            };
            if args.len() != 1 {
                return;
            }

            if is_valid_casecmp_operand(&args[0]) {
                let case_method =
                    std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("downcase");
                message = Some(format!("Use `casecmp` instead of `{case_method} eql?`."));

                let variable = match recv_call.receiver() {
                    Some(v) => v,
                    None => return,
                };
                replacement = Some(build_casecmp_replacement(source, &variable, &args[0], false));
            }
        }

        let (message, replacement) = match (message, replacement) {
            (Some(m), Some(r)) => (m, r),
            _ => return,
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(source, line, column, message);

        if let Some(ref mut corr) = corrections {
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Casecmp, "cops/performance/casecmp");
    crate::cop_autocorrect_fixture_tests!(Casecmp, "cops/performance/casecmp");

    #[test]
    fn supports_autocorrect() {
        assert!(Casecmp.supports_autocorrect());
    }
}
