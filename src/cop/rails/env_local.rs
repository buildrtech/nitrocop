use crate::cop::node_type::{CALL_NODE, OR_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnvLocal;

/// Check if a node is `Rails.env.development?` or `Rails.env.test?`.
fn is_rails_env_check(node: &ruby_prism::Node<'_>, env_method: &[u8]) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != env_method {
        return false;
    }

    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    let env_call = match recv.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if env_call.name().as_slice() != b"env" {
        return false;
    }

    let rails_recv = match env_call.receiver() {
        Some(r) => r,
        None => return false,
    };

    // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
    util::constant_name(&rails_recv) == Some(b"Rails")
}

impl Cop for EnvLocal {
    fn name(&self) -> &'static str {
        "Rails/EnvLocal"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE]
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
        // minimum_target_rails_version 7.1
        if !config.rails_version_at_least(7.1) {
            return;
        }

        let or_node = match node.as_or_node() {
            Some(o) => o,
            None => return,
        };

        let left: ruby_prism::Node<'_> = or_node.left();
        let right: ruby_prism::Node<'_> = or_node.right();

        // Check both orderings: dev? || test? or test? || dev?
        let matches = (is_rails_env_check(&left, b"development?")
            && is_rails_env_check(&right, b"test?"))
            || (is_rails_env_check(&left, b"test?") && is_rails_env_check(&right, b"development?"));

        if !matches {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.env.local?` instead of checking for development or test.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: "Rails.env.local?".to_string(),
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
    crate::cop_rails_fixture_tests!(EnvLocal, "cops/rails/env_local", 7.1);

    #[test]
    fn autocorrects_development_or_test_to_local() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(7.1)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &EnvLocal,
            b"Rails.env.development? || Rails.env.test?\n",
            b"Rails.env.local?\n",
            config,
        );
    }

    #[test]
    fn autocorrects_test_or_development_to_local() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(7.1)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &EnvLocal,
            b"Rails.env.test? || Rails.env.development?\n",
            b"Rails.env.local?\n",
            config,
        );
    }
}
