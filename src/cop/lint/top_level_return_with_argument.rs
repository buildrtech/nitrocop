use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TopLevelReturnWithArgument;

impl Cop for TopLevelReturnWithArgument {
    fn name(&self) -> &'static str {
        "Lint/TopLevelReturnWithArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = TopLevelReturnVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            offense_ranges: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        if let Some(ref mut corr) = corrections {
            for &(start, end) in &visitor.offense_ranges {
                corr.push(crate::correction::Correction {
                    start,
                    end,
                    replacement: "return".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
            for diag in &mut visitor.diagnostics {
                diag.corrected = true;
            }
        }

        diagnostics.extend(visitor.diagnostics);
    }
}

struct TopLevelReturnVisitor<'a, 'src> {
    cop: &'a TopLevelReturnWithArgument,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    offense_ranges: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for TopLevelReturnVisitor<'_, '_> {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // Only flag returns with arguments
        if node.arguments().is_some() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Top level return with argument detected.".to_string(),
            ));
            self.offense_ranges
                .push((loc.start_offset(), loc.end_offset()));
        }
    }

    // Don't recurse into def, class, module, or blocks (not top-level)
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
    fn visit_singleton_class_node(&mut self, _node: &ruby_prism::SingletonClassNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TopLevelReturnWithArgument,
        "cops/lint/top_level_return_with_argument"
    );
    crate::cop_autocorrect_fixture_tests!(
        TopLevelReturnWithArgument,
        "cops/lint/top_level_return_with_argument"
    );
}
