use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `shuffle.first`, `shuffle.last`, `shuffle[0]`, `shuffle[-1]`,
/// `shuffle.at(0)`, `shuffle.at(-1)`, `shuffle.slice(0)`, `shuffle.slice(-1)`.
///
/// ## Investigation (2026-03-14)
/// Original implementation only handled `.first` and `.last` on shuffle calls.
/// Corpus FNs (4) were caused by missing detection of bracket access (`[]`),
/// `.at()`, and `.slice()` patterns with integer arguments 0 or -1.
/// Added handling for these patterns to match RuboCop's Style/Sample cop.
pub struct Sample;

impl Cop for Sample {
    fn name(&self) -> &'static str {
        "Style/Sample"
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Determine which pattern we're looking at
        enum ShufflePattern {
            FirstLast,   // .first / .last (no args or with count arg)
            IndexAccess, // [0] / [-1] — method name is `[]`
            AtOrSlice,   // .at(0) / .at(-1) / .slice(0) / .slice(-1)
        }

        let pattern = match method_bytes {
            b"first" | b"last" => ShufflePattern::FirstLast,
            b"[]" => ShufflePattern::IndexAccess,
            b"at" | b"slice" => ShufflePattern::AtOrSlice,
            _ => return,
        };

        // For [] / at / slice, validate the argument(s)
        if matches!(
            pattern,
            ShufflePattern::IndexAccess | ShufflePattern::AtOrSlice
        ) {
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();

            if arg_list.len() == 1 {
                // Single arg: must be integer 0 or -1
                let arg = &arg_list[0];
                let is_valid = if let Some(int_node) = arg.as_integer_node() {
                    let val_str = std::str::from_utf8(int_node.location().as_slice()).unwrap_or("");
                    matches!(val_str, "0" | "-1")
                } else {
                    false
                };
                if !is_valid {
                    return;
                }
            } else if arg_list.len() == 2 && matches!(pattern, ShufflePattern::IndexAccess) {
                // Two args for []: first must be integer 0, second must be integer
                // e.g. shuffle[0, 2] is equivalent to sample(2)
                let first_valid = arg_list[0]
                    .as_integer_node()
                    .map(|n| std::str::from_utf8(n.location().as_slice()).unwrap_or("") == "0")
                    .unwrap_or(false);
                let second_valid = arg_list[1].as_integer_node().is_some();
                if !first_valid || !second_valid {
                    return;
                }
            } else {
                return;
            }
        }

        // Receiver must be a call to .shuffle
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if let Some(shuffle_call) = receiver.as_call_node() {
            if shuffle_call.name().as_slice() == b"shuffle" {
                // shuffle must have a receiver (the collection)
                if shuffle_call.receiver().is_none() {
                    return;
                }

                let loc = node.location();
                let incorrect = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                let (line, column) = source.offset_to_line_col(loc.start_offset());

                // Determine the correct replacement
                // For two-arg bracket access `shuffle[0, n]`, the size is the second arg.
                let two_arg_size: Option<String> = if matches!(pattern, ShufflePattern::IndexAccess)
                {
                    call.arguments().and_then(|a| {
                        let args: Vec<_> = a.arguments().iter().collect();
                        if args.len() == 2 {
                            Some(
                                std::str::from_utf8(args[1].location().as_slice())
                                    .unwrap_or("0")
                                    .to_string(),
                            )
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                let correct = if let Some(size) = two_arg_size {
                    if shuffle_call.arguments().is_some() {
                        let shuffle_args = shuffle_call
                            .arguments()
                            .map(|a| std::str::from_utf8(a.location().as_slice()).unwrap_or(""))
                            .unwrap_or("");
                        format!("sample({}, {})", size, shuffle_args)
                    } else {
                        format!("sample({})", size)
                    }
                } else if matches!(pattern, ShufflePattern::FirstLast) && call.arguments().is_some()
                {
                    let arg_src = call
                        .arguments()
                        .map(|a| {
                            let args: Vec<_> = a.arguments().iter().collect();
                            if !args.is_empty() {
                                std::str::from_utf8(args[0].location().as_slice())
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                String::new()
                            }
                        })
                        .unwrap_or_default();

                    if shuffle_call.arguments().is_some() {
                        let shuffle_args = shuffle_call
                            .arguments()
                            .map(|a| std::str::from_utf8(a.location().as_slice()).unwrap_or(""))
                            .unwrap_or("");
                        format!("sample({}, {})", arg_src, shuffle_args)
                    } else {
                        format!("sample({})", arg_src)
                    }
                } else if shuffle_call.arguments().is_some() {
                    let shuffle_args = shuffle_call
                        .arguments()
                        .map(|a| std::str::from_utf8(a.location().as_slice()).unwrap_or(""))
                        .unwrap_or("");
                    format!("sample({})", shuffle_args)
                } else {
                    "sample".to_string()
                };

                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of `{}`.", correct, incorrect),
                );

                if let Some(shuffle_selector) = shuffle_call.message_loc() {
                    if let Some(corr) = corrections.as_mut() {
                        corr.push(crate::correction::Correction {
                            start: shuffle_selector.start_offset(),
                            end: loc.end_offset(),
                            replacement: correct.clone(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                    diag.corrected = true;
                }

                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Sample, "cops/style/sample");
    crate::cop_autocorrect_fixture_tests!(Sample, "cops/style/sample");
}
