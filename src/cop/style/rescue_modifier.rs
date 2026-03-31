use crate::cop::node_type::RESCUE_MODIFIER_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RescueModifier;

impl Cop for RescueModifier {
    fn name(&self) -> &'static str {
        "Style/RescueModifier"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[RESCUE_MODIFIER_NODE]
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
        let rescue_mod = match node.as_rescue_modifier_node() {
            Some(r) => r,
            None => return,
        };

        // RuboCop points at the whole rescue modifier expression, not just the `rescue` keyword
        let loc = rescue_mod.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Avoid rescuing without specifying an error class.".to_string(),
        );

        if let Some(corr) = corrections.as_mut() {
            let indent = " ".repeat(column);
            let body_indent = format!("{indent}  ");
            let expr = source
                .byte_slice(
                    rescue_mod.expression().location().start_offset(),
                    rescue_mod.expression().location().end_offset(),
                    "",
                )
                .to_string();
            let fallback = source
                .byte_slice(
                    rescue_mod.rescue_expression().location().start_offset(),
                    rescue_mod.rescue_expression().location().end_offset(),
                    "",
                )
                .to_string();

            let replacement = format!(
                "begin\n{body_indent}{expr}\n{indent}rescue\n{body_indent}{fallback}\n{indent}end"
            );

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(RescueModifier, "cops/style/rescue_modifier");
    crate::cop_autocorrect_fixture_tests!(RescueModifier, "cops/style/rescue_modifier");

    #[test]
    fn inline_rescue_fires() {
        let source = b"x = foo rescue nil\n";
        let diags = run_cop_full(&RescueModifier, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Avoid rescuing"));
    }
}
