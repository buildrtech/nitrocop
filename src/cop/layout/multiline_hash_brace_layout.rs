use crate::cop::node_type::{HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/MultilineHashBraceLayout
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=2.
///
/// FP=0: no corpus false positives are currently known.
///
/// FN=2:
/// - `elastic/elasticsearch-ruby`: the outer hash had a heredoc in an earlier
///   element, but the last element was a normal hash pair. RuboCop still checks
///   brace layout there; only a heredoc in the last element forces the closing
///   brace placement. Fixed by narrowing the heredoc skip to the last element.
/// - `peritor/webistrano`: the remaining FN is a commented-out snippet that has
///   not reproduced locally as a normal AST-based offense. Leave it for future
///   investigation if it persists after the next corpus oracle run.
pub struct MultilineHashBraceLayout;

impl Cop for MultilineHashBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineHashBraceLayout"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE, KEYWORD_HASH_NODE]
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

        // KeywordHashNode (keyword args `foo(a: 1)`) has no braces — skip
        if node.as_keyword_hash_node().is_some() {
            return;
        }

        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        // Only check brace hashes
        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return;
        }

        let elements = hash.elements();
        if elements.is_empty() {
            return;
        }

        let last_elem = elements.iter().last().unwrap();

        // Only the last element can force the closing brace to move because of
        // its heredoc terminator. Earlier heredocs do not exempt the hash.
        if contains_heredoc(&last_elem) {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Get first and last element lines
        let first_elem = elements.iter().next().unwrap();
        let (first_elem_line, _) = source.offset_to_line_col(first_elem.location().start_offset());
        let (last_elem_line, _) =
            source.offset_to_line_col(last_elem.location().end_offset().saturating_sub(1));

        // Only check multiline hashes
        if open_line == close_line {
            return;
        }

        let open_same_as_first = open_line == first_elem_line;
        let close_same_as_last = close_line == last_elem_line;

        let last_elem_end = last_elem.location().end_offset();
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
                let correction = if want_same_line {
                    Correction {
                        start: last_elem_end,
                        end: closing_end,
                        replacement: "}".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    }
                } else {
                    Correction {
                        start: last_elem_end,
                        end: closing_start,
                        replacement: format!("\n{}", " ".repeat(opening_indent)),
                        cop_name: self.name(),
                        cop_index: 0,
                    }
                };
                corrections.push(correction);
                diagnostic.corrected = true;
            }
            diagnostics.push(diagnostic);
        };

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    emit(
                        "Closing hash brace must be on the same line as the last hash element when opening brace is on the same line as the first hash element.",
                        true,
                    );
                }
                if !open_same_as_first && close_same_as_last {
                    emit(
                        "Closing hash brace must be on the line after the last hash element when opening brace is on a separate line from the first hash element.",
                        false,
                    );
                }
            }
            "new_line" => {
                if close_same_as_last {
                    emit(
                        "Closing hash brace must be on the line after the last hash element.",
                        false,
                    );
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    emit(
                        "Closing hash brace must be on the same line as the last hash element.",
                        true,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Check if a hash element node contains a heredoc string.
/// Walks into AssocNode values and method call receivers/arguments.
fn contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
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
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if contains_heredoc(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if contains_heredoc(&arg) {
                    return true;
                }
            }
        }
    }
    if let Some(assoc) = node.as_assoc_node() {
        return contains_heredoc(&assoc.value());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineHashBraceLayout,
        "cops/layout/multiline_hash_brace_layout"
    );
    crate::cop_autocorrect_fixture_tests!(
        MultilineHashBraceLayout,
        "cops/layout/multiline_hash_brace_layout"
    );

    #[test]
    fn earlier_heredoc_still_checks_closing_brace() {
        let source = br#"config = { subject: <<~BODY,
             body line
           BODY
           attachment: "report.yml"
}
"#;
        let diagnostics = run_cop_full(&MultilineHashBraceLayout, source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected one offense: {diagnostics:?}"
        );
    }
}
