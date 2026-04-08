use crate::cop::node_type::{CALL_NODE, FALSE_NODE, TRUE_NODE};
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BelongsTo;

impl Cop for BelongsTo {
    fn name(&self) -> &'static str {
        "Rails/BelongsTo"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FALSE_NODE, TRUE_NODE]
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

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() || call.name().as_slice() != b"belongs_to" {
            return;
        }

        // Check for `required:` keyword argument
        let required_value = match keyword_arg_value(&call, b"required") {
            Some(v) => v,
            None => return,
        };

        let replacement = if required_value.as_true_node().is_some() {
            "optional: false"
        } else if required_value.as_false_node().is_some() {
            "optional: true"
        } else {
            return;
        };

        let mut pair_range: Option<(usize, usize)> = None;
        if let Some(args) = call.arguments() {
            'outer: for arg in args.arguments().iter() {
                if let Some(kw) = arg.as_keyword_hash_node() {
                    for elem in kw.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node()
                            && let Some(sym) = assoc.key().as_symbol_node()
                            && sym.unescaped() == b"required"
                        {
                            let l = assoc.location();
                            pair_range = Some((l.start_offset(), l.end_offset()));
                            break 'outer;
                        }
                    }
                }
                if let Some(hash) = arg.as_hash_node() {
                    for elem in hash.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node()
                            && let Some(sym) = assoc.key().as_symbol_node()
                            && sym.unescaped() == b"required"
                        {
                            let l = assoc.location();
                            pair_range = Some((l.start_offset(), l.end_offset()));
                            break 'outer;
                        }
                    }
                }
            }
        }

        let message = if required_value.as_true_node().is_some() {
            "You specified `required: true`, in Rails > 5.0 the required option is deprecated and you want to use `optional: false`."
        } else if required_value.as_false_node().is_some() {
            "You specified `required: false`, in Rails > 5.0 the required option is deprecated and you want to use `optional: true`."
        } else {
            return;
        };

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(source, line, column, message.to_string());

        if let Some(ref mut corr) = corrections
            && let Some((start, end)) = pair_range
        {
            corr.push(crate::correction::Correction {
                start,
                end,
                replacement: replacement.to_string(),
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

    crate::cop_rails_fixture_tests!(BelongsTo, "cops/rails/belongs_to", 5.0);

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
    fn autocorrects_required_false_to_optional_true() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &BelongsTo,
            b"belongs_to :blog, required: false\n",
            b"belongs_to :blog, optional: true\n",
            config_with_rails(5.0),
        );
    }

    #[test]
    fn autocorrects_required_true_to_optional_false() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &BelongsTo,
            b"belongs_to :author, required: true\n",
            b"belongs_to :author, optional: false\n",
            config_with_rails(5.0),
        );
    }
}
