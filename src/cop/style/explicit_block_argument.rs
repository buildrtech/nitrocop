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
/// - Fixed FPs on destructured block params `|(key, val)|` — these are
///   MultiTargetNode, not RequiredParameterNode, and were returning None
///   which matched (None, None) in the zip comparison.
/// - Fixed FPs on blocks with `&b` parameter — `extract_block_param_names`
///   now checks for block, rest, and keyword_rest params and bails out.
/// - Remaining FNs (2): `->{ yield }.call` — lambda containing yield. This
///   is a LambdaNode as receiver of `.call`, not a block attached to a call.
///   Would require visiting LambdaNode bodies, which is a different pattern.
pub struct ExplicitBlockArgument;

impl Cop for ExplicitBlockArgument {
    fn name(&self) -> &'static str {
        "Style/ExplicitBlockArgument"
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
        let mut visitor = ExplicitBlockArgumentVisitor {
            source,
            cop: self,
            diagnostics,
            corrections,
            def_depth: 0,
            def_block_params: Vec::new(),
        };
        visitor.visit(&parse_result.node());
    }
}

struct ExplicitBlockArgumentVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a ExplicitBlockArgument,
    diagnostics: &'a mut Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
    def_depth: u32,
    def_block_params: Vec<Option<Vec<u8>>>,
}

impl<'a> ExplicitBlockArgumentVisitor<'a> {
    /// Check if a call node has a yielding block: `something { |args| yield args }`
    /// where the block body is a single yield statement and args match.
    /// `call_start` is the start offset of the full expression (call + block).
    fn check_call_with_block(
        &mut self,
        block: &ruby_prism::BlockNode<'_>,
        call_start: usize,
        autocorrect_target: Option<(usize, usize)>,
    ) {
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

        // Get block params (may be empty for zero-arg blocks like `{ yield }`)
        // Returns None if block has non-simple params (destructured, &block, *rest, **kwrest)
        let block_param_names = match self.extract_block_param_names(block) {
            Some(names) => names,
            None => return,
        };

        // Get yield args (None if any arg is not a simple local variable read)
        let yield_arg_names = match self.extract_yield_arg_names(&yield_node) {
            Some(names) => names,
            None => return,
        };

        // Both must have same count
        if block_param_names.len() != yield_arg_names.len() {
            return;
        }

        // Each yield arg must match the corresponding block param
        for (param, arg) in block_param_names.iter().zip(yield_arg_names.iter()) {
            if param != arg {
                return;
            }
        }

        // Report the offense at the full call+block expression
        let (line, column) = self.source.offset_to_line_col(call_start);
        let mut diag = self.cop.diagnostic(
            self.source,
            line,
            column,
            "Consider using explicit block argument in the surrounding method's signature over `yield`.".to_string(),
        );

        // Conservative autocorrect subset: only receiver calls with no arguments,
        // in defs that already declare an explicit named &block parameter.
        if let (Some((replace_start, replace_end)), Some(block_param), Some(corrections)) = (
            autocorrect_target,
            self.def_block_params.last().and_then(|n| n.as_ref()),
            self.corrections.as_deref_mut(),
        ) {
            let replacement = format!("(&{})", String::from_utf8_lossy(block_param));
            corrections.push(crate::correction::Correction {
                start: replace_start,
                end: replace_end,
                replacement,
                cop_name: self.cop.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }

        self.diagnostics.push(diag);
    }

    /// Extract block parameter names as a list of byte slices.
    /// Returns `Some(vec![])` for blocks with no parameters.
    /// Returns `None` if block has non-simple params (destructured, &block, *rest, **kwrest).
    fn extract_block_param_names(&self, block: &ruby_prism::BlockNode<'_>) -> Option<Vec<Vec<u8>>> {
        let params = match block.parameters() {
            Some(p) => p,
            None => return Some(vec![]),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(p) => p,
            None => return Some(vec![]),
        };

        let params_node = match block_params.parameters() {
            Some(p) => p,
            None => return Some(vec![]),
        };

        // Bail out if block has &block, *rest, or **kwrest parameters
        if params_node.block().is_some()
            || params_node.rest().is_some()
            || params_node.keyword_rest().is_some()
        {
            return None;
        }

        let mut names = Vec::new();
        for p in params_node.requireds().into_iter() {
            match p.as_required_parameter_node() {
                Some(rp) => names.push(rp.name().as_slice().to_vec()),
                // Destructured param like |(key, val)| — not simple
                None => return None,
            }
        }
        Some(names)
    }

    /// Extract yield argument names (must all be local variable reads).
    /// Returns empty vec for bare `yield`.
    /// Returns `None` if any argument is not a simple local variable read.
    fn extract_yield_arg_names(
        &self,
        yield_node: &ruby_prism::YieldNode<'_>,
    ) -> Option<Vec<Vec<u8>>> {
        let args = match yield_node.arguments() {
            Some(a) => a,
            None => return Some(vec![]),
        };

        let mut names = Vec::new();
        for a in args.arguments().into_iter() {
            match a.as_local_variable_read_node() {
                Some(lv) => names.push(lv.name().as_slice().to_vec()),
                None => return None,
            }
        }
        Some(names)
    }
}

impl<'a, 'pr> Visit<'pr> for ExplicitBlockArgumentVisitor<'a> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.def_depth += 1;
        let block_name = node
            .parameters()
            .and_then(|p| p.block())
            .and_then(|b| b.name())
            .map(|n| n.as_slice().to_vec());
        self.def_block_params.push(block_name);
        ruby_prism::visit_def_node(self, node);
        self.def_block_params.pop();
        self.def_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call has a block that just yields
        if let Some(block_arg) = node.block() {
            if let Some(block) = block_arg.as_block_node() {
                let args_len = node.arguments().map(|a| a.arguments().len()).unwrap_or(0);
                let autocorrect_target = if args_len == 0 {
                    let block_loc = block.location();
                    let mut start = block_loc.start_offset();
                    if start > 0 && self.source.as_bytes()[start - 1] == b' ' {
                        start -= 1;
                    }
                    Some((start, block_loc.end_offset()))
                } else {
                    None
                };
                self.check_call_with_block(&block, node.location().start_offset(), autocorrect_target);
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'pr>) {
        // `super { yield }` (no explicit args) parses as ForwardingSuperNode
        if let Some(block) = node.block() {
            self.check_call_with_block(&block, node.location().start_offset(), None);
        }
        ruby_prism::visit_forwarding_super_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        // `super(args) { yield }` parses as SuperNode; block() returns Node
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                self.check_call_with_block(&block_node, node.location().start_offset(), None);
            }
        }
        ruby_prism::visit_super_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExplicitBlockArgument, "cops/style/explicit_block_argument");
    crate::cop_autocorrect_fixture_tests!(
        ExplicitBlockArgument,
        "cops/style/explicit_block_argument_autocorrect"
    );
}
