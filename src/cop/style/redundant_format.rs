use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, SPLAT_NODE,
    STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (FP=3, FN=37 in standard corpus; FP=3, FN=70 in extended):
///
/// FP root cause: `format 'text/plain', &:inspect` — format called with a block
/// argument (`&block`). nitrocop saw 1 positional string arg and flagged it, but
/// the block argument makes this a different kind of call. Fixed by checking
/// `call.block().is_some()` and skipping when a block is present.
///
/// FN root causes:
/// 1. `format(CONSTANT)` — single constant argument (ConstantReadNode/ConstantPathNode).
///    nitrocop registered interest in these nodes but only checked string/dstr in the match.
///    Fixed by handling constant nodes as valid single-arg patterns.
/// 2. `format('%s %s', 'foo', 'bar')` — multi-arg format calls where all format args are
///    string/symbol/numeric/boolean/nil literals. This is the `detect_unnecessary_fields`
///    method in vendor RuboCop. Implemented basic %s-only multi-arg detection with string,
///    symbol, integer, float, true, false, nil literals. Also handles width-padded %s like
///    `%-40s` and %d/%i/%u with integer literals.
/// 3. Splat check was wrong — checked the single arg node itself instead of iterating args
///    for SplatNode presence. Also need to check `call.block()` for block_argument (`&`).
pub struct RedundantFormat;

/// Check if a node is a literal that can be converted to a string via %s.
/// Matches vendor's ACCEPTABLE_LITERAL_TYPES = %i[str dstr sym dsym numeric boolean nil]
fn is_string_convertible_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
}

/// Check if a node is an integer literal (can be used with %d/%i/%u).
fn is_integer_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some() {
        return true;
    }
    // String literals that parse as integers
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        if let Ok(text) = std::str::from_utf8(content) {
            return text.parse::<i64>().is_ok();
        }
    }
    false
}

/// Get the string representation of a literal for %s formatting.
fn literal_to_string(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        return std::str::from_utf8(content).ok().map(|s| s.to_string());
    }
    if let Some(sym) = node.as_symbol_node() {
        let val = sym.unescaped().as_slice();
        return std::str::from_utf8(val).ok().map(|s| s.to_string());
    }
    if let Some(int) = node.as_integer_node() {
        return Some(int.value().to_string());
    }
    if let Some(f) = node.as_float_node() {
        // Ruby's %s on a float uses the default to_s
        return Some(format!("{}", f.value()));
    }
    if node.as_true_node().is_some() {
        return Some("true".to_string());
    }
    if node.as_false_node().is_some() {
        return Some("false".to_string());
    }
    if node.as_nil_node().is_some() {
        return Some("".to_string());
    }
    // For rational, imaginary, interpolated strings/symbols — too complex to inline
    None
}

/// Get integer value from a literal for %d/%i/%u formatting.
fn literal_to_integer(node: &ruby_prism::Node<'_>) -> Option<i64> {
    if let Some(int) = node.as_integer_node() {
        return Some(int.value());
    }
    if let Some(f) = node.as_float_node() {
        return Some(f.value() as i64);
    }
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        if let Ok(text) = std::str::from_utf8(content) {
            return text.parse::<i64>().ok();
        }
    }
    None
}

/// Represents a parsed format specifier in a format string.
struct FormatSpec {
    /// The type character: 's', 'd', 'i', 'u', 'f', etc.
    spec_type: u8,
    /// Width (if specified), negative means left-aligned
    width: Option<i32>,
    /// Precision (if specified)
    precision: Option<usize>,
    /// Flags
    flags: Vec<u8>,
}

/// Parse format specifiers from a format string. Returns None if the string
/// contains specifiers we can't handle (annotated, template, variable width, etc).
/// Returns Some(vec) of simple specifiers if all are handleable.
fn parse_simple_format_specs(fmt: &str) -> Option<Vec<FormatSpec>> {
    let bytes = fmt.as_bytes();
    let mut specs = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }

        i += 1;
        if i >= bytes.len() {
            return None;
        }

        // %% is literal
        if bytes[i] == b'%' {
            i += 1;
            continue;
        }

        // Reject annotated (%<name>...) and template (%{name})
        if bytes[i] == b'<' || bytes[i] == b'{' {
            return None;
        }

        // Reject positional (N$)
        let pos_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > pos_start && i < bytes.len() && bytes[i] == b'$' {
            return None; // positional args are complex
        }
        i = pos_start;

        // Parse flags
        let mut flags = Vec::new();
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            flags.push(bytes[i]);
            i += 1;
        }

        // Parse width
        let mut width: Option<i32> = None;
        if i < bytes.len() && bytes[i] == b'*' {
            return None; // variable width — too complex
        }
        let w_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > w_start {
            let w_str = std::str::from_utf8(&bytes[w_start..i]).ok()?;
            let w: i32 = w_str.parse().ok()?;
            if flags.contains(&b'-') {
                width = Some(-w);
            } else {
                width = Some(w);
            }
        }

        // Parse precision
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'*' {
                return None; // variable precision
            }
            let p_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > p_start {
                let p_str = std::str::from_utf8(&bytes[p_start..i]).ok()?;
                precision = Some(p_str.parse().ok()?);
            } else {
                precision = Some(0);
            }
        }

        // Type character
        if i >= bytes.len() {
            return None;
        }
        let spec_type = bytes[i];
        if !matches!(
            spec_type,
            b's' | b'd'
                | b'i'
                | b'u'
                | b'f'
                | b'g'
                | b'e'
                | b'x'
                | b'X'
                | b'o'
                | b'b'
                | b'B'
                | b'c'
                | b'p'
                | b'a'
                | b'A'
                | b'E'
                | b'G'
        ) {
            return None;
        }
        i += 1;

        specs.push(FormatSpec {
            spec_type,
            width,
            precision,
            flags,
        });
    }

    Some(specs)
}

