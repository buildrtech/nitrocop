use crate::cop::node_type::BEGIN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RescueStandardError;

fn check_rescue_node(
    cop: &dyn Cop,
    source: &SourceFile,
    rescue_node: &ruby_prism::RescueNode<'_>,
    enforced_style: &str,
    diagnostics: &mut Vec<Diagnostic>,
    pending_corrections: &mut Vec<Correction>,
    autocorrect_enabled: bool,
) {
    let exceptions: Vec<_> = rescue_node.exceptions().iter().collect();

    match enforced_style {
        "implicit" => {
            if exceptions.len() == 1 {
                if let Some(name) = crate::cop::util::constant_name(&exceptions[0]) {
                    if name == b"StandardError" {
                        let kw_loc = rescue_node.keyword_loc();
                        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                        let mut diag = cop.diagnostic(
                            source,
                            line,
                            column,
                            "Omit the error class when rescuing `StandardError` by itself."
                                .to_string(),
                        );

                        if autocorrect_enabled {
                            pending_corrections.push(Correction {
                                start: kw_loc.end_offset(),
                                end: exceptions[0].location().end_offset(),
                                replacement: String::new(),
                                cop_name: cop.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }

                        diagnostics.push(diag);
                    }
                }
            }
        }
        "explicit" => {
            if exceptions.is_empty() {
                let kw_loc = rescue_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                let mut diag = cop.diagnostic(
                    source,
                    line,
                    column,
                    "Specify `StandardError` explicitly when rescuing.".to_string(),
                );

                if autocorrect_enabled {
                    pending_corrections.push(Correction {
                        start: kw_loc.end_offset(),
                        end: kw_loc.end_offset(),
                        replacement: " StandardError".to_string(),
                        cop_name: cop.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }

                diagnostics.push(diag);
            }
        }
        _ => {}
    }

    if let Some(subsequent) = rescue_node.subsequent() {
        check_rescue_node(
            cop,
            source,
            &subsequent,
            enforced_style,
            diagnostics,
            pending_corrections,
            autocorrect_enabled,
        );
    }
}

impl Cop for RescueStandardError {
    fn name(&self) -> &'static str {
        "Style/RescueStandardError"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
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
        let begin_node = match node.as_begin_node() {
            Some(b) => b,
            None => return,
        };

        let rescue_clause = match begin_node.rescue_clause() {
            Some(r) => r,
            None => return,
        };

        let enforced_style = config.get_str("EnforcedStyle", "implicit");
        let mut pending_corrections = Vec::new();
        let autocorrect_enabled = corrections.is_some();

        check_rescue_node(
            self,
            source,
            &rescue_clause,
            enforced_style,
            diagnostics,
            &mut pending_corrections,
            autocorrect_enabled,
        );

        if let Some(corr) = corrections.as_mut() {
            corr.extend(pending_corrections);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(RescueStandardError, "cops/style/rescue_standard_error");
    crate::cop_autocorrect_fixture_tests!(RescueStandardError, "cops/style/rescue_standard_error");

    #[test]
    fn explicit_style_flags_bare_rescue() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("explicit".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"begin\n  foo\nrescue\n  bar\nend\n";
        let diags = run_cop_full_with_config(&RescueStandardError, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Specify"));
    }

    #[test]
    fn multiple_exceptions_not_flagged() {
        let source = b"begin\n  foo\nrescue StandardError, RuntimeError\n  bar\nend\n";
        let diags = run_cop_full(&RescueStandardError, source);
        assert!(diags.is_empty());
    }
}
