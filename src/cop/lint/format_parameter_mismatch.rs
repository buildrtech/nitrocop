use crate::cop::node_type::{
    ARRAY_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE,
    INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, SPLAT_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for mismatch between format string fields and arguments.
///
/// ## Corpus conformance investigation (2026-03-11)
///
/// **Root causes of FPs (54):**
/// 1. Heredoc format strings — RuboCop explicitly skips heredocs via `heredoc?` check
///    (source starts with `<<`). nitrocop was trying to parse heredoc content.
/// 2. Interpolated string (dstr) with zero format fields — RuboCop skips when
///    `expected_fields == 0 && first_arg.type?(:dstr, :array)`. Common with
///    `format("#{foo}", bar, baz)` where the interpolation IS the format.
/// 3. Zero fields + array RHS in String#% — `"text" % [value]` where string has
///    no format sequences. RuboCop skips when fields=0 and arg is array.
/// 4. format/sprintf with only 1 arg (just the format string, no extra args) —
///    RuboCop requires `arguments.size > 1` to consider it a format call.
///
/// **Fixes applied (round 1):**
/// - Skip heredoc format strings (check opening_loc starts with `<<`)
/// - Require args.len() > 1 for format/sprintf (matches RuboCop)
/// - Skip when zero fields AND format string is interpolated (dstr)
/// - Skip when zero fields AND RHS is array for String#%
///
/// ## Additional conformance investigation (2026-03-11)
///
/// **Root causes of remaining FPs (35) and FNs (13):**
/// 1. Format type character acceptance too broad — nitrocop was treating ANY
///    alphabetic character after `%` as a valid format type. RuboCop only accepts
///    `[bBdiouxXeEfgGaAcps]`. Characters like `%v`, `%n`, `%t`, `%r` are NOT
///    valid Ruby format types. This caused both FPs (over-counting fields) and
///    FNs (wrong field count masking mismatches).
/// 2. String#% splat handling — nitrocop special-cased splats in array RHS,
///    only firing when splat count > expected fields. RuboCop does NOT
///    special-case splats for `%` (only for format/sprintf). It counts child
///    nodes literally and compares directly.
/// 3. Numbered format without valid type — `%1$n` was counted as numbered
///    format even though `n` is not a valid type. RuboCop's SEQUENCE regex
///    requires TYPE at the end for numbered formats.
/// 4. Annotated named format without valid type — `%<name>` without a
///    following type character was counted as named. RuboCop requires TYPE
///    after `%<name>` (only template `%{name}` format has no TYPE requirement).
///
/// **Fixes applied (round 2):**
/// - Restrict format type to `[bBdiouxXeEfgGaAcps]` via `is_format_type()`
/// - Remove splat special-casing for String#% (literal count like RuboCop)
/// - Require valid type for numbered (`%N$X`) and annotated named (`%<name>X`)
pub struct FormatParameterMismatch;

impl Cop for FormatParameterMismatch {
    fn name(&self) -> &'static str {
        "Lint/FormatParameterMismatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            INTERPOLATED_STRING_NODE,
            KEYWORD_HASH_NODE,
            SPLAT_NODE,
            STRING_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for format/sprintf (bare or Kernel.method)
        if (method_name == b"format" || method_name == b"sprintf") && is_format_call(&call) {
            diagnostics.extend(check_format_sprintf(self, source, &call, method_name));
            return;
        }

        // Check for String#% operator
        if method_name == b"%" && call.receiver().is_some() {
            diagnostics.extend(check_string_percent(self, source, &call));
        }
    }
}

/// Returns true if this is a `format(...)` / `sprintf(...)` call (bare or Kernel.format)
fn is_format_call(call: &ruby_prism::CallNode<'_>) -> bool {
    match call.receiver() {
        None => true,
        Some(recv) => {
            recv.as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Kernel")
                || recv
                    .as_constant_path_node()
                    .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"Kernel"))
        }
    }
}

