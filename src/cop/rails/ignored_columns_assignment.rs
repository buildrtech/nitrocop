use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IgnoredColumnsAssignment;

impl Cop for IgnoredColumnsAssignment {
    fn name(&self) -> &'static str {
        "Rails/IgnoredColumnsAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `ignored_columns=` method
        if call.name().as_slice() != b"ignored_columns=" {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(source, line, column, "Use `+=` instead of `=`.".to_string());

        if let Some(ref mut corr) = corrections
            && let Some(args) = call.arguments()
            && let Some(first_arg) = args.arguments().iter().next()
        {
            let search_start = loc.end_offset();
            let search_end = first_arg.location().start_offset();
            if search_start < search_end {
                let haystack = &source.as_bytes()[search_start..search_end];
                if let Some(eq_idx) = haystack.iter().position(|b| *b == b'=') {
                    let eq_offset = search_start + eq_idx;
                    corr.push(crate::correction::Correction {
                        start: eq_offset,
                        end: eq_offset + 1,
                        replacement: "+=".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IgnoredColumnsAssignment,
        "cops/rails/ignored_columns_assignment"
    );

    #[test]
    fn autocorrects_assignment_to_plus_equals() {
        crate::testutil::assert_cop_autocorrect(
            &IgnoredColumnsAssignment,
            b"self.ignored_columns = [:one]\n",
            b"self.ignored_columns += [:one]\n",
        );
    }

    #[test]
    fn autocorrects_compact_assignment_spacing() {
        crate::testutil::assert_cop_autocorrect(
            &IgnoredColumnsAssignment,
            b"self.ignored_columns=[:one]\n",
            b"self.ignored_columns+=[:one]\n",
        );
    }
}
