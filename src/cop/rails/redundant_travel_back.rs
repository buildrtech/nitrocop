use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RedundantTravelBack;

impl Cop for RedundantTravelBack {
    fn name(&self) -> &'static str {
        "Rails/RedundantTravelBack"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["spec/**/*.rb"]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        // minimum_target_rails_version 5.2
        if !config.rails_version_at_least(5.2) {
            return;
        }

        let mut visitor = TravelBackVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            in_teardown_or_after: false,
            autocorrect: corrections.is_some(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);

        if let Some(correction_vec) = corrections {
            correction_vec.extend(visitor.corrections);
        }
    }
}

struct TravelBackVisitor<'a> {
    cop: &'a RedundantTravelBack,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<Correction>,
    in_teardown_or_after: bool,
    autocorrect: bool,
}

fn redundant_travel_back_line_range(
    source: &SourceFile,
    call_loc: &ruby_prism::Location<'_>,
) -> (usize, usize) {
    let bytes = source.as_bytes();
    let start_offset = call_loc.start_offset();
    let end_offset = call_loc.end_offset();

    let line_start = bytes[..start_offset]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |idx| idx + 1);

    let line_end = bytes[end_offset..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |idx| end_offset + idx + 1);

    (line_start, line_end)
}

impl<'a, 'pr> Visit<'pr> for TravelBackVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if we're entering an `after` block.
        // RuboCop only matches `after do...end` blocks (not `teardown do...end`).
        // For teardown, only `def teardown` is matched (handled in visit_def_node).
        // Shoulda-context `teardown do...end` blocks are NOT flagged by RuboCop.
        let enters_teardown =
            node.block().is_some() && node.receiver().is_none() && method_name == b"after";

        // Check if this is a `travel_back` call inside teardown/after
        if self.in_teardown_or_after && method_name == b"travel_back" && node.receiver().is_none() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Redundant `travel_back` detected. It is automatically called after each test."
                    .to_string(),
            );

            if self.autocorrect {
                let (start, end) = redundant_travel_back_line_range(self.source, &loc);
                if start < end {
                    self.corrections.push(Correction {
                        start,
                        end,
                        replacement: String::new(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }

            self.diagnostics.push(diagnostic);
        }

        let was = self.in_teardown_or_after;
        if enters_teardown {
            self.in_teardown_or_after = true;
        }
        ruby_prism::visit_call_node(self, node);
        self.in_teardown_or_after = was;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Also match `def teardown; ... travel_back; end`
        let is_teardown = node.name().as_slice() == b"teardown";

        let was = self.in_teardown_or_after;
        if is_teardown {
            self.in_teardown_or_after = true;
        }
        ruby_prism::visit_def_node(self, node);
        self.in_teardown_or_after = was;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(RedundantTravelBack, "cops/rails/redundant_travel_back", 5.2);

    #[test]
    fn autocorrect_fixture() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(5.2)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &RedundantTravelBack,
            include_bytes!("../../../tests/fixtures/cops/rails/redundant_travel_back/offense.rb"),
            include_bytes!("../../../tests/fixtures/cops/rails/redundant_travel_back/corrected.rb"),
            config,
        );
    }
}
