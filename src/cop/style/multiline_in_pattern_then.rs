use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineInPatternThen;

impl Cop for MultilineInPatternThen {
    fn name(&self) -> &'static str {
        "Style/MultilineInPatternThen"
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
        let mut visitor = MultilineInPatternThenVisitor {
            source,
            offenses: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        let src = source.as_bytes();
        for (then_start, then_end) in visitor.offenses {
            let (line, column) = source.offset_to_line_col(then_start);
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Do not use `then` for multi-line `in` statement.".to_string(),
            );

            if let Some(ref mut corr) = corrections {
                let mut start = then_start;
                if start > 0 && src[start - 1] == b' ' {
                    start -= 1;
                }

                corr.push(crate::correction::Correction {
                    start,
                    end: then_end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }

            diagnostics.push(diag);
        }
    }
}

struct MultilineInPatternThenVisitor<'a> {
    source: &'a SourceFile,
    offenses: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for MultilineInPatternThenVisitor<'_> {
    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        // Check if `then` keyword is used in a multi-line `in` pattern
        if let Some(then_loc) = node.then_loc() {
            if then_loc.as_slice() == b"then" {
                // Check if the pattern and body span multiple lines
                let pattern_loc = node.pattern().location();
                let (pattern_line, _) = self.source.offset_to_line_col(pattern_loc.start_offset());

                if let Some(stmts) = node.statements() {
                    let body_loc = stmts.location();
                    let (body_line, _) = self.source.offset_to_line_col(body_loc.start_offset());

                    if body_line > pattern_line {
                        self.offenses
                            .push((then_loc.start_offset(), then_loc.end_offset()));
                    }
                }
            }
        }

        // Visit children
        self.visit(&node.pattern());
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultilineInPatternThen,
        "cops/style/multiline_in_pattern_then"
    );
    crate::cop_autocorrect_fixture_tests!(
        MultilineInPatternThen,
        "cops/style/multiline_in_pattern_then"
    );
}
