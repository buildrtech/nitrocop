use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for uses of if/unless modifiers with multiple-line bodies.
///
/// ## Investigation findings (2026-03-15, updated 2026-03-18)
///
/// **Root cause of FNs (12, fixed):** The previous implementation used a
/// `lines_joined_by_backslash` function to exempt backslash-continued lines.
/// This was too broad — it exempted cases where the body itself spans multiple
/// physical lines joined by `\` (e.g., `raise "msg" \ "more" if cond`).
/// RuboCop flags these because `node.body.multiline?` checks if the body AST
/// node's first_line != last_line, regardless of `\` continuation.
///
/// **Root cause of FPs (44, across 15 repos, fixed):** NOT config-related as
/// previously documented. The actual root cause is RuboCop's
/// `part_of_ignored_node?` / `ignore_node` mechanism. When RuboCop flags a
/// multiline modifier if/unless, it calls `ignore_node(node)` which marks
/// the entire subtree as ignored. Any nested multiline modifier if/unless
/// inside the flagged node is then skipped via `part_of_ignored_node?`.
///
/// Common patterns:
/// - `module Foo...class Bar...end if defined?(X)...end if defined?(Y)`:
///   Only the outermost modifier is flagged; inner class modifiers are ignored.
///   (jruby test_ractor.rb: 6 inner class modifiers inside outer module modifier)
/// - `class Foo...def bar...end unless m?...end if Puma.jruby?`:
///   Only the class-level modifier is flagged; inner def modifiers are ignored.
///   (puma test_puma_server_ssl.rb, ruby/debug session.rb)
/// - `block { ...inner_call if cond... } unless outer_cond`:
///   Only the outer block modifier is flagged.
///   (ruby/debug server.rb)
///
/// **Fix:** Switched from `check_node` to `check_source` with a custom AST
/// visitor that tracks whether we're inside an already-flagged modifier
/// if/unless. When a multiline modifier is found, its subtree is marked as
/// ignored and nested modifiers are skipped.
pub struct MultilineIfModifier;

impl Cop for MultilineIfModifier {
    fn name(&self) -> &'static str {
        "Style/MultilineIfModifier"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = MultilineIfModifierVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            inside_flagged: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MultilineIfModifierVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a MultilineIfModifier,
    diagnostics: Vec<Diagnostic>,
    /// Whether we're currently inside a subtree of an already-flagged
    /// multiline modifier if/unless (RuboCop's `part_of_ignored_node?`).
    inside_flagged: bool,
}

impl MultilineIfModifierVisitor<'_> {
    /// Check if a modifier if/unless body spans multiple lines.
    /// Returns `Some((body_start_offset, body_start_line, body_start_col))` if multiline.
    fn check_body_multiline(
        &self,
        stmts: &ruby_prism::StatementsNode<'_>,
    ) -> Option<(usize, usize, usize)> {
        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.is_empty() {
            return None;
        }

        let first = &body_nodes[0];
        let last = &body_nodes[body_nodes.len() - 1];
        let body_start_line = self
            .source
            .offset_to_line_col(first.location().start_offset())
            .0;
        let body_end_line = self
            .source
            .offset_to_line_col(last.location().end_offset().saturating_sub(1))
            .0;

        if body_start_line < body_end_line {
            let body_start = first.location().start_offset();
            let (line, column) = self.source.offset_to_line_col(body_start);
            Some((body_start, line, column))
        } else {
            None
        }
    }
}

impl<'pr> Visit<'pr> for MultilineIfModifierVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let is_modifier_multiline = node
            .if_keyword_loc()
            .as_ref()
            .is_some_and(|loc| loc.as_slice() == b"if")
            && node.end_keyword_loc().is_none()
            && node
                .statements()
                .and_then(|stmts| self.check_body_multiline(&stmts))
                .is_some();

        if is_modifier_multiline && !self.inside_flagged {
            // Flag this offense
            if let Some(stmts) = node.statements() {
                if let Some((_offset, line, column)) = self.check_body_multiline(&stmts) {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Favor a normal if-statement over a modifier clause in a multiline statement.".to_string(),
                    ));
                }
            }

            // Mark subtree as ignored (RuboCop's ignore_node / part_of_ignored_node?)
            let was_inside = self.inside_flagged;
            self.inside_flagged = true;
            ruby_prism::visit_if_node(self, node);
            self.inside_flagged = was_inside;
        } else {
            // Not a modifier multiline, or inside an already-flagged node — visit children normally
            ruby_prism::visit_if_node(self, node);
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let is_modifier_multiline = node.keyword_loc().as_slice() == b"unless"
            && node.end_keyword_loc().is_none()
            && node
                .statements()
                .and_then(|stmts| self.check_body_multiline(&stmts))
                .is_some();

        if is_modifier_multiline && !self.inside_flagged {
            // Flag this offense
            if let Some(stmts) = node.statements() {
                if let Some((_offset, line, column)) = self.check_body_multiline(&stmts) {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Favor a normal unless-statement over a modifier clause in a multiline statement.".to_string(),
                    ));
                }
            }

            // Mark subtree as ignored
            let was_inside = self.inside_flagged;
            self.inside_flagged = true;
            ruby_prism::visit_unless_node(self, node);
            self.inside_flagged = was_inside;
        } else {
            ruby_prism::visit_unless_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineIfModifier, "cops/style/multiline_if_modifier");
}
