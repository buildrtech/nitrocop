use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10, updated 2026-03-14)
///
/// CI baseline reported FP=4, FN=87.
///
/// **FP root cause (4):** All FPs were `.when(...)` Arel method calls
/// (e.g., `Arel::Nodes::Case.new.when(...)`) misidentified as `when` keywords.
/// Fixed by checking if the keyword text is preceded by `.` or `&.`, which
/// indicates a method call rather than a keyword.
///
/// **FN root cause (87):** The original implementation only checked "space after"
/// for `keyword(` patterns at line starts. RuboCop also checks "space before"
/// keywords — e.g., `1and 2`, `""rescue a`, `self[:key]:super end`. Most FNs
/// came from compact/minified Ruby (camping gem) with missing spaces before
/// keywords like `rescue`, `and`, `or`, `if`, `super`.
///
/// Fixed by adding "space before missing" detection: when a keyword is preceded
/// by a non-whitespace, non-operator char (not in `\s(|{\[;,*=`), and the
/// keyword is in a code region per CodeMap, report an offense.
///
/// Also expanded "space after" detection to fire for any non-space char after
/// a keyword (not just `(`), matching RuboCop's broader checks. Keywords like
/// `break`, `defined?`, `next`, `not`, `rescue`, `super`, `yield` accept `(`
/// without complaint; `super` and `yield` also accept `[` and `super` accepts
/// `::`.
pub struct SpaceAroundKeyword;

/// Keywords that accept `(` immediately after them (no space required).
const ACCEPT_LEFT_PAREN: &[&[u8]] = &[
    b"break",
    b"defined?",
    b"next",
    b"not",
    b"rescue",
    b"super",
    b"yield",
];

/// Keywords that accept `[` immediately after them.
const ACCEPT_LEFT_SQUARE_BRACKET: &[&[u8]] = &[b"super", b"yield"];

/// Returns true if `ch` is a character that, when appearing before a keyword,
/// means we should NOT flag "space before missing". Mirrors RuboCop's
/// `space_before_missing?` which returns false for `[\s(|{\[;,*=]`.
fn accepted_before(ch: u8) -> bool {
    matches!(
        ch,
        b' ' | b'\t'
            | b'\n'
            | b'\r'
            | b'('
            | b'|'
            | b'{'
            | b'['
            | b';'
            | b','
            | b'*'
            | b'='
            | b'!'
            | b'+'
            | b'-'
            | b'/'
            | b'<'
            | b'>'
            | b'&'
            | b'.'
            | b'?'
    )
}

/// Returns true if the char after a keyword means "no space required".
/// Mirrors RuboCop's `space_after_missing?` which returns false for `[\s;,#\\)}\].]`.
fn accepted_after(ch: u8) -> bool {
    matches!(
        ch,
        b' ' | b'\t' | b'\n' | b'\r' | b';' | b',' | b'#' | b'\\' | b')' | b'}' | b']' | b'.'
    )
}

/// Returns true if `kw` is a word boundary — the byte after the keyword is
/// not alphanumeric or underscore (so `ifdef` doesn't match `if`).
fn is_word_end(bytes: &[u8], kw_end: usize) -> bool {
    if kw_end >= bytes.len() {
        return true;
    }
    let ch = bytes[kw_end];
    !(ch.is_ascii_alphanumeric() || ch == b'_')
}

/// Returns true if the byte before position `i` is a letter or underscore,
/// meaning this is NOT a keyword boundary (e.g., `sand` should not match `and`).
/// Digits are allowed before keywords: `1and` is parsed as `1 and ...`.
fn is_word_before(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    let ch = bytes[i - 1];
    ch.is_ascii_alphabetic() || ch == b'_'
}

