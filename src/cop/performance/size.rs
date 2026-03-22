use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Performance/Size flags `.count` (no args, no block) on receivers that are
/// known to be Array or Hash values: literals, `.to_a`/`.to_h` conversions,
/// and `Array()`/`Array[]`/`Hash()`/`Hash[]` constructors.
///
/// Root cause of 36 FNs: the cop previously only matched literal array/hash
/// receivers, missing `.to_a`/`.to_h` chains and `Array()`/`Hash()` calls.
/// Fixed by checking the receiver for conversion methods and constructor
/// patterns in addition to literals.
///
/// FP fix: RuboCop skips `.count` when `node.parent&.block_type?` — i.e., when
/// the `.count` call is the direct body of a block (single-statement block body
/// where the return value is used as the block's value). In Parser AST, a
/// single-statement block has the statement as a direct child of the block node,
/// while multi-statement blocks wrap in `begin`. In Prism, the body is always a
/// `StatementsNode`, so we check statement count and set a flag only for the
/// sole statement in a single-statement block body.
pub struct Size;

impl Cop for Size {
    fn name(&self) -> &'static str {
        "Performance/Size"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let mut visitor = SizeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            parent_is_block: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SizeVisitor<'a, 'src> {
    cop: &'a Size,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when the current node is the sole statement in a block body
    /// (matching RuboCop's `node.parent&.block_type?` for single-statement blocks).
    parent_is_block: bool,
}

impl SizeVisitor<'_, '_> {
    /// Visit a block-like node's body, setting parent_is_block for the sole
    /// statement if the body has exactly one statement.
    fn visit_block_body(&mut self, body: &ruby_prism::Node<'_>) {
        let is_single_stmt = body
            .as_statements_node()
            .is_some_and(|s| s.body().iter().count() == 1);
        let prev = self.parent_is_block;
        self.parent_is_block = is_single_stmt;
        self.visit(body);
        self.parent_is_block = prev;
    }
}

impl<'pr> Visit<'pr> for SizeVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"count"
            && node.arguments().is_none()
            && node.block().is_none()
            && !self.parent_is_block
        {
            if let Some(recv) = node.receiver() {
                if is_array_or_hash_receiver(&recv) {
                    let loc = node.message_loc().unwrap_or(node.location());
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `size` instead of `count`.".to_string(),
                    ));
                }
            }
        }
        // Clear the flag for children — they are not direct block body statements.
        let prev = self.parent_is_block;
        self.parent_is_block = false;
        ruby_prism::visit_call_node(self, node);
        self.parent_is_block = prev;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Visit parameters normally
        if let Some(params) = node.parameters() {
            let prev = self.parent_is_block;
            self.parent_is_block = false;
            self.visit(&params);
            self.parent_is_block = prev;
        }
        // Visit body — set parent_is_block only for single-statement bodies
        if let Some(body) = node.body() {
            self.visit_block_body(&body);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params) = node.parameters() {
            let prev = self.parent_is_block;
            self.parent_is_block = false;
            self.visit(&params);
            self.parent_is_block = prev;
        }
        if let Some(body) = node.body() {
            self.visit_block_body(&body);
        }
    }
}

/// Returns true if the node is known to produce an Array or Hash:
/// - Array/Hash literals
/// - `.to_a` / `.to_h` calls (any receiver)
/// - `Array[...]` / `Array(...)` / `Hash[...]` / `Hash(...)`
fn is_array_or_hash_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // Array or Hash literal (including keyword hash arguments)
    if node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
    {
        return true;
    }

    // Check for call-based patterns: .to_a, .to_h, Array[], Array(), Hash[], Hash()
    if let Some(call) = node.as_call_node() {
        let name = call.name();
        let name_bytes = name.as_slice();

        // .to_a or .to_h on any receiver
        if name_bytes == b"to_a" || name_bytes == b"to_h" {
            return true;
        }

        // Array[...] or Hash[...] — `[]` method on constant `Array` or `Hash`
        if name_bytes == b"[]" {
            if let Some(recv) = call.receiver() {
                if is_array_or_hash_constant(&recv) {
                    return true;
                }
            }
        }

        // Array(...) or Hash(...) — Kernel method call with no explicit receiver
        if (name_bytes == b"Array" || name_bytes == b"Hash") && call.receiver().is_none() {
            return true;
        }
    }

    false
}

/// Checks if a node is a constant `Array` or `Hash` (simple or qualified like `::Array`).
fn is_array_or_hash_constant(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        let name = c.name();
        let name_bytes = name.as_slice();
        return name_bytes == b"Array" || name_bytes == b"Hash";
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::Array or ::Hash (top-level constant path with no parent)
        if cp.parent().is_none() {
            let src = cp.location().as_slice();
            return src == b"::Array" || src == b"::Hash";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Size, "cops/performance/size");
}
