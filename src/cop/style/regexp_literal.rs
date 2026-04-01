use crate::cop::node_type::{INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP fix (2026-03): slashes inside `#{}` interpolation segments were wrongly
/// counted as inner slashes, causing false "Use %r" suggestions on regexps like
/// `/#{Regexp.quote("</")}/ `. RuboCop's `node_body` only examines `:str` children,
/// so interpolation content is excluded. Fixed by iterating over Prism's `parts()`
/// and only collecting `StringNode` content for the slash check.
pub struct RegexpLiteral;

fn push_slashes_diagnostic(
    cop: &RegexpLiteral,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    node_start: usize,
    node_end: usize,
    opening_len: usize,
    message: &str,
) {
    let (line, column) = source.offset_to_line_col(node_start);
    let mut diag = cop.diagnostic(source, line, column, message.to_string());

    if let Some(corr) = corrections.as_mut() {
        corr.push(crate::correction::Correction {
            start: node_start,
            end: node_start + opening_len,
            replacement: "/".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        corr.push(crate::correction::Correction {
            start: node_end.saturating_sub(1),
            end: node_end,
            replacement: "/".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }

    diagnostics.push(diag);
}

fn push_percent_r_diagnostic(
    cop: &RegexpLiteral,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    node_start: usize,
    node_end: usize,
    message: &str,
) {
    let (line, column) = source.offset_to_line_col(node_start);
    let mut diag = cop.diagnostic(source, line, column, message.to_string());

    if let Some(corr) = corrections.as_mut() {
        corr.push(crate::correction::Correction {
            start: node_start,
            end: node_start + 1,
            replacement: "%r{".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        corr.push(crate::correction::Correction {
            start: node_end.saturating_sub(1),
            end: node_end,
            replacement: "}".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }

    diagnostics.push(diag);
}

impl Cop for RegexpLiteral {
    fn name(&self) -> &'static str {
        "Style/RegexpLiteral"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            REGULAR_EXPRESSION_NODE,
        ]
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
        let enforced_style = config.get_str("EnforcedStyle", "slashes");
        let allow_inner_slashes = config.get_bool("AllowInnerSlashes", false);

        let (open_bytes, content_bytes, node_start, node_end): (Vec<u8>, Vec<u8>, usize, usize) =
            if let Some(re) = node.as_regular_expression_node() {
                let opening = re.opening_loc();
                let content = re.content_loc().as_slice();
                let loc = re.location();
                (
                    opening.as_slice().to_vec(),
                    content.to_vec(),
                    loc.start_offset(),
                    loc.end_offset(),
                )
            } else if let Some(re) = node.as_interpolated_regular_expression_node() {
                let opening = re.opening_loc();
                let loc = re.location();
                let open = opening.as_slice();
                let mut content = Vec::new();
                for part in re.parts().iter() {
                    if let Some(s) = part.as_string_node() {
                        content.extend_from_slice(s.location().as_slice());
                    }
                }
                (open.to_vec(), content, loc.start_offset(), loc.end_offset())
            } else {
                return;
            };

        let is_slash = open_bytes == b"/";
        let is_percent_r = open_bytes.starts_with(b"%r");
        let has_slash = content_bytes.contains(&b'/');

        let is_multiline = {
            let (start_line, _) = source.offset_to_line_col(node_start);
            let (end_line, _) = source.offset_to_line_col(node_end);
            end_line > start_line
        };

        let content_starts_with_space_or_eq =
            !content_bytes.is_empty() && (content_bytes[0] == b' ' || content_bytes[0] == b'=');

        match enforced_style {
            "slashes" => {
                if is_percent_r {
                    if has_slash && !allow_inner_slashes {
                        return;
                    }
                    if content_starts_with_space_or_eq {
                        return;
                    }
                    push_slashes_diagnostic(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        node_start,
                        node_end,
                        open_bytes.len(),
                        "Use `//` around regular expression.",
                    );
                }
            }
            "percent_r" => {
                if is_slash {
                    push_percent_r_diagnostic(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        node_start,
                        node_end,
                        "Use `%r` around regular expression.",
                    );
                }
            }
            "mixed" => {
                if is_multiline {
                    if is_slash {
                        push_percent_r_diagnostic(
                            self,
                            source,
                            diagnostics,
                            &mut corrections,
                            node_start,
                            node_end,
                            "Use `%r` around regular expression.",
                        );
                    }
                } else if is_percent_r {
                    if has_slash && !allow_inner_slashes {
                        return;
                    }
                    if content_starts_with_space_or_eq {
                        return;
                    }
                    push_slashes_diagnostic(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        node_start,
                        node_end,
                        open_bytes.len(),
                        "Use `//` around regular expression.",
                    );
                }
            }
            _ => {}
        }

        if enforced_style == "slashes" && is_slash && has_slash && !allow_inner_slashes {
            push_percent_r_diagnostic(
                self,
                source,
                diagnostics,
                &mut corrections,
                node_start,
                node_end,
                "Use `%r` around regular expression.",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RegexpLiteral, "cops/style/regexp_literal");
    crate::cop_autocorrect_fixture_tests!(RegexpLiteral, "cops/style/regexp_literal");
}
