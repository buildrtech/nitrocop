use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MapMethodChain;

/// Check if a call node has a block_pass argument with a symbol (e.g., `&:foo`).
fn has_symbol_block_pass(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(block) = call.block()
        && let Some(bp) = block.as_block_argument_node()
        && let Some(expr) = bp.expression()
    {
        return expr.as_symbol_node().is_some();
    }
    false
}

/// Check if a call is a map/collect call.
fn is_map_or_collect(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    name == b"map" || name == b"collect"
}

fn symbol_block_name(call: &ruby_prism::CallNode<'_>) -> Option<String> {
    let block = call.block()?.as_block_argument_node()?;
    let expr = block.expression()?;
    let sym = expr.as_symbol_node()?;
    let name = String::from_utf8_lossy(sym.unescaped()).to_string();
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '!' || c == '?')
    {
        Some(name)
    } else {
        None
    }
}

impl Cop for MapMethodChain {
    fn name(&self) -> &'static str {
        "Performance/MapMethodChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE, SYMBOL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop uses on_send (not on_csend), so skip safe navigation calls
        if outer_call
            .call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
        {
            return;
        }

        if !is_map_or_collect(&outer_call) || !has_symbol_block_pass(&outer_call) {
            return;
        }

        // The receiver must also be a map/collect with symbol block_pass
        let inner_node = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match inner_node.as_call_node() {
            Some(c) if is_map_or_collect(&c) && has_symbol_block_pass(&c) => c,
            _ => return,
        };

        let inner_start_offset = inner_call.location().start_offset();
        let inner_first_symbol = symbol_block_name(&inner_call);
        let inner_base_src = inner_call.receiver().map(|base| {
            source
                .byte_slice(
                    base.location().start_offset(),
                    base.location().end_offset(),
                    "",
                )
                .to_string()
        });
        let inner_uses_safe_nav = inner_call
            .call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.");

        // Walk down the receiver chain to find the deepest consecutive
        // map/collect call with symbol block_pass (the chain start).
        let mut chain_start = inner_call;
        while let Some(recv) = chain_start.receiver() {
            if let Some(c) = recv.as_call_node()
                && is_map_or_collect(&c)
                && has_symbol_block_pass(&c)
            {
                chain_start = c;
                continue;
            }
            break;
        }

        // RuboCop quirk: when walking down the chain, if the receiver of the
        // chain start is a non-map/collect call that also has a symbol block_pass
        // (e.g. `select(&:active).map(&:name).map(&:to_s)`), the recursive
        // find_begin_of_chained_map_method enters that receiver but returns nil
        // because it's not map/collect, causing the entire offense to be skipped.
        if let Some(recv) = chain_start.receiver()
            && let Some(c) = recv.as_call_node()
            && !is_map_or_collect(&c)
            && has_symbol_block_pass(&c)
        {
            return;
        }

        // Report at the chain start's selector (message_loc) position.
        let start_offset = chain_start.message_loc().map_or_else(
            || chain_start.location().start_offset(),
            |loc| loc.start_offset(),
        );
        let (line, column) = source.offset_to_line_col(start_offset);

        // Deduplicate: for chains of 3+ maps, multiple outer calls walk down
        // to the same chain_start. Skip if already reported at this position.
        if diagnostics
            .iter()
            .any(|d| d.location.line == line && d.location.column == column)
        {
            return;
        }

        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `map` with a block instead of chaining multiple `map` calls with symbol arguments."
                .to_string(),
        );

        // Conservative autocorrect for exactly two-hop chains.
        if !inner_uses_safe_nav
            && chain_start.location().start_offset() == inner_start_offset
            && let Some(first) = inner_first_symbol
            && let Some(second) = symbol_block_name(&outer_call)
            && let Some(ref mut corr) = corrections
        {
            let body = format!("e.{first}.{second}");
            let replacement = if let Some(base_src) = inner_base_src {
                format!("{base_src}.map {{ |e| {body} }}")
            } else {
                format!("map {{ |e| {body} }}")
            };
            corr.push(crate::correction::Correction {
                start: inner_start_offset,
                end: outer_call.location().end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapMethodChain, "cops/performance/map_method_chain");
    crate::cop_autocorrect_fixture_tests!(MapMethodChain, "cops/performance/map_method_chain");

    #[test]
    fn supports_autocorrect() {
        assert!(MapMethodChain.supports_autocorrect());
    }
}
