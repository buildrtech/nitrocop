use crate::cop::node_type::CALL_NODE;
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Pick;

impl Cop for Pick {
    fn name(&self) -> &'static str {
        "Rails/Pick"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // minimum_target_rails_version 6.0
        if !config.rails_version_at_least(6.0) {
            return;
        }

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"first" {
            return;
        }

        if chain.inner_method != b"pluck" {
            return;
        }

        // `.first` must have no arguments.
        // `.pluck(...).first` = one value (equivalent to pick)
        // `.pluck(...).first(n)` = first n elements (NOT equivalent)
        let outer_call = node.as_call_node().unwrap();
        if outer_call.arguments().is_some() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `pick` instead of `pluck.first`.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let inner = chain.inner_call;
            if let Some(inner_selector) = inner.message_loc()
                && let Some(outer_selector) = outer_call.message_loc()
            {
                corr.push(crate::correction::Correction {
                    start: inner_selector.start_offset(),
                    end: inner_selector.end_offset(),
                    replacement: "pick".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });

                let remove_start = inner.location().end_offset();
                let remove_end = outer_selector.end_offset();
                if remove_start < remove_end {
                    corr.push(crate::correction::Correction {
                        start: remove_start,
                        end: remove_end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(Pick, "cops/rails/pick", 6.0);

    #[test]
    fn autocorrects_pluck_first_to_pick() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(6.0)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &Pick,
            b"User.pluck(:name).first\n",
            b"User.pick(:name)\n",
            config,
        );
    }
}
