use crate::cop::node_type::CALL_NODE;
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedirectBackOrTo;

impl Cop for RedirectBackOrTo {
    fn name(&self) -> &'static str {
        "Rails/RedirectBackOrTo"
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
        // minimum_target_rails_version 7.0
        if !config.rails_version_at_least(7.0) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless `redirect_back`
        if call.receiver().is_some() || call.name().as_slice() != b"redirect_back" {
            return;
        }

        // Must have `fallback_location:` keyword argument
        if keyword_arg_value(&call, b"fallback_location").is_none() {
            return;
        }

        let mut fallback_value_src: Option<String> = None;
        let mut replacement_options: Vec<String> = Vec::new();
        let mut hash_range: Option<(usize, usize)> = None;

        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if let Some(kw_hash) = arg.as_keyword_hash_node() {
                    let loc = kw_hash.location();
                    hash_range = Some((loc.start_offset(), loc.end_offset()));
                    for elem in kw_hash.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node()
                            && let Some(sym) = assoc.key().as_symbol_node()
                        {
                            let key = sym.unescaped();
                            if key == b"fallback_location" {
                                let vloc = assoc.value().location();
                                fallback_value_src = Some(
                                    source
                                        .byte_slice(vloc.start_offset(), vloc.end_offset(), "")
                                        .to_string(),
                                );
                                continue;
                            }
                        }
                        let ploc = elem.location();
                        replacement_options.push(
                            source
                                .byte_slice(ploc.start_offset(), ploc.end_offset(), "")
                                .to_string(),
                        );
                    }
                    break;
                }

                if let Some(hash) = arg.as_hash_node() {
                    let loc = hash.location();
                    hash_range = Some((loc.start_offset(), loc.end_offset()));
                    for elem in hash.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node()
                            && let Some(sym) = assoc.key().as_symbol_node()
                        {
                            let key = sym.unescaped();
                            if key == b"fallback_location" {
                                let vloc = assoc.value().location();
                                fallback_value_src = Some(
                                    source
                                        .byte_slice(vloc.start_offset(), vloc.end_offset(), "")
                                        .to_string(),
                                );
                                continue;
                            }
                        }
                        let ploc = elem.location();
                        replacement_options.push(
                            source
                                .byte_slice(ploc.start_offset(), ploc.end_offset(), "")
                                .to_string(),
                        );
                    }
                    break;
                }
            }
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `redirect_back_or_to` instead of `redirect_back` with `:fallback_location` keyword argument.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: "redirect_back_or_to".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });

            if let (Some(fallback), Some((hash_start, hash_end))) = (fallback_value_src, hash_range)
            {
                let arg_replacement = if replacement_options.is_empty() {
                    fallback
                } else {
                    format!("{fallback}, {}", replacement_options.join(", "))
                };

                corr.push(crate::correction::Correction {
                    start: hash_start,
                    end: hash_end,
                    replacement: arg_replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
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
            &RedirectBackOrTo,
            include_bytes!("../../../tests/fixtures/cops/rails/redirect_back_or_to/offense.rb"),
            config_with_rails(7.0),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RedirectBackOrTo,
            include_bytes!("../../../tests/fixtures/cops/rails/redirect_back_or_to/no_offense.rb"),
            config_with_rails(7.0),
        );
    }

    #[test]
    fn skipped_when_rails_below_7() {
        let source = b"redirect_back(fallback_location: root_path)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &RedirectBackOrTo,
            source,
            config_with_rails(6.1),
            "test.rb",
        );
        assert!(diagnostics.is_empty(), "Should not fire on Rails < 7.0");
    }

    #[test]
    fn skipped_when_no_rails_version() {
        let source = b"redirect_back(fallback_location: root_path)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &RedirectBackOrTo,
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
