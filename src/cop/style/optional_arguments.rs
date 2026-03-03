use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OptionalArguments;

impl Cop for OptionalArguments {
    fn name(&self) -> &'static str {
        "Style/OptionalArguments"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = OptionalArgumentsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OptionalArgumentsVisitor<'a> {
    cop: &'a OptionalArguments,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for OptionalArgumentsVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
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
                        self.diagnostics.push(
                            self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Optional arguments should appear at the end of the argument list."
                                    .to_string(),
                            ),
                        );
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
}
