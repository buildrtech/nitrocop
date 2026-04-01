use crate::cop::node_type::{
    CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Enforces grouping of accessor declarations (`attr_reader`, `attr_writer`,
/// `attr_accessor`, `attr`) in class and module bodies.
///
/// ## Investigation findings (2026-03-15)
///
/// The original nitrocop implementation used a contiguity-based approach: it tracked
/// consecutive accessor declarations and grouped them by adjacency. This diverged
/// significantly from RuboCop's algorithm, which uses a sibling-based approach:
///
/// **RuboCop's algorithm:**
/// 1. Iterates ALL `send` nodes in the class/module body that are `attribute_accessor?`
/// 2. For each accessor, checks `previous_line_comment?` — if the source line immediately
///    before the accessor is a comment, the accessor is excluded from grouping
/// 3. Checks `groupable_accessor?` — examines the previous sibling (left sibling in the
///    statement list). An accessor is NOT groupable if:
///    - The previous sibling is a non-accessor send that is not an access modifier
///      (e.g., `sig { ... }`, `annotation_method :foo`) AND there's no blank line gap
///    - The previous sibling is a block node wrapping a send (Sorbet `sig { ... }`)
///      AND there's no blank line gap
/// 4. Finds all same-type, same-visibility siblings that are also groupable and not
///    preceded by a comment — reports offense if >1 such siblings exist
///
/// **Root causes of FPs (294):**
/// - Accessors preceded by a comment on the previous line were flagged (should be excluded)
/// - Accessors preceded by annotation method calls (Sorbet sig, etc.) were flagged
///
/// **Root causes of FNs (582):**
/// - Non-contiguous same-type accessors in the same visibility scope were missed because
///   the old code only checked adjacent sequences. RuboCop considers ALL siblings in the
///   class body, not just consecutive ones.
/// - Accessors separated by `def` blocks or other code were not grouped.
///
/// Fix: rewrote to match RuboCop's sibling-based `groupable_sibling_accessors` approach.
///
/// ## Investigation findings (2026-03-15, inline RBS annotations)
///
/// 67 FPs from accessors with inline RBS::Inline `#:` type comments (e.g.,
/// `attr_accessor :label #: String`). RuboCop's `groupable_accessor?` checks if
/// the previous sibling expression has an inline `#:` comment on the same line.
/// If it does, the current accessor is NOT groupable, because grouping would
/// lose per-attribute type annotations.
///
/// Fix: added `has_inline_rbs_comment()` check in `is_groupable_accessor()` to
/// detect `#:` on the previous sibling's source line and return false (not groupable).
///
/// ## Investigation findings (2026-03-27, block-form DSL calls)
///
/// 3 FNs remained in the corpus when an accessor group followed a block-form DSL call
/// such as `mattr_accessor ... do` or `config_section ... do`. RuboCop unwraps a
/// preceding block expression to its inner send and compares the accessor against that
/// send node's `last_line`, which is the call line rather than the `end` line.
///
/// Prism exposes these constructs as a `CallNode` whose `location()` spans through the
/// block terminator. The previous nitrocop port used that full span, so it treated the
/// first accessor as immediately adjacent to the block and marked it ungroupable. That
/// dropped the first accessor in longer groups and suppressed the entire offense when the
/// group only had two accessors.
///
/// Fix: when the previous sibling is a call with a real `BlockNode`, measure blank-line
/// spacing from the block start line instead of the call's full end line. This matches
/// RuboCop's unwrapped-send behavior without broadening grouping after ordinary calls.
pub struct AccessorGrouping;

const ACCESSOR_METHODS: &[&str] = &["attr_reader", "attr_writer", "attr_accessor", "attr"];

impl Cop for AccessorGrouping {
    fn name(&self) -> &'static str {
        "Style/AccessorGrouping"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            MODULE_NODE,
            SINGLETON_CLASS_NODE,
            STATEMENTS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "grouped");

        // Only check class and module bodies
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.body()
        } else if let Some(sclass) = node.as_singleton_class_node() {
            sclass.body()
        } else {
            return;
        };

        let body = match body {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        if enforced_style == "grouped" {
            check_grouped(self, source, &stmts, diagnostics, corrections.as_deref_mut());
        }
    }
}

