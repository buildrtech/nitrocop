use crate::cop::node_type::{DEF_NODE, FORWARDING_SUPER_NODE, STATEMENTS_NODE, SUPER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP fix: `has_comment_in_body` was skipping the first line (def line), missing inline
/// comments like `def initialize # comment`. RuboCop's `contains_comments?` checks the
/// full node range from the def line through (but not including) the end line.
///
/// FN fix: The cop only detected `super()` with zero args as redundant when both def and
/// super had no args. Now also detects `super(a, b)` as redundant when the explicit args
/// match the def's required parameters by name and order (e.g., `def initialize(a, b);
/// super(a, b); end`). This matches RuboCop's `same_args?` behavior.
///
/// FP fix (2025-03): `super` with a block (`super do...end` or `super() { }`) was
/// incorrectly flagged as redundant. The block adds behavior beyond simple forwarding,
/// so the method is NOT redundant. In Prism, both `ForwardingSuperNode` and `SuperNode`
/// have a `block()` field that is `Some(BlockNode)` when a block is attached. This
/// matches RuboCop's behavior where `node.body.begin_type?` returns false for block
/// calls, preventing the `initialize_forwards?` matcher from matching.
///
/// FP fix (2026-03): 20 FPs all caused by RuboCop's `contains_comments?` using
/// `find_end_line` which extends the comment check range beyond the method's `end`
/// keyword to the next sibling node's start line. Comments in the gap between `end`
/// and the next method/expression cause `allow_comments?` to return true. Examples:
/// twilio-ruby (7): `def initialize(v); super(v); end` followed by `##` doc comment;
/// puppet (2), viewcomponent, authlogic, solargraph, loofah, fusuma, midori, publiclab,
/// concurrent-ruby, pdf-reader, rumale (1 each): empty `def initialize; end` with
/// comments between `end` and the next code line. Fixed by adding `has_comment_after_end`
/// which scans source after the def node for comments before the next code line.
///
/// 2 FNs remain (fastlane: inline comment on def line `def initialize # required`;
/// fluentd: comment after super in body). Both involve comments that nitrocop correctly
/// detects via `has_comment_in_body`, yet RuboCop still flags them. The root cause
/// appears to be a subtle difference in RuboCop's `each_comment_in_lines` range
/// calculation that excludes these specific comments. Not worth fixing — 2 FNs out
/// of 167 total offenses (98.8% location match after FP fix).
pub struct RedundantInitialize;

impl Cop for RedundantInitialize {
    fn name(&self) -> &'static str {
        "Style/RedundantInitialize"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, FORWARDING_SUPER_NODE, STATEMENTS_NODE, SUPER_NODE]
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
        let allow_comments = config.get_bool("AllowComments", true);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must be named `initialize`
        if def_node.name().as_slice() != b"initialize" {
            return;
        }

        // Must not have a receiver (not def self.initialize)
        if def_node.receiver().is_some() {
            return;
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => {
                // Empty initialize method — only redundant if no parameters
                if def_node.parameters().is_some() {
                    return;
                }
                if allow_comments {
                    // Check for comments inside the method
                    let def_start = def_node.location().start_offset();
                    let def_end = def_node.location().end_offset();
                    let body_bytes = &source.as_bytes()[def_start..def_end];
                    if has_comment_in_body(body_bytes) {
                        return;
                    }
                    // Also check for comments after the end keyword, up to the
                    // next code line. RuboCop's `find_end_line` extends the
                    // comment range to the next sibling node's line, so comments
                    // in the gap between `end` and the next method/expression
                    // cause `allow_comments?` to return true.
                    if has_comment_after_end(source.as_bytes(), def_end) {
                        return;
                    }
                }
                let loc = def_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Remove unnecessary empty `initialize` method.".to_string(),
                ));
                return;
            }
        };

        // Check if the body is just a single `super` or `super(...)` call
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // Check for super call
        // ForwardingSuperNode = bare `super` (forwards all args)
        // SuperNode = super with explicit args `super(...)` or `super(a, b)`
        let is_forwarding_super = body_nodes[0].as_forwarding_super_node().is_some();
        let is_explicit_super = body_nodes[0].as_super_node().is_some();

        if !is_forwarding_super && !is_explicit_super {
            return;
        }

        // If super has a block (do...end or { }), it's NOT redundant — the block adds behavior.
        // e.g., `super do; bind_one; end` or `super() { |h, k| h[k] = [] }`
        if let Some(fwd) = body_nodes[0].as_forwarding_super_node() {
            if fwd.block().is_some() {
                return;
            }
        }
        if let Some(sup) = body_nodes[0].as_super_node() {
            if let Some(block) = sup.block() {
                // BlockArgumentNode (&block) is just forwarding, not adding behavior.
                // But a BlockNode (do...end / { }) adds behavior.
                if block.as_block_argument_node().is_none() {
                    return;
                }
            }
        }

        // For bare `super`: only redundant if the method has no default args,
        // rest args, keyword args, or block args (simple required params only)
        if is_forwarding_super {
            if let Some(params) = def_node.parameters() {
                // Has optionals, rest, keywords, keyword_rest, or block
                if !params.optionals().is_empty()
                    || params.rest().is_some()
                    || !params.keywords().is_empty()
                    || params.keyword_rest().is_some()
                    || params.block().is_some()
                    || params.posts().iter().next().is_some()
                {
                    return;
                }
            }
        }

        // For explicit `super(...)`: redundant if args match def's required params exactly
        if is_explicit_super {
            if let Some(super_node) = body_nodes[0].as_super_node() {
                if !super_args_match_params(&def_node, &super_node) {
                    return;
                }
            }
        }

        if allow_comments {
            let def_start = def_node.location().start_offset();
            let def_end = def_node.location().end_offset();
            let body_bytes = &source.as_bytes()[def_start..def_end];
            if has_comment_in_body(body_bytes) {
                return;
            }
            if has_comment_after_end(source.as_bytes(), def_end) {
                return;
            }
        }

        let loc = def_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Remove unnecessary `initialize` method.".to_string(),
        ));
    }
}

