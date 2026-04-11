use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/OptionalArguments: flags optional args not at the end of the arg list.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=23, FN=0.
///
/// FP=23: Fixed by skipping class methods (def self.xxx). RuboCop only defines
/// `on_def` (not `on_defs`), so it never checks class methods. In Prism, both
/// instance and class methods produce `DefNode` — class methods have a non-None
/// `receiver()`. Fixed by skipping DefNodes with a receiver.
pub struct OptionalArguments;

impl Cop for OptionalArguments {
    fn name(&self) -> &'static str {
        "Style/OptionalArguments"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = OptionalArgumentsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OptionalArgumentsVisitor<'a, 'corr> {
    cop: &'a OptionalArguments,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'corr mut Vec<crate::correction::Correction>>,
}

impl<'pr> Visit<'pr> for OptionalArgumentsVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // RuboCop only defines on_def (not on_defs), so class methods like
        // `def self.foo(a=1, b)` are not checked. In Prism, class methods
        // have a receiver — skip those.
        if node.receiver().is_some() {
            // Still visit the body for nested defs
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            return;
        }

        if let Some(params) = node.parameters() {
            let optionals: Vec<_> = params.optionals().iter().collect();
            // Filter posts to only RequiredParameterNode — destructured params
            // (MultiTargetNode) are not treated as required args by RuboCop.
            let has_required_posts = params
                .posts()
                .iter()
                .any(|p| p.as_required_parameter_node().is_some());

            // If there are optional args followed by required args (posts),
            // flag each optional arg
            if !optionals.is_empty() && has_required_posts {
                for opt in &optionals {
                    if let Some(opt_node) = opt.as_optional_parameter_node() {
                        let loc = opt_node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Optional arguments should appear at the end of the argument list."
                                .to_string(),
                        );
                        if let Some(corrections) = self.corrections.as_deref_mut() {
                            corrections.push(crate::correction::Correction {
                                start: loc.start_offset(),
                                end: loc.end_offset(),
                                replacement: String::from_utf8_lossy(opt_node.name().as_slice())
                                    .to_string(),
                                cop_name: self.cop.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
                        self.diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // Visit body
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OptionalArguments, "cops/style/optional_arguments");

    #[test]
    fn autocorrect_removes_optional_default_when_followed_by_required_arg() {
        crate::testutil::assert_cop_autocorrect(
            &OptionalArguments,
            b"def foo(a = 1, b)\nend\n",
            b"def foo(a, b)\nend\n",
        );
    }
}
