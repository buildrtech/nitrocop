use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03):
/// - FP=13 (11 opal/opal, 2 ruby-next/ruby-next) because nitrocop fired on repos
///   targeting Ruby < 3.0 which don't support endless methods.
/// - Root cause: RuboCop declares `minimum_target_ruby_version 3.0` so the cop only
///   fires when the project targets Ruby >= 3.0. nitrocop had no such check.
/// - Fix: Added TargetRubyVersion >= 3.0 guard at the start of check_node.
///
/// - Remaining FP=13 after TargetRubyVersion fix (corpus baseline sets 4.0):
///   nitrocop was missing two early-return guards from RuboCop's `on_def`:
///   (a) `node.assignment_method?` — skip setter methods (`def foo=(x)`) entirely.
///   (b) `use_heredoc?(node)` — skip methods whose body is or contains a heredoc.
///   Both cause multiline endless methods to be flagged by nitrocop but not RuboCop.
///   Fix: Added assignment method check (name ends with `=`) and heredoc body check.
pub struct EndlessMethod;

impl EndlessMethod {
    /// Returns true if the def node's body is or contains a heredoc.
    /// Mirrors RuboCop's `use_heredoc?` which checks for str-type heredoc nodes.
    /// Uses a source-text scan of the first line after `=` for `<<` heredoc openers,
    /// which is reliable because heredoc openers must appear on the `def` line.
    fn body_uses_heredoc(source: &SourceFile, def_node: &ruby_prism::DefNode<'_>) -> bool {
        // The heredoc opener (<<~FOO, <<-FOO, <<FOO) must appear on the same line
        // as the `=` sign. Scan from equal_loc to end-of-line for `<<`.
        let equal_loc = match def_node.equal_loc() {
            Some(loc) => loc,
            None => return false,
        };
        let src = source.as_bytes();
        let start = equal_loc.end_offset();
        // Scan forward on the same line for heredoc opener: `<<` followed by
        // `~`, `-`, `'`, `"`, `` ` ``, or a word character (identifier start).
        // This distinguishes heredocs from the `<<` shovel/bitshift operator.
        let mut i = start;
        while i + 1 < src.len() && src[i] != b'\n' {
            if src[i] == b'<' && src[i + 1] == b'<' {
                // Check what follows `<<`
                if i + 2 < src.len() {
                    let next = src[i + 2];
                    if next == b'~'
                        || next == b'-'
                        || next == b'\''
                        || next == b'"'
                        || next == b'`'
                        || next.is_ascii_alphabetic()
                        || next == b'_'
                    {
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }
}

impl Cop for EndlessMethod {
    fn name(&self) -> &'static str {
        "Style/EndlessMethod"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        // RuboCop: minimum_target_ruby_version 3.0
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version < 3.0 {
            return;
        }

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // RuboCop: return if node.assignment_method?
        // Skip setter methods (e.g. def foo=(x)) — they end with '='
        let name = def_node.name();
        let name_bytes = name.as_slice();
        if name_bytes.ends_with(b"=") {
            return;
        }

        // RuboCop: return if use_heredoc?(node)
        // Skip methods whose body is or contains a heredoc.
        // Heredocs in Prism are StringNode/InterpolatedStringNode with opening starting with "<<".
        if Self::body_uses_heredoc(source, &def_node) {
            return;
        }

        let style = config.get_str("EnforcedStyle", "allow_single_line");

        // Check if this is an endless method (has = sign, no end keyword)
        let is_endless = def_node.end_keyword_loc().is_none() && def_node.equal_loc().is_some();

        match style {
            "disallow" => {
                if is_endless {
                    let loc = def_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid endless method definitions.".to_string(),
                    ));
                }
            }
            "allow_single_line" => {
                if is_endless {
                    let loc = def_node.location();
                    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
                    let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                    if end_line > start_line {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid endless method definitions with multiple lines.".to_string(),
                        ));
                    }
                }
            }
            "allow_always" => {
                // No offenses for endless methods
            }
            "require_single_line" | "require_always" => {
                // These styles want endless methods to be used
                // We skip enforcement of "use endless" to keep this simple
                // and focus on the "avoid" cases
                if is_endless {
                    let loc = def_node.location();
                    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
                    let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                    if end_line > start_line && style == "require_single_line" {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid endless method definitions with multiple lines.".to_string(),
                        ));
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
    use crate::cop::CopConfig;

    fn ruby30_config() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.0)),
        );
        config
    }

    #[test]
    fn offense_with_ruby30() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &EndlessMethod,
            include_bytes!("../../../tests/fixtures/cops/style/endless_method/offense.rb"),
            ruby30_config(),
        );
    }

    #[test]
    fn no_offense() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &EndlessMethod,
            include_bytes!("../../../tests/fixtures/cops/style/endless_method/no_offense.rb"),
            ruby30_config(),
        );
    }
}
