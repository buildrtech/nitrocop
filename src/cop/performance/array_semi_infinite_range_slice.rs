use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/ArraySemiInfiniteRangeSlice
///
/// Investigation: 1 FN in corpus — `0x1f0..` (hex integer literal in endless range).
/// Root cause: `is_positive_int()` used `str::parse::<i64>()` which only handles decimal.
/// Fix: parse hex (0x), binary (0b), octal (0o/0), and underscored integer literals.
pub struct ArraySemiInfiniteRangeSlice;

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

fn is_string_receiver(receiver: &ruby_prism::Node<'_>) -> bool {
    receiver.as_string_node().is_some()
        || receiver.as_interpolated_string_node().is_some()
        || receiver.as_x_string_node().is_some()
        || receiver.as_interpolated_x_string_node().is_some()
}

/// Parse Ruby integer literal source into i64.
/// Handles decimal, hex (0x), octal (0o/0), binary (0b), and underscores.
fn parse_ruby_int_literal(src: &str) -> Option<i64> {
    let stripped = src.replace('_', "");
    if stripped.is_empty() {
        return None;
    }

    if let Some(hex) = stripped.strip_prefix("0x").or(stripped.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    if let Some(bin) = stripped.strip_prefix("0b").or(stripped.strip_prefix("0B")) {
        return i64::from_str_radix(bin, 2).ok();
    }
    if let Some(oct) = stripped.strip_prefix("0o").or(stripped.strip_prefix("0O")) {
        return i64::from_str_radix(oct, 8).ok();
    }
    if stripped.starts_with('0')
        && stripped.len() > 1
        && stripped.chars().all(|c| c.is_ascii_digit())
    {
        // Legacy octal like 077
        return i64::from_str_radix(&stripped[1..], 8).ok();
    }

    stripped.parse::<i64>().ok()
}

/// Parse a positive integer literal node.
fn parse_positive_int(node: &ruby_prism::Node<'_>, source: &SourceFile) -> Option<i64> {
    let int_node = node.as_integer_node()?;
    let loc = int_node.location();
    let src = source.byte_slice(loc.start_offset(), loc.end_offset(), "");
    let value = parse_ruby_int_literal(src)?;
    (value > 0).then_some(value)
}

fn is_exclusive_range(range: &ruby_prism::RangeNode<'_>) -> bool {
    range.operator_loc().as_slice() == b"..."
}

enum SliceRewrite {
    Drop(i64),
    Take(i64),
}

/// Check if a range node is a semi-infinite range with a positive integer literal endpoint.
/// Returns rewrite action with computed integer amount.
fn semi_infinite_range_rewrite(
    range: &ruby_prism::RangeNode<'_>,
    source: &SourceFile,
) -> Option<SliceRewrite> {
    match (range.left(), range.right()) {
        // Endless range: N.. or N...
        (Some(left), None) => parse_positive_int(&left, source).map(SliceRewrite::Drop),

        // Beginless range: ..N or ...N
        (None, Some(right)) => {
            let end = parse_positive_int(&right, source)?;
            if is_exclusive_range(range) {
                Some(SliceRewrite::Take(end))
            } else {
                Some(SliceRewrite::Take(end + 1))
            }
        }
        _ => None,
    }
}

impl Cop for ArraySemiInfiniteRangeSlice {
    fn name(&self) -> &'static str {
        "Performance/ArraySemiInfiniteRangeSlice"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();
        let is_bracket = method_bytes == b"[]";
        let is_slice = method_bytes == b"slice";

        if !is_bracket && !is_slice {
            return;
        }

        // Skip string literal receivers
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if is_string_receiver(&receiver) {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();
        let range = match first_arg.as_range_node() {
            Some(r) => r,
            None => return,
        };

        let rewrite = match semi_infinite_range_rewrite(&range, source) {
            Some(r) => r,
            None => return,
        };

        let (prefer, replacement_call) = match rewrite {
            SliceRewrite::Drop(value) => ("drop", format!("drop({value})")),
            SliceRewrite::Take(value) => ("take", format!("take({value})")),
        };

        let method_display = if is_bracket { "[]" } else { "slice" };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `{prefer}` instead of `{method_display}` with a semi-infinite range."),
        );

        if let Some(ref mut corr) = corrections {
            let receiver_source = node_source(source, &receiver);
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: format!("{receiver_source}.{replacement_call}"),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        ArraySemiInfiniteRangeSlice,
        "cops/performance/array_semi_infinite_range_slice"
    );
    crate::cop_autocorrect_fixture_tests!(
        ArraySemiInfiniteRangeSlice,
        "cops/performance/array_semi_infinite_range_slice"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(ArraySemiInfiniteRangeSlice.supports_autocorrect());
    }
}