/// Format a single value according to a format spec, producing the literal result string.
fn format_value(spec: &FormatSpec, node: &ruby_prism::Node<'_>) -> Option<String> {
    match spec.spec_type {
        b's' => {
            // %s — convert to string
            let val = literal_to_string(node)?;
            apply_width_padding(&val, spec)
        }
        b'd' | b'i' | b'u' => {
            // %d/%i/%u — integer formatting
            if !is_integer_literal(node) {
                return None;
            }
            let int_val = literal_to_integer(node)?;
            let mut formatted = if spec.flags.contains(&b'+') && int_val >= 0 {
                format!("+{}", int_val)
            } else if spec.flags.contains(&b' ') && int_val >= 0 {
                format!(" {}", int_val)
            } else {
                format!("{}", int_val)
            };

            // Apply precision (minimum digits)
            if let Some(prec) = spec.precision {
                let is_neg = int_val < 0;
                let digits: String = if is_neg {
                    formatted[1..].to_string()
                } else if formatted.starts_with('+') || formatted.starts_with(' ') {
                    formatted[1..].to_string()
                } else {
                    formatted.clone()
                };
                if digits.len() < prec {
                    let padded = format!("{:0>width$}", digits, width = prec);
                    if is_neg {
                        formatted = format!("-{}", padded);
                    } else if formatted.starts_with('+') {
                        formatted = format!("+{}", padded);
                    } else if formatted.starts_with(' ') {
                        formatted = format!(" {}", padded);
                    } else {
                        formatted = padded;
                    }
                } else if prec == 0 && int_val == 0 {
                    // %.d with value 0 produces empty string
                    formatted = String::new();
                }
            }

            // Apply zero-padding (flag '0') with width
            if spec.flags.contains(&b'0') && !spec.flags.contains(&b'-') {
                if let Some(w) = spec.width {
                    let w = w.unsigned_abs() as usize;
                    if formatted.len() < w {
                        let is_neg = formatted.starts_with('-');
                        let digits = if is_neg { &formatted[1..] } else { &formatted };
                        let padded = format!(
                            "{:0>width$}",
                            digits,
                            width = w - if is_neg { 1 } else { 0 }
                        );
                        formatted = if is_neg {
                            format!("-{}", padded)
                        } else {
                            padded
                        };
                    }
                }
            }

            apply_width_padding(&formatted, spec)
        }
        b'f' => {
            // %f — float formatting
            let float_val = if let Some(int) = node.as_integer_node() {
                int.value() as f64
            } else if let Some(f) = node.as_float_node() {
                f.value()
            } else if let Some(s) = node.as_string_node() {
                let content = s.content_loc().as_slice();
                let text = std::str::from_utf8(content).ok()?;
                text.parse::<f64>().ok()?
            } else {
                return None;
            };

            let prec = spec.precision.unwrap_or(6);
            let formatted = format!("{:.prec$}", float_val, prec = prec);
            apply_width_padding(&formatted, spec)
        }
        _ => None, // Other types not supported for inlining
    }
}

/// Apply width padding to a formatted string.
fn apply_width_padding(s: &str, spec: &FormatSpec) -> Option<String> {
    if let Some(w) = spec.width {
        let abs_w = w.unsigned_abs() as usize;
        if s.len() < abs_w {
            if w < 0 || spec.flags.contains(&b'-') {
                // Left-aligned
                Some(format!("{:<width$}", s, width = abs_w))
            } else {
                // Right-aligned
                Some(format!("{:>width$}", s, width = abs_w))
            }
        } else {
            Some(s.to_string())
        }
    } else {
        Some(s.to_string())
    }
}

