use crate::cop::node_type::INTEGER_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for octal, hex, binary, and decimal literals using uppercase prefixes
/// and corrects them to lowercase prefix or no prefix (in case of decimals).
///
/// ## Investigation notes (2026-03-18)
///
/// **FP root cause:** `0_30` (underscore-separated decimal starting with 0) was
/// incorrectly flagged as an octal literal. The old code stripped all underscores
/// before matching, turning `0_30` into `030` which matched the octal pattern.
/// RuboCop's regexes (`/^0O?[0-7]+$/`) match the original source without stripping
/// underscores, so `0_30` correctly does NOT match because `_` is not in `[0-7]`.
/// Fix: match the original source text without stripping underscores.
///
/// **FN root cause:** Negative integer literals like `-0O1` and `-01234` were missed.
/// Prism includes the `-` sign in the `IntegerNode` location for negative literals,
/// so `src_str` started with `-` and none of the `starts_with("0...")` checks matched.
/// RuboCop's `integer_part` helper strips leading `+`/`-` before checking.
/// Fix: strip leading sign before prefix matching, adjust column offset by 1.
///
/// **FP root cause (complex/rational suffixes):** `042i` and `042r` are complex and
/// rational number literals, not plain octals. Prism parses these as `ImaginaryNode`
/// or `RationalNode` wrapping an `IntegerNode`. The AST walker visits the inner
/// `IntegerNode`, which has source text `042` (without suffix), matching the octal
/// pattern. RuboCop's `on_int` only fires for standalone `:int` nodes (Parser gem uses
/// distinct `:complex`/`:rational` types). Fix: check the byte after the `IntegerNode`
/// location — if it's `i` or `r`, skip the node.
pub struct NumericLiteralPrefix;

impl Cop for NumericLiteralPrefix {
    fn name(&self) -> &'static str {
        "Style/NumericLiteralPrefix"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTEGER_NODE]
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
        let int_node = match node.as_integer_node() {
            Some(i) => i,
            None => return,
        };

        let loc = int_node.location();
        let src = loc.as_slice();

        // Skip integer literals that are part of complex (042i) or rational (042r) literals.
        // Prism visits the IntegerNode child inside ImaginaryNode/RationalNode, but RuboCop's
        // on_int callback only fires for standalone int nodes (Parser gem uses :complex/:rational
        // node types, not :int). Check the byte after the IntegerNode location.
        let source_bytes = source.as_bytes();
        let end = loc.start_offset() + src.len();
        if end < source_bytes.len() {
            let next_byte = source_bytes[end];
            if next_byte == b'i' || next_byte == b'r' {
                return;
            }
        }
        let src_str = match std::str::from_utf8(src) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Strip leading +/- sign, like RuboCop's integer_part helper.
        // Do NOT strip underscores — RuboCop's regexes match the original source
        // including underscores, so `0_30` correctly does NOT match octal patterns.
        let (sign_prefix, literal, sign_offset) =
            if src_str.starts_with('+') || src_str.starts_with('-') {
                (&src_str[..1], &src_str[1..], 1usize)
            } else {
                ("", src_str, 0usize)
            };

        let enforced_octal_style = config.get_str("EnforcedOctalStyle", "zero_with_o");

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        // Offset the column past the sign character so the diagnostic points at
        // the numeric literal, not the sign.
        let col = column + sign_offset;

        let mut emit = |message: &str, replacement: String| {
            let mut diag = self.diagnostic(source, line, col, message.to_string());
            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        };

        // Check uppercase hex prefix: 0X...
        if literal.starts_with("0X") {
            emit(
                "Use 0x for hexadecimal literals.",
                format!("{sign_prefix}0x{}", &literal[2..]),
            );
            return;
        }

        // Check uppercase binary prefix: 0B...
        if literal.starts_with("0B") {
            emit(
                "Use 0b for binary literals.",
                format!("{sign_prefix}0b{}", &literal[2..]),
            );
            return;
        }

        // Check decimal prefix: 0d... or 0D...
        if literal.starts_with("0d") || literal.starts_with("0D") {
            emit(
                "Do not use prefixes for decimal literals.",
                format!("{sign_prefix}{}", &literal[2..]),
            );
            return;
        }

        // Octal handling
        if enforced_octal_style == "zero_with_o" {
            // Bad: 0O... (uppercase)
            if literal.starts_with("0O") {
                emit(
                    "Use 0o for octal literals.",
                    format!("{sign_prefix}0o{}", &literal[2..]),
                );
                return;
            }
            // Bad: plain 0... without 'o' (e.g., 01234)
            // Must be octal: starts with 0, followed by digits 0-7 only, not 0x/0b/0d/0o
            // Underscores in the source mean it's a decimal with visual separators (e.g. 0_30),
            // not an octal literal.
            if literal.len() > 1
                && literal.starts_with('0')
                && !literal.starts_with("0x")
                && !literal.starts_with("0b")
                && !literal.starts_with("0o")
                && literal[1..].bytes().all(|b| b.is_ascii_digit() && b < b'8')
            {
                emit(
                    "Use 0o for octal literals.",
                    format!("{sign_prefix}0o{}", &literal[1..]),
                );
            }
        } else if enforced_octal_style == "zero_only" {
            // Bad: 0o... or 0O...
            if literal.starts_with("0o") || literal.starts_with("0O") {
                emit(
                    "Use 0 for octal literals.",
                    format!("{sign_prefix}0{}", &literal[2..]),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumericLiteralPrefix, "cops/style/numeric_literal_prefix");
    crate::cop_autocorrect_fixture_tests!(
        NumericLiteralPrefix,
        "cops/style/numeric_literal_prefix"
    );
}
