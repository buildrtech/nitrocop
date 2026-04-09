use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ImplicitExpect;

impl Cop for ImplicitExpect {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        // Config: EnforcedStyle — "is_expected" (default) or "should"
        let enforced_style = config.get_str("EnforcedStyle", "is_expected");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if enforced_style == "should" {
            // "should" style: flag `is_expected`
            if method_name == b"to" || method_name == b"not_to" || method_name == b"to_not" {
                let Some(recv) = call.receiver() else {
                    return;
                };
                let Some(recv_call) = recv.as_call_node() else {
                    return;
                };
                if recv_call.receiver().is_some() || recv_call.name().as_slice() != b"is_expected" {
                    return;
                }

                let loc = recv_call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `should` over `is_expected.to`.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let replacement = if method_name == b"to" {
                        "should"
                    } else {
                        "should_not"
                    };
                    corr.push(crate::correction::Correction {
                        start: recv_call.location().start_offset(),
                        end: call.message_loc().unwrap_or(call.location()).end_offset(),
                        replacement: replacement.to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
        } else {
            // Default "is_expected" style: flag `should` and `should_not`
            if method_name == b"should" {
                if call.receiver().is_some() {
                    return;
                }
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `is_expected.to` over `should`.".to_string(),
                );
                if let Some(ref mut corr) = corrections
                    && let Some(selector) = call.message_loc()
                {
                    corr.push(crate::correction::Correction {
                        start: selector.start_offset(),
                        end: selector.end_offset(),
                        replacement: "is_expected.to".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
                diagnostics.push(diagnostic);
            }

            if method_name == b"should_not" {
                if call.receiver().is_some() {
                    return;
                }
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `is_expected.to_not` over `should_not`.".to_string(),
                );
                if let Some(ref mut corr) = corrections
                    && let Some(selector) = call.message_loc()
                {
                    corr.push(crate::correction::Correction {
                        start: selector.start_offset(),
                        end: selector.end_offset(),
                        replacement: "is_expected.to_not".to_string(),
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
    crate::cop_fixture_tests!(ImplicitExpect, "cops/rspec/implicit_expect");

    #[test]
    fn should_style_flags_is_expected() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("should".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"is_expected.to eq(1)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitExpect, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("should"));
    }

    #[test]
    fn should_style_does_not_flag_should() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("should".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"should eq(1)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitExpect, source, config);
        assert!(diags.is_empty());
    }

    #[test]
    fn supports_autocorrect() {
        assert!(ImplicitExpect.supports_autocorrect());
    }

    #[test]
    fn autocorrects_should_to_is_expected_to() {
        crate::testutil::assert_cop_autocorrect(
            &ImplicitExpect,
            b"should be_truthy\n",
            b"is_expected.to be_truthy\n",
        );
    }

    #[test]
    fn autocorrects_is_expected_to_to_should_in_should_style() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("should".into()),
            )]),
            ..CopConfig::default()
        };

        crate::testutil::assert_cop_autocorrect_with_config(
            &ImplicitExpect,
            b"is_expected.to be_truthy\n",
            b"should be_truthy\n",
            config,
        );
    }
}
