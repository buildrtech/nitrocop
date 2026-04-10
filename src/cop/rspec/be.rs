use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Be;

impl Cop for Be {
    fn name(&self) -> &'static str {
        "RSpec/Be"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        // Look for `expect(...).to be` / `expect(...).not_to be` / `expect(...).to_not be`
        // The `be` is a CallNode with receiver being another CallNode (`.to`/`.not_to`/`.to_not`)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check that the argument is a bare `be` call (no args, no chain)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let first_arg = &arg_list[0];
        let be_call = match first_arg.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if be_call.name().as_slice() != b"be" {
            return;
        }

        // Must have no receiver (standalone `be`, not `foo.be`)
        if be_call.receiver().is_some() {
            return;
        }

        // Must have no arguments
        if be_call.arguments().is_some() {
            return;
        }

        // `expect(...).to be { ... }` has a block and is allowed.
        if be_call.block().is_some() {
            return;
        }

        let loc = be_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Don't use `be` without an argument.".to_string(),
        );

        if let Some(corrections) = corrections
            && let Some(selector) = be_call.message_loc()
        {
            corrections.push(crate::correction::Correction {
                start: selector.start_offset(),
                end: selector.end_offset(),
                replacement: "be_truthy".to_string(),
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
    crate::cop_fixture_tests!(Be, "cops/rspec/be");

    #[test]
    fn supports_autocorrect() {
        assert!(Be.supports_autocorrect());
    }

    #[test]
    fn autocorrect_rewrites_be_to_be_truthy() {
        crate::testutil::assert_cop_autocorrect(
            &Be,
            b"it { expect(foo).to be }\n",
            b"it { expect(foo).to be_truthy }\n",
        );
    }
}
