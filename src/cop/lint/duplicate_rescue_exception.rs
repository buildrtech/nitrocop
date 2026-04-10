use std::collections::HashSet;

use crate::cop::node_type::BEGIN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicateRescueException;

impl Cop for DuplicateRescueException {
    fn name(&self) -> &'static str {
        "Lint/DuplicateRescueException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
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
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let mut seen = HashSet::new();
        let mut rescue_opt = begin_node.rescue_clause();

        while let Some(rescue_node) = rescue_opt {
            let exceptions: Vec<_> = rescue_node.exceptions().iter().collect();
            for (idx, exception) in exceptions.iter().enumerate() {
                let text = exception.location().as_slice().to_vec();
                if !seen.insert(text) {
                    let loc = exception.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicate `rescue` exception detected.".to_string(),
                    );

                    // Conservative baseline autocorrect: only remove duplicates that
                    // appear in multi-exception lists within the same rescue clause.
                    if exceptions.len() > 1
                        && let Some(corrections) = corrections.as_deref_mut()
                    {
                        let bytes = source.as_bytes();
                        let mut start = loc.start_offset();
                        let mut end = loc.end_offset();

                        if idx + 1 < exceptions.len() {
                            // remove current exception and trailing comma/space
                            let next_start = exceptions[idx + 1].location().start_offset();
                            end = next_start;
                        } else {
                            // remove leading comma/space before the last exception
                            while start > 0 && bytes[start - 1].is_ascii_whitespace() {
                                start -= 1;
                            }
                            if start > 0 && bytes[start - 1] == b',' {
                                start -= 1;
                                while start > 0 && bytes[start - 1].is_ascii_whitespace() {
                                    start -= 1;
                                }
                            }
                        }

                        if start < end {
                            corrections.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
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
    crate::cop_fixture_tests!(
        DuplicateRescueException,
        "cops/lint/duplicate_rescue_exception"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(DuplicateRescueException.supports_autocorrect());
    }

    #[test]
    fn autocorrect_removes_duplicate_exception_in_same_rescue_list() {
        crate::testutil::assert_cop_autocorrect(
            &DuplicateRescueException,
            b"begin\n  a\nrescue TypeError, TypeError\n  b\nend\n",
            b"begin\n  a\nrescue TypeError\n  b\nend\n",
        );
    }
}
