use crate::cop::node_type::FLIP_FLOP_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FlipFlop;

impl Cop for FlipFlop {
    fn name(&self) -> &'static str {
        "Lint/FlipFlop"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[FLIP_FLOP_NODE]
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
        let flip_flop = match node.as_flip_flop_node() {
            Some(n) => n,
            None => return,
        };

        let loc = flip_flop.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of flip-flop operators.".to_string(),
        );
        if let Some(corrections) = corrections {
            corrections.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: "false".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FlipFlop, "cops/lint/flip_flop");

    #[test]
    fn autocorrect_rewrites_flip_flop_expression_to_false() {
        crate::testutil::assert_cop_autocorrect(
            &FlipFlop,
            b"puts x if (x == 5) .. (x == 10)\n",
            b"puts x if false\n",
        );
    }
}
