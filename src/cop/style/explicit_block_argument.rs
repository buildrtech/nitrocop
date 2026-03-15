use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ExplicitBlockArgument: Enforces explicit block argument over `yield`
/// inside a block that just passes its arguments through.
///
/// ## Investigation (2026-03)
///
/// Root causes of false positives (359 FPs, 347 from twilio-ruby):
/// - nitrocop was not checking that the block is inside a method definition
///   (`def`/`defs`). RuboCop's `on_yield` walks up to find `each_ancestor(:any_def)`
///   and skips if none is found. Blocks containing `yield` outside method defs
///   (e.g., in ERB/HAML templates, or top-level DSL code) are not flagged by RuboCop.
///
/// Root causes of false negatives (907 FNs):
/// - nitrocop required block parameters to be non-empty, missing the zero-arg case
///   (`3.times { yield }`) which RuboCop correctly flags.
///
/// Fixes applied:
/// - Switched from `check_node` to `check_source` with a visitor that tracks
///   `def_depth` to ensure blocks are inside method definitions.
/// - Added support for zero-arg blocks with zero-arg yield.
pub struct ExplicitBlockArgument;

impl Cop for ExplicitBlockArgument {
    fn name(&self) -> &'static str {
        "Style/ExplicitBlockArgument"
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
        let mut visitor = ExplicitBlockArgumentVisitor {
            source,
            cop: self,
            diagnostics,
            def_depth: 0,
        };
        visitor.visit(&parse_result.node());
    }
}

struct ExplicitBlockArgumentVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a ExplicitBlockArgument,
    diagnostics: &'a mut Vec<Diagnostic>,
    def_depth: u32,
}

impl<'a> ExplicitBlockArgumentVisitor<'a> {
    /// Check if a call node has a yielding block: `something { |args| yield args }`
    /// where the block body is a single yield statement and args match.
    /// `call_start` is the start offset of the full expression (call + block).
    fn check_call_with_block(&mut self, block: &ruby_prism::BlockNode<'_>, call_start: usize) {
        // Must be inside a method definition
        if self.def_depth == 0 {
            return;
        }

        // Must have a body
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // Single statement must be a yield
        let yield_node = match body_nodes[0].as_yield_node() {
            Some(y) => y,
            None => return,
        };

        // Get block params (may be None for zero-arg blocks like `{ yield }`)
        let block_param_names = self.extract_block_param_names(block);

        // Get yield args
        let yield_arg_names = self.extract_yield_arg_names(&yield_node);

        // Both must have same count
        if block_param_names.len() != yield_arg_names.len() {
            return;
        }

        // Each yield arg must match the corresponding block param
        for (param, arg) in block_param_names.iter().zip(yield_arg_names.iter()) {
            match (param, arg) {
                (Some(p), Some(a)) if p == a => {}
                (None, None) => {} // both zero-arg: ok
                _ => return,
            }
        }

        // Report the offense at the full call+block expression
        let (line, column) = self.source.offset_to_line_col(call_start);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Consider using explicit block argument in the surrounding method's signature over `yield`.".to_string(),
        ));
    }

    /// Extract block parameter names as a list of byte slices.
    /// Returns empty vec for blocks with no parameters.
    fn extract_block_param_names(&self, block: &ruby_prism::BlockNode<'_>) -> Vec<Option<Vec<u8>>> {
        let params = match block.parameters() {
            Some(p) => p,
            None => return vec![],
        };

        let block_params = match params.as_block_parameters_node() {
            Some(p) => p,
            None => return vec![],
        };

        let params_node = match block_params.parameters() {
            Some(p) => p,
            None => return vec![],
        };

        params_node
            .requireds()
            .into_iter()
            .map(|p| {
                p.as_required_parameter_node()
                    .map(|rp| rp.name().as_slice().to_vec())
            })
            .collect()
    }

    /// Extract yield argument names (must all be local variable reads).
    /// Returns empty vec for bare `yield`.
    fn extract_yield_arg_names(
        &self,
        yield_node: &ruby_prism::YieldNode<'_>,
    ) -> Vec<Option<Vec<u8>>> {
        let args = match yield_node.arguments() {
            Some(a) => a,
            None => return vec![],
        };

        args.arguments()
            .into_iter()
            .map(|a| {
                a.as_local_variable_read_node()
                    .map(|lv| lv.name().as_slice().to_vec())
            })
            .collect()
    }
}

impl<'a, 'pr> Visit<'pr> for ExplicitBlockArgumentVisitor<'a> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.def_depth += 1;
        ruby_prism::visit_def_node(self, node);
        self.def_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call has a block that just yields
        if let Some(block_arg) = node.block() {
            if let Some(block) = block_arg.as_block_node() {
                self.check_call_with_block(&block, node.location().start_offset());
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'pr>) {
        // `super { yield }` (no explicit args) parses as ForwardingSuperNode
        if let Some(block) = node.block() {
            self.check_call_with_block(&block, node.location().start_offset());
        }
        ruby_prism::visit_forwarding_super_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        // `super(args) { yield }` parses as SuperNode; block() returns Node
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                self.check_call_with_block(&block_node, node.location().start_offset());
            }
        }
        ruby_prism::visit_super_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExplicitBlockArgument, "cops/style/explicit_block_argument");
}
