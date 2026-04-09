use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Performance/Count: flags `select/reject/filter/find_all { }.count/size/length`.
///
/// Investigation (2026-03): Fixed false negatives when `select/find_all { }.size/length`
/// appeared as a sub-expression (e.g., `find_all { |c| ... }.length > 1`). Root cause:
/// the block-body skip logic compared only start offsets, but chained CallNodes share the
/// same start offset as their enclosing expression. Fixed by comparing both start AND end
/// offsets to ensure the call IS the entire sole statement, not just a prefix of it.
pub struct Count;

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

fn block_source_with_optional_negation(
    source: &SourceFile,
    block: &ruby_prism::Node<'_>,
    negate: bool,
) -> Option<String> {
    if let Some(block_arg) = block.as_block_argument_node() {
        if !negate {
            return Some(node_source(source, block));
        }

        let expr = block_arg.expression()?;
        if let Some(sym) = expr.as_symbol_node() {
            let method = std::str::from_utf8(sym.unescaped()).ok()?;
            return Some(format!("{{ |element| !element.{method} }}"));
        }

        let expr_src = node_source(source, &expr);
        return Some(format!("{{ !{expr_src}.call }}"));
    }

    let block_node = block.as_block_node()?;
    let mut block_src = node_source(source, block);
    if !negate {
        return Some(block_src);
    }

    let body = block_node.body()?;
    let target = if let Some(stmts) = body.as_statements_node() {
        stmts.body().iter().last().unwrap_or(body)
    } else {
        body
    };

    let block_loc = block.location();
    let target_loc = target.location();
    if target_loc.start_offset() < block_loc.start_offset()
        || target_loc.end_offset() > block_loc.end_offset()
    {
        return None;
    }

    let rel_start = target_loc.start_offset() - block_loc.start_offset();
    let rel_end = target_loc.end_offset() - block_loc.start_offset();
    if rel_end > block_src.len() || rel_start > rel_end {
        return None;
    }

    let target_src = source.byte_slice(target_loc.start_offset(), target_loc.end_offset(), "");
    block_src.replace_range(rel_start..rel_end, &format!("!({target_src})"));
    Some(block_src)
}

fn build_count_replacement(
    source: &SourceFile,
    inner_call: &ruby_prism::CallNode<'_>,
    selector: &str,
) -> Option<String> {
    let receiver = inner_call.receiver()?;
    let receiver_src = node_source(source, &receiver);
    let block = inner_call.block()?;

    let negate = selector == "reject";
    let block_src = block_source_with_optional_negation(source, &block, negate)?;

    if block.as_block_argument_node().is_some() {
        if negate {
            Some(format!("{receiver_src}.count {block_src}"))
        } else {
            Some(format!("{receiver_src}.count({block_src})"))
        }
    } else {
        Some(format!("{receiver_src}.count {block_src}"))
    }
}

