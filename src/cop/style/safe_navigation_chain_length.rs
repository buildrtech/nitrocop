use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Enforces safe navigation chains length to not exceed the configured maximum.
///
/// ## Investigation findings (2026-03-15)
///
/// **Root cause of 28 FPs:** nitrocop counted `&.` operators across block boundaries,
/// but RuboCop's Parser gem AST wraps `a&.method { block }` in a `block` node that
/// breaks the csend ancestor chain. In Prism, blocks are children of CallNode, so
/// naive receiver-chain walking doesn't see the boundary.
///
/// **Fix:** When counting the safe navigation chain downward through receivers,
/// if a CallNode has a block, count that `&.` but stop recursing into its receiver.
/// This matches RuboCop's behavior where block nodes break `each_ancestor` traversal.
///
/// **Message fix:** Changed from "Do not chain more than N safe navigation operators.
/// (found M)" to "Avoid safe navigation chains longer than N calls." to match RuboCop.
pub struct SafeNavigationChainLength;

impl Cop for SafeNavigationChainLength {
    fn name(&self) -> &'static str {
        "Style/SafeNavigationChainLength"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max = config.get_usize("Max", 2);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must use safe navigation (&.)
        if !is_safe_nav(&call) {
            return;
        }

        // Count the chain length
        let chain_len = count_safe_nav_chain(node);
        if chain_len <= max {
            return;
        }

        // Only report on the outermost call in the chain
        // (skip if this node is itself a receiver of another &. call)
        // We can't walk up, so we report on every call that exceeds the limit
        // but only the outermost will have the full chain.

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Avoid safe navigation chains longer than {} calls.", max),
        );
        if let Some(corrections) = corrections {
            corrections.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: "nil".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }
        diagnostics.push(diagnostic);
    }
}

fn is_safe_nav(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(op) = call.call_operator_loc() {
        op.as_slice() == b"&."
    } else {
        false
    }
}

fn count_safe_nav_chain(node: &ruby_prism::Node<'_>) -> usize {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return 0,
    };

    if !is_safe_nav(&call) {
        return 0;
    }

    let recv_count = match call.receiver() {
        Some(r) => count_safe_nav_chain_receiver(&r),
        None => 0,
    };

    1 + recv_count
}

/// Count safe navigation chain length walking down through receivers.
/// A block-bearing `&.` call acts as a chain boundary — in RuboCop's Parser AST,
/// `a&.method { block }` wraps the csend in a block node, which stops the
/// ancestor traversal. So we don't count it or recurse past it.
fn count_safe_nav_chain_receiver(node: &ruby_prism::Node<'_>) -> usize {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return 0,
    };

    if !is_safe_nav(&call) {
        return 0;
    }

    // A block on the receiver breaks the chain — stop counting here.
    if call.block().is_some() {
        return 0;
    }

    let recv_count = match call.receiver() {
        Some(r) => count_safe_nav_chain_receiver(&r),
        None => 0,
    };

    1 + recv_count
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationChainLength,
        "cops/style/safe_navigation_chain_length"
    );

    #[test]
    fn autocorrect_replaces_overlong_safe_navigation_chain_with_nil() {
        crate::testutil::assert_cop_autocorrect(
            &SafeNavigationChainLength,
            b"a&.b&.c&.d\n",
            b"nil\n",
        );
    }
}
