use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TopLevelHashWithIndifferentAccess;

impl Cop for TopLevelHashWithIndifferentAccess {
    fn name(&self) -> &'static str {
        "Rails/TopLevelHashWithIndifferentAccess"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        // minimum_target_rails_version 5.1
        if !config.rails_version_at_least(5.1) {
            return;
        }

        // Check for ConstantReadNode: `HashWithIndifferentAccess`
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"HashWithIndifferentAccess" {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid top-level `HashWithIndifferentAccess`.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.start_offset(),
                        replacement: "ActiveSupport::".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
        }

        // Check for ConstantPathNode: `::HashWithIndifferentAccess`
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    if name.as_slice() == b"HashWithIndifferentAccess" {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid top-level `HashWithIndifferentAccess`.".to_string(),
                        );

                        if let Some(ref mut corr) = corrections {
                            let insert_at = loc.start_offset() + 2; // after leading ::
                            corr.push(crate::correction::Correction {
                                start: insert_at,
                                end: insert_at,
                                replacement: "ActiveSupport::".to_string(),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    crate::cop_rails_fixture_tests!(
        TopLevelHashWithIndifferentAccess,
        "cops/rails/top_level_hash_with_indifferent_access",
        5.1
    );

    fn config_with_rails(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        options.insert(
            "__RailtiesInLockfile".to_string(),
            serde_yml::Value::Bool(true),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn autocorrects_top_level_constant() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &TopLevelHashWithIndifferentAccess,
            b"HashWithIndifferentAccess.new\n",
            b"ActiveSupport::HashWithIndifferentAccess.new\n",
            config_with_rails(5.1),
        );
    }

    #[test]
    fn autocorrects_rooted_top_level_constant() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &TopLevelHashWithIndifferentAccess,
            b"::HashWithIndifferentAccess.new\n",
            b"::ActiveSupport::HashWithIndifferentAccess.new\n",
            config_with_rails(5.1),
        );
    }
}
