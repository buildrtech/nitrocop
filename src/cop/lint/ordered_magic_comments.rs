use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that encoding magic comments precede frozen_string_literal comments.
///
/// ## Investigation notes
///
/// FP=3 were caused by overly loose encoding comment detection. The `is_encoding_comment`
/// function used substring matching (`windows().any()`) which matched `encoding: ` or
/// `coding: ` anywhere in a comment line. This caused false positives in two scenarios:
///
/// 1. **Double space after colon**: `# encoding:  utf-8` (two spaces) is not recognized
///    by RuboCop's `SimpleComment` regex which expects exactly `coding: ` (one space)
///    before the token. Found in hitobito repo (2 FPs).
///
/// 2. **Emacs `ruby encoding` prefix**: `# -*- ruby encoding: us-ascii -*-` has `ruby`
///    before `encoding`, so RuboCop's emacs token parser doesn't match it as an encoding
///    token (tokens are split by `;` and must start with `(en)?coding`). Found in ftpd repo (1 FP).
///
/// Fix: rewrote `is_encoding_comment` to match RuboCop's `MagicComment` parsing rules:
/// - Simple format requires `(en)?coding: ` at the start of comment body with single space
/// - Emacs format requires `(en)?coding` at the start of a `;`-delimited token
pub struct OrderedMagicComments;

impl Cop for OrderedMagicComments {
    fn name(&self) -> &'static str {
        "Lint/OrderedMagicComments"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut encoding_line: Option<usize> = None;
        let mut frozen_string_line: Option<usize> = None;

        for (i, line) in source.lines().enumerate() {
            let line_num = i + 1; // 1-indexed

            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &line[start..])
                .unwrap_or(&[]);

            if trimmed.is_empty() {
                continue;
            }

            // Skip shebang
            if trimmed.starts_with(b"#!") {
                continue;
            }

            // Stop at first non-comment line
            if !trimmed.starts_with(b"#") {
                break;
            }