/// Check if any argument in the arg list is a splat or double-splat.
fn has_splat_arg(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        if arg.as_splat_node().is_some() {
            return true;
        }
        // Check for **kwargs (keyword hash containing AssocSplatNode)
        if let Some(kh) = arg.as_keyword_hash_node() {
            for elem in kh.elements().iter() {
                if elem.as_assoc_splat_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

/// Escape control characters in a string for display.
fn escape_control_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\x07' => result.push_str("\\a"),
            '\x08' => result.push_str("\\b"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\x0B' => result.push_str("\\v"),
            '\x0C' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            '\x1B' => result.push_str("\\e"),
            c if c.is_control() => {
                result.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

impl Cop for RedundantFormat {
    fn name(&self) -> &'static str {
        "Style/RedundantFormat"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
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

        let method_bytes = call.name().as_slice();
        if method_bytes != b"format" && method_bytes != b"sprintf" {
            return;
        }

        // Must be called without a receiver, or on Kernel/::Kernel
        if let Some(receiver) = call.receiver() {
            let is_kernel = if let Some(cr) = receiver.as_constant_read_node() {
                cr.name().as_slice() == b"Kernel"
            } else if let Some(cp) = receiver.as_constant_path_node() {
                cp.parent().is_none()
                    && cp
                        .name()
                        .map(|n| n.as_slice() == b"Kernel")
                        .unwrap_or(false)
            } else {
                false
            };
            if !is_kernel {
                return;
            }
        }

        // Skip if a block argument is present (e.g., `format 'text/plain', &:inspect`)
        if call.block().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check for splat/double-splat arguments
        if has_splat_arg(&arg_list) {
            return;
        }

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("format");

        if arg_list.len() == 1 {
            let arg = &arg_list[0];

            // Single string/dstr argument
            if arg.as_string_node().is_some() || arg.as_interpolated_string_node().is_some() {
                let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{arg_src}` directly instead of `{method_str}`."),
                ));
                return;
            }

            // Single constant argument: format(FORMAT), format(Foo::BAR)
            if arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some() {
                let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{arg_src}` directly instead of `{method_str}`."),
                ));
                return;
            }
        }

        // Multi-arg: format('%s %s', 'foo', 'bar') — detect unnecessary fields
        self.detect_unnecessary_fields(source, &call, &arg_list, method_str, diagnostics);
    }
}

impl RedundantFormat {
    fn detect_unnecessary_fields(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        arg_list: &[ruby_prism::Node<'_>],
        method_str: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if arg_list.len() < 2 {
            return;
        }

        // First arg must be a plain string (not interpolated)
        let fmt_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };

        let fmt_content = fmt_node.content_loc().as_slice();
        let fmt_str = match std::str::from_utf8(fmt_content) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Parse format specifiers
        let specs = match parse_simple_format_specs(fmt_str) {
            Some(s) => s,
            None => return,
        };

        // Number of format args (everything after the format string)
        let format_args = &arg_list[1..];

        // Must have exactly the right number of args
        if specs.len() != format_args.len() {
            return;
        }

        // All args must be literals, and each must match its specifier
        let mut parts = Vec::new();
        let mut last_end = 0;
        let bytes = fmt_str.as_bytes();
        let mut spec_idx = 0;
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] != b'%' {
                i += 1;
                continue;
            }

            // Save text before this specifier
            if i > last_end {
                parts.push(fmt_str[last_end..i].to_string());
            }

            let spec_start = i;
            i += 1;
            if i >= bytes.len() {
                return;
            }

            // %% literal
            if bytes[i] == b'%' {
                parts.push("%".to_string());
                i += 1;
                last_end = i;
                continue;
            }

            // This is a real specifier — find its end
            // Skip flags
            while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
                i += 1;
            }
            // Skip width
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Skip precision
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Type char
            if i >= bytes.len() {
                return;
            }
            i += 1; // skip type char
            last_end = i;

            if spec_idx >= specs.len() {
                return;
            }

            let formatted = match format_value(&specs[spec_idx], &format_args[spec_idx]) {
                Some(s) => s,
                None => return,
            };
            parts.push(formatted);
            spec_idx += 1;
        }

        // Remaining text after last specifier
        if last_end < bytes.len() {
            parts.push(fmt_str[last_end..].to_string());
        }

        let result = parts.join("");

        // Escape control chars and quote the result
        let escaped = escape_control_chars(&result);

        // Determine quoting: if any arg is interpolated, use double quotes
        let has_interpolation = format_args.iter().any(|a| {
            a.as_interpolated_string_node().is_some() || a.as_interpolated_symbol_node().is_some()
        });

        let quoted = if has_interpolation || escaped.contains('\\') {
            format!("\"{}\"", escaped)
        } else {
            format!("'{}'", escaped)
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` directly instead of `{}`.", quoted, method_str),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFormat, "cops/style/redundant_format");
}
