use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Investigation (2026-03-04)
///
/// 10 FNs in jruby and natalie repos, all involving `\c` and `\C-` control
/// character escapes in regex patterns (e.g., `/\c(\cH\ch/.match(str)`).
///
/// **Root cause:** RuboCop's parser gem pre-interprets regex content, so `\c(`
/// becomes byte `\x08` before `LITERAL_REGEX` checks it. Prism gives raw source,
/// so `is_literal_regex()` saw `(` after `\c` and rejected it as non-literal.
///
/// **Fix:** Added handling for `\cX` (3-byte) and `\C-X`/`\M-X` (4-byte) control
/// and meta character escapes in `is_literal_regex()`, treating them as literal
/// character sequences.
///
/// ## Extended corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=4, FN=0. All 4 FPs from files containing
/// invalid multibyte regex escapes that crash RuboCop's parser, causing all
/// other cops to be skipped. Not a cop logic issue. Fixed by adding the
/// affected files to `repo_excludes.json`.
pub struct StringInclude;

/// Check if a single byte is in RuboCop's literal character allowlist.
/// Matches: `[\w\s\-,"'!#%&<>=;:`~/]` from RuboCop's `Util::LITERAL_REGEX`.
fn is_literal_char(b: u8) -> bool {
    match b {
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => true,
        b' ' | b'\t' | b'\n' | b'\r' | 0x0C => true,
        b'-' | b',' | b'"' | b'\'' | b'!' | b'#' | b'%' | b'&' | b'<' | b'>' | b'=' | b';'
        | b':' | b'`' | b'~' | b'/' => true,
        _ => false,
    }
}

/// Characters that, when preceded by a backslash, form a regex metachar class.
fn is_regex_escape_metachar(b: u8) -> bool {
    matches!(
        b,
        b'A' | b'b'
            | b'B'
            | b'd'
            | b'D'
            | b'g'
            | b'G'
            | b'h'
            | b'H'
            | b'k'
            | b'p'
            | b'P'
            | b'R'
            | b'w'
            | b'W'
            | b'X'
            | b's'
            | b'S'
            | b'z'
            | b'Z'
            | b'0'..=b'9'
    )
}

/// Check if a regex pattern contains only characters RuboCop considers literal.
fn is_literal_regex(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }

    let mut i = 0;
    while i < content.len() {
        if content[i] == b'\\' {
            if i + 1 >= content.len() {
                return false;
            }
            let next = content[i + 1];
            if next == b'c' {
                if i + 2 >= content.len() {
                    return false;
                }
                i += 3;
            } else if (next == b'C' || next == b'M')
                && i + 2 < content.len()
                && content[i + 2] == b'-'
            {
                if i + 3 >= content.len() {
                    return false;
                }
                i += 4;
            } else if is_regex_escape_metachar(next) {
                return false;
            } else {
                i += 2;
            }
        } else if is_literal_char(content[i]) {
            i += 1;
        } else {
            return false;
        }
    }

    true
}

fn is_simple_regex_node(node: &ruby_prism::Node<'_>) -> bool {
    let regex_node = match node.as_regular_expression_node() {
        Some(r) => r,
        None => return false,
    };

    // RuboCop NodePattern requires `(regopt)` (no flags).
    let closing = regex_node.closing_loc().as_slice();
    if closing.len() > 1 {
        return false;
    }

    is_literal_regex(regex_node.content_loc().as_slice())
}

#[inline]
fn control_char(b: u8) -> u8 {
    b & 0x1f
}

/// Decode regex content as RuboCop's interpret_string_escapes would for this literal subset.
fn decode_regex_literal_bytes(content: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(content.len());
    let mut i = 0;

    while i < content.len() {
        let b = content[i];
        if b != b'\\' {
            out.push(b);
            i += 1;
            continue;
        }

        if i + 1 >= content.len() {
            return None;
        }

        let next = content[i + 1];

        if next == b'c' {
            if i + 2 >= content.len() {
                return None;
            }
            out.push(control_char(content[i + 2]));
            i += 3;
            continue;
        }

        if next == b'C' && i + 3 < content.len() && content[i + 2] == b'-' {
            out.push(control_char(content[i + 3]));
            i += 4;
            continue;
        }

        if next == b'M' && i + 3 < content.len() && content[i + 2] == b'-' {
            if content[i + 3] == b'\\' {
                // \M-\cX
                if i + 5 < content.len() && content[i + 4] == b'c' {
                    out.push(control_char(content[i + 5]) | 0x80);
                    i += 6;
                    continue;
                }
                // \M-\C-X
                if i + 6 < content.len() && content[i + 4] == b'C' && content[i + 5] == b'-' {
                    out.push(control_char(content[i + 6]) | 0x80);
                    i += 7;
                    continue;
                }
            }

            out.push(content[i + 3] | 0x80);
            i += 4;
            continue;
        }

        match next {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'f' => out.push(0x0C),
            b'v' => out.push(0x0B),
            b'a' => out.push(0x07),
            b'e' => out.push(0x1B),
            _ => out.push(next),
        }
        i += 2;
    }

    Some(out)
}