            let comment = &trimmed[1..]; // skip #
            let comment_trimmed = comment
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &comment[start..])
                .unwrap_or(&[]);

            // Handle emacs-style: -*- coding: utf-8 -*-
            let comment_lower: Vec<u8> = comment_trimmed
                .iter()
                .map(|b| b.to_ascii_lowercase())
                .collect();

            if is_encoding_comment(&comment_lower) && encoding_line.is_none() {
                encoding_line = Some(line_num);
            } else if is_frozen_string_comment(&comment_lower) && frozen_string_line.is_none() {
                frozen_string_line = Some(line_num);
            }

            if encoding_line.is_some() && frozen_string_line.is_some() {
                break;
            }
        }

        if let (Some(enc_line), Some(fsl_line)) = (encoding_line, frozen_string_line) {
            if enc_line > fsl_line {
                // Encoding comment appears after frozen_string_literal
                let mut diag = self.diagnostic(
                    source,
                    enc_line,
                    0,
                    "The encoding magic comment should precede all other magic comments."
                        .to_string(),
                );

                if let Some(corr) = corrections.as_mut() {
                    if let (Some((fsl_start, fsl_end)), Some((enc_start, enc_end))) = (
                        line_range_without_newline(source, fsl_line),
                        line_range_without_newline(source, enc_line),
                    ) {
                        let fsl_text = String::from_utf8_lossy(&source.as_bytes()[fsl_start..fsl_end]);
                        let enc_text = String::from_utf8_lossy(&source.as_bytes()[enc_start..enc_end]);

                        corr.push(crate::correction::Correction {
                            start: fsl_start,
                            end: fsl_end,
                            replacement: enc_text.to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        corr.push(crate::correction::Correction {
                            start: enc_start,
                            end: enc_end,
                            replacement: fsl_text.to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }

                diagnostics.push(diag);
            }
        }
    }
}

fn line_range_without_newline(source: &SourceFile, line_num: usize) -> Option<(usize, usize)> {
    let line = source.lines().nth(line_num.checked_sub(1)?)?;
    let start = source.line_start_offset(line_num);
    Some((start, start + line.len()))
}

fn is_encoding_comment(lower: &[u8]) -> bool {
    // Match encoding magic comments following RuboCop's MagicComment parsing rules.
    //
    // Simple format: `# encoding: utf-8` or `# coding: utf-8`
    //   - (en)?coding must appear at the start of the comment body (after `# `)
    //   - Exactly one space after the colon (RuboCop regex uses literal `: ` + TOKEN)
    //   - An optional frozen_string_literal prefix is allowed
    //   - `# encoding:  utf-8` (double space) does NOT match
    //
    // Emacs format: `# -*- coding: utf-8 -*-`
    //   - (en)?coding must be at the start of a `;`-delimited token inside `-*-...-*-`
    //   - `# -*- ruby encoding: us-ascii -*-` does NOT match (token starts with `ruby`)
    if is_emacs_encoding_comment(lower) {
        return true;
    }
    is_simple_encoding_comment(lower)
}

/// Check if comment body matches simple encoding format: `(en)?coding: <token>`
/// The comment body must start with the encoding keyword (optionally preceded by
/// a frozen_string_literal declaration).
fn is_simple_encoding_comment(lower: &[u8]) -> bool {
    // Try matching at the start, or after a frozen_string_literal prefix
    starts_with_coding_token(lower)
        || lower
            .windows(22)
            .position(|w| w == b"frozen_string_literal:" || w == b"frozen-string-literal:")
            .map(|pos| {
                // Skip past the frozen_string_literal value to find coding
                let rest = &lower[pos + 22..];
                // Skip whitespace + value (true/false) + whitespace
                let rest = skip_fsl_value(rest);
                starts_with_coding_token(rest)
            })
            .unwrap_or(false)
}

/// Check if the byte slice starts with `(en)?coding: ` followed by an alphanumeric token.
fn starts_with_coding_token(s: &[u8]) -> bool {
    let after_keyword = if s.starts_with(b"encoding: ") {
        Some(&s[10..])
    } else if s.starts_with(b"coding: ") {
        Some(&s[8..])
    } else {
        None
    };
    // Verify the character after `: ` is alphanumeric (part of the encoding name token)
    after_keyword.is_some_and(|rest| rest.first().is_some_and(|&b| b.is_ascii_alphanumeric()))
}

/// Skip past whitespace + true/false + whitespace in a frozen_string_literal value.
fn skip_fsl_value(s: &[u8]) -> &[u8] {
    let s = skip_whitespace(s);
    let s = if s.starts_with(b"true") {
        &s[4..]
    } else if s.starts_with(b"false") {
        &s[5..]
    } else {
        return s;
    };
    skip_whitespace(s)
}

fn skip_whitespace(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(s.len());
    &s[start..]
}

/// Check if comment body matches emacs encoding format: `-*- coding: utf-8 -*-`
/// Each `;`-delimited token inside the `-*-` markers is checked.
fn is_emacs_encoding_comment(lower: &[u8]) -> bool {
    // Find `-*-` markers
    let start = match lower.windows(3).position(|w| w == b"-*-") {
        Some(pos) => pos + 3,
        None => return false,
    };
    let rest = &lower[start..];
    // Find closing `-*-`
    let end = match rest.windows(3).position(|w| w == b"-*-") {
        Some(pos) => pos,
        None => return false,
    };
    let content = &rest[..end];

    // Split by `;` and check each token
    for token in content.split(|&b| b == b';') {
        let trimmed = skip_whitespace(token);
        if trimmed.starts_with(b"encoding") || trimmed.starts_with(b"coding") {
            // Check for `: ` or `:` followed by whitespace then value
            let after = if trimmed.starts_with(b"encoding") {
                &trimmed[8..]
            } else {
                &trimmed[6..]
            };
            let after = skip_whitespace(after);
            if after.starts_with(b":") {
                let after_colon = skip_whitespace(&after[1..]);
                if after_colon
                    .first()
                    .is_some_and(|&b| b.is_ascii_alphanumeric())
                {
                    return true;
                }
            }
        }
    }
    false
}

fn is_frozen_string_comment(lower: &[u8]) -> bool {
    lower.windows(22).any(|w| w == b"frozen_string_literal:")
        || lower.windows(22).any(|w| w == b"frozen-string-literal:")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        OrderedMagicComments,
        "cops/lint/ordered_magic_comments",
        basic = "basic.rb",
        with_coding = "with_coding.rb",
        with_shebang = "with_shebang.rb",
    );

    #[test]
    fn autocorrect_basic_scenario() {
        crate::testutil::assert_cop_autocorrect(
            &OrderedMagicComments,
            include_bytes!("../../../tests/fixtures/cops/lint/ordered_magic_comments/offense/basic.rb"),
            b"# encoding: ascii\n# frozen_string_literal: true\np 'hello'\n",
        );
    }

    #[test]
    fn autocorrect_with_coding_scenario() {
        crate::testutil::assert_cop_autocorrect(
            &OrderedMagicComments,
            include_bytes!("../../../tests/fixtures/cops/lint/ordered_magic_comments/offense/with_coding.rb"),
            b"# coding: utf-8\n# frozen_string_literal: true\nx = 1\n",
        );
    }

    #[test]
    fn autocorrect_with_shebang_scenario() {
        crate::testutil::assert_cop_autocorrect(
            &OrderedMagicComments,
            include_bytes!("../../../tests/fixtures/cops/lint/ordered_magic_comments/offense/with_shebang.rb"),
            b"#!/usr/bin/env ruby\n# encoding: utf-8\n# frozen_string_literal: true\nputs 'hi'\n",
        );
    }

    #[test]
    fn no_offense_encoding_after_regular_comments() {
        // encoding comment after a copyright block is not a magic comment
        let diags = crate::testutil::run_cop_full(
            &OrderedMagicComments,
            b"# frozen_string_literal: true\n\
              \n\
              #  Copyright (c) 2012-2024, Example Corp.\n\
              #  Licensed under the MIT License.\n\
              \n\
              # encoding:  utf-8\n\
              \n\
              require \"spec_helper\"\n",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for encoding after regular comments, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_emacs_ruby_encoding_after_blank_line() {
        // `# -*- ruby encoding: us-ascii -*-` is not a valid encoding magic comment
        // because 'ruby encoding' is not recognized by RuboCop's emacs parser
        let diags = crate::testutil::run_cop_full(
            &OrderedMagicComments,
            b"# frozen_string_literal: true\n\
              \n\
              # -*- ruby encoding: us-ascii -*-\n\
              \n\
              module Ftpd\nend\n",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for emacs ruby encoding comment, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_encoding_double_space_after_colon() {
        // `# encoding:  utf-8` with double space is not recognized by RuboCop
        let diags = crate::testutil::run_cop_full(
            &OrderedMagicComments,
            b"# frozen_string_literal: true\n\
              # encoding:  utf-8\n\
              p 'hello'\n",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for encoding with double space, got: {:?}",
            diags
        );
    }
}
