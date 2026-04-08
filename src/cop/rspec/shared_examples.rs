use crate::cop::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/SharedExamples: enforces consistent use of string or symbol for shared example names.
///
/// Root cause of FN=57: missing `shared_context` and `include_context` methods.
/// RuboCop's `SharedGroups.all` includes `shared_examples`, `shared_examples_for`, AND `shared_context`.
/// RuboCop's `Includes.all` includes `include_examples`, `include_context`, `it_behaves_like`,
/// `it_should_behave_like`. Both were missing from nitrocop's method list.
pub struct SharedExamples;

/// Methods that accept shared example titles.
/// Must match RuboCop's SharedGroups.all + Includes.all.
const SHARED_EXAMPLE_METHODS: &[&[u8]] = &[
    b"it_behaves_like",
    b"it_should_behave_like",
    b"shared_examples",
    b"shared_examples_for",
    b"shared_context",
    b"include_examples",
    b"include_context",
];

impl Cop for SharedExamples {
    fn name(&self) -> &'static str {
        "RSpec/SharedExamples"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE]
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
        // Config: EnforcedStyle — "string" (default) or "symbol"
        let enforced_style = config.get_str("EnforcedStyle", "string");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for RSpec.shared_examples / RSpec.shared_context etc. as well
        let is_shared = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && (method_name == b"shared_examples"
                    || method_name == b"shared_examples_for"
                    || method_name == b"shared_context")
        } else {
            SHARED_EXAMPLE_METHODS.contains(&method_name)
        };

        if !is_shared {
            return;
        }

        // Get the first argument — should be a string (default enforced style)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            if enforced_style == "symbol" {
                // "symbol" style: flag string arguments, prefer symbols
                if let Some(s) = arg.as_string_node() {
                    let str_val = std::str::from_utf8(s.unescaped()).unwrap_or("");
                    let sym_name = str_val.to_lowercase().replace(' ', "_");
                    let replacement = format!(":{sym_name}");
                    let loc = s.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Prefer `{replacement}` over `{str_val:?}` to symbolize shared examples."),
                    );

                    if let Some(ref mut corr) = corrections {
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
            } else {
                // Default "string" style: flag symbol arguments, prefer strings
                if let Some(sym) = arg.as_symbol_node() {
                    let sym_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                    let title = sym_name.replace('_', " ");
                    let replacement = format!("'{title}'");
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Prefer '{}' over `:{sym_name}` to titleize shared examples.",
                            title
                        ),
                    );

                    if let Some(ref mut corr) = corrections {
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
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SharedExamples, "cops/rspec/shared_examples");

    #[test]
    fn symbol_style_flags_string_args() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("symbol".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"it_behaves_like 'some example'\n";
        let diags = crate::testutil::run_cop_full_with_config(&SharedExamples, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains(":some_example"));
    }

    #[test]
    fn symbol_style_does_not_flag_symbol_args() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("symbol".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"it_behaves_like :some_example\n";
        let diags = crate::testutil::run_cop_full_with_config(&SharedExamples, source, config);
        assert!(diags.is_empty());
    }

    #[test]
    fn autocorrects_symbol_to_string_title() {
        crate::testutil::assert_cop_autocorrect(
            &SharedExamples,
            b"shared_examples :foo_bar_baz\n",
            b"shared_examples 'foo bar baz'\n",
        );
    }

    #[test]
    fn autocorrects_string_to_symbol_when_enforced() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("symbol".into()),
            )]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &SharedExamples,
            b"include_examples 'Foo Bar Baz'\n",
            b"include_examples :foo_bar_baz\n",
            config,
        );
    }
}
