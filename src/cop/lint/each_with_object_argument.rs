use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EachWithObjectArgument;

fn is_immutable_literal(node: &ruby_prism::Node<'_>) -> bool {
    matches!(
        node,
        ruby_prism::Node::IntegerNode { .. }
            | ruby_prism::Node::FloatNode { .. }
            | ruby_prism::Node::SymbolNode { .. }
    )
}

impl Cop for EachWithObjectArgument {
    fn name(&self) -> &'static str {
        "Lint/EachWithObjectArgument"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"each_with_object" {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        let first_arg = match args.first() {
            Some(a) => a,
            None => return,
        };

        if !is_immutable_literal(&first_arg) {
            return;
        }

        let loc = first_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "`each_with_object` called with an immutable argument.".to_string(),
        );
        if let Some(corrections) = corrections {
            corrections.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: "[]".to_string(),
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
    crate::cop_fixture_tests!(
        EachWithObjectArgument,
        "cops/lint/each_with_object_argument"
    );

    #[test]
    fn autocorrect_replaces_immutable_object_with_array_literal() {
        crate::testutil::assert_cop_autocorrect(
            &EachWithObjectArgument,
            b"[1, 2, 3].each_with_object(0) { |x, sum| sum + x }\n",
            b"[1, 2, 3].each_with_object([]) { |x, sum| sum + x }\n",
        );
    }
}
