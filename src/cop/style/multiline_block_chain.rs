use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation history
///
/// ### Location fix (2026-03-18)
/// Changed offense location from method name (message_loc) to the dot operator
/// (call_operator_loc) before the chained method. RuboCop reports the offense
/// at `range_between(receiver.loc.end.begin_pos, send_node.source_range.end_pos)`,
/// which starts at the block closing delimiter. The corpus comparison uses
/// line:col, and the dot is the correct location for matching.
///
/// ### Previous failed attempt (commit 38898a01, reverted f8166f95)
/// Combined TWO changes: location fix + intermediate chain walk. The chain walk
/// was too aggressive, swinging from FN=162 to FP=212. The location-only fix
/// was separated out as the safe first step.
///
/// ### Remaining gaps
/// FN from missing intermediate chain walk: RuboCop's `each_node(:call)` walks
/// through non-block intermediate calls (e.g., `.foo.bar do...end` where `.foo`
/// has no block). This needs careful implementation to avoid the over-detection
/// seen in the reverted commit.
pub struct MultilineBlockChain;

/// Visitor that checks for multiline block chains.
/// RuboCop triggers on_block, then checks if the block's send_node
/// has a receiver that is itself a multiline block. We replicate this
/// by visiting CallNodes that have blocks and checking their receiver chain.
struct BlockChainVisitor<'a> {
    source: &'a SourceFile,
    cop_name: &'static str,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BlockChainVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Only check calls that have a real block (do..end or {..}).
        // This matches RuboCop's on_block trigger — only block-to-block chains.
        let has_block = if let Some(block) = node.block() {
            block.as_block_node().is_some()
        } else {
            false
        };

        if has_block {
            // Walk the receiver chain looking for a call with a multiline block
            self.check_receiver_chain(node);
        }

        // Continue traversal into children
        ruby_prism::visit_call_node(self, node);
    }
}

impl BlockChainVisitor<'_> {
    fn check_receiver_chain(&mut self, node: &ruby_prism::CallNode<'_>) {
        let receiver = match node.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Does the receiver call have a real block (do..end or {..})?
        let recv_block = match recv_call.block() {
            Some(b) => b,
            None => return,
        };
        if recv_block.as_block_node().is_none() {
            return;
        }

        // Is the receiver's block multiline?
        let block_loc = recv_block.location();
        let (block_start, _) = self.source.offset_to_line_col(block_loc.start_offset());
        let (block_end, _) = self
            .source
            .offset_to_line_col(block_loc.end_offset().saturating_sub(1));

        if block_start == block_end {
            return;
        }

        // Multiline block chain: receiver has a multiline block,
        // and current node also has a block.
        // RuboCop reports at the dot (call operator) before the method name,
        // not at the method name itself. Fall back to message_loc if no dot.
        let loc = node
            .call_operator_loc()
            .or_else(|| node.message_loc())
            .unwrap_or_else(|| node.location());
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(Diagnostic {
            path: self.source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.cop_name.to_string(),
            message: "Avoid multi-line chains of blocks.".to_string(),

            corrected: false,
        });
    }
}

impl Cop for MultilineBlockChain {
    fn name(&self) -> &'static str {
        "Style/MultilineBlockChain"
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
        let mut visitor = BlockChainVisitor {
            source,
            cop_name: self.name(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineBlockChain, "cops/style/multiline_block_chain");
}
