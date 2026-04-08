use crate::cop::node_type::CALL_NODE;
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FreezeTime;

impl Cop for FreezeTime {
    fn name(&self) -> &'static str {
        "Rails/FreezeTime"
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
        // minimum_target_rails_version 5.2
        if !config.rails_version_at_least(5.2) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"travel_to" {
            return;
        }

        if call.receiver().is_some() {
            return;
        }

        // Argument should be Time.now, Time.current, or Time.zone.now
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        if !is_time_now_pattern(&arg_list[0]) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `freeze_time` instead of `travel_to(Time.now)`.".to_string(),
        );

        // Conservative autocorrect parity: only autocorrect plain one-argument calls.
        // Skip block-pass / extra-argument forms for now.
        if call.block().is_none()
            && arg_list.len() == 1
            && let Some(ref mut corr) = corrections
        {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: "freeze_time".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

/// Check if a node represents Time.now, Time.current, or Time.zone.now
fn is_time_now_pattern(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let method_name = call.name().as_slice();

    // Time.now or Time.current
    // Handle both ConstantReadNode (Time) and ConstantPathNode (::Time)
    if method_name == b"now" || method_name == b"current" {
        if let Some(recv) = call.receiver() {
            if util::constant_name(&recv) == Some(b"Time") {
                return true;
            }
            // Time.zone.now
            if method_name == b"now"
                && let Some(zone_call) = recv.as_call_node()
                && zone_call.name().as_slice() == b"zone"
                && let Some(time_recv) = zone_call.receiver()
                && util::constant_name(&time_recv) == Some(b"Time")
            {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::assert_cop_autocorrect_with_config;
    use std::collections::HashMap;

    crate::cop_rails_fixture_tests!(FreezeTime, "cops/rails/freeze_time", 5.2);

    fn autocorrect_config() -> CopConfig {
        CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(5.2)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn supports_autocorrect() {
        assert!(FreezeTime.supports_autocorrect());
    }

    #[test]
    fn autocorrect_fixture_with_rails_config() {
        assert_cop_autocorrect_with_config(
            &FreezeTime,
            include_bytes!("../../../tests/fixtures/cops/rails/freeze_time/offense.rb"),
            include_bytes!("../../../tests/fixtures/cops/rails/freeze_time/corrected.rb"),
            autocorrect_config(),
        );
    }
}
