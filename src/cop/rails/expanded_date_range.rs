use crate::cop::node_type::{CALL_NODE, RANGE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExpandedDateRange;

impl Cop for ExpandedDateRange {
    fn name(&self) -> &'static str {
        "Rails/ExpandedDateRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, RANGE_NODE]
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

        let range = match node.as_range_node() {
            Some(r) => r,
            None => return,
        };

        let left = match range.left() {
            Some(l) => l,
            None => return,
        };

        let right = match range.right() {
            Some(r) => r,
            None => return,
        };

        // Left should be a call to .beginning_of_day
        let left_call = match left.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if left_call.name().as_slice() != b"beginning_of_day" {
            return;
        }

        // Right should be a call to .end_of_day
        let right_call = match right.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if right_call.name().as_slice() != b"end_of_day" {
            return;
        }

        // RuboCop only flags expanded ranges when both sides share the same receiver,
        // e.g. `date.beginning_of_day..date.end_of_day`.
        // `start_date.beginning_of_day..end_date.end_of_day` should be allowed.
        let left_recv = match left_call.receiver() {
            Some(r) => r.location(),
            None => return,
        };
        let right_recv = match right_call.receiver() {
            Some(r) => r.location(),
            None => return,
        };
        let bytes = source.as_bytes();
        let left_src = &bytes[left_recv.start_offset()..left_recv.end_offset()];
        let right_src = &bytes[right_recv.start_offset()..right_recv.end_offset()];
        if left_src != right_src {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `all_day` instead of explicit date range expansion.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_internal;

    crate::cop_rails_fixture_tests!(ExpandedDateRange, "cops/rails/expanded_date_range", 5.1);

    #[test]
    fn different_receivers_are_not_flagged() {
        let source = b"where(recorded_at: start_date.beginning_of_day..end_date.end_of_day)\n";
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(6.1)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        let diags = run_cop_full_internal(&ExpandedDateRange, source, config, "test.rb");
        assert!(
            diags.is_empty(),
            "start_date..end_date expanded ranges should not be flagged"
        );
    }
}
