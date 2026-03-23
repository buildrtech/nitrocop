use std::collections::HashMap;

use crate::cop::node_type::{CLASS_NODE, SYMBOL_NODE};
use crate::cop::util::{is_dsl_call, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/DuplicateAssociation
///
/// Detects two kinds of duplicate associations in ActiveRecord models:
/// 1. Same association name used multiple times (any association type)
/// 2. Same `class_name:` option used in multiple `has_many`/`has_one`/`has_and_belongs_to_many`
///    associations that have no other options (excludes `belongs_to`)
///
/// Supports all four association methods: `has_many`, `has_one`, `belongs_to`,
/// `has_and_belongs_to_many`. Accepts both symbol and string first arguments.
///
/// ## Implementation notes
///
/// RuboCop's `register_offense` flags ALL members of a duplicate group, including the first
/// occurrence. The implementation groups calls by name and then flags all members of groups
/// with >1 member (both passes: name duplicates and class_name duplicates).
///
/// Message format for name duplicates: "Association `x` is defined multiple times. Don't
/// repeat associations." (matching RuboCop exactly).
///
/// ## Investigation findings
///
/// **FP (block associations):** RuboCop uses `class_send_nodes` which calls
/// `each_child_node(:send)` on the class body. In Parser gem's AST, a call with a
/// `do...end` block is wrapped in a `(block (send ...) ...)` node, so the `send` is
/// NOT a direct child of the class body — it's skipped. In Prism, calls with blocks
/// are still `CallNode` direct children. Fix: skip calls where `call.block().is_some()`.
///
/// **FN (if/else branches):** When the class body has exactly one statement that is an
/// `if` node, RuboCop's `each_child_node(:send)` is called on the `if` node itself,
/// finding `send` nodes that are direct children (i.e., single-statement branches).
/// Fix: detect this pattern and collect calls from the if/else branches.
pub struct DuplicateAssociation;

/// Association method names we track.
const ASSOCIATION_METHODS: &[&[u8]] = &[
    b"has_many",
    b"has_one",
    b"belongs_to",
    b"has_and_belongs_to_many",
];

/// Check if the parent class looks like an ActiveRecord base class.
fn is_active_record_parent(parent: &[u8]) -> bool {
    parent == b"ApplicationRecord" || parent == b"ActiveRecord::Base" || parent.ends_with(b"Record")
}

impl Cop for DuplicateAssociation {
    fn name(&self) -> &'static str {
        "Rails/DuplicateAssociation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SYMBOL_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        // Only check classes that inherit from ActiveRecord
        let parent = parent_class_name(source, &class);
        if let Some(parent_name) = parent {
            if !is_active_record_parent(parent_name) {
                return;
            }
        } else {
            // No parent class at all — skip
            return;
        }

        let calls = collect_class_send_nodes(&class);

        // --- Pass 1: Duplicate association names ---
        // Group calls by name, then flag ALL occurrences in groups with >1 member.
        // RuboCop's `register_offense` flags every member of a duplicate group,
        // including the first occurrence — not just subsequent ones.
        let mut name_groups: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

        for (idx, call) in calls.iter().enumerate() {
            if !is_association_call(call) {
                continue;
            }

            let name = match extract_first_name_arg(call) {
                Some(n) => n,
                None => continue,
            };

            name_groups.entry(name).or_default().push(idx);
        }

        for (name, indices) in &name_groups {
            if indices.len() <= 1 {
                continue;
            }
            let name_str = String::from_utf8_lossy(name);
            for &idx in indices {
                let call = &calls[idx];
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Association `{name_str}` is defined multiple times. Don't repeat associations."
                    ),
                ));
            }
        }

        // --- Pass 2: Duplicate class_name (has_* only, not belongs_to) ---
        // Only flag when the hash argument has exactly one pair: `class_name: 'X'`
        // RuboCop flags ALL members of a duplicate group, not just subsequent ones.
        let mut class_name_groups: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

        for (idx, call) in calls.iter().enumerate() {
            // Skip belongs_to — RuboCop excludes it from class_name duplicate check
            if !is_association_call(call) || is_dsl_call(call, b"belongs_to") {
                continue;
            }

            if let Some(cn_source) = extract_sole_class_name(source, call) {
                class_name_groups.entry(cn_source).or_default().push(idx);
            }
        }

        for (cn_source, indices) in &class_name_groups {
            if indices.len() <= 1 {
                continue;
            }
            let cn_str = String::from_utf8_lossy(cn_source);
            for &idx in indices {
                let call = &calls[idx];
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Association `class_name: {cn_str}` is defined multiple times. Don't repeat associations."
                    ),
                ));
            }
        }
    }
}

