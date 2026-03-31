use crate::cop::node_type::{CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DigChain;

impl Cop for DigChain {
    fn name(&self) -> &'static str {
        "Style/DigChain"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "dig" {
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

        // Check for hash/keyword hash args (not supported)
        for arg in &arg_list {
            if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                return;
            }
        }

        // Check if the receiver is also a dig call
        let receiver = match call.receiver() {
            Some(r) => r,
            None => {
                // No receiver - check if receiver-less dig is chained
                return;
            }
        };

        if let Some(recv_call) = receiver.as_call_node() {
            let recv_method = std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
            if recv_method == "dig" {
                // Check that inner dig also has arguments
                if let Some(inner_args) = recv_call.arguments() {
                    let inner_list: Vec<_> = inner_args.arguments().iter().collect();
                    if inner_list.is_empty() {
                        return;
                    }
                    // Check for hash/keyword hash args in inner call
                    for arg in &inner_list {
                        if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                            return;
                        }
                    }
                } else {
                    return;
                }

                // Only report if the receiver's receiver is NOT also a dig call.
                // This ensures we only fire once per chain (at the innermost pair),
                // avoiding duplicate reports for triple+ chains like dig.dig.dig.
                if let Some(inner_recv) = recv_call.receiver() {
                    if let Some(inner_recv_call) = inner_recv.as_call_node() {
                        let inner_recv_method =
                            std::str::from_utf8(inner_recv_call.name().as_slice()).unwrap_or("");
                        if inner_recv_method == "dig" {
                            return; // Let the innermost pair report
                        }
                    }
                }

                let loc = recv_call.message_loc().unwrap_or(recv_call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `dig` with multiple parameters instead of chaining.".to_string(),
                );

                if let Some(corr) = corrections.as_mut() {
                    if let Some(base_receiver) = recv_call.receiver() {
                        let base_loc = base_receiver.location();
                        let base_source = source
                            .byte_slice(base_loc.start_offset(), base_loc.end_offset(), "")
                            .to_string();

                        let mut combined_args = Vec::new();
                        if let Some(inner_args) = recv_call.arguments() {
                            for arg in inner_args.arguments().iter() {
                                let arg_loc = arg.location();
                                combined_args.push(
                                    source
                                        .byte_slice(
                                            arg_loc.start_offset(),
                                            arg_loc.end_offset(),
                                            "",
                                        )
                                        .to_string(),
                                );
                            }
                        }
                        for arg in &arg_list {
                            let arg_loc = arg.location();
                            combined_args.push(
                                source
                                    .byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "")
                                    .to_string(),
                            );
                        }

                        corr.push(crate::correction::Correction {
                            start: call.location().start_offset(),
                            end: call.location().end_offset(),
                            replacement: format!("{base_source}.dig({})", combined_args.join(", ")),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }

                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DigChain, "cops/style/dig_chain");
    crate::cop_autocorrect_fixture_tests!(DigChain, "cops/style/dig_chain");
}
