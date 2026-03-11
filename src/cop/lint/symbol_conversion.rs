use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/SymbolConversion checks for uses of literal strings converted to a symbol
/// where a literal symbol could be used instead, and for unnecessarily quoted symbols.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=331, FN=279 on the March 10, 2026 run.
///
/// The previous regex-based correction logic only understood identifier-like
/// symbols. That caused:
/// - FPs on quoted hash labels that still require quotes, such as `"7_days":`
/// - FNs on quoted operator and variable symbols such as `:"+", :"@ivar"`
///
/// Fix: generate corrections using Ruby-like symbol literal rules instead of
/// assuming every convertible symbol is an identifier. Hash labels now only
/// autocorrect when the key can be written as a bare Ruby label.
///
/// Remaining gap: `EnforcedStyle: consistent` is still not implemented in this
/// port; the corpus regressions fixed here are all strict-style behavior.
/// Post-fix corpus rerun: actual offenses moved from 8,232 down to 8,192
/// against a RuboCop expected total of 8,180, and `check-cop.py --rerun`
/// cleared its FP-regression gate. Remaining localized divergence appears in a
/// small number of repos, dominated by jruby's file-drop-noise repo.
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=37, FN=100.
///
/// FP=37: Likely from quoting rules differences — nitrocop's
/// `can_be_unquoted_symbol()` may accept symbols that actually need quoting
/// in edge cases, or operator symbols in hash keys that RuboCop skips.
///
/// FN=100: Likely from missing `EnforcedStyle: consistent` mode (RuboCop
/// flags unquoted keys when any key in the same hash needs quoting) and
/// possibly missing `.to_sym`/`.intern` detection on interpolated strings.
///
/// Deferred: requires implementing the `consistent` EnforcedStyle and careful
/// comparison of quoting rules with RuboCop's `properly_quoted?` logic.
pub struct SymbolConversion;

const BARE_OPERATOR_SYMBOLS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"&", b"|", b"^", b"<<", b">>", b"<", b">", b"<=", b">=", b"==",
    b"===", b"<=>", b"=~", b"!~", b"!", b"~", b"+@", b"-@", b"**", b"[]", b"[]=", b"`",
];

fn is_identifier_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_identifier_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_method_name_symbol(value: &[u8]) -> bool {
    if value.is_empty() || !is_identifier_start(value[0]) {
        return false;
    }

    let main = match value.last() {
        Some(b'!' | b'?' | b'=') => &value[..value.len() - 1],
        _ => value,
    };

    !main.is_empty() && main.iter().copied().all(is_identifier_continue)
}

fn is_hash_label_symbol(value: &[u8]) -> bool {
    if value.is_empty() || !is_identifier_start(value[0]) {
        return false;
    }

    let main = if let Some(&last) = value.last() {
        if last == b'!' || last == b'?' {
            &value[..value.len() - 1]
        } else {
            value
        }
    } else {
        return false;
    };

    !main.is_empty() && main.iter().copied().all(is_identifier_continue)
}

fn is_instance_variable_symbol(value: &[u8]) -> bool {
    value.len() > 1
        && value[0] == b'@'
        && is_identifier_start(value[1])
        && value[2..].iter().copied().all(is_identifier_continue)
}

fn is_class_variable_symbol(value: &[u8]) -> bool {
    value.len() > 2
        && value.starts_with(b"@@")
        && is_identifier_start(value[2])
        && value[3..].iter().copied().all(is_identifier_continue)
}

fn is_global_variable_symbol(value: &[u8]) -> bool {
    value.len() > 1
        && value[0] == b'$'
        && is_identifier_start(value[1])
        && value[2..].iter().copied().all(is_identifier_continue)
}

fn is_operator_symbol(value: &[u8]) -> bool {
    BARE_OPERATOR_SYMBOLS.contains(&value)
}

fn can_be_unquoted_symbol(value: &[u8]) -> bool {
    is_method_name_symbol(value)
        || is_instance_variable_symbol(value)
        || is_class_variable_symbol(value)
        || is_global_variable_symbol(value)
        || is_operator_symbol(value)
}

