use crate::cop::node_type::CLASS_NODE;
use crate::cop::util::parent_class_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActionControllerTestCase;

impl Cop for ActionControllerTestCase {
    fn name(&self) -> &'static str {
        "Rails/ActionControllerTestCase"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
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
        // minimum_target_rails_version 5.0
        if !config.rails_version_at_least(5.0) {
            return;
        }

        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let parent = match parent_class_name(source, &class) {
            Some(p) => p,
            None => return,
        };

        if parent == b"ActionController::TestCase" {
            let loc = class.class_keyword_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Use `ActionDispatch::IntegrationTest` instead of `ActionController::TestCase`."
                    .to_string(),
            );

            if let Some(ref mut corr) = corrections
                && let Some(superclass) = class.superclass()
            {
                let super_loc = superclass.location();
                corr.push(crate::correction::Correction {
                    start: super_loc.start_offset(),
                    end: super_loc.end_offset(),
                    replacement: "ActionDispatch::IntegrationTest".to_string(),
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
    crate::cop_rails_fixture_tests!(
        ActionControllerTestCase,
        "cops/rails/action_controller_test_case",
        5.0
    );

    #[test]
    fn autocorrects_action_controller_test_case_superclass() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(5.0)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &ActionControllerTestCase,
            b"class UsersControllerTest < ActionController::TestCase\nend\n",
            b"class UsersControllerTest < ActionDispatch::IntegrationTest\nend\n",
            config,
        );
    }
}
