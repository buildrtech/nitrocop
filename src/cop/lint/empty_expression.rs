use crate::cop::node_type::{EMBEDDED_STATEMENTS_NODE, PARENTHESES_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for empty expressions: `()`, empty `begin..end`, and empty
/// string interpolation `#{}`.
///
/// ## Investigation (2026-03-18)
/// FN=5 were all empty `#{}` interpolation in strings/heredocs/backticks.
/// In RuboCop (parser gem), `#{}` produces `(dstr (begin))` where the inner
/// `(begin)` node triggers `on_begin`. In Prism, `#{}` becomes an
/// `EmbeddedStatementsNode` with no statements — added handling for this
/// node type to match RuboCop behavior.
pub struct EmptyExpression;

impl Cop for EmptyExpression {
    fn name(&self) -> &'static str {
        "Lint/EmptyExpression"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PARENTHESES_NODE, EMBEDDED_STATEMENTS_NODE]
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

        // Empty parentheses: ()
        if let Some(parens) = node.as_parentheses_node() {
            if parens.body().is_none() {
                let loc = parens.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic =
                    self.diagnostic(source, line, column, "Avoid empty expressions.".to_string());

                if let Some(corrections) = corrections.as_deref_mut() {
                    corrections.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: "nil".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
            
            return;
        }

        // Empty string interpolation: #{}
        if let Some(embedded) = node.as_embedded_statements_node() {
            if embedded.statements().is_none() {
                let loc = embedded.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid empty expressions.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyExpression, "cops/lint/empty_expression");

    #[test]
    fn supports_autocorrect() {
        assert!(EmptyExpression.supports_autocorrect());
    }

    #[test]
    fn autocorrect_empty_parentheses_to_nil() {
        crate::testutil::assert_cop_autocorrect(&EmptyExpression, b"x = ()\n", b"x = nil\n");
    }
}
