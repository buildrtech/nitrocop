use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/MultilineArrayBraceLayout
///
/// ## Investigation findings
///
/// **FN root cause (83 FNs):** The cop only handled `[...]` bracket arrays,
/// skipping percent literal arrays (`%w()`, `%i()`, `%W()`, `%I()`, etc.).
/// Many corpus repos (especially devdocs) use `%w(` with the closing `)` on
/// the same line as the last element when the opening is on a separate line.
/// Fixed by removing the `[`/`]` check and accepting all explicit array types.
///
/// **FP root cause (3 FPs):** Arrays containing heredocs in the last element
/// (e.g., `[<<~MSG, ...]`) have a Prism end_offset on the opening delimiter
/// line, not the heredoc body end. This made the cop think the closing brace
/// was on a different line than the last element. RuboCop skips arrays where
/// the last element contains a heredoc. Fixed by adding `contains_heredoc`
/// check matching the sibling `MultilineHashBraceLayout` cop.
pub struct MultilineArrayBraceLayout;

impl Cop for MultilineArrayBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineArrayBraceLayout"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
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

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        // Implicit arrays have no opening/closing braces
        let opening = match array.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let closing = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let elements = array.elements();
        if elements.is_empty() {
            return;
        }

        let last_elem = elements.iter().last().unwrap();

        // Skip arrays where the last element contains a heredoc — the heredoc
        // body forces the closing brace placement and adjusting it would
        // produce invalid code.
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

        // Only check multiline arrays
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
                        replacement: String::from_utf8_lossy(closing.as_slice()).into_owned(),
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
                        "The closing array brace must be on the same line as the last array element when the opening brace is on the same line as the first array element.",
                        true,
                    );
                }
                if !open_same_as_first && close_same_as_last {
                    emit(
                        "The closing array brace must be on the line after the last array element when the opening brace is on a separate line from the first array element.",
                        false,
                    );
                }
            }
            "new_line" => {
                if close_same_as_last {
                    emit(
                        "The closing array brace must be on the line after the last array element.",
                        false,
                    );
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    emit(
                        "The closing array brace must be on the same line as the last array element.",
                        true,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Check if an array element node contains a heredoc string.
/// Walks into method call receivers/arguments.
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
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineArrayBraceLayout,
        "cops/layout/multiline_array_brace_layout"
    );
    crate::cop_autocorrect_fixture_tests!(
        MultilineArrayBraceLayout,
        "cops/layout/multiline_array_brace_layout"
    );
}
