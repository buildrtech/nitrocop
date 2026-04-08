use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RenderPlainText;

impl Cop for RenderPlainText {
    fn name(&self) -> &'static str {
        "Rails/RenderPlainText"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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
        let content_type_compat = config.get_bool("ContentTypeCompatibility", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.name().as_slice() != b"render" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let mut text_value_range: Option<(usize, usize)> = None;
        let mut rest_options: Vec<String> = Vec::new();
        let mut has_content_type = false;
        let mut content_type_compatible = false;

        for arg in args.arguments().iter() {
            if let Some(hash) = arg.as_hash_node() {
                for elem in hash.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node()
                        && let Some(sym) = assoc.key().as_symbol_node()
                    {
                        let key = sym.unescaped();
                        if key == b"text" {
                            let vloc = assoc.value().location();
                            text_value_range = Some((vloc.start_offset(), vloc.end_offset()));
                            continue;
                        }
                        if key == b"content_type" {
                            has_content_type = true;
                            if let Some(s) = assoc.value().as_string_node()
                                && s.unescaped() == b"text/plain"
                            {
                                content_type_compatible = true;
                            }
                            continue;
                        }
                    }

                    let ploc = elem.location();
                    rest_options.push(
                        source
                            .byte_slice(ploc.start_offset(), ploc.end_offset(), "")
                            .to_string(),
                    );
                }
                continue;
            }

            if let Some(kw_hash) = arg.as_keyword_hash_node() {
                for elem in kw_hash.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node()
                        && let Some(sym) = assoc.key().as_symbol_node()
                    {
                        let key = sym.unescaped();
                        if key == b"text" {
                            let vloc = assoc.value().location();
                            text_value_range = Some((vloc.start_offset(), vloc.end_offset()));
                            continue;
                        }
                        if key == b"content_type" {
                            has_content_type = true;
                            if let Some(s) = assoc.value().as_string_node()
                                && s.unescaped() == b"text/plain"
                            {
                                content_type_compatible = true;
                            }
                            continue;
                        }
                    }

                    let ploc = elem.location();
                    rest_options.push(
                        source
                            .byte_slice(ploc.start_offset(), ploc.end_offset(), "")
                            .to_string(),
                    );
                }
                continue;
            }

            let loc = arg.location();
            rest_options.push(
                source
                    .byte_slice(loc.start_offset(), loc.end_offset(), "")
                    .to_string(),
            );
        }

        let Some((text_start, text_end)) = text_value_range else {
            return;
        };

        // RuboCop parity:
        // - content_type present: only flag when value is exactly "text/plain"
        // - content_type absent: only flag when ContentTypeCompatibility is false
        let compatible_content_type = if has_content_type {
            content_type_compatible
        } else {
            !content_type_compat
        };
        if !compatible_content_type {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `render plain:` instead of `render text:`.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let text_value = source.byte_slice(text_start, text_end, "").to_string();
            let replacement = if rest_options.is_empty() {
                format!("render plain: {text_value}")
            } else {
                format!("render plain: {text_value}, {}", rest_options.join(", "))
            };

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

    crate::cop_fixture_tests!(RenderPlainText, "cops/rails/render_plain_text");

    fn config_with_content_type_compat(enabled: bool) -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "ContentTypeCompatibility".to_string(),
                serde_yml::Value::Bool(enabled),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn autocorrects_text_to_plain_and_removes_text_plain_content_type() {
        crate::testutil::assert_cop_autocorrect(
            &RenderPlainText,
            b"render text: 'Ruby!', content_type: 'text/plain'\n",
            b"render plain: 'Ruby!'\n",
        );
    }

    #[test]
    fn autocorrects_and_keeps_other_options() {
        crate::testutil::assert_cop_autocorrect(
            &RenderPlainText,
            b"render text: error_message, content_type: 'text/plain', status: :unprocessable_entity\n",
            b"render plain: error_message, status: :unprocessable_entity\n",
        );
    }

    #[test]
    fn content_type_compat_true_skips_without_content_type() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let source = b"render text: 'hello'\n";
        assert_cop_no_offenses_full_with_config(
            &RenderPlainText,
            source,
            config_with_content_type_compat(true),
        );
    }

    #[test]
    fn content_type_compat_false_flags_without_content_type() {
        use crate::testutil::run_cop_full_with_config;

        let source = b"render text: 'hello'\n";
        let diags = run_cop_full_with_config(
            &RenderPlainText,
            source,
            config_with_content_type_compat(false),
        );
        assert!(
            !diags.is_empty(),
            "ContentTypeCompatibility:false should flag render text: without content_type"
        );
    }

    #[test]
    fn content_type_compat_false_skips_non_plain_content_type() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let source = b"render text: 'hello', content_type: 'text/html'\n";
        assert_cop_no_offenses_full_with_config(
            &RenderPlainText,
            source,
            config_with_content_type_compat(false),
        );
    }
}
