use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN fix:
/// - RuboCop also flags calls without parentheses when the first argument is a
///   ternary whose condition uses `&&` or `||` (for example
///   `puts ready && synced ? "ok" : "missing"`). The initial implementation
///   only handled predicate methods with boolean operator arguments and missed
///   the ternary-first-argument path entirely.
pub struct RequireParentheses;

fn is_ternary(if_node: &ruby_prism::IfNode<'_>) -> bool {
    if_node.if_keyword_loc().is_none()
}

fn is_assignment_method(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    name.ends_with(b"=") && name != b"=="
}

fn build_parenthesized_call_rewrite(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    cop_name: &'static str,
) -> Option<crate::correction::Correction> {
    if call.block().is_some() {
        return None;
    }
    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;
    let args_start = first_arg.location().start_offset();
    let args_end = args.location().end_offset();

    let call_start = call.location().start_offset();
    let prefix = source.try_byte_slice(call_start, args_start)?;
    let args_src = source.try_byte_slice(args_start, args_end)?;

    Some(crate::correction::Correction {
        start: call_start,
        end: call.location().end_offset(),
        replacement: format!("{}({})", prefix.trim_end(), args_src),
        cop_name,
        cop_index: 0,
    })
}

impl Cop for RequireParentheses {
    fn name(&self) -> &'static str {
        "Lint/RequireParentheses"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, OR_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut corrections = corrections;
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Must NOT have parentheses
        if call.opening_loc().is_some() {
            return;
        }

        if let Some(first_arg) = args.arguments().iter().next() {
            if let Some(ternary) = first_arg.as_if_node() {
                let condition = ternary.predicate();
                if is_ternary(&ternary)
                    && !is_assignment_method(&call)
                    && call.name().as_slice() != b"[]"
                    && (condition.as_and_node().is_some() || condition.as_or_node().is_some())
                {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Use parentheses in the method call to avoid confusion about precedence."
                            .to_string(),
                    );
                    if let Some(corrections) = corrections.as_deref_mut()
                        && let Some(correction) =
                            build_parenthesized_call_rewrite(source, &call, self.name())
                    {
                        corrections.push(correction);
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                    return;
                }
            }
        }

        let name = call.name();
        if !name.as_slice().ends_with(b"?") {
            return;
        }

        let has_boolean_arg = args
            .arguments()
            .iter()
            .any(|arg| arg.as_and_node().is_some() || arg.as_or_node().is_some());

        if has_boolean_arg {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Use parentheses in the method call to avoid confusion about precedence."
                    .to_string(),
            );
            if let Some(corrections) = corrections.as_deref_mut()
                && let Some(correction) =
                    build_parenthesized_call_rewrite(source, &call, self.name())
            {
                corrections.push(correction);
                diagnostic.corrected = true;
            }
            diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireParentheses, "cops/lint/require_parentheses");

    #[test]
    fn supports_autocorrect() {
        assert!(RequireParentheses.supports_autocorrect());
    }

    #[test]
    fn autocorrect_wraps_predicate_call_args() {
        crate::testutil::assert_cop_autocorrect(
            &RequireParentheses,
            b"day_is? 'tuesday' || true\n",
            b"day_is?('tuesday' || true)\n",
        );
    }
}
