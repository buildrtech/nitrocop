use crate::cop::node_type::HASH_NODE;
use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for trailing commas in hash literals.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=105, FN=0.
///
/// FP=105: Fixed. When a hash element's value contains a heredoc, Prism's
/// `location().end_offset()` for the pair node ends at the heredoc opening
/// tag on the source line (e.g., after `<<RUBY`), not after the heredoc
/// body/terminator. The heredoc body appears later in the source between
/// `last_end` and `closing_start`, so scanning that range finds commas inside
/// heredoc content (e.g., `hello, world`).
///
/// Fix: When any hash element contains a heredoc, scan only from the start of
/// the closing `}`'s line instead of from `last_end`. This skips over heredoc
/// bodies that sit between the last element and the closing brace.
pub struct TrailingCommaInHashLiteral;

fn push_remove_diagnostic(
    cop: &TrailingCommaInHashLiteral,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    abs_offset: usize,
    message: &str,
) {
    let (line, column) = source.offset_to_line_col(abs_offset);
    let mut diag = cop.diagnostic(source, line, column, message.to_string());
    if let Some(corr) = corrections.as_deref_mut() {
        corr.push(crate::correction::Correction {
            start: abs_offset,
            end: abs_offset + 1,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn push_insert_diagnostic(
    cop: &TrailingCommaInHashLiteral,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    insert_offset: usize,
    message: &str,
) {
    let (line, column) = source.offset_to_line_col(insert_offset);
    let mut diag = cop.diagnostic(source, line, column, message.to_string());
    if let Some(corr) = corrections.as_deref_mut() {
        corr.push(crate::correction::Correction {
            start: insert_offset,
            end: insert_offset,
            replacement: ",".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

impl Cop for TrailingCommaInHashLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInHashLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE]
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
        // Note: keyword_hash_node (keyword args like `foo(a: 1)`) intentionally not
        // handled — this cop only applies to trailing commas in hash literals.
        let hash_node = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let closing_loc = hash_node.closing_loc();
        let elements: Vec<ruby_prism::Node<'_>> = hash_node.elements().iter().collect();
        let last_elem = match elements.last() {
            Some(e) => e,
            None => return,
        };

        let last_end = last_elem.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();

        // For heredoc elements, Prism's location.end_offset() is at the heredoc
        // opening tag (e.g., `<<RUBY`), not the closing terminator. The heredoc body
        // sits between last_end and closing_start in the source, so scanning that
        // range could find commas inside heredoc content. For multiline hashes with
        // heredocs, scan only from the start of the closing bracket's line.
        let effective_last_end = if any_heredoc(&elements) {
            let open_line = source
                .offset_to_line_col(hash_node.opening_loc().start_offset())
                .0;
            let close_line = source.offset_to_line_col(closing_start).0;
            if open_line == close_line {
                // Single-line brackets: heredoc bodies are below, safe to use last_end
                last_end
            } else {
                // Multiline brackets: scan from start of `}`'s line
                let mut pos = closing_start;
                while pos > 0 && bytes[pos - 1] != b'\n' {
                    pos -= 1;
                }
                pos
            }
        } else {
            last_end
        };
        let has_comma = has_trailing_comma(bytes, effective_last_end, closing_start);

        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");
        let last_line = source.offset_to_line_col(last_end).0;
        let close_line = source.offset_to_line_col(closing_start).0;
        let is_multiline = close_line > last_line;

        // Helper: find the absolute offset of the trailing comma for diagnostics.
        // Uses effective_last_end to avoid scanning through heredoc content.
        let find_comma_offset = || -> Option<usize> {
            let search_range = &bytes[effective_last_end..closing_start];
            search_range
                .iter()
                .position(|&b| b == b',')
                .map(|off| effective_last_end + off)
        };

        match style {
            "comma" | "consistent_comma" => {
                // Require trailing comma in multiline; no opinion on single-line
                if is_multiline && !has_comma {
                    push_insert_diagnostic(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        last_end,
                        "Put a comma after the last item of a multiline hash.",
                    );
                }
            }
            _ => {
                // no_comma: flag trailing commas
                if has_comma
                    && let Some(abs_offset) = find_comma_offset()
                {
                    push_remove_diagnostic(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        abs_offset,
                        "Avoid comma after the last item of a hash.",
                    );
                }
            }
        }
    }
}

/// Returns true if any element in the hash contains a heredoc.
fn any_heredoc(elements: &[ruby_prism::Node<'_>]) -> bool {
    elements.iter().any(|e| is_heredoc_element(e))
}

/// Returns true if a node is or contains a heredoc.
/// Handles pair nodes (hash elements) by checking both key and value.
fn is_heredoc_element(node: &ruby_prism::Node<'_>) -> bool {
    // Check pair nodes (hash key-value pairs)
    if let Some(pair) = node.as_assoc_node() {
        return is_heredoc_element(&pair.value());
    }
    // Direct string heredoc
    if let Some(s) = node.as_interpolated_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    if let Some(s) = node.as_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    // Check method calls on heredocs (e.g., <<~SQL.strip.chomp)
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if is_heredoc_element(&recv) {
                return true;
            }
        }
        // Check arguments for heredocs (e.g., method(<<RUBY))
        if let Some(args) = call.arguments() {
            if args.arguments().iter().any(|a| is_heredoc_element(&a)) {
                return true;
            }
        }
    }
    // Check array values containing heredocs (e.g., key: [method(<<RUBY)])
    if let Some(arr) = node.as_array_node() {
        return arr.elements().iter().any(|e| is_heredoc_element(&e));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        TrailingCommaInHashLiteral,
        "cops/style/trailing_comma_in_hash_literal"
    );
    crate::cop_autocorrect_fixture_tests!(
        TrailingCommaInHashLiteral,
        "cops/style/trailing_comma_in_hash_literal"
    );
}