fn check_format_sprintf(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    method_name: &[u8],
) -> Vec<Diagnostic> {
    let args = match call.arguments() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
    // RuboCop requires arguments.size > 1 (format string + at least one arg)
    if arg_list.len() <= 1 {
        return Vec::new();
    }

    let first = &arg_list[0];

    // Skip heredoc format strings (RuboCop behavior)
    if is_heredoc_node(first) {
        return Vec::new();
    }

    // Format string must be a string literal (or interpolated string)
    let fmt_str = extract_format_string(first);
    let fmt_str = match fmt_str {
        Some(s) => s,
        None => return Vec::new(), // Variable or non-literal — can't check
    };

    // If the format string contains interpolation that could affect format sequences, bail
    if fmt_str.contains_interpolation {
        // Still try to count sequences that don't depend on interpolation
        // but if we can't determine the count reliably, bail
        if fmt_str.has_format_affecting_interpolation {
            return Vec::new();
        }
    }

    // Count remaining args (excluding the format string)
    let remaining_args = &arg_list[1..];

    // If any remaining arg is a splat, be conservative for format/sprintf
    let has_splat = remaining_args.iter().any(|a| a.as_splat_node().is_some());

    let arg_count = remaining_args.len();

    // Parse format sequences
    let parse_result = parse_format_string(&fmt_str.value);
    match parse_result {
        FormatParseResult::Fields(field_count) => {
            // When expected fields is zero and format string is interpolated (dstr)
            // or first arg is array, skip — matches RuboCop's behavior where dynamic
            // content may contain the actual format sequences at runtime
            if field_count.count == 0
                && !field_count.named
                && (fmt_str.contains_interpolation || first.as_array_node().is_some())
            {
                return Vec::new();
            }

            // For named formats (%{name} or %<name>), expect exactly 1 hash arg
            if field_count.named {
                if arg_count != 1 {
                    let method_str = std::str::from_utf8(method_name).unwrap_or("format");
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![cop.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Number of arguments ({}) to `{}` doesn't match the number of fields ({}).",
                            arg_count, method_str, 1
                        ),
                    )];
                }
                return Vec::new();
            }

            if has_splat {
                // With splat, can't know exact count — skip
                return Vec::new();
            }

            if arg_count != field_count.count {
                let method_str = std::str::from_utf8(method_name).unwrap_or("format");
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![cop.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Number of arguments ({}) to `{}` doesn't match the number of fields ({}).",
                        arg_count, method_str, field_count.count
                    ),
                )];
            }
        }
        FormatParseResult::Invalid => {
            let _method_str = std::str::from_utf8(method_name).unwrap_or("format");
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.".to_string(),
            )];
        }
    }

    Vec::new()
}

