use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct InlineComment;

impl Cop for InlineComment {
    fn name(&self) -> &'static str {
        "Style/InlineComment"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        let mut corrections = corrections;

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();

            if start == 0 {
                continue;
            }

            let mut line_start = start;
            while line_start > 0 && bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }

            let mut line_end = bytes.len();
            let mut i = start;
            while i < bytes.len() {
                if bytes[i] == b'\n' {
                    line_end = i;
                    break;
                }
                i += 1;
            }

            let before_on_line = &bytes[line_start..start];
            if before_on_line.iter().all(|&b| b == b' ' || b == b'\t') {
                continue;
            }

            let comment_bytes = &bytes[start..loc.end_offset()];
            let comment_text = match std::str::from_utf8(comment_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let after_hash = comment_text.trim_start_matches('#').trim_start();
            if after_hash.starts_with("rubocop:") || after_hash.starts_with("nitrocop-") {
                continue;
            }

            let (line, col) = source.offset_to_line_col(start);
            let mut diagnostic =
                self.diagnostic(source, line, col, "Avoid trailing inline comments.".to_string());

            if let Some(corrections) = corrections.as_deref_mut()
                && let Some(code_text) = source.try_byte_slice(line_start, start)
            {
                let code_trimmed = code_text.trim_end();
                let indent_len = code_text
                    .as_bytes()
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                let indent = &code_text[..indent_len];

                let mut replacement = String::new();
                replacement.push_str(indent);
                replacement.push_str(comment_text);
                replacement.push('\n');
                replacement.push_str(code_trimmed);
                if line_end < bytes.len() && bytes[line_end] == b'\n' {
                    replacement.push('\n');
                }

                let mut replace_end = line_end;
                if replace_end < bytes.len() && bytes[replace_end] == b'\n' {
                    replace_end += 1;
                }

                corrections.push(crate::correction::Correction {
                    start: line_start,
                    end: replace_end,
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InlineComment, "cops/style/inline_comment");

    #[test]
    fn supports_autocorrect() {
        assert!(InlineComment.supports_autocorrect());
    }

    #[test]
    fn autocorrect_moves_inline_comment_above_statement() {
        crate::testutil::assert_cop_autocorrect(
            &InlineComment,
            b"x = 42 # meaning\n",
            b"# meaning\nx = 42\n",
        );
    }
}
