use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/RedundantStringChars
///
/// Flags redundant uses of `.chars` followed by methods that can be called
/// directly on the string. RuboCop flags these patterns:
/// - `.chars.first` / `.chars[n]` / `.chars.last` -> use `[]`
/// - `.chars.length` / `.chars.size` / `.chars.empty?` -> use directly on string
/// - `.chars.take(n)` / `.chars.first(n)` / `.chars.slice(range)` -> use `[range].chars`
///
/// Investigation: 7 FNs were all `.chars.length` patterns. Added support for
/// `length`, `size`, `empty?`, `take`, and `slice` outer methods to match
/// RuboCop's full detection set.
///
/// FP=3 fix: Safe navigation chains (`&.chars&.first`, `.chars&.first`,
/// `&.chars.first`) are now skipped. RuboCop doesn't flag these because
/// the receiver might be nil, and the replacement `str[0]` would also
/// need safe navigation.
pub struct RedundantStringChars;

fn arg_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

fn build_replacement(
    source: &SourceFile,
    chain: &crate::cop::util::MethodChain<'_>,
    outer_call: &ruby_prism::CallNode<'_>,
) -> Option<String> {
    let receiver = chain.inner_call.receiver()?;
    let recv_loc = receiver.location();
    let recv_source = source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");

    let args: Vec<_> = outer_call
        .arguments()
        .map(|a| a.arguments().iter().collect())
        .unwrap_or_default();

    let suffix = match chain.outer_method {
        b"first" => {
            if args.is_empty() {
                "[0]".to_string()
            } else {
                let first = arg_source(source, &args[0]);
                format!("[0...{first}].chars")
            }
        }
        b"last" => {
            if !args.is_empty() {
                return None;
            }
            "[-1]".to_string()
        }
        b"[]" => {
            if args.len() > 1 {
                return None;
            }
            if args.is_empty() {
                return None;
            }
            let first = arg_source(source, &args[0]);
            if first.contains("..") {
                format!("[{first}].chars")
            } else {
                format!("[{first}]")
            }
        }
        b"length" => ".length".to_string(),
        b"size" => ".size".to_string(),
        b"empty?" => ".empty?".to_string(),
        b"take" => {
            if args.is_empty() {
                return None;
            }
            let first = arg_source(source, &args[0]);
            format!("[0...{first}].chars")
        }
        b"slice" => {
            if args.is_empty() {
                return None;
            }
            let joined = args
                .iter()
                .map(|a| arg_source(source, a))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{joined}].chars")
        }
        _ => return None,
    };

    Some(format!("{recv_source}{suffix}"))
}

impl Cop for RedundantStringChars {
    fn name(&self) -> &'static str {
        "Performance/RedundantStringChars"
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

        if chain.inner_method != b"chars" {
            return;
        }

        // The inner call must have a receiver (str.chars)
        if chain.inner_call.receiver().is_none() {
            return;
        }

        // Skip safe navigation chains (&.chars&.first, &.chars.first, .chars&.first)
        // RuboCop doesn't flag these because the receiver might be nil.
        if chain
            .inner_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
            return;
        }

        let outer_call = node.as_call_node().unwrap();

        if outer_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
            return;
        }
        let has_args = outer_call.arguments().is_some();

        let message = match chain.outer_method {
            b"first" => {
                if has_args {
                    "Use `[0...2].chars` instead of `chars.first(2)`.".to_string()
                } else {
                    "Use `[]` instead of `chars.first`.".to_string()
                }
            }
            b"last" => {
                // `.chars.last(n)` is not equivalent to a simple string slice for edge cases
                // (e.g. empty string, negative values). RuboCop explicitly excludes this.
                if has_args {
                    return;
                }
                "Use `[]` instead of `chars.first`.".to_string()
            }
            b"[]" => {
                // `.chars[n, m]` (two-arg form) is not flagged by RuboCop
                if let Some(args) = outer_call.arguments() {
                    if args.arguments().iter().count() > 1 {
                        return;
                    }
                }
                "Use `[]` instead of `chars.first`.".to_string()
            }
            b"length" => "Use `.length` instead of `chars.length`.".to_string(),
            b"size" => "Use `.size` instead of `chars.size`.".to_string(),
            b"empty?" => "Use `.empty?` instead of `chars.empty?`.".to_string(),
            b"take" => "Use `[0...2].chars` instead of `chars.take(2)`.".to_string(),
            b"slice" => "Use `[0..2].chars` instead of `chars.slice(0..2)`.".to_string(),
            _ => return,
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(source, line, column, message);

        if let Some(ref mut corr) = corrections
            && let Some(replacement) = build_replacement(source, &chain, &outer_call)
        {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
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

    crate::cop_fixture_tests!(
        RedundantStringChars,
        "cops/performance/redundant_string_chars"
    );
    crate::cop_autocorrect_fixture_tests!(
        RedundantStringChars,
        "cops/performance/redundant_string_chars"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(RedundantStringChars.supports_autocorrect());
    }
}
