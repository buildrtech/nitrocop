use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Flags constant definitions (constant assignment, class, module) inside blocks.
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of FPs:** The original implementation used a blacklist approach —
/// `direct_in_block` was set to `true` when entering a `BlockNode`, then selectively
/// reset to `false` for known compound nodes (if, case, begin, etc.). Any node type
/// NOT in the blacklist (e.g., `CaseMatchNode` for pattern matching) would leak the
/// flag through, causing false positives for constants inside those constructs within
/// blocks.
///
/// **Root cause of FNs:** `LambdaNode` (`->`) was not handled. In RuboCop, `any_block`
/// includes `block`, `numblock`, `itblock`, and `lambda` nodes. Prism represents `->` as
/// a separate `LambdaNode`, which the original code did not visit.
///
/// **Fix:** Switched to a whitelist approach. Instead of propagating a flag and resetting
/// it for known compound nodes, we now directly inspect block body children at the block
/// body level. `visit_block_body` iterates over the direct children of the block's
/// `StatementsNode` and checks each for constant/class/module definitions. Only direct
/// children of the block body are flagged — any deeper nesting is handled by normal
/// recursive visitation. Added `visit_lambda_node` for `->` syntax.
pub struct ConstantDefinitionInBlock;

impl Cop for ConstantDefinitionInBlock {
    fn name(&self) -> &'static str {
        "Lint/ConstantDefinitionInBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| vec!["enums".to_string()]);
        let mut visitor = BlockConstVisitor {
            cop: self,
            source,
            allowed_methods,
            current_block_method: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlockConstVisitor<'a, 'src> {
    cop: &'a ConstantDefinitionInBlock,
    source: &'src SourceFile,
    allowed_methods: Vec<String>,
    current_block_method: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl BlockConstVisitor<'_, '_> {
    fn current_method_allowed(&self) -> bool {
        if let Some(method_name) = self.current_block_method.last() {
            self.allowed_methods.iter().any(|a| a == method_name)
        } else {
            false
        }
    }

    /// Check the direct children of a block body for constant/class/module definitions.
    /// This implements the RuboCop pattern `{^any_block [^begin ^^any_block]}`:
    /// a constant must be either a direct child of a block, or a direct child of a
    /// `begin` (StatementsNode) that is itself a direct child of a block.
    fn visit_block_body(&mut self, body: &ruby_prism::Node<'_>) {
        if let Some(stmts) = body.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.check_block_body_child(&stmt);
            }
        } else {
            // Single-expression body (no StatementsNode wrapper)
            self.check_block_body_child(body);
        }
        // Now visit the body normally for nested blocks
        self.visit(body);
    }

    fn check_block_body_child(&mut self, node: &ruby_prism::Node<'_>) {
        if self.current_method_allowed() {
            return;
        }

        // Check for constant assignment (FOO = 1)
        // RuboCop's pattern uses `nil?` for the namespace, which means only bare
        // constant writes are flagged, not namespaced ones (Mod::FOO = 1, ::FOO = 1).
        if node.as_constant_write_node().is_some() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define constants this way within a block.".to_string(),
            ));
        }

        // Check for class definition (class Foo; end)
        if node.as_class_node().is_some() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define constants this way within a block.".to_string(),
            ));
        }

        // Check for module definition (module Foo; end)
        if node.as_module_node().is_some() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define constants this way within a block.".to_string(),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for BlockConstVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit_block_body(&body);
        }
        // Don't call ruby_prism::visit_block_node — we already visited the body above.
        // We skip parameters since they can't contain constant definitions.
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // In RuboCop, `->` lambdas are `any_block` with method_name `lambda`.
        self.current_block_method.push("lambda".to_string());
        if let Some(body) = node.body() {
            self.visit_block_body(&body);
        }
        self.current_block_method.pop();
        // Don't call ruby_prism::visit_lambda_node — we already visited the body above.
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // If this call has a block, record the method name for AllowedMethods check
        if node.block().is_some() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            self.current_block_method.push(name);
            ruby_prism::visit_call_node(self, node);
            self.current_block_method.pop();
        } else {
            ruby_prism::visit_call_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ConstantDefinitionInBlock,
        "cops/lint/constant_definition_in_block"
    );
}
