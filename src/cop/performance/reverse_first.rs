use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-22)
///
/// Extended corpus reported FP=1, FN=0.
///
/// FP=1: `.reverse(:retired_at_epoch_ms).first` in archivesspace repo.
/// Sequel's `reverse(:column)` is an ordering method (ORDER BY ... DESC),
/// not Array#reverse. RuboCop's NodePattern `(call $(call _ :reverse) :first ...)`
/// requires `.reverse` with no arguments. Fixed by checking
/// `chain.inner_call.arguments().is_none()`.
pub struct ReverseFirst;

impl Cop for ReverseFirst {
    fn name(&self) -> &'static str {
        "Performance/ReverseFirst"
    }

    fn uses_node_check(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"reverse" || chain.outer_method != b"first" {
            return;
        }

        // RuboCop's pattern `(call $(call _ :reverse) :first ...)` requires
        // reverse with NO arguments. `.reverse(:column)` is Sequel's ordering
        // method, not Array#reverse.
        if chain.inner_call.arguments().is_some() {
            return;
        }

        // RuboCop's NodePattern: (call $(call _ :reverse) :first (int _)?)
        // Only flag when first's argument is absent or an integer literal.
        let outer_call = node.as_call_node().unwrap();
        if let Some(args) = outer_call.arguments() {
            if let Some(first_arg) = args.arguments().iter().next() {
                if first_arg.as_integer_node().is_none() {
                    return;
                }
            }
        }

        // Report at the inner call's selector (.reverse), matching RuboCop's
        // `receiver.loc.selector.begin_pos`
        let inner_msg_loc = chain
            .inner_call
            .message_loc()
            .unwrap_or(chain.inner_call.location());
        let (line, column) = source.offset_to_line_col(inner_msg_loc.start_offset());

        let good_method = if let Some(args) = outer_call.arguments() {
            if let Some(first_arg) = args.arguments().iter().next() {
                let arg_text = std::str::from_utf8(first_arg.location().as_slice()).unwrap_or("n");
                let dot = match outer_call.call_operator_loc() {
                    Some(loc) if loc.as_slice() == b"&." => "&.",
                    _ => ".",
                };
                format!("last({arg_text}){dot}reverse")
            } else {
                "last".to_string()
            }
        } else {
            "last".to_string()
        };

        let bad_method_start = inner_msg_loc.start_offset();
        let bad_method_end = outer_call.location().end_offset();
        let bad_method = source
            .byte_slice(bad_method_start, bad_method_end, "reverse.first")
            .to_string();

        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `{good_method}` instead of `{bad_method}`."),
        );

        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: bad_method_start,
                end: bad_method_end,
                replacement: good_method,
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

    crate::cop_fixture_tests!(ReverseFirst, "cops/performance/reverse_first");
    crate::cop_autocorrect_fixture_tests!(ReverseFirst, "cops/performance/reverse_first");
}
