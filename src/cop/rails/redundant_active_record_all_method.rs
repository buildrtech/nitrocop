use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Detects redundant `all` used as a receiver for Active Record query methods.
///
/// ## Investigation findings (2026-03-08)
///
/// Root causes of corpus divergence (FP=419, FN=194):
///
/// 1. **FN=194**: The method list (`REDUNDANT_AFTER_ALL`) had only 18 methods vs vendor's
///    100+ from `ActiveRecord::Querying::QUERYING_METHODS`. Expanded to match vendor.
///
/// 2. **FP=419**: Multiple causes:
///    - Offense location was reported at the outer call node (full chain start) instead
///      of at the `all` method name position. Fixed to use `inner_call.message_loc()`.
///    - Message included extra "Remove `all` from the chain." text not in vendor.
///    - Missing check to skip `all` called with arguments (e.g., `page.all(:param)`).
///    - Missing `sensitive_association_method?` logic: `delete_all`/`destroy_all` should
///      only be flagged when receiver of `all` is a constant (model), not an association.
pub struct RedundantActiveRecordAllMethod;

/// ActiveRecord::Querying::QUERYING_METHODS (from activerecord 7.1.0)
/// plus `empty?` which is inherited from Enumerable but still valid.
const QUERYING_METHODS: &[&[u8]] = &[
    b"and",
    b"annotate",
    b"any?",
    b"async_average",
    b"async_count",
    b"async_ids",
    b"async_maximum",
    b"async_minimum",
    b"async_pick",
    b"async_pluck",
    b"async_sum",
    b"average",
    b"calculate",
    b"count",
    b"create_or_find_by",
    b"create_or_find_by!",
    b"create_with",
    b"delete_all",
    b"delete_by",
    b"destroy_all",
    b"destroy_by",
    b"distinct",
    b"eager_load",
    b"except",
    b"excluding",
    b"exists?",
    b"extending",
    b"extract_associated",
    b"fifth",
    b"fifth!",
    b"find",
    b"find_by",
    b"find_by!",
    b"find_each",
    b"find_in_batches",
    b"find_or_create_by",
    b"find_or_create_by!",
    b"find_or_initialize_by",
    b"find_sole_by",
    b"first",
    b"first!",
    b"first_or_create",
    b"first_or_create!",
    b"first_or_initialize",
    b"forty_two",
    b"forty_two!",
    b"fourth",
    b"fourth!",
    b"from",
    b"group",
    b"having",
    b"ids",
    b"in_batches",
    b"in_order_of",
    b"includes",
    b"invert_where",
    b"joins",
    b"last",
    b"last!",
    b"left_joins",
    b"left_outer_joins",
    b"limit",
    b"lock",
    b"many?",
    b"maximum",
    b"merge",
    b"minimum",
    b"none",
    b"none?",
    b"offset",
    b"one?",
    b"only",
    b"optimizer_hints",
    b"or",
    b"order",
    b"pick",
    b"pluck",
    b"preload",
    b"readonly",
    b"references",
    b"regroup",
    b"reorder",
    b"reselect",
    b"rewhere",
    b"second",
    b"second!",
    b"second_to_last",
    b"second_to_last!",
    b"select",
    b"sole",
    b"strict_loading",
    b"sum",
    b"take",
    b"take!",
    b"third",
    b"third!",
    b"third_to_last",
    b"third_to_last!",
    b"touch_all",
    b"unscope",
    b"update_all",
    b"where",
    b"with",
    b"without",
];

/// Methods that could be Enumerable block methods instead of AR query methods.
/// When called with a block, these should NOT be flagged as redundant `all`.
const POSSIBLE_ENUMERABLE_BLOCK_METHODS: &[&[u8]] = &[
    b"any?", b"count", b"find", b"none?", b"one?", b"select", b"sum",
];

/// Methods that are sensitive on associations — `delete_all` and `destroy_all`
/// behave differently on `ActiveRecord::Relation` vs `CollectionProxy`.
/// Only flag these when the receiver of `all` is a constant (i.e., a model class).
const SENSITIVE_METHODS_ON_ASSOCIATION: &[&[u8]] = &[b"delete_all", b"destroy_all"];

impl Cop for RedundantActiveRecordAllMethod {
    fn name(&self) -> &'static str {
        "Rails/RedundantActiveRecordAllMethod"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
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
        let allowed_receivers = config.get_string_array("AllowedReceivers");

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"all" {
            return;
        }

        // Skip if `all` is called with arguments (e.g., `page.all(:parameter)`)
        // — that's not ActiveRecord's `all`.
        if chain.inner_call.arguments().is_some() {
            return;
        }

        if !QUERYING_METHODS.contains(&chain.outer_method) {
            return;
        }

        // Skip when a possible Enumerable block method is called with a block
        // (e.g., `all.select { |r| r.active? }` uses Ruby's Enumerable#select)
        if POSSIBLE_ENUMERABLE_BLOCK_METHODS.contains(&chain.outer_method) {
            let outer_call = match node.as_call_node() {
                Some(c) => c,
                None => return,
            };
            if outer_call.block().is_some() {
                return;
            }
            // Also check for block pass: all.select(&:active?)
            if let Some(args) = outer_call.arguments() {
                if args
                    .arguments()
                    .iter()
                    .any(|a| a.as_block_argument_node().is_some())
                {
                    return;
                }
            }
        }

        // For sensitive methods (delete_all, destroy_all), only flag when the
        // receiver of `all` is a constant (model class). Skip for associations
        // (non-const receivers) and no-receiver calls.
        if SENSITIVE_METHODS_ON_ASSOCIATION.contains(&chain.outer_method) {
            match chain.inner_call.receiver() {
                Some(recv) => {
                    // Only flag if receiver is a constant (e.g., User.all.delete_all)
                    if recv.as_constant_read_node().is_none()
                        && recv.as_constant_path_node().is_none()
                    {
                        return;
                    }
                }
                // No receiver (e.g., `all.delete_all`) — skip
                None => return,
            }
        }

        // Skip if receiver of the `all` call is in AllowedReceivers
        if let Some(ref receivers) = allowed_receivers {
            if let Some(recv) = chain.inner_call.receiver() {
                let recv_str = std::str::from_utf8(recv.location().as_slice()).unwrap_or("");
                if receivers.iter().any(|r| r == recv_str) {
                    return;
                }
            }
        }

        // Report at the `all` method name location
        let msg_loc = chain
            .inner_call
            .message_loc()
            .unwrap_or(chain.inner_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Redundant `all` detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantActiveRecordAllMethod,
        "cops/rails/redundant_active_record_all_method"
    );
}
