use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Conservative autocorrect baseline (nitrocop-only): rewrite
/// `require_dependency` to `require` for already-flagged calls in Rails >= 6.0
/// (Zeitwerk mode), where `require_dependency` is obsolete.
pub struct RequireDependency;

impl Cop for RequireDependency {
    fn name(&self) -> &'static str {
        "Rails/RequireDependency"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"require_dependency" {
            return;
        }

        // Must have at least one argument
        if call.arguments().is_none() {
            return;
        }

        // Receiverless call or Kernel.require_dependency
        let is_valid_receiver = match call.receiver() {
            None => true,
            Some(recv) => {
                if let Some(cr) = recv.as_constant_read_node() {
                    cr.name().as_slice() == b"Kernel"
                } else if let Some(cp) = recv.as_constant_path_node() {
                    if let Some(name) = cp.name() {
                        name.as_slice() == b"Kernel" && cp.parent().is_none()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };

        if !is_valid_receiver {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Do not use `require_dependency` with Zeitwerk mode.".to_string(),
        );

        if let Some(ref mut corrs) = corrections
            && let Some(selector) = call.message_loc()
        {
            corrs.push(crate::correction::Correction {
                start: selector.start_offset(),
                end: selector.end_offset(),
                replacement: "require".to_string(),
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
    crate::cop_rails_fixture_tests!(RequireDependency, "cops/rails/require_dependency", 6.0);

    #[test]
    fn autocorrects_require_dependency_to_require_for_zeitwerk() {
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
            &RequireDependency,
            b"require_dependency 'some_lib'\n",
            b"require 'some_lib'\n",
            config,
        );
    }
}
