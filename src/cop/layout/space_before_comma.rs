use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=0, FN=7.
///
/// Fixed the remaining syntax gap in command-form calls and autocorrect:
/// RuboCop removes the entire whitespace span before `,`, but nitrocop only
/// removed one ASCII space. The accepted fix now trims the full contiguous
/// space/tab run before the comma, which covers cases like `break  1  , 2`
/// as well as the sampled `break`/`next`/`yield` forms from `rufo`.
///
/// Acceptance gate after this patch (`scripts/check-cop.py --verbose --rerun`):
/// expected=3,134, actual=3,162, CI baseline=3,127, raw excess=28,
/// missing=0, file-drop noise=103. The rerun passes against the CI baseline
/// once that existing parser-crash noise is applied.
///
/// ## FP fix: character literal `?\ ,` (2026-03-25)
///
/// All 5 FPs were from Ruby character literals using escaped space (`?\ `).
/// In `?\ ,`, the space is part of the character literal, not whitespace
/// before the comma. Fixed by checking for `?\` immediately before the
/// whitespace run and skipping those offenses.
pub struct SpaceBeforeComma;

fn whitespace_before_comma_start(bytes: &[u8], comma_offset: usize) -> Option<usize> {
    let mut start = comma_offset;
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }

    (start < comma_offset).then_some(start)
}

impl Cop for SpaceBeforeComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComma"
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
        let bytes = source.as_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if byte == b',' && code_map.is_code(i) {
                let Some(start) = whitespace_before_comma_start(bytes, i) else {
                    continue;
                };

                // Skip Ruby character literal for escaped space: ?\ ,
                if start >= 2 && bytes[start - 1] == b'\\' && bytes[start - 2] == b'?' {
                    continue;
                }

                let (line, column) = source.offset_to_line_col(start);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Space found before comma.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start,
                        end: i,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
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

    crate::cop_fixture_tests!(SpaceBeforeComma, "cops/layout/space_before_comma");
    crate::cop_autocorrect_fixture_tests!(SpaceBeforeComma, "cops/layout/space_before_comma");

    #[test]
    fn autocorrect_remove_space() {
        let input = b"foo(1 , 2)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeComma, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2)\n");
    }

    #[test]
    fn autocorrect_multiple() {
        let input = b"foo(1 , 2 , 3)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeComma, input);
        assert_eq!(corrections.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2, 3)\n");
    }
}
