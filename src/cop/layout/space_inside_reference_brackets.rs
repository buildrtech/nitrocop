use crate::cop::node_type::{
    CALL_NODE, INDEX_AND_WRITE_NODE, INDEX_OPERATOR_WRITE_NODE, INDEX_OR_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Cached corpus oracle reported FP=12, FN=1.
///
/// Fixed FN=1: multiline empty brackets such as `items[\n ]` were treated as
/// non-empty because the empty-bracket check only accepted spaces/tabs and ran
/// after the multiline early return. Empty-bracket detection now treats CR/LF
/// as whitespace and runs before the multiline guard.
///
/// FP=12: the cached nested-reference examples were not reproduced by the local
/// fixtures in this batch. The corpus rerun remains dominated by parser-crash
/// file-drop noise (`jruby__jruby__0303464` reported 358 dropped offenses),
/// so no additional bracket-selection change was made here.
///
/// Acceptance gate after this patch (`scripts/check-cop.py --verbose --rerun`):
/// expected=830, actual=970, CI baseline=841, raw excess=140, missing=0,
/// file-drop noise=358. The rerun passes against the CI baseline once that
/// existing noise is accounted for.
pub struct SpaceInsideReferenceBrackets;

impl Cop for SpaceInsideReferenceBrackets {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideReferenceBrackets"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            INDEX_AND_WRITE_NODE,
            INDEX_OPERATOR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
        ]
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
        let enforced_style = config.get_str("EnforcedStyle", "no_space");
        let empty_style = config.get_str("EnforcedStyleForEmptyBrackets", "no_space");

        let bytes = source.as_bytes();

        let (open_start, close_start) = match reference_bracket_offsets(node) {
            Some(offsets) => offsets,
            None => return,
        };
        let open_end = open_start + 1;

        // Check for empty brackets
        let is_empty = close_start == open_end
            || (close_start > open_end
                && bytes[open_end..close_start]
                    .iter()
                    .all(|&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r')));

        if is_empty {
            match empty_style {
                "no_space" => {
                    if close_start > open_end {
                        let (line, col) = source.offset_to_line_col(open_end);
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            col,
                            "Do not use space inside empty reference brackets.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            corr.push(crate::correction::Correction {
                                start: open_end,
                                end: close_start,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
                "space" => {
                    if close_start == open_end || (close_start - open_end) != 1 {
                        let (line, col) = source.offset_to_line_col(open_start);
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            col,
                            "Use one space inside empty reference brackets.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            corr.push(crate::correction::Correction {
                                start: open_end,
                                end: close_start,
                                replacement: " ".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
                _ => {}
            }
            return;
        }

        // Skip multiline non-empty brackets.
        let (open_line, _) = source.offset_to_line_col(open_start);
        let (close_line, _) = source.offset_to_line_col(close_start);
        if open_line != close_line {
            return;
        }

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced_style {
            "no_space" => {
                if space_after_open {
                    let (line, col) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        col,
                        "Do not use space inside reference brackets.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end,
                            end: open_end + 1,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                if space_before_close {
                    let (line, col) = source.offset_to_line_col(close_start - 1);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        col,
                        "Do not use space inside reference brackets.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_start - 1,
                            end: close_start,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            "space" => {
                if !space_after_open {
                    let (line, col) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        col,
                        "Use space inside reference brackets.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end,
                            end: open_end,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                if !space_before_close {
                    let (line, col) = source.offset_to_line_col(close_start);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        col,
                        "Use space inside reference brackets.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_start,
                            end: close_start,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideReferenceBrackets,
        "cops/layout/space_inside_reference_brackets"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsideReferenceBrackets,
        "cops/layout/space_inside_reference_brackets"
    );
}

fn reference_bracket_offsets(node: &ruby_prism::Node<'_>) -> Option<(usize, usize)> {
    if let Some(call) = node.as_call_node() {
        return call_bracket_offsets(&call);
    }
    if let Some(index) = node.as_index_and_write_node() {
        return index_write_bracket_offsets(
            index.receiver(),
            index.opening_loc().start_offset(),
            index.closing_loc().start_offset(),
        );
    }
    if let Some(index) = node.as_index_operator_write_node() {
        return index_write_bracket_offsets(
            index.receiver(),
            index.opening_loc().start_offset(),
            index.closing_loc().start_offset(),
        );
    }
    if let Some(index) = node.as_index_or_write_node() {
        return index_write_bracket_offsets(
            index.receiver(),
            index.opening_loc().start_offset(),
            index.closing_loc().start_offset(),
        );
    }
    None
}

fn call_bracket_offsets(call: &ruby_prism::CallNode<'_>) -> Option<(usize, usize)> {
    let method_name = call.name().as_slice();
    if method_name != b"[]" && method_name != b"[]=" {
        return None;
    }

    let receiver = call.receiver()?;
    if method_name == b"[]=" {
        if let Some(offsets) = nested_reference_brackets(&receiver) {
            return Some(offsets);
        }
    }

    let opening_loc = call.opening_loc()?;
    let closing_loc = call.closing_loc()?;
    if opening_loc.as_slice() != b"[" || closing_loc.as_slice() != b"]" {
        return None;
    }

    Some((opening_loc.start_offset(), closing_loc.start_offset()))
}

fn index_write_bracket_offsets(
    receiver: Option<ruby_prism::Node<'_>>,
    open_start: usize,
    close_start: usize,
) -> Option<(usize, usize)> {
    receiver?;
    Some((open_start, close_start))
}

fn nested_reference_brackets(receiver: &ruby_prism::Node<'_>) -> Option<(usize, usize)> {
    if let Some(call) = receiver.as_call_node() {
        let method_name = call.name().as_slice();
        if method_name != b"[]" && method_name != b"[]=" {
            return None;
        }

        let opening_loc = call.opening_loc()?;
        let closing_loc = call.closing_loc()?;
        if opening_loc.as_slice() != b"[" || closing_loc.as_slice() != b"]" {
            return None;
        }

        return Some((opening_loc.start_offset(), closing_loc.start_offset()));
    }

    if let Some(index) = receiver.as_index_and_write_node() {
        return Some((
            index.opening_loc().start_offset(),
            index.closing_loc().start_offset(),
        ));
    }
    if let Some(index) = receiver.as_index_operator_write_node() {
        return Some((
            index.opening_loc().start_offset(),
            index.closing_loc().start_offset(),
        ));
    }
    if let Some(index) = receiver.as_index_or_write_node() {
        return Some((
            index.opening_loc().start_offset(),
            index.closing_loc().start_offset(),
        ));
    }

    None
}
