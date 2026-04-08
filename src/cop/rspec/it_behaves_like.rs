use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ItBehavesLike: Enforce `it_behaves_like` vs `it_should_behave_like` style.
/// Default prefers `it_behaves_like`.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=0, FN=12 (all from jruby).
///
/// FN=12: All FNs had receivers (e.g., `@state.it_should_behave_like @shared_desc`).
/// The cop was requiring `call.receiver().is_none()`, but vendor RuboCop uses
/// `(send _ % ...)` which matches any receiver. Removed the receiver check.
pub struct ItBehavesLike;

impl Cop for ItBehavesLike {
    fn name(&self) -> &'static str {
        "RSpec/ItBehavesLike"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Note: vendor RuboCop uses `(send _ % ...)` with `_` for any receiver,
        // so we match calls with or without a receiver.

        let name = call.name().as_slice();
        let style = config.get_str("EnforcedStyle", "it_behaves_like");

        let (bad_method, good_method) = if style == "it_should_behave_like" {
            (b"it_behaves_like" as &[u8], "it_should_behave_like")
        } else {
            (b"it_should_behave_like" as &[u8], "it_behaves_like")
        };

        if name != bad_method {
            return;
        }

        let bad_name = std::str::from_utf8(bad_method).unwrap_or("?");
        let loc = call.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Prefer `{}` over `{}` when including examples in a nested context.",
                good_method, bad_name
            ),
        );

        if let Some(ref mut corr) = corrections
            && let Some(selector_loc) = call.message_loc()
        {
            corr.push(crate::correction::Correction {
                start: selector_loc.start_offset(),
                end: selector_loc.end_offset(),
                replacement: good_method.to_string(),
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

    crate::cop_fixture_tests!(ItBehavesLike, "cops/rspec/it_behaves_like");
    crate::cop_autocorrect_fixture_tests!(ItBehavesLike, "cops/rspec/it_behaves_like");
}
