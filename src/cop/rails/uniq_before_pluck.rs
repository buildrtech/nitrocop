/// Rails/UniqBeforePluck — flag `pluck(...).uniq` and suggest `distinct.pluck(...)`.
///
/// ## Root cause (2026-03)
/// Original implementation used `CONSTANT_PATH_NODE` / `CONSTANT_READ_NODE` as interested
/// node types and tried to walk *up* from the model constant to find a `.pluck(...).uniq`
/// chain via `as_method_chain`. This never worked because `as_method_chain` expects a
/// `CallNode` as input — a constant read node is not a call node, so it always returned
/// `None` and produced zero offenses.
///
/// ## Fix (2026-03, round 1)
/// Switched to `CALL_NODE` as the interested type.  On every `CallNode`, check whether the
/// method name is `uniq` or `uniq!` (no block arguments), and whether the receiver is also a
/// `CallNode` whose method name is `pluck`.  In conservative mode (default), additionally
/// require that the root receiver of the `pluck` call is a constant (model class), not an
/// instance variable or a chained scope/association.
///
/// ## Fix (2026-03, round 2) — false positives from `uniq!`
/// RuboCop uses `RESTRICT_ON_SEND = %i[uniq].freeze` — it only triggers on `uniq`, NOT on
/// `uniq!`. The round-1 implementation also flagged `uniq!`, causing 2 false positives in the
/// corpus. Fixed by only checking for `uniq` (not `uniq!`).
///
/// Offense is reported at the `uniq` selector location (matching RuboCop's
/// `node.loc.selector`), i.e., `message_loc()` of the `uniq` call node.
use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UniqBeforePluck;

impl Cop for UniqBeforePluck {
    fn name(&self) -> &'static str {
        "Rails/UniqBeforePluck"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let uniq_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Only interested in `uniq` method calls.
        // RuboCop restricts on :uniq only (RESTRICT_ON_SEND = %i[uniq].freeze).
        // `uniq!` is intentionally excluded — flagging it would be a false positive.
        let method_name = uniq_call.name().as_slice();
        if method_name != b"uniq" {
            return;
        }

        // uniq/uniq! must not have a block argument
        if uniq_call.block().is_some() {
            return;
        }

        // The receiver of `uniq` must be a `pluck(...)` call
        let receiver = match uniq_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let pluck_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if pluck_call.name().as_slice() != b"pluck" {
            return;
        }

        let style = config.get_str("EnforcedStyle", "conservative");

        // In conservative mode, only flag if the root receiver of pluck is a constant (model class)
        if style == "conservative" {
            let pluck_receiver = match pluck_call.receiver() {
                Some(r) => r,
                None => return, // no receiver (bare `pluck(...).uniq`) — skip in conservative
            };
            let is_const = pluck_receiver.as_constant_read_node().is_some()
                || pluck_receiver.as_constant_path_node().is_some();
            if !is_const {
                return;
            }
        }

        // Report at the `uniq` selector (message_loc), matching RuboCop's node.loc.selector
        let loc = uniq_call
            .message_loc()
            .unwrap_or_else(|| uniq_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `distinct` before `pluck`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UniqBeforePluck, "cops/rails/uniq_before_pluck");
}
