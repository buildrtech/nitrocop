use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use regex::Regex;
use std::sync::LazyLock;

static ASSERT_NOT_AUTOCORRECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^assert(\(| ) *! *").unwrap());

pub struct AssertNot;

impl Cop for AssertNot {
    fn name(&self) -> &'static str {
        "Rails/AssertNot"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/test/**/*"]
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

        // Must be a receiverless `assert` call
        if call.receiver().is_some() || call.name().as_slice() != b"assert" {
            return;
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a `!` (negation) call
        let first_arg = &arg_list[0];
        let neg_call = match first_arg.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if neg_call.name().as_slice() != b"!" {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Prefer `assert_not` over `assert !`.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let call_src = source.byte_slice(loc.start_offset(), loc.end_offset(), "");
            let replacement = ASSERT_NOT_AUTOCORRECT_RE
                .replace(call_src, "assert_not$1")
                .to_string();
            if !replacement.is_empty() && replacement != call_src {
                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AssertNot, "cops/rails/assert_not");
    crate::cop_autocorrect_fixture_tests!(AssertNot, "cops/rails/assert_not");
}
