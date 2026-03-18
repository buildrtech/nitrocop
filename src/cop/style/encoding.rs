use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/Encoding checks for unnecessary utf-8 encoding comments.
///
/// ## Investigation findings (2026-03-18)
///
/// ### FP root cause fixed:
/// - Non-magic comment lines (e.g., `# This is a description`) on line 1 or 2
///   should stop magic comment processing. Previously we checked the first 3 lines
///   regardless; now we match RuboCop's behavior of only processing contiguous
///   magic comment lines (frozen_string_literal, encoding, shareable_constant_value,
///   typed, rbs_inline). Regular comments terminate the search.
///
/// ### FN root cause fixed:
/// - `# coding: utf-8` format was already handled but lacked fixture coverage.
///   Added coding_format.rb scenario.
pub struct Encoding;

impl Cop for Encoding {
    fn name(&self) -> &'static str {
        "Style/Encoding"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let total_len = source.as_bytes().len();
        let mut byte_offset: usize = 0;

        // Process contiguous magic comment lines at the top of the file,
        // matching RuboCop's behavior: skip shebangs, then process each line
        // as a magic comment. Stop as soon as a non-magic-comment line is found.
        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline

            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s.trim(),
                Err(_) => {
                    break;
                }
            };

            // Skip shebang lines
            if line_str.starts_with("#!") {
                byte_offset += line_len;
                continue;
            }

            // Non-comment or blank line: stop processing
            if !line_str.starts_with('#') {
                break;
            }

            // Must be a valid magic comment to continue processing.
            // RuboCop uses MagicComment.parse(line).valid? which checks for
            // encoding, frozen_string_literal, shareable_constant_value, typed, rbs_inline.
            if !is_magic_comment(line_str) {
                break;
            }

            // Check if it's specifically a UTF-8 encoding comment
            if is_utf8_encoding_comment(line_str) {
                let mut diag = self.diagnostic(
                    source,
                    i + 1,
                    0,
                    "Unnecessary utf-8 encoding comment.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    let end = std::cmp::min(byte_offset + line_len, total_len);
                    corr.push(crate::correction::Correction {
                        start: byte_offset,
                        end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }

            byte_offset += line_len;
        }
    }
}

/// Check if a comment line is a valid Ruby magic comment.
/// Matches RuboCop's MagicComment.parse(line).valid? behavior:
/// recognized types are encoding/coding, frozen_string_literal, shareable_constant_value,
/// typed (Sorbet), and rbs_inline.
fn is_magic_comment(line: &str) -> bool {
    let lower = line.to_lowercase();

    // Extract content after # (with optional space)
    let content = if let Some(rest) = lower.strip_prefix("# ") {
        rest.trim()
    } else if let Some(rest) = lower.strip_prefix('#') {
        rest.trim()
    } else {
        return false;
    };

    // Emacs style: -*- ... -*-
    if content.starts_with("-*-") && content.ends_with("-*-") {
        // Any emacs-style magic comment is valid
        return true;
    }

    // Vim style: vim: ...
    if content.starts_with("vim:") || content.starts_with("vim :") {
        return true;
    }

    // Simple magic comment keywords (case-insensitive)
    // encoding/coding, frozen_string_literal/frozen-string-literal,
    // shareable_constant_value/shareable-constant-value, typed, rbs_inline
    let magic_prefixes = &[
        "encoding:",
        "coding:",
        "frozen_string_literal:",
        "frozen-string-literal:",
        "shareable_constant_value:",
        "shareable-constant-value:",
        "typed:",
        "rbs_inline:",
    ];

    for prefix in magic_prefixes {
        if content.starts_with(prefix) {
            return true;
        }
    }

    false
}

/// Check if a comment line is a UTF-8 encoding magic comment.
fn is_utf8_encoding_comment(line: &str) -> bool {
    let lower = line.to_lowercase();

    // Standard magic comment formats:
    // # encoding: utf-8
    // # coding: utf-8
    // # -*- encoding: utf-8 -*-
    // # -*- coding: utf-8 -*-
    // # vim:fileencoding=utf-8
    // # vim: fileencoding=utf-8

    // Check for standard Ruby encoding/coding magic comment
    if let Some(rest) = lower.strip_prefix("# ").or_else(|| lower.strip_prefix("#")) {
        let rest = rest.trim();

        // Emacs style: -*- encoding: utf-8 -*- or -*- coding: utf-8 -*-
        if rest.starts_with("-*-") && rest.ends_with("-*-") {
            let inner = &rest[3..rest.len() - 3].trim();
            // Check if it contains encoding or coding with utf-8
            let inner_lower = inner.to_lowercase();
            if (inner_lower.contains("encoding") || inner_lower.contains("coding"))
                && contains_utf8(&inner_lower)
            {
                return true;
            }
            return false;
        }

        // vim style: vim:fileencoding=utf-8 or vim: fileencoding=utf-8
        if rest.starts_with("vim:") || rest.starts_with("vim :") {
            if contains_utf8(&lower) {
                return true;
            }
            return false;
        }

        // Standard format: encoding: utf-8, coding: utf-8
        // Also handle: Encoding: UTF-8
        if let Some(after) = strip_encoding_prefix(rest) {
            let value = after.trim();
            if value.eq_ignore_ascii_case("utf-8") {
                return true;
            }
        }
    }

    false
}

/// Strip the encoding/coding prefix and colon, returning the value part.
fn strip_encoding_prefix(s: &str) -> Option<&str> {
    let lower = s.to_lowercase();
    for prefix in &["encoding:", "coding:"] {
        if lower.starts_with(prefix) {
            return Some(&s[prefix.len()..]);
        }
    }
    None
}

/// Check if a string contains "utf-8" (case insensitive).
fn contains_utf8(s: &str) -> bool {
    s.contains("utf-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        Encoding,
        "cops/style/encoding",
        standard = "standard.rb",
        mixed_case = "mixed_case.rb",
        after_shebang = "after_shebang.rb",
        coding_format = "coding_format.rb",
    );

    #[test]
    fn autocorrect_remove_encoding() {
        let input = b"# encoding: utf-8\nx = 1\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&Encoding, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn no_fp_regular_comment_before_encoding() {
        // A regular comment (not a magic comment) on line 1 should stop processing.
        // RuboCop only processes contiguous magic comment lines.
        let input = b"# This is a regular comment\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag encoding comment after a regular (non-magic) comment: {:?}",
            diags
        );
    }

    #[test]
    fn flags_encoding_after_frozen_string_literal() {
        // frozen_string_literal IS a valid magic comment, so processing continues
        let input = b"# frozen_string_literal: true\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(
            diags.len(),
            1,
            "Should flag encoding comment after frozen_string_literal"
        );
    }

    #[test]
    fn flags_coding_format() {
        // # coding: utf-8 is a valid encoding magic comment
        let input = b"# coding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(diags.len(), 1, "Should flag # coding: utf-8");
    }

    #[test]
    fn no_fp_encoding_after_non_magic_comment_line2() {
        // Shebang is OK, but a regular comment on line 2 should stop processing
        let input =
            b"#!/usr/bin/env ruby\n# A description comment\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag encoding after non-magic comment: {:?}",
            diags
        );
    }

    #[test]
    fn flags_after_shebang_then_magic_comment() {
        // shebang → frozen_string_literal → encoding: all valid magic comments
        let input = b"#!/usr/bin/env ruby\n# frozen_string_literal: true\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(
            diags.len(),
            1,
            "Should flag encoding after shebang + frozen_string_literal"
        );
    }
}
