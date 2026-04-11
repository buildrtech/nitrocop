use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Inquiry;

impl Cop for Inquiry {
    fn name(&self) -> &'static str {
        "Rails/Inquiry"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"inquiry" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // RuboCop only flags inquiry when the receiver is a string literal or array literal.
        // Method call results (e.g., `ROLES.key(flag)&.inquiry`) are not flagged.
        let is_string_or_array = receiver.as_string_node().is_some()
            || receiver.as_interpolated_string_node().is_some()
            || receiver.as_array_node().is_some();
        if !is_string_or_array {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Avoid `String#inquiry`. Use direct comparison or predicate methods.".to_string(),
        );
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
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Inquiry, "cops/rails/inquiry");

    #[test]
    fn autocorrect_replaces_string_inquiry_with_nil() {
        crate::testutil::assert_cop_autocorrect(&Inquiry, b"'prod'.inquiry\n", b"nil\n");
    }
}
