use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/EmptyCaseCondition flags `case` statements with no predicate expression
/// and suggests using `if`/`elsif` chains instead.
///
/// ## Investigation findings (2026-03-15)
///
/// ### FP root causes (65 FPs):
/// - **Parent is send/csend/return/break/next (major, ~65 FPs):** RuboCop skips when the
///   `case` node's parent AST type is `return`, `break`, `next`, `send`, or `csend`.
///   This covers patterns like `case ... end.should` (send), `return case ... end`,
///   `do_something case ... end` (send). The old nitrocop code used a text-based heuristic
///   (`!trimmed.starts_with("case")`) that caught some of these but missed `send` parents
///   where `case` starts at the beginning of the line.
/// - **Branch contains return statement:** RuboCop skips when any branch body contains a
///   `return` statement (or has a return as a descendant). Pattern: `case; when cond;
///   return foo; end`. The old code didn't check for this at all.
///
/// ### FN root causes (77 FNs):
/// - **Assignment parent (major, ~77 FNs):** `v = case; when ...; end` — the text heuristic
///   `!trimmed.starts_with("case")` incorrectly suppressed flagging because `case` isn't at
///   the start of the line. But assignment (`lvasgn`, `ivasgn`, etc.) is NOT in RuboCop's
///   `NOT_SUPPORTED_PARENT_TYPES`, so these should be flagged.
///
/// ### Fix:
/// Replaced text-based line heuristic with proper AST parent tracking via a visitor.
/// Added `NOT_SUPPORTED_PARENT_TYPES` check (return/break/next/call) and
/// `branch_contains_return` check matching RuboCop's behavior.
pub struct EmptyCaseCondition;

impl Cop for EmptyCaseCondition {
    fn name(&self) -> &'static str {
        "Style/EmptyCaseCondition"
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
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let mut visitor = EmptyCaseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            parent_kind: ParentKind::Other,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corrs) = corrections.as_mut() {
            corrs.extend(visitor.corrections);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ParentKind {
    /// return, break, next, send, csend — case not supported as if-replacement
    Unsupported,
    /// Any other parent type
    Other,
}

struct EmptyCaseVisitor<'a> {
    cop: &'a EmptyCaseCondition,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<Correction>,
    parent_kind: ParentKind,
}

/// Visitor that checks if a subtree contains any return node.
struct ReturnFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, _node: &ruby_prism::ReturnNode<'pr>) {
        self.found = true;
    }
}

/// Check if any branch body of a case node contains a return statement.
fn branch_contains_return(case_node: &ruby_prism::CaseNode<'_>) -> bool {
    let mut finder = ReturnFinder { found: false };
    for when_ref in case_node.conditions().iter() {
        if let Some(when_node) = when_ref.as_when_node() {
            if let Some(stmts) = when_node.statements() {
                finder.visit(&stmts.as_node());
                if finder.found {
                    return true;
                }
            }
        }
    }
    if let Some(else_clause) = case_node.else_clause() {
        if let Some(stmts) = else_clause.statements() {
            finder.visit(&stmts.as_node());
            if finder.found {
                return true;
            }
        }
    }
    false
}

fn build_case_autocorrect(case_node: &ruby_prism::CaseNode<'_>, source: &SourceFile) -> Option<String> {
    let when_nodes: Vec<_> = case_node
        .conditions()
        .iter()
        .filter_map(|cond| cond.as_when_node())
        .collect();

    if when_nodes.is_empty() {
        return None;
    }

    if when_nodes
        .iter()
        .any(|when_node| when_node.conditions().iter().count() != 1)
    {
        return None;
    }

    let case_kw_loc = case_node.case_keyword_loc();
    let first_when = when_nodes.first()?;
    let first_when_kw_start = first_when.location().start_offset();
    let first_when_kw_end = first_when_kw_start + 4;

    // Conservative guard: only autocorrect `case` lines that contain just the keyword.
    let bytes = source.as_bytes();
    let mut case_line_end = case_kw_loc.end_offset();
    while case_line_end < bytes.len() && bytes[case_line_end] != b'\n' {
        case_line_end += 1;
    }
    let case_line_src = &bytes[case_kw_loc.start_offset()..case_line_end];
    if std::str::from_utf8(case_line_src).ok()?.trim() != "case" {
        return None;
    }

    let mut replacement = String::from_utf8_lossy(
        &bytes[case_node.location().start_offset()..case_node.location().end_offset()],
    )
    .into_owned();

    let base = case_node.location().start_offset();
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();

    // Remove `case` header (from `case` keyword up to first `when`).
    edits.push((
        case_kw_loc.start_offset() - base,
        first_when_kw_start - base,
        "",
    ));

    // Rewrite first `when` to `if`.
    edits.push((first_when_kw_start - base, first_when_kw_end - base, "if"));

    // Rewrite subsequent `when` to `elsif`.
    for when_node in when_nodes.iter().skip(1) {
        let when_kw_start = when_node.location().start_offset();
        edits.push((when_kw_start - base, when_kw_start + 4 - base, "elsif"));
    }

    edits.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end, text) in edits {
        replacement.replace_range(start..end, text);
    }

    Some(replacement)
}

