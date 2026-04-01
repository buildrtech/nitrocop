use crate::cop::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus conformance fix: RuboCop's NodePattern for format/sprintf is
/// `(send nil? :format _ _ ...)` — the `nil?` means it only matches bare calls
/// with no receiver. Previously nitrocop also matched `Kernel.format(...)` and
/// `Kernel.sprintf(...)` via `is_kernel_constant()`, causing 13 FPs in jruby and
/// natalie corpus repos. Fixed by requiring `receiver().is_none()` for format/sprintf.
pub struct FormatString;

const AUTOCORRECTABLE_METHODS: &[&[u8]] = &[
    b"to_d", b"to_f", b"to_h", b"to_i", b"to_r", b"to_s", b"to_sym",
];
const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"<<",
    b">>", b"|", b"&", b"^",
];

fn node_source(node: &ruby_prism::Node<'_>) -> String {
    String::from_utf8_lossy(node.location().as_slice()).into_owned()
}

fn percent_rhs_is_uncorrectable(rhs: &ruby_prism::Node<'_>) -> bool {
    if rhs.as_local_variable_read_node().is_some() {
        return true;
    }

    if let Some(call) = rhs.as_call_node() {
        return !AUTOCORRECTABLE_METHODS.contains(&call.name().as_slice());
    }

    false
}

fn is_operator_call_without_parens(node: &ruby_prism::Node<'_>) -> bool {
    let Some(call) = node.as_call_node() else {
        return false;
    };

    OPERATOR_METHODS.contains(&call.name().as_slice()) && call.opening_loc().is_none()
}

impl Cop for FormatString {
    fn name(&self) -> &'static str {
        "Style/FormatString"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
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

        let method_bytes = call.name().as_slice();
        let style = config.get_str("EnforcedStyle", "format");

        match method_bytes {
            b"%" => {
                if style == "percent" {
                    return;
                }

                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };

                let is_string_receiver = receiver.as_string_node().is_some()
                    || receiver.as_interpolated_string_node().is_some();

                if !is_string_receiver {
                    let has_array_or_hash_arg = call.arguments().is_some_and(|args| {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        arg_list.len() == 1
                            && (arg_list[0].as_array_node().is_some()
                                || arg_list[0].as_hash_node().is_some()
                                || arg_list[0].as_keyword_hash_node().is_some())
                    });
                    if !has_array_or_hash_arg {
                        return;
                    }
                }

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "format" {
                    "format"
                } else {
                    "sprintf"
                };
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `String#%`.", preferred),
                );

                if let (Some(args), Some(corr)) = (call.arguments(), corrections.as_mut()) {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        let rhs = &arg_list[0];
                        if !percent_rhs_is_uncorrectable(rhs) {
                            let rhs_src = if let Some(array) = rhs.as_array_node() {
                                array
                                    .elements()
                                    .iter()
                                    .map(|n| node_source(&n))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else if let Some(hash) = rhs.as_hash_node() {
                                hash.elements()
                                    .iter()
                                    .map(|n| node_source(&n))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else if let Some(hash) = rhs.as_keyword_hash_node() {
                                hash.elements()
                                    .iter()
                                    .map(|n| node_source(&n))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else {
                                node_source(rhs)
                            };

                            let replacement =
                                format!("{}({}, {})", preferred, node_source(&receiver), rhs_src);
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
                }

                diagnostics.push(diag);
            }
            b"format" | b"sprintf" => {
                if method_bytes == b"format" && style == "format" {
                    return;
                }
                if method_bytes == b"sprintf" && style == "sprintf" {
                    return;
                }

                if call.receiver().is_some() {
                    return;
                }

                let arg_count = call
                    .arguments()
                    .map(|a| a.arguments().iter().count())
                    .unwrap_or(0);
                if arg_count < 2 {
                    return;
                }

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "sprintf" {
                    "sprintf"
                } else if style == "format" {
                    "format"
                } else {
                    "String#%"
                };
                let current = if method_bytes == b"format" {
                    "format"
                } else {
                    "sprintf"
                };
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `{}`.", preferred, current),
                );

                if let Some(corr) = corrections.as_mut() {
                    if style == "format" || style == "sprintf" {
                        if let Some(sel) = call.message_loc() {
                            corr.push(crate::correction::Correction {
                                start: sel.start_offset(),
                                end: sel.end_offset(),
                                replacement: preferred.to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    } else if style == "percent" {
                        let args = call
                            .arguments()
                            .map(|a| a.arguments().iter().collect::<Vec<_>>());
                        if let Some(arg_list) = args {
                            if !arg_list.is_empty() {
                                let format_arg = node_source(&arg_list[0]);
                                let param_args = &arg_list[1..];
                                if !param_args.is_empty() {
                                    let rhs = if param_args.len() == 1 {
                                        let single = &param_args[0];
                                        if single.as_hash_node().is_some()
                                            || single.as_keyword_hash_node().is_some()
                                        {
                                            let src = node_source(single);
                                            if src.trim_start().starts_with('{') {
                                                src
                                            } else {
                                                format!("{{ {} }}", src)
                                            }
                                        } else if is_operator_call_without_parens(single) {
                                            format!("({})", node_source(single))
                                        } else {
                                            node_source(single)
                                        }
                                    } else {
                                        format!(
                                            "[{}]",
                                            param_args
                                                .iter()
                                                .map(|n| node_source(n))
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        )
                                    };

                                    let replacement = format!("{} % {}", format_arg, rhs);
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
                        }
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
    crate::cop_fixture_tests!(FormatString, "cops/style/format_string");
    crate::cop_autocorrect_fixture_tests!(FormatString, "cops/style/format_string");
}
