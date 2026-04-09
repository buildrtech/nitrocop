use crate::cop::factory_bot::{FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE, is_factory_call};
use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FactoryNameStyle;

fn symbol_literal_from_string(value: &str) -> String {
    let is_plain = !value.is_empty()
        && value
            .chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && value
            .chars()
            .all(|c| c == '_' || c.is_ascii_alphanumeric());

    if is_plain {
        format!(":{value}")
    } else {
        format!(":\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

impl Cop for FactoryNameStyle {
    fn name(&self) -> &'static str {
        "FactoryBot/FactoryNameStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !FACTORY_BOT_METHODS.contains(&method_name) {
            return;
        }

        let explicit_only = config.get_bool("ExplicitOnly", false);
        if !is_factory_call(call.receiver(), explicit_only) {
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

        let first_arg = &arg_list[0];
        let style = config.get_str("EnforcedStyle", "symbol");

        if style == "symbol" {
            // Flag string names (but not interpolated strings or namespaced strings with /)
            if let Some(str_node) = first_arg.as_string_node() {
                let value = str_node.unescaped();
                let value_str = std::str::from_utf8(value).unwrap_or("");

                // Skip namespaced names (contain /)
                if value_str.contains('/') {
                    return;
                }

                // Skip multi-line code strings (contain newlines/tabs) — not factory names
                if value_str.contains('\n') || value_str.contains('\r') || value_str.contains('\t')
                {
                    return;
                }

                let loc = first_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use symbol to refer to a factory.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: symbol_literal_from_string(value_str),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
            // Skip interpolated strings
        } else if style == "string" {
            // Flag symbol names
            if let Some(sym) = first_arg.as_symbol_node() {
                let loc = first_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use string to refer to a factory.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let value = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FactoryNameStyle, "cops/factorybot/factory_name_style");

    #[test]
    fn supports_autocorrect() {
        assert!(FactoryNameStyle.supports_autocorrect());
    }

    #[test]
    fn autocorrects_string_factory_name_to_symbol_style() {
        crate::testutil::assert_cop_autocorrect(
            &FactoryNameStyle,
            b"create('user')\n",
            b"create(:user)\n",
        );
    }
}