impl Cop for Count {
    fn name(&self) -> &'static str {
        "Performance/Count"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let mut visitor = CountVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            single_stmt_block_body_range: None,
            corrections: Vec::new(),
            autocorrect_enabled: corrections.is_some(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

struct CountVisitor<'a, 'src> {
    cop: &'a Count,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    autocorrect_enabled: bool,
    /// Byte offset range (start, end) of the sole statement in the current block body, if any.
    /// RuboCop skips `select{}.count` when its direct parent is a block node
    /// (`node.parent&.block_type?`). We track the range of the single
    /// statement so we only skip when the count call IS that statement, not
    /// when it's nested inside a comparison or other expression.
    single_stmt_block_body_range: Option<(usize, usize)>,
}

impl<'pr> Visit<'pr> for CountVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Record the byte range of the sole statement in the block body.
        let prev = self.single_stmt_block_body_range;
        self.single_stmt_block_body_range = single_statement_range(node.body());
        ruby_prism::visit_block_node(self, node);
        self.single_stmt_block_body_range = prev;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas are block-like in parser gem
        let prev = self.single_stmt_block_body_range;
        self.single_stmt_block_body_range = single_statement_range(node.body());
        ruby_prism::visit_lambda_node(self, node);
        self.single_stmt_block_body_range = prev;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

impl CountVisitor<'_, '_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Outer method must be count/size/length
        let outer = call.name().as_slice();
        let outer_name = match outer {
            b"count" => "count",
            b"size" => "size",
            b"length" => "length",
            _ => return,
        };

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver must be a CallNode (the inner select/reject/filter/find_all)
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner = inner_call.name().as_slice();
        let inner_name = match inner {
            b"select" => "select",
            b"reject" => "reject",
            b"filter" => "filter",
            b"find_all" => "find_all",
            _ => return,
        };

        // The inner call must have a block (normal block or block_pass like &:symbol)
        let inner_block = match inner_call.block() {
            Some(b) => b,
            None => return,
        };

        // If the block is a regular block (not block_pass), it must have a body.
        // RuboCop: `node.receiver.call_type? || node.receiver.body`
        // block_pass → call_type? is true (receiver is call node in parser-gem)
        // regular block → check body is present (non-empty block)
        if let Some(block_node) = inner_block.as_block_node() {
            if block_node.body().is_none() {
                return;
            }

            // RuboCop's Parser gem has separate `block` and `numblock` node types.
            // `numblock` (used for _1/_2 numbered params and Ruby 3.4 `it`) returns
            // false for `block_type?`, causing RuboCop to skip these chains.
            // Match that behavior: skip when the block uses numbered or it params.
            if let Some(params) = block_node.parameters() {
                if params.as_numbered_parameters_node().is_some()
                    || params.as_it_parameters_node().is_some()
                {
                    return;
                }
            }
        }

        // Skip if the outer call (count/size/length) has arguments.
        // RuboCop's NodePattern only matches argumentless count/size/length.
        if call.arguments().is_some() {
            return;
        }

        // Skip if the outer call (count/size/length) itself has a block:
        // e.g. `select { |e| e.odd? }.count { |e| e > 2 }` is allowed
        if call.block().is_some() {
            return;
        }

        // Skip if this call is the direct sole statement of a block body.
        // RuboCop: `return false if node.parent&.block_type?`
        // We compare both start AND end offsets to ensure the call IS the
        // entire statement, not just a sub-expression (e.g., in
        // `find_all { |c| c == u }.length > 1`, the `.length` call shares
        // the same start offset as the `>` call but has a smaller end offset).
        if let Some((start, end)) = self.single_stmt_block_body_range {
            let loc = call.location();
            if loc.start_offset() == start && loc.end_offset() == end {
                return;
            }
        }

        // Report the offense at the inner selector call (select/reject/filter/find_all),
        // not at the outer count/size/length call. This matches RuboCop's behavior
        // and produces the correct line for multi-line chains.
        let loc = inner_call
            .message_loc()
            .unwrap_or_else(|| inner_call.location());
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Use `count` instead of `{inner_name}...{outer_name}`."),
        );

        if self.autocorrect_enabled
            && let Some(replacement) = build_count_replacement(self.source, &inner_call, inner_name)
        {
            let outer_loc = call.location();
            self.corrections.push(crate::correction::Correction {
                start: outer_loc.start_offset(),
                end: outer_loc.end_offset(),
                replacement,
                cop_name: self.cop.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        self.diagnostics.push(diagnostic);
    }
}

/// If the block/lambda body has exactly one statement, return its (start, end) byte offsets.
fn single_statement_range(body: Option<ruby_prism::Node<'_>>) -> Option<(usize, usize)> {
    let body = body?;
    match body.as_statements_node() {
        Some(stmts) if stmts.body().len() == 1 => {
            let node = stmts.body().iter().next().unwrap();
            let loc = node.location();
            Some((loc.start_offset(), loc.end_offset()))
        }
        Some(_) => None,
        // Body is a single non-statements node
        None => {
            let loc = body.location();
            Some((loc.start_offset(), loc.end_offset()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Count, "cops/performance/count");
    crate::cop_autocorrect_fixture_tests!(Count, "cops/performance/count");

    #[test]
    fn supports_autocorrect() {
        assert!(Count.supports_autocorrect());
    }
}
