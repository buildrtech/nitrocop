use crate::cop::node_type::{
    LOCAL_VARIABLE_WRITE_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_PARAMETER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ItAssignment;

const MSG: &str = "Avoid assigning to local variable `it`, since `it` will be the default block parameter in Ruby 3.4+. Consider using a different variable name.";

impl Cop for ItAssignment {
    fn name(&self) -> &'static str {
        "Style/ItAssignment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            LOCAL_VARIABLE_WRITE_NODE,
            REQUIRED_PARAMETER_NODE,
            OPTIONAL_PARAMETER_NODE,
        ]
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
        let name_bytes: &[u8];
        let start_offset: usize;
        let end_offset: usize;
        let mut corrections = corrections;

        if let Some(w) = node.as_local_variable_write_node() {
            name_bytes = w.name().as_slice();
            start_offset = w.name_loc().start_offset();
            end_offset = w.name_loc().end_offset();
        } else if let Some(p) = node.as_required_parameter_node() {
            name_bytes = p.name().as_slice();
            start_offset = p.location().start_offset();
            end_offset = p.location().end_offset();
        } else if let Some(p) = node.as_optional_parameter_node() {
            name_bytes = p.name().as_slice();
            start_offset = p.name_loc().start_offset();
            end_offset = p.name_loc().end_offset();
        } else {
            return;
        }

        if name_bytes != b"it" {
            return;
        }

        let (line, column) = source.offset_to_line_col(start_offset);
        let mut diagnostic = self.diagnostic(source, line, column, MSG.to_string());
        if let Some(corrections) = corrections.as_deref_mut() {
            corrections.push(crate::correction::Correction {
                start: start_offset,
                end: end_offset,
                replacement: "it_var".to_string(),
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
    crate::cop_fixture_tests!(ItAssignment, "cops/style/it_assignment");

    #[test]
    fn autocorrect_renames_it_assignment() {
        crate::testutil::assert_cop_autocorrect(&ItAssignment, b"it = 1\n", b"it_var = 1\n");
    }
}