impl Cop for SpaceAroundKeyword {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundKeyword"
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
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Quick dispatch on first byte to candidate keywords.
            let candidates: &[&[u8]] = match bytes[i] {
                b'a' => &[b"and"],
                b'b' => &[b"begin", b"break"],
                b'c' => &[b"case"],
                b'd' => &[b"defined?", b"do"],
                b'e' => &[b"else", b"elsif", b"end", b"ensure"],
                b'i' => &[b"if", b"in"],
                b'n' => &[b"next", b"not"],
                b'o' => &[b"or"],
                b'r' => &[b"rescue", b"return"],
                b's' => &[b"super"],
                b't' => &[b"then"],
                b'u' => &[b"unless", b"until"],
                b'w' => &[b"when", b"while"],
                b'y' => &[b"yield"],
                b'B' => &[b"BEGIN"],
                b'E' => &[b"END"],
                _ => {
                    i += 1;
                    continue;
                }
            };

            for &kw in candidates {
                let kw_len = kw.len();
                if i + kw_len > len {
                    continue;
                }
                if &bytes[i..i + kw_len] != kw {
                    continue;
                }
                if !is_word_end(bytes, i + kw_len) {
                    continue;
                }
                if is_word_before(bytes, i) {
                    continue;
                }
                if !code_map.is_code(i) {
                    continue;
                }

                // Check if preceded by `.` or `&.` — that makes it a method call, not a keyword
                if is_method_call(bytes, i) {
                    continue;
                }

                // Check if preceded by `def ` — that's a method definition named after the keyword
                if preceded_by_def(bytes, i) {
                    continue;
                }

                let kw_str = std::str::from_utf8(kw).unwrap_or("");

                // --- Check "space before missing" ---
                if i > 0 && !accepted_before(bytes[i - 1]) {
                    let (line, column) = source.offset_to_line_col(i);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Space before keyword `{kw_str}` is missing."),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: i,
                            end: i,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }

                // --- Check "space after missing" ---
                if i + kw_len < len {
                    let after = bytes[i + kw_len];
                    let skip_after = accepted_after(after)
                        || (after == b'(' && is_accept_left_paren(kw))
                        || (after == b'[' && is_accept_left_bracket(kw))
                        || (after == b':'
                            && kw == b"super"
                            && i + kw_len + 1 < len
                            && bytes[i + kw_len + 1] == b':')
                        || (after == b'&' && i + kw_len + 1 < len && bytes[i + kw_len + 1] == b'.');

                    if !skip_after {
                        let (line, column) = source.offset_to_line_col(i);
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Space after keyword `{kw_str}` is missing."),
                        );
                        if let Some(ref mut corr) = corrections {
                            corr.push(crate::correction::Correction {
                                start: i + kw_len,
                                end: i + kw_len,
                                replacement: " ".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
            }
            i += 1;
        }
    }
}

/// Check if the keyword at position `i` is a method call (preceded by `.` or `&.`).
fn is_method_call(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    // Skip whitespace before the keyword to find the actual preceding token
    let mut j = i - 1;
    while j > 0 && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r')
    {
        j -= 1;
    }
    if bytes[j] == b'.' {
        // Could be `&.` or just `.`
        return true;
    }
    false
}

/// Check if the keyword is preceded by `def ` (method definition).
fn preceded_by_def(bytes: &[u8], i: usize) -> bool {
    i >= 4 && &bytes[i - 4..i] == b"def "
}

/// Returns true if this keyword accepts `(` immediately after it.
fn is_accept_left_paren(kw: &[u8]) -> bool {
    ACCEPT_LEFT_PAREN.contains(&kw)
}

/// Returns true if this keyword accepts `[` immediately after it.
fn is_accept_left_bracket(kw: &[u8]) -> bool {
    ACCEPT_LEFT_SQUARE_BRACKET.contains(&kw)
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundKeyword, "cops/layout/space_around_keyword");
    crate::cop_autocorrect_fixture_tests!(SpaceAroundKeyword, "cops/layout/space_around_keyword");

    #[test]
    fn autocorrect_insert_space() {
        let input = b"if(x)\n  y\nend\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&SpaceAroundKeyword, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"if (x)\n  y\nend\n");
    }
}
