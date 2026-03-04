use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/VoidExpect: flags `expect(...)` or `expect { ... }` calls that are not
/// chained with `.to`, `.not_to`, or `.to_not`.
///
/// Investigation: 52 FPs from asciidoctor-pdf project which uses parenthesized
/// expect calls like `(expect res.exitstatus).to be 0`. In Prism AST, this creates
/// `CallNode("to", receiver: ParenthesesNode(StatementsNode(CallNode("expect"))))`.
/// The original `check_node` approach visited `StatementsNode` and flagged any direct
/// `expect` child — but the `StatementsNode` inside `ParenthesesNode` also matches,
/// causing FPs when the parens are actually chained with `.to`. Switched to
/// `check_source` with a visitor that first collects chained expect calls (receivers
/// of `.to`/`.not_to`/`.to_not`, looking through `ParenthesesNode`), then flags
/// statement-level `expect` calls not in the chained set.
pub struct VoidExpect;

/// Matcher methods that chain on expect
const MATCHER_METHODS: &[&[u8]] = &[b"to", b"not_to", b"to_not"];

impl Cop for VoidExpect {
    fn name(&self) -> &'static str {
        "RSpec/VoidExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = VoidExpectVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            chained_expect_offsets: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct VoidExpectVisitor<'a> {
    cop: &'a VoidExpect,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Start offsets of expect calls that are receivers of .to/.not_to/.to_not
    chained_expect_offsets: Vec<usize>,
}

/// If the node is a receiverless `expect` call (directly or wrapped in parentheses),
/// return its start offset.
fn extract_expect_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"expect" && call.receiver().is_none() {
            return Some(call.location().start_offset());
        }
    }
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if body_nodes.len() == 1 {
                    if let Some(call) = body_nodes[0].as_call_node() {
                        if call.name().as_slice() == b"expect" && call.receiver().is_none() {
                            return Some(call.location().start_offset());
                        }
                    }
                }
            }
        }
    }
    None
}

impl Visit<'_> for VoidExpectVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Record chained expects: .to/.not_to/.to_not whose receiver is an expect call.
        // This runs BEFORE visiting children, so when we later visit the inner
        // StatementsNode (inside ParenthesesNode), the offset is already recorded.
        let name = node.name();
        if MATCHER_METHODS.iter().any(|m| name.as_slice() == *m) {
            if let Some(receiver) = node.receiver() {
                if let Some(offset) = extract_expect_offset(&receiver) {
                    self.chained_expect_offsets.push(offset);
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'_>) {
        for stmt in node.body().iter() {
            // Direct expect call as a statement
            if let Some(call) = stmt.as_call_node() {
                if call.name().as_slice() == b"expect" && call.receiver().is_none() {
                    let offset = call.location().start_offset();
                    if !self.chained_expect_offsets.contains(&offset) {
                        let (line, column) = self.source.offset_to_line_col(offset);
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.".to_string(),
                        ));
                    }
                }
            }
            // Parenthesized expect as a statement: (expect ...)
            if let Some(parens) = stmt.as_parentheses_node() {
                if let Some(body) = parens.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        let body_nodes: Vec<_> = stmts.body().iter().collect();
                        if body_nodes.len() == 1 {
                            if let Some(call) = body_nodes[0].as_call_node() {
                                if call.name().as_slice() == b"expect" && call.receiver().is_none()
                                {
                                    let offset = call.location().start_offset();
                                    if !self.chained_expect_offsets.contains(&offset) {
                                        let loc = parens.location();
                                        let (line, column) =
                                            self.source.offset_to_line_col(loc.start_offset());
                                        self.diagnostics.push(self.cop.diagnostic(
                                            self.source,
                                            line,
                                            column,
                                            "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.".to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Continue visiting child nodes
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VoidExpect, "cops/rspec/void_expect");
}
