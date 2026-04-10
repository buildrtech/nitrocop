use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RequireRangeParentheses;

impl Cop for RequireRangeParentheses {
    fn name(&self) -> &'static str {
        "Lint/RequireRangeParentheses"
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
        let mut visitor = RangeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
            in_parens: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RangeVisitor<'a, 'src> {
    cop: &'a RequireRangeParentheses,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
    in_parens: bool,
}

impl<'pr> Visit<'pr> for RangeVisitor<'_, '_> {
    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        let old = self.in_parens;
        self.in_parens = true;
        ruby_prism::visit_parentheses_node(self, node);
        self.in_parens = old;
    }

    fn visit_range_node(&mut self, node: &ruby_prism::RangeNode<'pr>) {
        // Skip ranges inside parentheses
        if self.in_parens {
            ruby_prism::visit_range_node(self, node);
            return;
        }

        let left = match node.left() {
            Some(l) => l,
            None => {
                ruby_prism::visit_range_node(self, node);
                return;
            }
        };
        let right = match node.right() {
            Some(r) => r,
            None => {
                ruby_prism::visit_range_node(self, node);
                return;
            }
        };

        // Check if operator and right side are on different lines
        let operator_loc = node.operator_loc();
        let op_end = operator_loc.start_offset() + operator_loc.as_slice().len();
        let right_start = right.location().start_offset();

        let (op_line, _) = self.source.offset_to_line_col(op_end);
        let (right_line, _) = self.source.offset_to_line_col(right_start);

        if op_line != right_line {
            let left_src = std::str::from_utf8(left.location().as_slice()).unwrap_or("...");
            let op_src = std::str::from_utf8(operator_loc.as_slice()).unwrap_or("..");

            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "Wrap the endless range literal `{left_src}{op_src}` to avoid precedence ambiguity."
                ),
            );

            if let Some(corrections) = self.corrections.as_deref_mut() {
                corrections.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: op_end,
                    replacement: format!("({left_src}{op_src})"),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            self.diagnostics.push(diagnostic);
        }

        ruby_prism::visit_range_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RequireRangeParentheses,
        "cops/lint/require_range_parentheses"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(RequireRangeParentheses.supports_autocorrect());
    }

    #[test]
    fn autocorrect_wraps_operator_prefix_as_endless_range() {
        crate::testutil::assert_cop_autocorrect(
            &RequireRangeParentheses,
            b"1..\n42\n",
            b"(1..)\n42\n",
        );
    }
}
