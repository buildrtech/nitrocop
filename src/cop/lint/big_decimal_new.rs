use crate::cop::node_type::CALL_NODE;
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BigDecimalNew;

impl Cop for BigDecimalNew {
    fn name(&self) -> &'static str {
        "Lint/BigDecimalNew"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        if call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let name = match constant_name(&receiver) {
            Some(n) => n,
            None => return,
        };

        if name != b"BigDecimal" {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "`BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            // Remove `.new`.
            let receiver_end = receiver.location().end_offset();
            let method_end = loc.end_offset();
            corr.push(crate::correction::Correction {
                start: receiver_end,
                end: method_end,
                replacement: "".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });

            // For `::BigDecimal.new(...)`, remove the leading `::` to match RuboCop.
            if let Some(cp) = receiver.as_constant_path_node() {
                if cp.parent().is_none() {
                    let recv_start = receiver.location().start_offset();
                    let src = source.as_bytes();
                    if src.get(recv_start) == Some(&b':') && src.get(recv_start + 1) == Some(&b':')
                    {
                        corr.push(crate::correction::Correction {
                            start: recv_start,
                            end: recv_start + 2,
                            replacement: "".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                }
            }

            diag.corrected = true;
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BigDecimalNew, "cops/lint/big_decimal_new");
    crate::cop_autocorrect_fixture_tests!(BigDecimalNew, "cops/lint/big_decimal_new");
}
