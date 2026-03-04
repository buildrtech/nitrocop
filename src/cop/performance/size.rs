use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/Size flags `.count` (no args, no block) on receivers that are
/// known to be Array or Hash values: literals, `.to_a`/`.to_h` conversions,
/// and `Array()`/`Array[]`/`Hash()`/`Hash[]` constructors.
///
/// Root cause of 36 FNs: the cop previously only matched literal array/hash
/// receivers, missing `.to_a`/`.to_h` chains and `Array()`/`Hash()` calls.
/// Fixed by checking the receiver for conversion methods and constructor
/// patterns in addition to literals.
pub struct Size;

impl Cop for Size {
    fn name(&self) -> &'static str {
        "Performance/Size"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"count" {
            return;
        }

        // Must have no arguments and no block
        if call.arguments().is_some() || call.block().is_some() {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_array_or_hash_receiver(&recv) {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `size` instead of `count`.".to_string(),
        ));
    }
}

/// Returns true if the node is known to produce an Array or Hash:
/// - Array/Hash literals
/// - `.to_a` / `.to_h` calls (any receiver)
/// - `Array[...]` / `Array(...)` / `Hash[...]` / `Hash(...)`
fn is_array_or_hash_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // Array or Hash literal
    if node.as_array_node().is_some() || node.as_hash_node().is_some() {
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

/// Checks if a node is a simple constant read of `Array` or `Hash`.
fn is_array_or_hash_constant(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        let name = c.name();
        let name_bytes = name.as_slice();
        return name_bytes == b"Array" || name_bytes == b"Hash";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Size, "cops/performance/size");
}
