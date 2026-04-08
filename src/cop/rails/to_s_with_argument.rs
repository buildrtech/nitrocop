use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ToSWithArgument;

impl Cop for ToSWithArgument {
    fn name(&self) -> &'static str {
        "Rails/ToSWithArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        // minimum_target_rails_version 7.0
        if !config.rails_version_at_least(7.0) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"to_s" {
            return;
        }

        if call.receiver().is_none() {
            return;
        }

        // Must have at least one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the argument is a symbol
        if arg_list[0].as_symbol_node().is_some() {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Use `to_formatted_s` instead.".to_string(),
            );

            if let Some(ref mut corr) = corrections
                && let Some(selector) = call.message_loc()
            {
                corr.push(crate::correction::Correction {
                    start: selector.start_offset(),
                    end: selector.end_offset(),
                    replacement: "to_formatted_s".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(ToSWithArgument, "cops/rails/to_s_with_argument", 7.0);

    #[test]
    fn autocorrects_to_s_with_symbol_arg_to_to_formatted_s() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(7.0)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &ToSWithArgument,
            b"time.to_s(:db)\n",
            b"time.to_formatted_s(:db)\n",
            config,
        );
    }
}