/// Info about each statement in the class/module body.
struct StmtInfo {
    /// Index in the statement list
    idx: usize,
    /// Whether this statement is an accessor call (attr_reader, etc.)
    is_accessor: bool,
    /// The accessor method name (e.g., "attr_reader"), empty if not accessor
    accessor_name: String,
    /// Visibility scope of this statement (public/protected/private)
    visibility: &'static str,
    /// Whether this accessor is "groupable" per RuboCop's logic
    groupable: bool,
    /// Whether the line before this accessor is a comment
    has_previous_line_comment: bool,
}

fn check_grouped(
    cop: &AccessorGrouping,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
) {
    let stmt_list: Vec<_> = stmts.body().iter().collect();
    if stmt_list.is_empty() {
        return;
    }

    // Build info for each statement
    let mut infos: Vec<StmtInfo> = Vec::with_capacity(stmt_list.len());
    let mut current_visibility: &'static str = "public";

    for (idx, stmt) in stmt_list.iter().enumerate() {
        let mut info = StmtInfo {
            idx,
            is_accessor: false,
            accessor_name: String::new(),
            visibility: current_visibility,
            groupable: true,
            has_previous_line_comment: false,
        };

        if let Some(call) = stmt.as_call_node() {
            let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

            // Track bare visibility modifiers
            if matches!(name, "private" | "protected" | "public")
                && call.arguments().is_none()
                && call.block().is_none()
            {
                current_visibility = match name {
                    "private" => "private",
                    "protected" => "protected",
                    _ => "public",
                };
                info.visibility = current_visibility;
                infos.push(info);
                continue;
            }

            if ACCESSOR_METHODS.contains(&name) && call.receiver().is_none() {
                info.is_accessor = true;
                info.accessor_name = name.to_string();

                // Check previous_line_comment: is the source line before this accessor a comment?
                info.has_previous_line_comment =
                    previous_line_is_comment(source, stmt.location().start_offset());

                // Check groupable_accessor: examine the previous sibling
                info.groupable = is_groupable_accessor(source, &stmt_list, idx);
            }
        }

        infos.push(info);
    }

    // For each accessor, find groupable sibling accessors (same type, same visibility,
    // both groupable and not preceded by a comment)
    // Use a set to avoid reporting the same accessor twice
    let mut reported = vec![false; stmt_list.len()];

    for i in 0..infos.len() {
        if !infos[i].is_accessor {
            continue;
        }
        if reported[i] {
            continue;
        }
        // Skip accessors that have a previous line comment or are not groupable
        if infos[i].has_previous_line_comment || !infos[i].groupable {
            continue;
        }

        // Find all groupable siblings with the same accessor type and visibility
        let mut group: Vec<usize> = Vec::new();
        for j in 0..infos.len() {
            if !infos[j].is_accessor {
                continue;
            }
            if infos[j].accessor_name != infos[i].accessor_name {
                continue;
            }
            if infos[j].visibility != infos[i].visibility {
                continue;
            }
            if !infos[j].groupable || infos[j].has_previous_line_comment {
                continue;
            }
            group.push(j);
        }

        if group.len() > 1 {
            for &g in &group {
                if !reported[g] {
                    reported[g] = true;
                    let stmt = &stmt_list[infos[g].idx];
                    let loc = stmt.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Group together all `{}` attributes.",
                            infos[g].accessor_name
                        ),
                    ));
                }
            }
        }
    }

    if let Some(corrections) = corrections {
        emit_grouped_autocorrections(cop, source, &stmt_list, &infos, corrections);
    }
}

struct AccessorCallInfo {
    start_offset: usize,
    end_offset: usize,
    start_line: usize,
    end_line: usize,
    args_start: usize,
    args_end: usize,
    args_source: String,
}