/// Collect call nodes from the class body, matching RuboCop's `class_send_nodes` behavior.
///
/// RuboCop uses `each_child_node(:send)` which:
/// - Skips calls wrapped in blocks (in Parser AST, `do...end` creates a `block` parent)
/// - When the class body is a single `if` node, finds sends that are direct children
///   of that `if` (i.e., single-statement branches)
fn collect_class_send_nodes<'a>(
    class_node: &ruby_prism::ClassNode<'a>,
) -> Vec<ruby_prism::CallNode<'a>> {
    let body = match class_node.body() {
        Some(b) => b,
        None => return Vec::new(),
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return Vec::new(),
    };

    let body_nodes: Vec<_> = stmts.body().iter().collect();

    // When the class body has exactly one statement that is an if node,
    // RuboCop's each_child_node(:send) is called on the if node itself,
    // finding send nodes that are direct children (single-statement branches).
    if body_nodes.len() == 1 {
        if let Some(if_node) = body_nodes[0].as_if_node() {
            return collect_calls_from_if_branches(&if_node);
        }
    }

    // Normal case: collect direct call children, excluding those with blocks
    body_nodes
        .iter()
        .filter_map(|node| {
            let call = node.as_call_node()?;
            // In Parser AST, calls with do...end blocks are wrapped in (block (send ...))
            // nodes, so each_child_node(:send) on the class body skips them.
            if call.block().is_some() {
                return None;
            }
            Some(call)
        })
        .collect()
}

/// Collect call nodes from the branches of an if/else node.
/// Only collects from single-statement branches (matching Parser AST behavior
/// where multi-statement branches are wrapped in `begin` nodes).
fn collect_calls_from_if_branches<'a>(
    if_node: &ruby_prism::IfNode<'a>,
) -> Vec<ruby_prism::CallNode<'a>> {
    let mut calls = Vec::new();

    // Collect from the if-branch
    if let Some(stmts) = if_node.statements() {
        for node in stmts.body().iter() {
            if let Some(call) = node.as_call_node() {
                if call.block().is_none() {
                    calls.push(call);
                }
            }
        }
    }

    // Collect from else/elsif branches
    if let Some(subsequent) = if_node.subsequent() {
        if let Some(else_clause) = subsequent.as_else_node() {
            if let Some(stmts) = else_clause.statements() {
                for node in stmts.body().iter() {
                    if let Some(call) = node.as_call_node() {
                        if call.block().is_none() {
                            calls.push(call);
                        }
                    }
                }
            }
        }
        // elsif is another IfNode — recurse
        if let Some(elsif_node) = subsequent.as_if_node() {
            calls.extend(collect_calls_from_if_branches(&elsif_node));
        }
    }

    calls
}

/// Check if the call is one of the four association methods.
fn is_association_call(call: &ruby_prism::CallNode<'_>) -> bool {
    ASSOCIATION_METHODS.iter().any(|m| is_dsl_call(call, m))
}

/// Extract the first argument (association name) as either a symbol or string.
fn extract_first_name_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;
    if let Some(sym) = first_arg.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = first_arg.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// If the call has exactly one extra argument beyond the name, and that argument
/// is a keyword hash with exactly one pair `class_name: <value>`, return the
/// source text of the value (e.g., `'Foo'`).
///
/// This matches RuboCop's `class_name` node pattern: `(hash (pair (sym :class_name) $_))`
/// combined with the `arguments.one?` guard.
fn extract_sole_class_name(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();

    // Must have exactly 2 arguments: name + hash (arguments.one? in RuboCop
    // refers to the rest-args after the name capture, so 1 extra arg)
    if arg_list.len() != 2 {
        return None;
    }

    // The second arg should be a keyword hash with exactly one pair
    let hash_node = arg_list[1].as_keyword_hash_node()?;
    let elements: Vec<_> = hash_node.elements().iter().collect();
    if elements.len() != 1 {
        return None;
    }

    let assoc = elements[0].as_assoc_node()?;
    let key_sym = assoc.key().as_symbol_node()?;
    if key_sym.unescaped() != b"class_name" {
        return None;
    }

    // Return the source text of the value node (e.g., 'Foo' or "Foo")
    let value = assoc.value();
    let start = value.location().start_offset();
    let end = value.location().end_offset();
    Some(source.as_bytes()[start..end].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateAssociation, "cops/rails/duplicate_association");
}
