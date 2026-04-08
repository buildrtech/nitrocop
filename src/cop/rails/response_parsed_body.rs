use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ResponseParsedBody;

impl Cop for ResponseParsedBody {
    fn name(&self) -> &'static str {
        "Rails/ResponseParsedBody"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &[
            "spec/controllers/**/*.rb",
            "spec/requests/**/*.rb",
            "test/controllers/**/*.rb",
            "test/integration/**/*.rb",
        ]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE]
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

        if call.name().as_slice() != b"parse" {
            return;
        }

        // Must have exactly 1 argument (response.body) — no keyword args or extra args.
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        if arg_call.name().as_slice() != b"body" {
            return;
        }

        // The receiver of .body should be `response`
        let body_recv = match arg_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let body_recv_call = match body_recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if body_recv_call.name().as_slice() != b"response" {
            return;
        }

        // Receiver must be constant `JSON` or `Nokogiri::HTML`/`Nokogiri::HTML5`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check for JSON.parse(response.body)
        if util::constant_name(&recv) == Some(b"JSON") {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Prefer `response.parsed_body` to `JSON.parse(response.body)`.".to_string(),
            );

            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: "response.parsed_body".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }

        // Check for Nokogiri::HTML.parse(response.body) / Nokogiri::HTML5.parse(response.body)
        if let Some(cp) = recv.as_constant_path_node()
            && let Some(name) = cp.name()
        {
            let name_bytes = name.as_slice();
            if (name_bytes == b"HTML" || name_bytes == b"HTML5")
                && let Some(parent) = cp.parent()
                && util::constant_name(&parent) == Some(b"Nokogiri")
            {
                let const_name = std::str::from_utf8(name_bytes).unwrap_or("HTML");
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Prefer `response.parsed_body` to `Nokogiri::{const_name}.parse(response.body)`."
                    ),
                );

                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: "response.parsed_body".to_string(),
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

    crate::cop_rails_fixture_tests!(ResponseParsedBody, "cops/rails/response_parsed_body", 5.0);

    #[test]
    fn autocorrects_json_parse_response_body() {
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
            &ResponseParsedBody,
            b"JSON.parse(response.body)\n",
            b"response.parsed_body\n",
            config,
        );
    }

    #[test]
    fn autocorrects_nokogiri_parse_response_body() {
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
            &ResponseParsedBody,
            b"Nokogiri::HTML.parse(response.body)\n",
            b"response.parsed_body\n",
            config,
        );
    }
}