fn emit_grouped_autocorrections(
    cop: &AccessorGrouping,
    source: &SourceFile,
    stmt_list: &[ruby_prism::Node<'_>],
    infos: &[StmtInfo],
    corrections: &mut Vec<crate::correction::Correction>,
) {
    let mut i = 0;
    while i < infos.len() {
        let Some(mut current) = accessor_call_info(source, &stmt_list[infos[i].idx]) else {
            i += 1;
            continue;
        };

        if !infos[i].is_accessor || !infos[i].groupable || infos[i].has_previous_line_comment {
            i += 1;
            continue;
        }

        let mut run = vec![current];
        let mut j = i + 1;

        while j < infos.len() {
            if !infos[j].is_accessor
                || infos[j].accessor_name != infos[i].accessor_name
                || infos[j].visibility != infos[i].visibility
                || !infos[j].groupable
                || infos[j].has_previous_line_comment
            {
                break;
            }

            let Some(next_call) = accessor_call_info(source, &stmt_list[infos[j].idx]) else {
                break;
            };

            // Keep this conservative: only merge physically adjacent accessor lines.
            let prev_end_line = run.last().map(|r| r.end_line).unwrap_or(0);
            if next_call.start_line != prev_end_line + 1 {
                break;
            }

            // Skip calls with trailing comments/code so line deletion is safe.
            if !line_contains_only_call(source, next_call.start_offset, next_call.end_offset) {
                break;
            }

            current = next_call;
            run.push(current);
            j += 1;
        }

        if run.len() > 1 {
            let merged_args = run
                .iter()
                .map(|r| r.args_source.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            let first = &run[0];
            corrections.push(crate::correction::Correction {
                start: first.args_start,
                end: first.args_end,
                replacement: merged_args,
                cop_name: cop.name(),
                cop_index: 0,
            });

            for call in run.iter().skip(1) {
                let delete_start = line_start_offset(source, call.start_offset);
                let delete_end = extend_to_line_break(source, call.end_offset);
                corrections.push(crate::correction::Correction {
                    start: delete_start,
                    end: delete_end,
                    replacement: String::new(),
                    cop_name: cop.name(),
                    cop_index: 0,
                });
            }
        }

        i = if run.len() > 1 { j } else { i + 1 };
    }
}

fn accessor_call_info(
    source: &SourceFile,
    stmt: &ruby_prism::Node<'_>,
) -> Option<AccessorCallInfo> {
    let call = stmt.as_call_node()?;
    if call.receiver().is_some() || call.block().is_some() {
        return None;
    }

    let args = call.arguments()?;
    let args_loc = args.location();
    let call_loc = call.location();

    let args_source = std::str::from_utf8(&source.as_bytes()[args_loc.start_offset()..args_loc.end_offset()])
        .ok()?
        .trim()
        .to_string();

    if args_source.is_empty() {
        return None;
    }

    let (start_line, _) = source.offset_to_line_col(call_loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(call_loc.end_offset());

    Some(AccessorCallInfo {
        start_offset: call_loc.start_offset(),
        end_offset: call_loc.end_offset(),
        start_line,
        end_line,
        args_start: args_loc.start_offset(),
        args_end: args_loc.end_offset(),
        args_source,
    })
}

fn line_start_offset(source: &SourceFile, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    line_start
}

fn line_contains_only_call(source: &SourceFile, call_start: usize, call_end: usize) -> bool {
    let bytes = source.as_bytes();
    let line_start = line_start_offset(source, call_start);

    let mut line_end = call_end;
    while line_end < bytes.len() && bytes[line_end] != b'\n' {
        line_end += 1;
    }

    bytes[line_start..call_start]
        .iter()
        .all(|b| b.is_ascii_whitespace())
        && bytes[call_end..line_end]
            .iter()
            .all(|b| b.is_ascii_whitespace())
}

fn extend_to_line_break(source: &SourceFile, end: usize) -> usize {
    let bytes = source.as_bytes();
    if end >= bytes.len() {
        return end;
    }

    if bytes[end] == b'\r' && end + 1 < bytes.len() && bytes[end + 1] == b'\n' {
        end + 2
    } else if bytes[end] == b'\n' {
        end + 1
    } else {
        end
    }
}

/// Check if the source line immediately before the given offset is a comment line.
/// Matches RuboCop's `previous_line_comment?` which checks `processed_source[node.first_line - 2]`.
fn previous_line_is_comment(source: &SourceFile, start_offset: usize) -> bool {
    let (line, _) = source.offset_to_line_col(start_offset);
    if line <= 1 {
        return false;
    }
    // Get the previous line (line is 1-based, so line-2 is the 0-based index of previous line)
    let prev_line_idx = line - 2;
    for (i, source_line) in source.lines().enumerate() {
        if i == prev_line_idx {
            let trimmed = source_line
                .iter()
                .copied()
                .skip_while(|&b| b == b' ' || b == b'\t')
                .collect::<Vec<_>>();
            return trimmed.first() == Some(&b'#');
        }
    }
    false
}

/// Check if an accessor at index `idx` in `stmt_list` is "groupable" per RuboCop's logic.
///
/// RuboCop's `groupable_accessor?` examines the previous sibling (left sibling):
/// 1. No previous sibling -> groupable
/// 2. Previous is a block type (e.g., `sig { ... }`) -> unwrap to send child; if unwrapped
///    is not a send, groupable. Otherwise treat as send case below.
/// 3. Previous is NOT a send type (def, class, constant, etc.) -> groupable
/// 4. Previous IS a send: groupable only if it's an accessor, access modifier, OR there's
///    a blank line gap (> 1 line between them)
/// 5. Previous expression has an inline RBS `#:` annotation comment -> NOT groupable
fn is_groupable_accessor(
    source: &SourceFile,
    stmt_list: &[ruby_prism::Node<'_>],
    idx: usize,
) -> bool {
    if idx == 0 {
        return true;
    }

    let prev = &stmt_list[idx - 1];
    let curr = &stmt_list[idx];

    // Check if previous is a call node (send type in RuboCop terms).
    // In Prism, a call with a block (like `sig { ... }`) is still a CallNode.
    if let Some(prev_call) = prev.as_call_node() {
        let prev_name = std::str::from_utf8(prev_call.name().as_slice()).unwrap_or("");
        let prev_end_line = previous_expression_last_line(source, &prev_call);
        let curr_start_line = source.offset_to_line_col(curr.location().start_offset()).0;

        // RuboCop: accessors with RBS::Inline `#:` annotations on the previous expression
        // are not groupable. Check if the previous sibling's source line contains `#:`.
        if has_inline_rbs_comment(source, prev.location().start_offset()) {
            return false;
        }

        // Previous is an accessor — groupable
        if ACCESSOR_METHODS.contains(&prev_name) && prev_call.receiver().is_none() {
            return true;
        }

        // Previous is a bare access modifier — groupable
        if matches!(prev_name, "private" | "protected" | "public")
            && prev_call.arguments().is_none()
            && prev_call.block().is_none()
        {
            return true;
        }

        // Previous is some other send (annotation, macro, etc.) — NOT groupable
        // unless there's a blank line gap (> 1 line between them)
        return curr_start_line - prev_end_line > 1;
    }

    // Previous is not a send type (def, class, constant assignment, begin, etc.)
    // Per RuboCop: `return true unless previous_expression.send_type?` -> groupable
    true
}

/// RuboCop unwraps a previous block expression to its inner send before comparing
/// line spacing. Prism keeps block-form sends as a single `CallNode` whose location
/// extends through `end`, so use the block start line to recover the inner send span.
fn previous_expression_last_line(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> usize {
    if let Some(block) = call.block().and_then(|b| b.as_block_node()) {
        return source.offset_to_line_col(block.location().start_offset()).0;
    }

    source.offset_to_line_col(call.location().end_offset()).0
}

/// Check if the source line containing the node at `start_offset` has an inline
/// RBS::Inline annotation comment (`#:` syntax). RuboCop checks
/// `processed_source.comments.any? { |c| same_line?(c, prev) && c.text.start_with?('#:') }`.
fn has_inline_rbs_comment(source: &SourceFile, start_offset: usize) -> bool {
    let (line, _) = source.offset_to_line_col(start_offset);
    // line is 1-based; get the 0-based index
    let line_idx = line - 1;
    for (i, source_line) in source.lines().enumerate() {
        if i == line_idx {
            // Look for `#:` in the line (not at the start — it's an inline comment)
            // We need to find a `#` that's followed by `:` and is a comment, not inside a string.
            // Simple heuristic: find `#:` after the code portion. Since these are accessor
            // declarations, the pattern is `attr_reader :foo #: Type`.
            if let Some(pos) = source_line.windows(2).position(|w| w == b"#:") {
                // Make sure it's not at the start (that would be a regular comment, not inline)
                // and that it's preceded by whitespace (i.e., it's a trailing comment)
                if pos > 0 {
                    return true;
                }
            }
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AccessorGrouping, "cops/style/accessor_grouping");
    crate::cop_autocorrect_fixture_tests!(AccessorGrouping, "cops/style/accessor_grouping");
}