fn check_string_percent(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Vec<Diagnostic> {
    let receiver = call.receiver().unwrap();

    // Skip heredoc receivers (RuboCop behavior)
    if is_heredoc_node(&receiver) {
        return Vec::new();
    }

    // Receiver must be a string literal
    let fmt_str = extract_format_string(&receiver);
    let fmt_str = match fmt_str {
        Some(s) => s,
        None => return Vec::new(),
    };

    if fmt_str.contains_interpolation && fmt_str.has_format_affecting_interpolation {
        return Vec::new();
    }

    let args = match call.arguments() {
        Some(a) => a,
        None => return Vec::new(),
    };
    let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return Vec::new();
    }

    let rhs = &arg_list[0];

    // Parse format sequences
    let parse_result = parse_format_string(&fmt_str.value);
    match parse_result {
        FormatParseResult::Fields(field_count) => {
            if field_count.named {
                // Named formats expect a hash — don't check further
                return Vec::new();
            }

            // When expected fields is zero and first arg is dstr or array,
            // skip — matches RuboCop's offending_node? guard
            if field_count.count == 0
                && (rhs.as_array_node().is_some() || rhs.as_interpolated_string_node().is_some())
            {
                return Vec::new();
            }

            // Also skip when format string is interpolated (dstr) with zero fields
            if field_count.count == 0 && fmt_str.contains_interpolation {
                return Vec::new();
            }

            // RHS must be an array literal for us to check count
            let array_elements = match rhs.as_array_node() {
                Some(arr) => {
                    let elems: Vec<ruby_prism::Node<'_>> = arr.elements().iter().collect();
                    elems
                }
                None => {
                    // Single non-array argument — could be a variable that evaluates to array
                    // For Hash literals, skip (named format)
                    if rhs.as_hash_node().is_some() || rhs.as_keyword_hash_node().is_some() {
                        return Vec::new();
                    }
                    return Vec::new();
                }
            };

            let arg_count = array_elements.len();

            // RuboCop does NOT special-case splats for String#% —
            // it just counts child nodes literally (including splat nodes)
            // and compares against expected fields
            if arg_count != field_count.count {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![cop.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Number of arguments ({}) to `String#%` doesn't match the number of fields ({}).",
                        arg_count, field_count.count
                    ),
                )];
            }
        }
        FormatParseResult::Invalid => {
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.".to_string(),
            )];
        }
    }

    Vec::new()
}