fn to_double_quoted_string_literal(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x0B => out.push_str("\\v"),
            0x0C => out.push_str("\\f"),
            0x20..=0x7E => out.push(char::from(b)),
            _ => out.push_str(&format!("\\x{:02X}", b)),
        }
    }
    out.push('"');
    out
}

impl Cop for StringInclude {
    fn name(&self) -> &'static str {
        "Performance/StringInclude"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE]
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

        let name = call.name().as_slice();
        let dot = call
            .call_operator_loc()
            .map(|op| source.byte_slice(op.start_offset(), op.end_offset(), "."))
            .unwrap_or(".");

        let replacement = match name {
            b"match?" | b"match" => {
                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let args: Vec<_> = arguments.arguments().iter().collect();
                if args.len() != 1 {
                    return;
                }
                let first_arg = &args[0];

                if let Some(regex_node) = first_arg.as_regular_expression_node() {
                    if !is_simple_regex_node(first_arg) {
                        return;
                    }
                    let literal =
                        match decode_regex_literal_bytes(regex_node.content_loc().as_slice()) {
                            Some(v) => v,
                            None => return,
                        };
                    let recv_loc = receiver.location();
                    let recv_source =
                        source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
                    format!(
                        "{recv_source}{dot}include?({})",
                        to_double_quoted_string_literal(&literal)
                    )
                } else if let Some(regex_node) = receiver.as_regular_expression_node() {
                    if !is_simple_regex_node(&receiver) {
                        return;
                    }
                    let literal =
                        match decode_regex_literal_bytes(regex_node.content_loc().as_slice()) {
                            Some(v) => v,
                            None => return,
                        };
                    let arg_loc = first_arg.location();
                    let arg_source =
                        source.byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "");
                    format!(
                        "{arg_source}{dot}include?({})",
                        to_double_quoted_string_literal(&literal)
                    )
                } else {
                    return;
                }
            }
            b"===" => {
                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };
                if !is_simple_regex_node(&receiver) {
                    return;
                }
                let regex_node = match receiver.as_regular_expression_node() {
                    Some(r) => r,
                    None => return,
                };
                let first_arg = match call.arguments().and_then(|a| a.arguments().iter().next()) {
                    Some(a) => a,
                    None => return,
                };
                let literal = match decode_regex_literal_bytes(regex_node.content_loc().as_slice())
                {
                    Some(v) => v,
                    None => return,
                };
                let arg_loc = first_arg.location();
                let arg_source =
                    source.byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "");
                format!(
                    "{arg_source}.include?({})",
                    to_double_quoted_string_literal(&literal)
                )
            }
            b"=~" => {
                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };
                let first_arg = match call.arguments().and_then(|a| a.arguments().iter().next()) {
                    Some(a) => a,
                    None => return,
                };

                if let Some(regex_node) = first_arg.as_regular_expression_node() {
                    if !is_simple_regex_node(&first_arg) {
                        return;
                    }
                    let literal =
                        match decode_regex_literal_bytes(regex_node.content_loc().as_slice()) {
                            Some(v) => v,
                            None => return,
                        };
                    let recv_loc = receiver.location();
                    let recv_source =
                        source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
                    format!(
                        "{recv_source}.include?({})",
                        to_double_quoted_string_literal(&literal)
                    )
                } else if let Some(regex_node) = receiver.as_regular_expression_node() {
                    if !is_simple_regex_node(&receiver) {
                        return;
                    }
                    let literal =
                        match decode_regex_literal_bytes(regex_node.content_loc().as_slice()) {
                            Some(v) => v,
                            None => return,
                        };
                    let arg_loc = first_arg.location();
                    let arg_source =
                        source.byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "");
                    format!(
                        "{arg_source}.include?({})",
                        to_double_quoted_string_literal(&literal)
                    )
                } else {
                    return;
                }
            }
            b"!~" => {
                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };
                let first_arg = match call.arguments().and_then(|a| a.arguments().iter().next()) {
                    Some(a) => a,
                    None => return,
                };
                let regex_node = match first_arg.as_regular_expression_node() {
                    Some(r) => r,
                    None => return,
                };
                if !is_simple_regex_node(&first_arg) {
                    return;
                }
                let literal = match decode_regex_literal_bytes(regex_node.content_loc().as_slice())
                {
                    Some(v) => v,
                    None => return,
                };
                let recv_loc = receiver.location();
                let recv_source =
                    source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
                format!(
                    "!{recv_source}.include?({})",
                    to_double_quoted_string_literal(&literal)
                )
            }
            _ => return,
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `String#include?` instead of a regex match with literal-only pattern.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
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

    crate::cop_fixture_tests!(StringInclude, "cops/performance/string_include");
    crate::cop_autocorrect_fixture_tests!(StringInclude, "cops/performance/string_include");

    #[test]
    fn supports_autocorrect() {
        assert!(StringInclude.supports_autocorrect());
    }
}
