use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

/// Standard/BlockSingleLineBraces enforces that single-line blocks use
/// `{...}` instead of `do...end`. Multi-line blocks are allowed with
/// either style. This cop comes from the `standard-custom` gem.
pub struct BlockSingleLineBraces;

impl Cop for BlockSingleLineBraces {
    fn name(&self) -> &'static str {
        "Standard/BlockSingleLineBraces"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = Visitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            corrections,
            ignored_blocks: HashSet::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct Visitor<'a> {
    source: &'a SourceFile,
    cop: &'a BlockSingleLineBraces,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
    ignored_blocks: HashSet<usize>,
}

impl<'a> Visitor<'a> {
    fn check_block(&mut self, block_node: &ruby_prism::BlockNode<'_>, allow_autocorrect: bool) {
        let start_offset = block_node.location().start_offset();
        if self.ignored_blocks.contains(&start_offset) {
            return;
        }

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();
        let opening = opening_loc.as_slice();

        // proper_block_style? = multiline || braces
        if opening != b"do" {
            return; // already braces
        }

        let (open_line, _) = self.source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = self.source.offset_to_line_col(closing_loc.start_offset());
        if open_line != close_line {
            return; // multi-line, allow do..end
        }

        // Single-line do..end block — flag it
        let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
        let mut diagnostic = self.cop.diagnostic(
            self.source,
            line,
            column,
            "Prefer `{...}` over `do...end` for single-line blocks.".to_string(),
        );

        if allow_autocorrect {
            let would_break = self.correction_would_break_code(block_node);
            if let Some(ref mut corrections) = self.corrections {
                if !would_break {
                    corrections.push(crate::correction::Correction {
                        start: opening_loc.start_offset(),
                        end: opening_loc.end_offset(),
                        replacement: "{".to_string(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    corrections.push(crate::correction::Correction {
                        start: closing_loc.start_offset(),
                        end: closing_loc.end_offset(),
                        replacement: "}".to_string(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }
        }

        self.diagnostics.push(diagnostic);
    }

    fn correction_would_break_code(&self, block_node: &ruby_prism::BlockNode<'_>) -> bool {
        // Check if block has keyword parameters
        if let Some(params) = block_node.parameters() {
            if let Some(block_params) = params.as_block_parameters_node() {
                if let Some(inner) = block_params.parameters() {
                    // Check for keyword params
                    if !inner.keywords().is_empty() || inner.keyword_rest().is_some() {
                        // Now check if the parent send node has non-parenthesized args
                        // We can't easily check this without parent context,
                        // so check if this block is in the ignored set (which means
                        // it's in a non-parenthesized arg position). Since we already
                        // returned early for ignored blocks, reaching here means the
                        // send IS parenthesized, so correction is safe.
                        return false;
                    }
                }
            }
        }
        false
    }
}

fn is_operator_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"|" | b"^"
            | b"&"
            | b"<=>"
            | b"=="
            | b"==="
            | b"=~"
            | b">"
            | b">="
            | b"<"
            | b"<="
            | b"<<"
            | b">>"
            | b"+"
            | b"-"
            | b"*"
            | b"/"
            | b"%"
            | b"**"
            | b"~"
            | b"+@"
            | b"-@"
            | b"!@"
            | b"~@"
            | b"[]"
            | b"[]="
            | b"!"
            | b"!="
            | b"!~"
            | b"`"
    )
}

/// Collect blocks that are arguments to non-parenthesized method calls.
/// These blocks cannot be safely converted because `do...end` and `{...}`
/// bind differently in that context.
fn collect_ignored_blocks(node: &ruby_prism::Node<'_>, ignored: &mut HashSet<usize>) {
    if let Some(block_node) = node.as_block_node() {
        ignored.insert(block_node.location().start_offset());
        return;
    }
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            collect_ignored_blocks(&recv, ignored);
        }
        return;
    }
    // Hash without braces (keyword hash in method args)
    if let Some(hash_node) = node.as_hash_node() {
        if hash_node.opening_loc().as_slice() != b"{" {
            // unbraced hash — recurse into children
            for element in hash_node.elements().iter() {
                collect_ignored_blocks(&element, ignored);
            }
        }
        return;
    }
    if let Some(pair) = node.as_assoc_node() {
        collect_ignored_blocks(&pair.key(), ignored);
        collect_ignored_blocks(&pair.value(), ignored);
    }
}

impl<'a> Visit<'a> for Visitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'a>) {
        let name = node.name();
        let name_bytes = name.as_slice();

        let is_parenthesized = node.opening_loc().is_some() && name_bytes != b"[]";
        let is_operator = is_operator_method(name_bytes);
        let is_assignment = name_bytes.ends_with(b"=") && !is_operator;
        let has_args = node.arguments().is_some();

        // For non-parenthesized, non-operator, non-assignment calls with args,
        // collect argument blocks as ignored
        if has_args && !is_parenthesized && !is_operator && !is_assignment {
            for arg in node.arguments().unwrap().arguments().iter() {
                collect_ignored_blocks(&arg, &mut self.ignored_blocks);
            }
        }

        // Check the block attached to this call (if any)
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                // If the call has non-parenthesized arguments, still report but
                // don't autocorrect (do...end → {} would change block binding)
                let allow_autocorrect =
                    !(has_args && !is_parenthesized && !is_operator && !is_assignment);
                self.check_block(&block_node, allow_autocorrect);
            }
        }

        // Visit children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'a>) {
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                self.check_block(&block_node, true);
            }
        }
        ruby_prism::visit_super_node(self, node);
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'a>) {
        if let Some(block_node) = node.block() {
            self.check_block(&block_node, true);
        }
        ruby_prism::visit_forwarding_super_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        BlockSingleLineBraces,
        "cops/standard/block_single_line_braces"
    );
}