fn has_comment_in_body(body_bytes: &[u8]) -> bool {
    // Check all lines except the last (end keyword line) for comments.
    // RuboCop's `contains_comments?` checks the node range from start_line
    // to end_line (exclusive), so the `end` line is excluded but the `def`
    // line is included.
    let mut in_string = false;
    let line_count = body_bytes.iter().filter(|&&b| b == b'\n').count();
    // If there are no newlines, this is a single-line def (e.g., `def initialize; end`)
    // and there are no interior lines to check — the end line IS the def line.
    if line_count == 0 {
        return false;
    }
    let mut current_line = 0;
    for &b in body_bytes {
        if b == b'\n' {
            current_line += 1;
            in_string = false;
            continue;
        }
        // Skip the last line (the `end` keyword line)
        if current_line == line_count {
            continue;
        }
        if b == b'#' && !in_string {
            return true;
        }
        if b == b'"' || b == b'\'' {
            in_string = !in_string;
        }
    }
    false
}

/// Check for comments in the source after the def node's `end` keyword, up to the
/// next non-blank, non-comment line. RuboCop's `contains_comments?` uses `find_end_line`
/// which extends the comment check range to the right sibling's start line. Comments
/// in this gap (between `end` and the next code) cause `allow_comments?` to return true,
/// preventing the offense from being registered.
fn has_comment_after_end(source_bytes: &[u8], def_end_offset: usize) -> bool {
    // Scan forward from the end of the def node
    let remaining = &source_bytes[def_end_offset..];
    // Skip the rest of the current line (after `end`)
    let mut pos = 0;
    while pos < remaining.len() && remaining[pos] != b'\n' {
        pos += 1;
    }
    if pos < remaining.len() {
        pos += 1; // skip the newline
    }
    // Now scan subsequent lines. For each line:
    // - If it's blank (only whitespace), continue
    // - If it starts with whitespace then `#`, it's a comment → return true
    // - Otherwise it's code → return false (stop scanning)
    while pos < remaining.len() {
        // Find the content of this line
        let line_start = pos;
        while pos < remaining.len() && remaining[pos] != b'\n' {
            pos += 1;
        }
        let line = &remaining[line_start..pos];
        if pos < remaining.len() {
            pos += 1; // skip newline
        }
        // Check if this line is blank (only whitespace)
        let trimmed_start = line.iter().position(|&b| b != b' ' && b != b'\t');
        match trimmed_start {
            None => continue, // blank line
            Some(idx) => {
                if line[idx] == b'#' {
                    return true; // found a comment
                }
                return false; // found code, stop
            }
        }
    }
    false
}

/// Check if super's explicit arguments match the def's required parameters exactly.
/// Returns true if they match (making the method redundant), false otherwise.
fn super_args_match_params(
    def_node: &ruby_prism::DefNode<'_>,
    super_node: &ruby_prism::SuperNode<'_>,
) -> bool {
    let super_args: Vec<_> = match super_node.arguments() {
        Some(args) => args.arguments().iter().collect(),
        None => vec![],
    };

    let params = def_node.parameters();

    // Collect required parameter names from the def
    let param_names: Vec<_> = match &params {
        Some(p) => {
            // Must have only required params (no optionals, rest, keywords, block, posts)
            if !p.optionals().is_empty()
                || p.rest().is_some()
                || !p.keywords().is_empty()
                || p.keyword_rest().is_some()
                || p.block().is_some()
                || p.posts().iter().next().is_some()
            {
                return false;
            }
            p.requireds()
                .iter()
                .filter_map(|r| r.as_required_parameter_node().map(|n| n.name()))
                .collect()
        }
        None => vec![],
    };

    // Must have the same count
    if super_args.len() != param_names.len() {
        return false;
    }

    // Each super arg must be a local variable read matching the corresponding param name
    for (arg, param_name) in super_args.iter().zip(param_names.iter()) {
        match arg.as_local_variable_read_node() {
            Some(lvar) if lvar.name().as_slice() == param_name.as_slice() => {}
            _ => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantInitialize, "cops/style/redundant_initialize");
}