/// Helper macro to implement visitor methods that set parent_kind for their children.
/// This ensures case nodes nested as direct children see the correct parent type.
macro_rules! visit_with_parent {
    ($method:ident, $node_type:ty, $default_visit:path, $kind:expr) => {
        fn $method(&mut self, node: &$node_type) {
            let prev = self.parent_kind;
            self.parent_kind = $kind;
            $default_visit(self, node);
            self.parent_kind = prev;
        }
    };
}

impl<'pr> Visit<'pr> for EmptyCaseVisitor<'_> {
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // Only flag if case has no predicate (empty case condition)
        if node.predicate().is_none() && self.parent_kind != ParentKind::Unsupported {
            // Skip if any branch body contains a return statement (or descendant)
            if !branch_contains_return(node) {
                let case_kw_loc = node.case_keyword_loc();
                let case_offset = case_kw_loc.start_offset();
                let (line, column) = self.source.offset_to_line_col(case_offset);
                let mut diagnostic = self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Do not use empty `case` condition, instead use an `if` expression."
                        .to_string(),
                );

                if let Some(replacement) = build_case_autocorrect(node, self.source) {
                    self.corrections.push(Correction {
                        start: node.location().start_offset(),
                        end: node.location().end_offset(),
                        replacement,
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                self.diagnostics.push(diagnostic);
            }
        }

        // Continue visiting child nodes (reset parent kind since case's children
        // are when/else nodes, not unsupported parents)
        let prev = self.parent_kind;
        self.parent_kind = ParentKind::Other;
        ruby_prism::visit_case_node(self, node);
        self.parent_kind = prev;
    }

    // Unsupported parent types: return, break, next, send/csend (CallNode)
    visit_with_parent!(
        visit_return_node,
        ruby_prism::ReturnNode<'pr>,
        ruby_prism::visit_return_node,
        ParentKind::Unsupported
    );
    visit_with_parent!(
        visit_break_node,
        ruby_prism::BreakNode<'pr>,
        ruby_prism::visit_break_node,
        ParentKind::Unsupported
    );
    visit_with_parent!(
        visit_next_node,
        ruby_prism::NextNode<'pr>,
        ruby_prism::visit_next_node,
        ParentKind::Unsupported
    );
    visit_with_parent!(
        visit_call_node,
        ruby_prism::CallNode<'pr>,
        ruby_prism::visit_call_node,
        ParentKind::Unsupported
    );

    // All other node types reset parent_kind to Other so that case nodes
    // nested deeper (e.g. inside a block body within a call) are not suppressed.
    visit_with_parent!(
        visit_def_node,
        ruby_prism::DefNode<'pr>,
        ruby_prism::visit_def_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_block_node,
        ruby_prism::BlockNode<'pr>,
        ruby_prism::visit_block_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_lambda_node,
        ruby_prism::LambdaNode<'pr>,
        ruby_prism::visit_lambda_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_if_node,
        ruby_prism::IfNode<'pr>,
        ruby_prism::visit_if_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_unless_node,
        ruby_prism::UnlessNode<'pr>,
        ruby_prism::visit_unless_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_while_node,
        ruby_prism::WhileNode<'pr>,
        ruby_prism::visit_while_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_until_node,
        ruby_prism::UntilNode<'pr>,
        ruby_prism::visit_until_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_for_node,
        ruby_prism::ForNode<'pr>,
        ruby_prism::visit_for_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_begin_node,
        ruby_prism::BeginNode<'pr>,
        ruby_prism::visit_begin_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_class_node,
        ruby_prism::ClassNode<'pr>,
        ruby_prism::visit_class_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_module_node,
        ruby_prism::ModuleNode<'pr>,
        ruby_prism::visit_module_node,
        ParentKind::Other
    );
    visit_with_parent!(
        visit_singleton_class_node,
        ruby_prism::SingletonClassNode<'pr>,
        ruby_prism::visit_singleton_class_node,
        ParentKind::Other
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyCaseCondition, "cops/style/empty_case_condition");
    crate::cop_autocorrect_fixture_tests!(EmptyCaseCondition, "cops/style/empty_case_condition");
}
