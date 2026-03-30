use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashLookupMethod;

impl Cop for HashLookupMethod {
    fn name(&self) -> &'static str {
        "Style/HashLookupMethod"
    }

    fn default_enabled(&self) -> bool {
        false
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

        let style = config.get_str("EnforcedStyle", "brackets");
        let method_bytes = call.name().as_slice();

        match style {
            "brackets" => {
                // Flag fetch calls, suggest []
                if method_bytes == b"fetch" {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        // Only flag fetch with exactly 1 argument (no default)
                        if arg_list.len() == 1 && call.block().is_none() {
                            let Some(receiver) = call.receiver() else {
                                return;
                            };

                            let loc = call.message_loc().unwrap_or_else(|| call.location());
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            let mut diag = self.diagnostic(
                                source,
                                line,
                                column,
                                "Use `[]` instead of `fetch`.".to_string(),
                            );

                            if let Some(ref mut corr) = corrections {
                                let receiver_src =
                                    std::str::from_utf8(receiver.location().as_slice())
                                        .unwrap_or("");
                                let key_src =
                                    std::str::from_utf8(arg_list[0].location().as_slice())
                                        .unwrap_or("");
                                if !receiver_src.is_empty() && !key_src.is_empty() {
                                    let mut replacement = format!("{receiver_src}[{key_src}]");
                                    if call
                                        .call_operator_loc()
                                        .is_some_and(|loc| loc.as_slice() == b"&.")
                                    {
                                        replacement = format!("({replacement})");
                                    }
                                    let call_loc = call.location();
                                    corr.push(crate::correction::Correction {
                                        start: call_loc.start_offset(),
                                        end: call_loc.end_offset(),
                                        replacement,
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
            "fetch" => {
                // Flag [] calls, suggest fetch
                if method_bytes == b"[]" {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            let Some(receiver) = call.receiver() else {
                                return;
                            };

                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            let mut diag = self.diagnostic(
                                source,
                                line,
                                column,
                                "Use `fetch` instead of `[]`.".to_string(),
                            );

                            if let Some(ref mut corr) = corrections {
                                let receiver_src =
                                    std::str::from_utf8(receiver.location().as_slice())
                                        .unwrap_or("");
                                let key_src =
                                    std::str::from_utf8(arg_list[0].location().as_slice())
                                        .unwrap_or("");
                                if !receiver_src.is_empty() && !key_src.is_empty() {
                                    let operator = if call
                                        .call_operator_loc()
                                        .is_some_and(|loc| loc.as_slice() == b"&.")
                                    {
                                        "&."
                                    } else {
                                        "."
                                    };
                                    let call_loc = call.location();
                                    corr.push(crate::correction::Correction {
                                        start: call_loc.start_offset(),
                                        end: call_loc.end_offset(),
                                        replacement: format!(
                                            "{receiver_src}{operator}fetch({key_src})"
                                        ),
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
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashLookupMethod, "cops/style/hash_lookup_method");
    crate::cop_autocorrect_fixture_tests!(HashLookupMethod, "cops/style/hash_lookup_method");
}
