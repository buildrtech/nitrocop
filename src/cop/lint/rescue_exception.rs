use crate::cop::node_type::{BEGIN_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation: FP=1 was caused by the directive parser rejecting
/// `# # rubocop:disable Lint/RescueException` as a YARD doc nested comment.
/// The double-# pattern is legitimate when inline (code before the comment).
/// Fixed in the directive parser by skipping the YARD rejection for inline comments.
pub struct RescueException;

impl Cop for RescueException {
    fn name(&self) -> &'static str {
        "Lint/RescueException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        // Match BeginNode to get rescue_clause
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let mut rescue_opt = begin_node.rescue_clause();

        while let Some(rescue_node) = rescue_opt {
            for exception in rescue_node.exceptions().iter() {
                let is_exception = if let Some(const_read) = exception.as_constant_read_node() {
                    // Bare `Exception`
                    const_read.name().as_slice() == b"Exception"
                } else if let Some(const_path) = exception.as_constant_path_node() {
                    // Only match `::Exception` (top-level), not `Gem::Exception` etc.
                    const_path.parent().is_none()
                        && const_path
                            .name()
                            .is_some_and(|n| n.as_slice() == b"Exception")
                } else {
                    false
                };

                if is_exception {
                    // Point at the `rescue` keyword, matching RuboCop's resbody node location
                    let loc = rescue_node.keyword_loc();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid rescuing the `Exception` class. Perhaps you meant `StandardError`?"
                            .to_string(),
                    );

                    if let Some(corrections) = corrections.as_deref_mut() {
                        let exception_loc = exception.location();
                        corrections.push(crate::correction::Correction {
                            start: exception_loc.start_offset(),
                            end: exception_loc.end_offset(),
                            replacement: "StandardError".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }

                    diagnostics.push(diagnostic);
                }
            }
            rescue_opt = rescue_node.subsequent();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RescueException, "cops/lint/rescue_exception");

    #[test]
    fn supports_autocorrect() {
        assert!(RescueException.supports_autocorrect());
    }

    #[test]
    fn autocorrect_replaces_exception_with_standard_error() {
        crate::testutil::assert_cop_autocorrect(
            &RescueException,
            b"begin\n  foo\nrescue Exception\n  bar\nend\n",
            b"begin\n  foo\nrescue StandardError\n  bar\nend\n",
        );
    }
}
