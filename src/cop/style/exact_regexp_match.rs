use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP fix: RuboCop uses `regexp_parser` which tokenizes the regex and requires
/// exactly `:bos`, `:literal`, `:eos`. Our `is_literal_string` was treating
/// non-literal escape sequences (`\n`, `\t`, `\r`, `\0`, etc.) as literal
/// escaped chars, and also matching empty inner patterns (`/\A\z/`). Fixed by:
/// 1. Rejecting empty inner (no `:literal` token between anchors).
/// 2. Rejecting all non-literal escape sequences: C-style escapes (`\n`, `\t`,
///    `\r`, `\f`, `\a`, `\e`, `\v`), octal (`\0`-`\9`), hex/unicode (`\x`,
///    `\u`), control/meta (`\c`, `\C`, `\M`), backrefs (`\k`, `\g`, `\N`),
///    and the `\G` anchor.
pub struct ExactRegexpMatch;

impl ExactRegexpMatch {
    /// Check if a regex node is an exact match pattern like /\Afoo\z/
    fn is_exact_match_regex(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(regex) = node.as_regular_expression_node() {
            // Must have no meaningful regex flags (no /i, /m, /x, etc.)
            // Note: flags() includes base node flags (like encoding), so we
            // check specific regex flag methods instead of flags() != 0.
            if regex.is_ignore_case()
                || regex.is_extended()
                || regex.is_multi_line()
                || regex.is_once()
            {
                return false;
            }
            let bytes = regex.unescaped();
            return Self::is_exact_match_pattern(bytes);
        }
        false
    }

    fn is_exact_match_pattern(bytes: &[u8]) -> bool {
        // Must start with \A and end with \z
        if bytes.len() < 4 {
            return false;
        }
        if !bytes.starts_with(b"\\A") || !bytes.ends_with(b"\\z") {
            return false;
        }
        let inner = &bytes[2..bytes.len() - 2];
        // RuboCop requires a :literal token between :bos and :eos,
        // so the inner part must be non-empty and a simple literal.
        if inner.is_empty() {
            return false;
        }
        // The inner part must be a simple literal (no metacharacters)
        Self::is_literal_string(inner)
    }

    fn is_literal_string(bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return true;
        }
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            // Check for regex metacharacters
            match b {
                b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
                | b'^' | b'$' => return false,
                b'\\' => {
                    // Escape sequence
                    if i + 1 < bytes.len() {
                        let next = bytes[i + 1];
                        match next {
                            // Character class escapes
                            b'd' | b'D' | b'w' | b'W' | b's' | b'S' | b'h' | b'H' => {
                                return false;
                            }
                            // Anchor and boundary escapes
                            b'b' | b'B' | b'A' | b'z' | b'Z' | b'G' => return false,
                            // Unicode property escapes
                            b'R' | b'p' | b'P' => return false,
                            // C-style escape sequences (non-literal in regexp parser)
                            b'n' | b't' | b'r' | b'f' | b'a' | b'e' | b'v' => return false,
                            // Octal escapes (\0-\9)
                            b'0'..=b'9' => return false,
                            // Hex, unicode, control, meta escapes
                            b'x' | b'u' | b'c' | b'C' | b'M' => return false,
                            // Named/numbered backreferences and subexpression calls
                            b'k' | b'g' | b'N' => return false,
                            // Literal escape of a special punctuation char (e.g. \., \\, \/)
                            _ => {
                                i += 2;
                                continue;
                            }
                        }
                    }
                    return false;
                }
                _ => {}
            }
            i += 1;
        }
        true
    }

    fn exact_match_literal(node: &ruby_prism::Node<'_>) -> Option<String> {
        let regex = node.as_regular_expression_node()?;
        let bytes = regex.unescaped();
        if bytes.len() < 4 || !bytes.starts_with(b"\\A") || !bytes.ends_with(b"\\z") {
            return None;
        }
        let inner = &bytes[2..bytes.len() - 2];
        let mut out = Vec::with_capacity(inner.len());
        let mut i = 0;
        while i < inner.len() {
            if inner[i] == b'\\' {
                if i + 1 >= inner.len() {
                    return None;
                }
                out.push(inner[i + 1]);
                i += 2;
            } else {
                out.push(inner[i]);
                i += 1;
            }
        }
        Some(String::from_utf8_lossy(&out).to_string())
    }

    fn single_quoted_literal_content(raw: &str) -> String {
        raw.replace('\\', "\\\\").replace('\'', "\\'")
    }
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    String::from_utf8_lossy(
        &source.as_bytes()[node.location().start_offset()..node.location().end_offset()],
    )
    .to_string()
}

impl Cop for ExactRegexpMatch {
    fn name(&self) -> &'static str {
        "Style/ExactRegexpMatch"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE]
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

        match method_bytes {
            b"=~" | b"!~" | b"===" => {
                // receiver =~ /\Astring\z/
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 && Self::is_exact_match_regex(&arg_list[0]) {
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let op = if method_bytes == b"!~" { "!=" } else { "==" };
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `string {} 'string'`.", op),
                        );

                        if let (Some(corr), Some(receiver), Some(literal)) = (
                            corrections.as_mut(),
                            call.receiver(),
                            Self::exact_match_literal(&arg_list[0]),
                        ) {
                            corr.push(crate::correction::Correction {
                                start: loc.start_offset(),
                                end: loc.end_offset(),
                                replacement: format!(
                                    "{} {} '{}'",
                                    node_source(source, &receiver),
                                    op,
                                    Self::single_quoted_literal_content(&literal)
                                ),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }

                        diagnostics.push(diag);
                    }
                }
            }
            b"match" | b"match?" => {
                // string.match(/\Astring\z/) or string.match?(/\Astring\z/)
                if call.receiver().is_none() {
                    return;
                }
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 && Self::is_exact_match_regex(&arg_list[0]) {
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `string == 'string'`.".to_string(),
                        );

                        if let (Some(corr), Some(receiver), Some(literal)) = (
                            corrections.as_mut(),
                            call.receiver(),
                            Self::exact_match_literal(&arg_list[0]),
                        ) {
                            corr.push(crate::correction::Correction {
                                start: loc.start_offset(),
                                end: loc.end_offset(),
                                replacement: format!(
                                    "{} == '{}'",
                                    node_source(source, &receiver),
                                    Self::single_quoted_literal_content(&literal)
                                ),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }

                        diagnostics.push(diag);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExactRegexpMatch, "cops/style/exact_regexp_match");
    crate::cop_autocorrect_fixture_tests!(ExactRegexpMatch, "cops/style/exact_regexp_match");
}
