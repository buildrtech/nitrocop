use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for comparison chains like `x < y < z`.
///
/// ## FP fix (2026-03): Skip set operations as center value
/// RuboCop skips flagging when the center value (RHS of the inner comparison)
/// is a set operation (`&`, `|`, `^`). Due to Ruby operator precedence,
/// `x >= y & z < w` parses as `(x >= (y & z)) < w`. The center value `(y & z)`
/// uses set operation `&`, so RuboCop does not flag it.
pub struct MultipleComparison;

impl Cop for MultipleComparison {
    fn name(&self) -> &'static str {
        "Lint/MultipleComparison"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // Pattern: (send (send _ COMP _) COMP _)
        // i.e., x < y < z
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();
        if !is_comparison(outer_method) {
            return;
        }

        // The receiver of the outer call should itself be a comparison call
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner_method = inner_call.name().as_slice();
        if !is_comparison(inner_method) {
            return;
        }

        // Check if the center value (RHS of inner comparison) is a set operation.
        // Due to Ruby operator precedence, `x >= y & z < w` parses as
        // `(x >= (y & z)) < w`. RuboCop skips these cases.
        if let Some(inner_args) = inner_call.arguments() {
            let args = inner_args.arguments();
            if args.len() == 1 {
                if let Some(center_call) = args.iter().next().and_then(|a| a.as_call_node()) {
                    let center_method = center_call.name().as_slice();
                    if is_set_operation(center_method) {
                        return;
                    }
                }
            }
        }

        let loc = outer_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Use the `&&` operator to compare multiple values.".to_string(),
        );

        if let Some(corr) = corrections.as_mut()
            && let Some(inner_receiver) = inner_call.receiver()
            && let Some(inner_args) = inner_call.arguments()
            && let Some(outer_args) = outer_call.arguments()
        {
            let inner_arg_list: Vec<_> = inner_args.arguments().iter().collect();
            let outer_arg_list: Vec<_> = outer_args.arguments().iter().collect();
            if inner_arg_list.len() == 1 && outer_arg_list.len() == 1 {
                let lhs_loc = inner_receiver.location();
                let center_loc = inner_arg_list[0].location();
                let rhs_loc = outer_arg_list[0].location();
                let lhs = String::from_utf8_lossy(
                    &source.as_bytes()[lhs_loc.start_offset()..lhs_loc.end_offset()],
                );
                let center = String::from_utf8_lossy(
                    &source.as_bytes()[center_loc.start_offset()..center_loc.end_offset()],
                );
                let rhs = String::from_utf8_lossy(
                    &source.as_bytes()[rhs_loc.start_offset()..rhs_loc.end_offset()],
                );
                let inner_op = std::str::from_utf8(inner_method).unwrap_or("<");
                let outer_op = std::str::from_utf8(outer_method).unwrap_or("<");

                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: format!(
                        "{lhs} {inner_op} {center} && {center} {outer_op} {rhs}"
                    ),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }

        diagnostics.push(diag);
    }
}

fn is_comparison(method: &[u8]) -> bool {
    matches!(method, b"<" | b">" | b"<=" | b">=")
}

fn is_set_operation(method: &[u8]) -> bool {
    matches!(method, b"&" | b"|" | b"^")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleComparison, "cops/lint/multiple_comparison");
    crate::cop_autocorrect_fixture_tests!(MultipleComparison, "cops/lint/multiple_comparison");
}
