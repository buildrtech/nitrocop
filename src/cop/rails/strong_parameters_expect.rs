use crate::cop::node_type::{CALL_NODE, LOCAL_VARIABLE_READ_NODE};
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StrongParametersExpect;

/// Check if a node is a `params` receiver (local variable or method call).
fn is_params_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.name().as_slice() == b"params" && call.receiver().is_none();
    }
    if let Some(lvar) = node.as_local_variable_read_node() {
        return lvar.name().as_slice() == b"params";
    }
    false
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    source
        .byte_slice(
            node.location().start_offset(),
            node.location().end_offset(),
            "",
        )
        .to_string()
}

fn autocorrect_replacement(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    is_require_permit: bool,
    is_permit_require: bool,
) -> Option<String> {
    if is_require_permit {
        let permit_args = call.arguments()?;
        let permit_list: Vec<_> = permit_args.arguments().iter().collect();
        if permit_list.is_empty() {
            return None;
        }

        let require_call = call.receiver()?.as_call_node()?;
        if require_call.name().as_slice() != b"require" {
            return None;
        }
        let req_args = require_call.arguments()?;
        let req_first = req_args.arguments().iter().next()?;
        let req_sym = req_first.as_symbol_node()?;
        let key = String::from_utf8_lossy(req_sym.unescaped()).to_string();

        let mut permit_srcs = Vec::new();
        for arg in permit_list {
            permit_srcs.push(node_source(source, &arg));
        }

        return Some(format!(
            "params.expect({key}: [{}])",
            permit_srcs.join(", ")
        ));
    }

    if is_permit_require {
        let require_args = call.arguments()?;
        let require_first = require_args.arguments().iter().next()?;
        let require_sym = require_first.as_symbol_node()?;
        let require_key = require_sym.unescaped();

        let permit_call = call.receiver()?.as_call_node()?;
        if permit_call.name().as_slice() != b"permit" {
            return None;
        }
        let permit_args = permit_call.arguments()?;
        let permit_first = permit_args.arguments().iter().next()?;

        // Conservative: only autocorrect when permit has a hash-first arg whose first key
        // matches the required key.
        let hash_src = if let Some(kw) = permit_first.as_keyword_hash_node() {
            let first_pair = kw.elements().iter().next()?.as_assoc_node()?;
            let first_key = first_pair.key().as_symbol_node()?;
            if first_key.unescaped() != require_key {
                return None;
            }
            node_source(source, &permit_first)
        } else if let Some(hash) = permit_first.as_hash_node() {
            let first_pair = hash.elements().iter().next()?.as_assoc_node()?;
            let first_key = first_pair.key().as_symbol_node()?;
            if first_key.unescaped() != require_key {
                return None;
            }
            node_source(source, &permit_first)
        } else {
            return None;
        };

        return Some(format!("params.expect({hash_src})"));
    }

    None
}

impl Cop for StrongParametersExpect {
    fn name(&self) -> &'static str {
        "Rails/StrongParametersExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LOCAL_VARIABLE_READ_NODE]
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
        // minimum_target_rails_version 8.0
        if !config.rails_version_at_least(8.0) {
            return;
        }

        // Pattern 1: params.require(:x).permit(:a, :b)
        // Pattern 2: params.permit(x: [:a, :b]).require(:x)
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        let is_require_permit = chain.inner_method == b"require" && chain.outer_method == b"permit";
        let is_permit_require = chain.inner_method == b"permit" && chain.outer_method == b"require";

        if !is_require_permit && !is_permit_require {
            return;
        }

        // Check if the innermost receiver is `params`
        let inner_receiver = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_params_receiver(&inner_receiver) {
            return;
        }

        // For require.permit, permit must have arguments
        if is_require_permit {
            let outer_call = node.as_call_node().unwrap();
            if outer_call.arguments().is_none() {
                return;
            }
        }

        let msg_loc = chain
            .inner_call
            .message_loc()
            .unwrap_or(chain.inner_call.location());

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `expect(...)` instead.".to_string(),
        );

        if let Some(ref mut corr) = corrections
            && let Some(call_node) = node.as_call_node()
            && let Some(replacement) =
                autocorrect_replacement(source, &call_node, is_require_permit, is_permit_require)
        {
            let loc = call_node.location();
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
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
    use crate::cop::CopConfig;
    use std::collections::HashMap;

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
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &StrongParametersExpect,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/strong_parameters_expect/offense.rb"
            ),
            config_with_rails(8.0),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &StrongParametersExpect,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/strong_parameters_expect/no_offense.rb"
            ),
            config_with_rails(8.0),
        );
    }

    #[test]
    fn supports_autocorrect() {
        assert!(StrongParametersExpect.supports_autocorrect());
    }

    #[test]
    fn autocorrects_require_permit_to_expect() {
        let input = b"params.require(:user).permit(:name, :age)\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect_with_config(
            &StrongParametersExpect,
            input,
            config_with_rails(8.0),
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"params.expect(user: [:name, :age])\n");
    }

    #[test]
    fn autocorrects_permit_require_to_expect() {
        let input = b"params.permit(user: [:name, :age]).require(:user)\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect_with_config(
            &StrongParametersExpect,
            input,
            config_with_rails(8.0),
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"params.expect(user: [:name, :age])\n");
    }

    #[test]
    fn skipped_when_rails_below_8() {
        // On Rails 7.x, the cop should never fire
        let source = b"params.require(:user).permit(:name)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &StrongParametersExpect,
            source,
            config_with_rails(7.1),
            "test.rb",
        );
        assert!(diagnostics.is_empty(), "Should not fire on Rails < 8.0");
    }

    #[test]
    fn skipped_when_no_rails_version() {
        // Default (no TargetRailsVersion) should be 5.0, so cop doesn't fire
        let source = b"params.require(:user).permit(:name)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &StrongParametersExpect,
            source,
            CopConfig::default(),
            "test.rb",
        );
        assert!(
            diagnostics.is_empty(),
            "Should not fire when TargetRailsVersion defaults to 5.0"
        );
    }
}
