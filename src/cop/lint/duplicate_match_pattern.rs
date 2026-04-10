use ruby_prism::Visit;
use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that there are no repeated patterns in `case...in` expressions.
pub struct DuplicateMatchPattern;

impl Cop for DuplicateMatchPattern {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMatchPattern"
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
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = MatchVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(c) = corrections {
            c.extend(visitor.corrections);
        }
    }
}

struct MatchVisitor<'a, 'src> {
    cop: &'a DuplicateMatchPattern,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
}

impl<'pr> Visit<'pr> for MatchVisitor<'_, '_> {
    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        let mut seen = HashSet::new();

        for clause in node.conditions().iter() {
            if let Some(in_node) = clause.as_in_node() {
                let pattern = in_node.pattern();
                let pattern_src = &self.source.as_bytes()
                    [pattern.location().start_offset()..pattern.location().end_offset()];
                let key = pattern_src.to_vec();

                if !seen.insert(key) {
                    let loc = pattern.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Duplicate `in` pattern detected.".to_string(),
                    );

                    let mut start = in_node.location().start_offset();
                    if start > 0 && self.source.as_bytes()[start - 1] == b'\n' {
                        start -= 1;
                    }
                    self.corrections.push(crate::correction::Correction {
                        start,
                        end: in_node.location().end_offset(),
                        replacement: String::new(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;

                    self.diagnostics.push(diagnostic);
                }
            }
        }

        ruby_prism::visit_case_match_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMatchPattern, "cops/lint/duplicate_match_pattern");

    #[test]
    fn supports_autocorrect() {
        assert!(DuplicateMatchPattern.supports_autocorrect());
    }

    #[test]
    fn autocorrect_removes_duplicate_in_branch() {
        crate::testutil::assert_cop_autocorrect(
            &DuplicateMatchPattern,
            b"case x\nin 'first'\n  do_something\nin 'first'\n  do_something_else\nend\n",
            b"case x\nin 'first'\n  do_something\nend\n",
        );
    }
}
