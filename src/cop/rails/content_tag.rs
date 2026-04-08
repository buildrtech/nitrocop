use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ContentTag;

impl Cop for ContentTag {
    fn name(&self) -> &'static str {
        "Rails/ContentTag"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
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

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop's ContentTag checks legacy `tag()` calls, NOT `content_tag()`.
        // RESTRICT_ON_SEND = [:tag]
        if call.name().as_slice() != b"tag" {
            return;
        }

        // Must be a receiverless call
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // RuboCop: return if node.arguments.count >= 3
        if arg_list.len() >= 3 {
            return;
        }

        let first_arg = &arg_list[0];

        // Allow variables, method calls, constants, splats
        // Only flag when first arg is a string or symbol literal with a valid tag name
        let tag_name = if let Some(s) = first_arg.as_string_node() {
            s.unescaped().to_vec()
        } else if let Some(sym) = first_arg.as_symbol_node() {
            sym.unescaped().to_vec()
        } else {
            // Not a literal string/symbol — skip (variable, send, const, splat, etc.)
            return;
        };

        // Must be a valid HTML tag name: starts with letter, only letters/digits/hyphens
        if !is_valid_tag_name(&tag_name) {
            return;
        }

        let preferred_method = preferred_method_from_tag_name(&tag_name);
        let tag_name_str = String::from_utf8_lossy(&tag_name);
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `tag.{preferred_method}` instead of `tag(:{tag_name_str})`."),
        );

        if let Some(ref mut corr) = corrections
            && let Some(selector_loc) = call.message_loc()
        {
            let mut replacement = format!("tag.{preferred_method}");
            if arg_list.len() > 1 {
                let rest = arg_list[1..]
                    .iter()
                    .map(|n| {
                        source
                            .byte_slice(n.location().start_offset(), n.location().end_offset(), "")
                            .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                replacement.push('(');
                replacement.push_str(&rest);
                replacement.push(')');
            }

            corr.push(crate::correction::Correction {
                start: selector_loc.start_offset(),
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

/// Check if the bytes form a valid HTML tag name: ^[a-zA-Z-][a-zA-Z0-9-]*$
fn is_valid_tag_name(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    let first = name[0];
    if !first.is_ascii_alphabetic() && first != b'-' {
        return false;
    }
    name.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'-')
}

fn preferred_method_from_tag_name(name: &[u8]) -> String {
    name.iter()
        .map(|&b| {
            if b == b'-' {
                '_'
            } else {
                (b as char).to_ascii_lowercase()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    crate::cop_rails_fixture_tests!(ContentTag, "cops/rails/content_tag", 5.1);

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
    fn autocorrects_simple_tag_symbol() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &ContentTag,
            b"tag(:p)\n",
            b"tag.p\n",
            config_with_rails(5.1),
        );
    }

    #[test]
    fn autocorrects_tag_with_options() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &ContentTag,
            b"tag(:br, class: \"classname\")\n",
            b"tag.br(class: \"classname\")\n",
            config_with_rails(5.1),
        );
    }
}
