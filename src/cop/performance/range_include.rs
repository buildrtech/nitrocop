use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, RANGE_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/RangeInclude - flags `Range#include?` and `Range#member?`, suggesting `Range#cover?`.
///
/// Investigation: 15 FNs were all `.member?()` calls on ranges. The cop originally only checked
/// for `include?`. Ruby's `Range#member?` is an alias for `Range#include?` and both should be
/// flagged. Fix: check for both method names and use the correct method name in the message.
pub struct RangeInclude;

impl Cop for RangeInclude {
    fn name(&self) -> &'static str {
        "Performance/RangeInclude"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, RANGE_NODE, STATEMENTS_NODE]
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

        let method_name = call.name().as_slice();
        if method_name != b"include?" && method_name != b"member?" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver is a RangeNode directly or wrapped in parentheses
        let is_range = receiver.as_range_node().is_some()
            || receiver
                .as_parentheses_node()
                .and_then(|p| p.body())
                .and_then(|b| {
                    // The body of parentheses is a StatementsNode
                    let stmts = b.as_statements_node()?;
                    let body = stmts.body();
                    if body.len() == 1 {
                        body.iter().next()?.as_range_node()
                    } else {
                        None
                    }
                })
                .is_some();

        if !is_range {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let method = if method_name == b"member?" {
            "member?"
        } else {
            "include?"
        };
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `Range#cover?` instead of `Range#{method}`."),
        );

        if let Some(ref mut corr) = corrections
            && let Some(selector_loc) = call.message_loc()
        {
            corr.push(crate::correction::Correction {
                start: selector_loc.start_offset(),
                end: selector_loc.end_offset(),
                replacement: "cover?".to_string(),
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
    crate::cop_fixture_tests!(RangeInclude, "cops/performance/range_include");
    crate::cop_autocorrect_fixture_tests!(RangeInclude, "cops/performance/range_include");

    #[test]
    fn supports_autocorrect() {
        assert!(RangeInclude.supports_autocorrect());
    }
}
