use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=3.
///
/// FP=0: previous false positives in heredoc-heavy calls were fixed by
/// recursing into nested call arguments, keyword hashes, and assoc values when
/// checking whether the last argument contains a conflicting heredoc.
///
/// FN=3: this cop previously skipped brace-layout checks when *any* argument
/// contained a heredoc. RuboCop only skips when the *last* argument contains a
/// heredoc terminator that forces the closing parenthesis placement. Narrowing
/// the skip to the last argument fixes heredoc-first calls like
/// `foo(<<~EOS, arg ... ).call`.
pub struct MultilineMethodCallBraceLayout;

impl Cop for MultilineMethodCallBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallBraceLayout"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have explicit parentheses
        let opening = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let closing = match call.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        if opening.as_slice() != b"(" || closing.as_slice() != b")" {
            return;
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Only a heredoc in the last argument can force the closing paren to a
        // later line. Earlier heredoc arguments do not exempt the call.
        let last_arg = arg_list.last().unwrap();
        if is_heredoc_node(last_arg) {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline calls (opening paren to closing paren)
        if open_line == close_line {
            return;
        }

        let first_arg = &arg_list[0];

        let (first_arg_line, _) = source.offset_to_line_col(first_arg.location().start_offset());

        // Compute the effective end of the last argument. In Prism, `&block`
        // arguments are stored in the CallNode's `block` field, not in the
        // arguments list. For `define_method(method, &lambda do...end)`, the
        // BlockArgumentNode's end offset includes the block's `end`, so use
        // it when present to correctly determine the last arg's line.
        let last_arg_end = if let Some(block) = call.block() {
            if block.as_block_argument_node().is_some() {
                // &block_arg — its span includes the block content
                block.location().end_offset().saturating_sub(1)
            } else {
                // Regular do...end block — `)` comes before the block, not after
                last_arg.location().end_offset().saturating_sub(1)
            }
        } else {
            last_arg.location().end_offset().saturating_sub(1)
        };
        let (last_arg_line, _) = source.offset_to_line_col(last_arg_end);

        let open_same_as_first = open_line == first_arg_line;
        let close_same_as_last = close_line == last_arg_line;

        let last_arg_end = last_arg.location().end_offset();
        let closing_start = closing.start_offset();
        let closing_end = closing.end_offset();
        let opening_line_start = source.line_start_offset(open_line);
        let opening_indent = source.as_bytes()[opening_line_start..]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();

        let mut emit = |message: &str, want_same_line: bool| {
            let mut diagnostic =
                self.diagnostic(source, close_line, close_col, message.to_string());

            if let Some(corrections) = corrections.as_mut() {
                if want_same_line {
                    let between = &source.as_bytes()[last_arg_end..closing_start];
                    if between
                        .iter()
                        .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
                    {
                        corrections.push(Correction {
                            start: last_arg_end,
                            end: closing_end,
                            replacement: ")".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                } else {
                    corrections.push(Correction {
                        start: last_arg_end,
                        end: closing_start,
                        replacement: format!("\n{}", " ".repeat(opening_indent)),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }

            diagnostics.push(diagnostic);
        };

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    emit(
                        "Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.",
                        true,
                    );
                }
                if !open_same_as_first && close_same_as_last {
                    emit(
                        "Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.",
                        false,
                    );
                }
            }
            "new_line" => {
                if close_same_as_last {
                    emit(
                        "Closing method call brace must be on the line after the last argument.",
                        false,
                    );
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    emit(
                        "Closing method call brace must be on the same line as the last argument.",
                        true,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Check if a node is or contains a heredoc string (opening starts with `<<`).
/// Also walks into method call receivers to detect `<<~SQL.tr(...)` patterns
/// where the heredoc is wrapped in a method call, and into keyword hash pairs
/// to detect heredocs used as keyword argument values (e.g., `key: <<~HEREDOC`).
fn is_heredoc_node(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_interpolated_string_node() {
        if let Some(open) = s.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    if let Some(s) = node.as_string_node() {
        if let Some(open) = s.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    // Check if this is a method call on a heredoc (e.g., <<~SQL.tr("\n", ""))
    // or a method call with a heredoc argument (e.g., raw(<<~HEREDOC.chomp))
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if is_heredoc_node(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if is_heredoc_node(&arg) {
                    return true;
                }
            }
        }
    }
    // Check inside keyword hash nodes (keyword arguments like `key: <<~HEREDOC`)
    if let Some(kw_hash) = node.as_keyword_hash_node() {
        for element in kw_hash.elements().iter() {
            if is_heredoc_node(&element) {
                return true;
            }
        }
    }
    // Check the value side of association (key-value) pairs
    if let Some(assoc) = node.as_assoc_node() {
        return is_heredoc_node(&assoc.value());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineMethodCallBraceLayout,
        "cops/layout/multiline_method_call_brace_layout"
    );
    crate::cop_autocorrect_fixture_tests!(
        MultilineMethodCallBraceLayout,
        "cops/layout/multiline_method_call_brace_layout"
    );

    #[test]
    fn heredoc_only_in_earlier_argument_still_checks_brace_layout() {
        let source = br#"foo(<<~EOS, arg
  text
EOS
).do_something
"#;
        let diagnostics = run_cop_full(&MultilineMethodCallBraceLayout, source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected one offense: {diagnostics:?}"
        );
    }
}
