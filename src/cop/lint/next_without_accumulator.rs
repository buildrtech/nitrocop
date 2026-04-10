use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Detects bare `next` (without accumulator argument) inside `reduce`/`inject` blocks.
///
/// Corpus investigation (FN=2): Both FN cases were `next unless condition` inside
/// reduce/inject blocks. Prism parses `next unless cond` as an UnlessNode containing
/// a NextNode — the default Visit traversal correctly descends into UnlessNode and
/// finds the bare NextNode. The cop logic is correct and test fixtures cover both
/// FN patterns (`next unless memo` in reduce, `next unless Integer === value` in inject).
/// Corpus FN=2 is a stale baseline issue; a re-run should clear it.
pub struct NextWithoutAccumulator;

impl Cop for NextWithoutAccumulator {
    fn name(&self) -> &'static str {
        "Lint/NextWithoutAccumulator"
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
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = NextWithoutAccVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
            in_reduce_block: false,
            acc_name_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NextWithoutAccVisitor<'a, 'src> {
    cop: &'a NextWithoutAccumulator,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
    in_reduce_block: bool,
    acc_name_stack: Vec<Option<String>>,
}

impl<'pr> Visit<'pr> for NextWithoutAccVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        let is_reduce = method_name == b"reduce" || method_name == b"inject";

        if is_reduce && node.receiver().is_some() {
            // Check if this call has a block
            if let Some(block) = node.block()
                && let Some(block_node) = block.as_block_node()
            {
                let old = self.in_reduce_block;
                self.in_reduce_block = true;

                let acc_name = block_node
                    .parameters()
                    .and_then(|params| params.as_block_parameters_node())
                    .and_then(|bp| bp.parameters())
                    .and_then(|p| p.requireds().iter().next())
                    .and_then(|first| first.as_required_parameter_node())
                    .and_then(|rp| std::str::from_utf8(rp.name().as_slice()).ok())
                    .map(|s| s.to_string());
                self.acc_name_stack.push(acc_name);

                if let Some(body) = block_node.body() {
                    self.visit(&body);
                }

                self.acc_name_stack.pop();
                self.in_reduce_block = old;
                return;
            }
        }

        // Visit receiver and arguments normally
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_next_node(&mut self, node: &ruby_prism::NextNode<'pr>) {
        if self.in_reduce_block && node.arguments().is_none() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `next` with an accumulator argument in a `reduce`.".to_string(),
            );

            if let Some(acc_name) = self.acc_name_stack.last().and_then(|name| name.as_deref())
                && let Some(corrections) = self.corrections.as_mut()
            {
                let start = loc.start_offset();
                let end = start.saturating_add(4);
                corrections.push(crate::correction::Correction {
                    start,
                    end,
                    replacement: format!("next {acc_name}"),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            self.diagnostics.push(diagnostic);
        }
    }

    // Don't recurse into nested methods/classes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NextWithoutAccumulator, "cops/lint/next_without_accumulator");
    crate::cop_autocorrect_fixture_tests!(
        NextWithoutAccumulator,
        "cops/lint/next_without_accumulator"
    );
}
