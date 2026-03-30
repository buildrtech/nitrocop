use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LambdaCall;

impl Cop for LambdaCall {
    fn name(&self) -> &'static str {
        "Style/LambdaCall"
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

        let Some(receiver) = call.receiver() else {
            return;
        };

        let enforced_style = config.get_str("EnforcedStyle", "call");

        match enforced_style {
            "call" => {
                let name = call.name();
                if name.as_slice() != b"call" {
                    return;
                }

                // Explicit call already good.
                if call
                    .message_loc()
                    .is_some_and(|loc| loc.as_slice() == b"call")
                {
                    return;
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer the use of `lambda.call(...)` over `lambda.(...)`.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let receiver_src =
                        std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
                    let operator = call
                        .call_operator_loc()
                        .and_then(|op| std::str::from_utf8(op.as_slice()).ok())
                        .unwrap_or(".");
                    if !receiver_src.is_empty() {
                        let args: Vec<String> = call
                            .arguments()
                            .map(|a| {
                                a.arguments()
                                    .iter()
                                    .filter_map(|arg| {
                                        std::str::from_utf8(arg.location().as_slice())
                                            .ok()
                                            .map(ToOwned::to_owned)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        let replacement = if args.is_empty() {
                            format!("{receiver_src}{operator}call")
                        } else {
                            format!("{receiver_src}{operator}call({})", args.join(", "))
                        };
                        corr.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: loc.end_offset(),
                            replacement,
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }

                diagnostics.push(diag);
            }
            "braces" => {
                let name = call.name();
                if name.as_slice() != b"call" {
                    return;
                }

                let msg_loc = match call.message_loc() {
                    Some(loc) => loc,
                    None => return, // Already implicit
                };

                if msg_loc.as_slice() != b"call" {
                    return;
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer the use of `lambda.(...)` over `lambda.call(...)`.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let receiver_src =
                        std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
                    let operator = call
                        .call_operator_loc()
                        .and_then(|op| std::str::from_utf8(op.as_slice()).ok())
                        .unwrap_or(".");
                    if !receiver_src.is_empty() {
                        let args: Vec<String> = call
                            .arguments()
                            .map(|a| {
                                a.arguments()
                                    .iter()
                                    .filter_map(|arg| {
                                        std::str::from_utf8(arg.location().as_slice())
                                            .ok()
                                            .map(ToOwned::to_owned)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        let replacement = if args.is_empty() {
                            format!("{receiver_src}{operator}()")
                        } else {
                            format!("{receiver_src}{operator}({})", args.join(", "))
                        };
                        corr.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: loc.end_offset(),
                            replacement,
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }

                diagnostics.push(diag);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LambdaCall, "cops/style/lambda_call");
    crate::cop_autocorrect_fixture_tests!(LambdaCall, "cops/style/lambda_call");
}
