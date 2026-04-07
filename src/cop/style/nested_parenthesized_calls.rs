use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for nested method calls without parentheses inside a parenthesized outer call.
///
/// ## Investigation notes (2026-03-15)
/// - 147 FPs caused by `!=` and `!~` operators not being recognized as operators.
///   The old character-based check (`!b.is_ascii_alphanumeric() && *b != b'!' ...`) excluded `!`
///   from the "operator character" set (because `!` also appears at the end of method names like
///   `save!`), which meant `!=` and `!~` were not skipped as operators.
/// - Fixed by replacing the character-based heuristic with an explicit operator method name list,
///   matching RuboCop's `operator_method?` behavior.
pub struct NestedParenthesizedCalls;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"<<",
    b">>", b"|", b"&", b"^", b"~", b"!", b"=~", b"!~", b"[]", b"[]=", b"+@", b"-@",
];

impl Cop for NestedParenthesizedCalls {
    fn name(&self) -> &'static str {
        "Style/NestedParenthesizedCalls"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let allowed_methods = config
            .options
            .get("AllowedMethods")
            .and_then(|v| v.as_sequence());

        // Looking for outer_method(inner_method arg) where inner_method has no parens
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Outer call must have actual parentheses (not [] brackets)
        let opening = match outer_call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        // Skip [] and []= calls — brackets are not parentheses
        if opening.as_slice() == b"[" {
            return;
        }

        let args = match outer_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let outer_arg_count = args.arguments().iter().count();

        for arg in args.arguments().iter() {
            let inner_call = match arg.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            // Inner call must NOT have parentheses
            if inner_call.opening_loc().is_some() {
                continue;
            }

            // Inner call must have arguments (otherwise it's just a method call)
            if inner_call.arguments().is_none() {
                continue;
            }

            // Must have a method name (not an operator)
            let inner_name = inner_call.name();
            let inner_bytes = inner_name.as_slice();

            // Skip operator methods (e.g. +, !=, !~, ==, <=>, etc.)
            if OPERATOR_METHODS.contains(&inner_bytes) {
                continue;
            }

            // Skip setter methods (ending with =)
            if inner_bytes.last() == Some(&b'=')
                && inner_bytes.len() > 1
                && inner_bytes[inner_bytes.len() - 2] != b'!'
            {
                continue;
            }

            // Check AllowedMethods - only allowed when outer has 1 arg and inner has 1 arg
            if let Some(allowed) = allowed_methods {
                if outer_arg_count == 1 {
                    let inner_arg_count = inner_call
                        .arguments()
                        .map(|a| a.arguments().iter().count())
                        .unwrap_or(0);
                    if inner_arg_count == 1 {
                        let name_str = std::str::from_utf8(inner_bytes).unwrap_or("");
                        if allowed.iter().any(|m| m.as_str() == Some(name_str)) {
                            continue;
                        }
                    }
                }
            }

            let inner_src = std::str::from_utf8(inner_call.location().as_slice()).unwrap_or("");
            let loc = inner_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                format!("Add parentheses to nested method call `{inner_src}`."),
            );
            if let Some(corrections) = corrections.as_mut() {
                if let (Some(msg_loc), Some(inner_args)) =
                    (inner_call.message_loc(), inner_call.arguments())
                {
                    if let Some(first_arg) = inner_args.arguments().iter().next() {
                        corrections.push(crate::correction::Correction {
                            start: msg_loc.end_offset(),
                            end: first_arg.location().start_offset(),
                            replacement: "(".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        corrections.push(crate::correction::Correction {
                            start: loc.end_offset(),
                            end: loc.end_offset(),
                            replacement: ")".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NestedParenthesizedCalls,
        "cops/style/nested_parenthesized_calls"
    );
    crate::cop_autocorrect_fixture_tests!(
        NestedParenthesizedCalls,
        "cops/style/nested_parenthesized_calls"
    );
}
