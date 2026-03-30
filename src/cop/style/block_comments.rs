use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Style/BlockComments: Do not use block comments (`=begin`/`=end`).
///
/// Investigation: 19 FPs were caused by `=begin` appearing inside heredoc
/// strings (e.g., test files for rdoc/yard/coderay parsers). Fixed by
/// switching from `check_lines` to `check_source` to access the CodeMap
/// and skip `=begin` lines that fall within heredoc byte ranges.
///
/// Additional 4 FPs from `=begin` appearing after `__END__` data section
/// markers. RuboCop stops parsing at `__END__`, so block comments in the
/// data section are not flagged. Fixed by using `is_not_string()` which
/// covers heredocs, string literals, and `__END__` data sections.
///
/// Final 2 FPs from `=begin` appearing inside an outer `=begin`...`=end`
/// block (e.g., `louismullie/treat` `spec/workers/agnostic.rb`). Ruby does
/// not support nested block comments — a `=begin` inside a block comment is
/// just comment text, not a new block comment. Fixed by tracking when we are
/// inside a block comment and skipping inner `=begin` markers.
pub struct BlockComments;

impl Cop for BlockComments {
    fn name(&self) -> &'static str {
        "Style/BlockComments"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in &lines {
            line_offsets.push(offset);
            offset += line.len() + 1;
        }

        let mut i = 0usize;
        while i < lines.len() {
            let line = lines[i];

            // =begin must be at the start of a line
            if line.starts_with(b"=begin") && (line.len() == 6 || line[6].is_ascii_whitespace()) {
                // Skip =begin inside heredocs (e.g., test files for rdoc/yard)
                // or after __END__ data section marker (not real code).
                // is_not_string() returns false for heredocs, strings, and __END__ data.
                if let Some(offset) = source.line_col_to_offset(i + 1, 0) {
                    if !code_map.is_not_string(offset) {
                        i += 1;
                        continue;
                    }
                }

                let mut diag =
                    self.diagnostic(source, i + 1, 0, "Do not use block comments.".to_string());

                // Find matching =end; nested =begin inside the block are plain text.
                let mut end_line = i + 1;
                while end_line < lines.len() {
                    let candidate = lines[end_line];
                    if candidate.starts_with(b"=end")
                        && (candidate.len() == 4 || candidate[4].is_ascii_whitespace())
                    {
                        break;
                    }
                    end_line += 1;
                }

                if end_line < lines.len() {
                    if let Some(ref mut corr) = corrections {
                        let mut replacement_lines = Vec::new();
                        for content_line in &lines[i + 1..end_line] {
                            let text = std::str::from_utf8(content_line).unwrap_or("");
                            if text.is_empty() {
                                replacement_lines.push("#".to_string());
                            } else {
                                replacement_lines.push(format!("# {text}"));
                            }
                        }
                        let replacement = replacement_lines.join("\n");
                        let start = line_offsets[i];
                        let end = line_offsets[end_line] + lines[end_line].len();
                        corr.push(crate::correction::Correction {
                            start,
                            end,
                            replacement,
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }

                    diagnostics.push(diag);
                    i = end_line + 1;
                    continue;
                }

                diagnostics.push(diag);
            }

            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockComments, "cops/style/block_comments");
    crate::cop_autocorrect_fixture_tests!(BlockComments, "cops/style/block_comments");
}