fn escape_double_quoted_symbol(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{0C}' => escaped.push_str("\\f"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Compute the correction string for a symbol value.
/// Returns Ruby-like symbol literal syntax, e.g. `:+`, `:@ivar`, or `:"foo-bar"`.
fn symbol_correction(value: &[u8]) -> Option<String> {
    let value_str = std::str::from_utf8(value).ok()?;
    if can_be_unquoted_symbol(value) {
        Some(format!(":{value_str}"))
    } else {
        Some(format!(":\"{}\"", escape_double_quoted_symbol(value_str)))
    }
}

fn hash_key_correction(value: &[u8]) -> Option<String> {
    if !is_hash_label_symbol(value) {
        return None;
    }

    Some(std::str::from_utf8(value).ok()?.to_string())
}

fn normalize_single_quoted_source(source: &str) -> Option<String> {
    if source.starts_with(":'") && source.ends_with('\'') {
        let inner = source.strip_prefix(":'")?.strip_suffix('\'')?;
        return Some(format!(":\"{}\"", escape_double_quoted_symbol(inner)));
    }

    if source.starts_with('\'') && source.ends_with('\'') {
        let inner = source.strip_prefix('\'')?.strip_suffix('\'')?;
        return Some(format!("\"{}\"", escape_double_quoted_symbol(inner)));
    }

    None
}

fn source_matches_correction(source: &[u8], correction: &str) -> bool {
    let Ok(source_str) = std::str::from_utf8(source) else {
        return false;
    };

    source_str == correction
        || normalize_single_quoted_source(source_str)
            .as_deref()
            .is_some_and(|normalized| normalized == correction)
}

impl Cop for SymbolConversion {
    fn name(&self) -> &'static str {
        "Lint/SymbolConversion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _style = config.get_str("EnforcedStyle", "strict");

        // Check SymbolNode: quoted symbols (standalone or hash keys)
        if let Some(sym) = node.as_symbol_node() {
            self.check_symbol_node(source, &sym, diagnostics);
            return;
        }

        // Check CallNode: .to_sym / .intern patterns
        if let Some(call) = node.as_call_node() {
            self.check_call_node(source, &call, diagnostics);
        }
    }
}

impl SymbolConversion {
    /// Check a SymbolNode for unnecessary quoting.
    /// Handles:
    /// - Standalone quoted symbols: `:"foo"`, `:'bar'`
    /// - Hash keys (colon-style): `'foo': val`, `"foo": val`
    /// - Hash keys/values (rocket-style): `:'foo' => val`, `{ foo: :'bar' }`
    fn check_symbol_node(
        &self,
        source: &SourceFile,
        sym: &ruby_prism::SymbolNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let src = sym.location().as_slice();
        let value = sym.unescaped();

        // Determine what kind of symbol this is based on source representation
        let opening = sym.opening_loc().map(|l| l.as_slice());

        // Check if this is a hash key in colon-style: 'foo': or "foo":
        // In Prism, these have opening_loc of "'" or "\"" and source ends with ':
        let is_colon_hash_key = match opening {
            Some(b"'" | b"\"") => src.ends_with(b"':") || src.ends_with(b"\":"),
            _ => false,
        };

        if is_colon_hash_key {
            // Hash key with colon style: 'foo': val or "foo": val
            // Only flag if the value can be a bare hash key
            if let Some(value_str) = hash_key_correction(value) {
                let loc = sym.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Unnecessary symbol conversion; use `{value_str}:` instead."),
                ));
            }
            return;
        }

        // For standalone symbols or rocket-style hash keys/values:
        // Check if the symbol is unnecessarily quoted
        // Opening must be :" or :' (quoted symbol syntax)
        match opening {
            Some(b":\"" | b":'") => {}
            _ => return,
        }

        let correction = match symbol_correction(value) {
            Some(c) => c,
            None => return,
        };

        // RuboCop leaves quoted setter-like symbols alone in strict mode.
        if correction.ends_with('=') {
            return;
        }

        if source_matches_correction(src, &correction) {
            return;
        }

        let loc = sym.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Unnecessary symbol conversion; use `{correction}` instead."),
        ));
    }

    /// Check a CallNode for .to_sym / .intern on string/symbol/dstr receivers.
    fn check_call_node(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let method_name = call.name().as_slice();

        // Must be .to_sym or .intern
        if method_name != b"to_sym" && method_name != b"intern" {
            return;
        }

        // Must have no arguments
        if call.arguments().is_some() {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // String receiver: "foo".to_sym
        if let Some(str_node) = recv.as_string_node() {
            let value = str_node.unescaped();
            let correction = match symbol_correction(value) {
                Some(c) => c,
                None => return,
            };
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Unnecessary symbol conversion; use `{correction}` instead."),
            ));
            return;
        }

        // Symbol receiver: :foo.to_sym
        if let Some(sym_node) = recv.as_symbol_node() {
            let value = sym_node.unescaped();
            let correction = match symbol_correction(value) {
                Some(c) => c,
                None => return,
            };
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Unnecessary symbol conversion; use `{correction}` instead."),
            ));
            return;
        }

        // Interpolated string receiver: "foo-#{bar}".to_sym
        if let Some(dstr) = recv.as_interpolated_string_node() {
            // Reconstruct the interpolated content from source
            let dstr_loc = dstr.location();
            let dstr_src = dstr_loc.as_slice();
            // Strip the surrounding quotes from the dstr source
            let inner = if dstr_src.starts_with(b"\"") && dstr_src.ends_with(b"\"") {
                &dstr_src[1..dstr_src.len() - 1]
            } else {
                return; // heredoc or other form, skip
            };
            let inner_str = match std::str::from_utf8(inner) {
                Ok(s) => s,
                Err(_) => return,
            };
            let correction = format!(":\"{}\"", inner_str);
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Unnecessary symbol conversion; use `{correction}` instead."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SymbolConversion, "cops/lint/symbol_conversion");
}
