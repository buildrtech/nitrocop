use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct HttpPositionalArguments;

const HTTP_METHODS: &[&[u8]] = &[b"get", b"post", b"put", b"patch", b"delete", b"head"];

fn compact_hash_literal_braces(arg_source: &str) -> String {
    let trimmed = arg_source.trim();
    if let Some(inner) = trimmed.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return format!("{{{}}}", inner.trim());
    }
    arg_source.to_string()
}

impl Cop for HttpPositionalArguments {
    fn name(&self) -> &'static str {
        "Rails/HttpPositionalArguments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // minimum_target_rails_version 5.0
        if !config.rails_version_at_least(5.0) {
            return;
        }

        // First, check if the file includes Rack::Test::Methods — if so, skip entirely
        let mut checker = RackTestChecker { found: false };
        checker.visit(&parse_result.node());
        if checker.found {
            return;
        }

        let mut visitor = HttpPosArgsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            autocorrect_enabled: corrections.is_some(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(ref mut corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

/// Scans AST for `include Rack::Test::Methods`
struct RackTestChecker {
    found: bool,
}

impl<'pr> Visit<'pr> for RackTestChecker {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if !self.found && node.receiver().is_none() && node.name().as_slice() == b"include" {
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if is_rack_test_methods(&arg) {
                        self.found = true;
                        return;
                    }
                }
            }
        }
        if !self.found {
            ruby_prism::visit_call_node(self, node);
        }
    }
}

/// Check if node is `Rack::Test::Methods` constant path
fn is_rack_test_methods(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cp) = node.as_constant_path_node() {
        // Check Methods
        if cp.name().is_none_or(|n| n.as_slice() != b"Methods") {
            return false;
        }
        // Check parent is Rack::Test
        if let Some(parent) = cp.parent() {
            if let Some(cp2) = parent.as_constant_path_node() {
                if cp2.name().is_none_or(|n| n.as_slice() != b"Test") {
                    return false;
                }
                // Check grandparent is Rack
                if let Some(gp) = cp2.parent() {
                    if let Some(cr) = gp.as_constant_read_node() {
                        return cr.name().as_slice() == b"Rack";
                    }
                }
            }
        }
    }
    false
}

struct HttpPosArgsVisitor<'a> {
    cop: &'a HttpPositionalArguments,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    autocorrect_enabled: bool,
}

impl<'pr> Visit<'pr> for HttpPosArgsVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        if HTTP_METHODS.contains(&method_name) && node.receiver().is_none() {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                // Only flag explicit HashNode (old-style positional: `get path, {params}` or
                // `get path, {params}, headers`).
                // A keyword_hash_node means keyword args (`get path, params: ...`), which is
                // the correct new-style syntax this cop promotes — don't flag it.
                if arg_list.len() >= 2 && arg_list[1].as_hash_node().is_some() {
                    let loc = node.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use keyword arguments for HTTP request methods.".to_string(),
                    );

                    // Conservative RuboCop-aligned baseline: second positional arg -> params,
                    // optional third positional arg -> session. Keep any remaining
                    // positional args unchanged.
                    if self.autocorrect_enabled {
                        let params_raw = self
                            .source
                            .byte_slice(
                                arg_list[1].location().start_offset(),
                                arg_list[1].location().end_offset(),
                                "",
                            )
                            .to_string();
                        let params = compact_hash_literal_braces(&params_raw);

                        let mut replacement = format!("params: {params}");
                        let mut correction_end = arg_list[1].location().end_offset();

                        if arg_list.len() >= 3 {
                            let session_raw = self
                                .source
                                .byte_slice(
                                    arg_list[2].location().start_offset(),
                                    arg_list[2].location().end_offset(),
                                    "",
                                )
                                .to_string();
                            let session = compact_hash_literal_braces(&session_raw);
                            replacement.push_str(&format!(", session: {session}"));
                            correction_end = arg_list[2].location().end_offset();
                        }

                        if arg_list.len() > 3 {
                            for extra in arg_list.iter().skip(3) {
                                let extra_src = self
                                    .source
                                    .byte_slice(
                                        extra.location().start_offset(),
                                        extra.location().end_offset(),
                                        "",
                                    )
                                    .to_string();
                                replacement.push_str(&format!(", {extra_src}"));
                                correction_end = extra.location().end_offset();
                            }
                        }

                        self.corrections.push(crate::correction::Correction {
                            start: arg_list[1].location().start_offset(),
                            end: correction_end,
                            replacement,
                            cop_name: self.cop.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }

                    self.diagnostics.push(diagnostic);
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
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
            &HttpPositionalArguments,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/http_positional_arguments/offense.rb"
            ),
            config_with_rails(5.0),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &HttpPositionalArguments,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/http_positional_arguments/no_offense.rb"
            ),
            config_with_rails(5.0),
        );
    }

    #[test]
    fn autocorrect_fixture() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &HttpPositionalArguments,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/http_positional_arguments/offense.rb"
            ),
            include_bytes!(
                "../../../tests/fixtures/cops/rails/http_positional_arguments/corrected.rb"
            ),
            config_with_rails(5.0),
        );
    }

    #[test]
    fn supports_autocorrect() {
        assert!(HttpPositionalArguments.supports_autocorrect());
    }

    #[test]
    fn autocorrect_keeps_trailing_positional_args_after_session() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &HttpPositionalArguments,
            b"get :new, { user_id: 1 }, { token: 'x' }, :json\n",
            b"get :new, params: {user_id: 1}, session: {token: 'x'}, :json\n",
            config_with_rails(5.0),
        );
    }

    #[test]
    fn skipped_when_no_target_rails_version() {
        // Non-Rails projects (e.g. sinatra) have no TargetRailsVersion.
        // RuboCop uses `requires_gem('railties', '>= 5.0')` which skips the cop
        // entirely when railties is not installed. Nitrocop should do the same.
        let source = b"get :index, { user_id: 1 }, { \"ACCEPT\" => \"text/html\" }\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &HttpPositionalArguments,
            source,
            CopConfig::default(), // no TargetRailsVersion
            "test/some_test.rb",
        );
        assert!(
            diagnostics.is_empty(),
            "Should not fire when TargetRailsVersion is not set (non-Rails project)"
        );
    }
}