/// Returns true if the node is a heredoc string (opening starts with `<<`).
fn is_heredoc_node(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(str_node) = node.as_interpolated_string_node() {
        if let Some(open) = str_node.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    if let Some(str_node) = node.as_string_node() {
        if let Some(open) = str_node.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    false
}

struct FormatString {
    value: String,
    contains_interpolation: bool,
    has_format_affecting_interpolation: bool,
}

fn extract_format_string(node: &ruby_prism::Node<'_>) -> Option<FormatString> {
    if let Some(s) = node.as_string_node() {
        let val = s.unescaped();
        return Some(FormatString {
            value: String::from_utf8_lossy(val).to_string(),
            contains_interpolation: false,
            has_format_affecting_interpolation: false,
        });
    }

    if let Some(interp) = node.as_interpolated_string_node() {
        let mut result = String::new();
        let mut has_interp = false;
        let mut format_affecting = false;
        for part in interp.parts().iter() {
            if let Some(s) = part.as_string_node() {
                let val = s.unescaped();
                result.push_str(&String::from_utf8_lossy(val));
            } else {
                has_interp = true;
                // Check if the interpolation could affect format parsing
                // If the string part right before this ends with `%` or `%-` etc.,
                // the interpolation could be part of a format specifier
                // Check if the string part before interpolation ends with a
                // partial format specifier. Covers all flag chars: - + space 0 #
                if result.ends_with('%')
                    || result.ends_with("%-")
                    || result.ends_with("%+")
                    || result.ends_with("%0")
                    || result.ends_with("%.")
                    || result.ends_with("%#")
                    || result.ends_with("% ")
                {
                    format_affecting = true;
                }
            }
        }
        return Some(FormatString {
            value: result,
            contains_interpolation: has_interp,
            has_format_affecting_interpolation: format_affecting,
        });
    }

    None
}

struct FieldCount {
    count: usize,
    named: bool,
}

enum FormatParseResult {
    Fields(FieldCount),
    Invalid,
}

/// Returns true if the byte is a valid Ruby format conversion type character.
/// Matches RuboCop's FormatString::TYPE = [bBdiouxXeEfgGaAcps]
fn is_format_type(b: u8) -> bool {
    matches!(
        b,
        b'b' | b'B'
            | b'd'
            | b'i'
            | b'o'
            | b'u'
            | b'x'
            | b'X'
            | b'e'
            | b'E'
            | b'f'
            | b'g'
            | b'G'
            | b'a'
            | b'A'
            | b'c'
            | b'p'
            | b's'
    )
}

fn parse_format_string(fmt: &str) -> FormatParseResult {
    let bytes = fmt.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut count = 0;
    let mut has_numbered = false;
    let mut has_unnumbered = false;
    let mut has_named = false;
    let mut max_numbered = 0;

    while i < len {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        i += 1; // skip '%'

        if i >= len {
            break;
        }

        // `%%` is a literal percent
        if bytes[i] == b'%' {
            i += 1;
            continue;
        }

        // Named format: %{name} or %<name>
        if bytes[i] == b'{' {
            has_named = true;
            // Skip to closing }
            while i < len && bytes[i] != b'}' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'<' {
            // Skip to closing >
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
                // After >, may have more_flags, width, precision before TYPE
                // Skip flags
                while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
                    i += 1;
                }
                // Skip width
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                // Skip precision
                if i < len && bytes[i] == b'.' {
                    i += 1;
                    while i < len && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                // TYPE is required for %<name>X format (annotated named)
                if i < len && is_format_type(bytes[i]) {
                    has_named = true;
                    i += 1;
                }
            }
            continue;
        }

        // Check for numbered: %1$s, %2$d, etc.
        // Flags, width, precision, then conversion
        let start = i;
        // Skip flags
        while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            i += 1;
        }

        // Check for `*` (dynamic width — counts as an extra arg)
        let mut extra_args = 0;
        if i < len && bytes[i] == b'*' {
            extra_args += 1;
            i += 1;
        } else {
            // Skip width digits
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Check for `$` (numbered argument)
        if i < len && bytes[i] == b'$' {
            // This is a numbered format like %1$s
            // Extract the number
            let num_str = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
            // Remove any flag characters from the front to get the number
            let num_part: String = num_str.chars().filter(|c| c.is_ascii_digit()).collect();
            let parsed_num = num_part.parse::<usize>().ok();
            i += 1; // skip '$'
            // Skip the rest of the format specifier after $
            // Skip flags again
            while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
                i += 1;
            }
            // Skip width (could be * for dynamic width with numbered ref)
            if i < len && bytes[i] == b'*' {
                i += 1;
                // Skip optional digit_dollar for width arg: *N$
                let w_start = i;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i < len && bytes[i] == b'$' {
                    i += 1;
                } else {
                    i = w_start; // not a digit_dollar, reset
                    // but still consumed the *
                }
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Skip precision
            if i < len && bytes[i] == b'.' {
                i += 1;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Conversion type must be valid — only then count as numbered format
            // (matches RuboCop: SEQUENCE regex requires TYPE at end)
            if i < len && is_format_type(bytes[i]) {
                if let Some(n) = parsed_num {
                    has_numbered = true;
                    if n > max_numbered {
                        max_numbered = n;
                    }
                }
                i += 1;
            }
            continue;
        }

        // Skip precision
        if i < len && bytes[i] == b'.' {
            i += 1;
            if i < len && bytes[i] == b'*' {
                extra_args += 1;
                i += 1;
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }

        // Conversion specifier — must be a valid Ruby format type.
        // RuboCop's FormatString::TYPE = [bBdiouxXeEfgGaAcps]
        // If no valid type follows, this is NOT a format sequence at all
        // (the preceding flags/width/precision/star are just literal text).
        if i < len && is_format_type(bytes[i]) {
            has_unnumbered = true;
            count += 1 + extra_args;
            i += 1;
        }
    }

    // Check for mixing
    let mix_count = [has_named, has_numbered, has_unnumbered]
        .iter()
        .filter(|&&b| b)
        .count();
    if mix_count > 1 {
        return FormatParseResult::Invalid;
    }

    if has_named {
        return FormatParseResult::Fields(FieldCount {
            count: 1,
            named: true,
        });
    }

    if has_numbered {
        return FormatParseResult::Fields(FieldCount {
            count: max_numbered,
            named: false,
        });
    }

    FormatParseResult::Fields(FieldCount {
        count,
        named: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FormatParameterMismatch,
        "cops/lint/format_parameter_mismatch"
    );
}
