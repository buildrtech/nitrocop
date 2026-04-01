use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
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
///
/// **Root cause of remaining FPs (2, fixed 2026-03-18):** Parser gem difference
/// in how block nodes report `multiline?`. In the parser gem, a method call
/// with a block becomes a `:block` node whose `multiline?` checks whether `{`
/// and `}` are on different lines (not the overall expression span). In Prism,
/// the body is a `CallNode` with a block child, and the overall span includes
/// the receiver/args. Examples: `receiver.method(args)\n  .chain { block }`
/// and `method("long" +\n  "args") { block }` — both have single-line `{}`
/// braces, so RuboCop considers them non-multiline. Fixed by checking block
/// delimiter positions when body is a CallNode with a block.
pub struct MultilineIfModifier;

impl Cop for MultilineIfModifier {
    fn name(&self) -> &'static str {
        "Style/MultilineIfModifier"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let mut visitor = MultilineIfModifierVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            autocorrect_enabled: corrections.is_some(),
            inside_flagged: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corrections) = corrections {
            corrections.extend(visitor.corrections);
        }
    }
}

struct MultilineIfModifierVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a MultilineIfModifier,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<Correction>,
    autocorrect_enabled: bool,
    /// Whether we're currently inside a subtree of an already-flagged
    /// multiline modifier if/unless (RuboCop's `part_of_ignored_node?`).
    inside_flagged: bool,
}

impl MultilineIfModifierVisitor<'_> {
    fn source_slice(&self, start: usize, end: usize) -> String {
        String::from_utf8_lossy(&self.source.as_bytes()[start..end]).into_owned()
    }

    fn multiline_if_replacement(&self, node: &ruby_prism::IfNode<'_>) -> Option<String> {
        let statements = node.statements()?;
        let body = self.source_slice(
            statements.location().start_offset(),
            statements.location().end_offset(),
        );
        let predicate = self.source_slice(
            node.predicate().location().start_offset(),
            node.predicate().location().end_offset(),
        );
        Some(format!("if {}\n{}\nend", predicate.trim(), body.trim()))
    }

    fn multiline_unless_replacement(&self, node: &ruby_prism::UnlessNode<'_>) -> Option<String> {
        let statements = node.statements()?;
        let body = self.source_slice(
            statements.location().start_offset(),
            statements.location().end_offset(),
        );
        let predicate = self.source_slice(
            node.predicate().location().start_offset(),
            node.predicate().location().end_offset(),
        );
        Some(format!("unless {}\n{}\nend", predicate.trim(), body.trim()))
    }

    /// Check if a modifier if/unless body spans multiple lines.
    /// Returns `Some((body_start_offset, body_start_line, body_start_col))` if multiline.
    ///
    /// Special handling for CallNode with a block: In the parser gem, a method call
    /// with a block becomes a `:block` node whose `multiline?` checks whether the
    /// `{`/`}` (or `do`/`end`) delimiters are on different lines — not the overall
    /// expression span. In Prism, the body is a `CallNode` with a block child. We
    /// replicate RuboCop's behavior by checking the block's delimiter positions when
    /// the body is a single CallNode with a block.
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

        // If the body is a single CallNode with a block, check block delimiter
        // positions instead of the overall CallNode span. This matches RuboCop's
        // parser gem where the body would be a :block node whose multiline? checks
        // opening/closing brace positions.
        if body_nodes.len() == 1 {
            if let Some(call) = first.as_call_node() {
                if let Some(block) = call.block().and_then(|b| b.as_block_node()) {
                    let open_line = self
                        .source
                        .offset_to_line_col(block.opening_loc().start_offset())
                        .0;
                    let close_line = self
                        .source
                        .offset_to_line_col(block.closing_loc().start_offset())
                        .0;
                    if open_line == close_line {
                        // Block braces on same line — not multiline per RuboCop
                        return None;
                    }
                }
            }
        }

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
                    let mut diagnostic = self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Favor a normal if-statement over a modifier clause in a multiline statement.".to_string(),
                    );

                    if self.autocorrect_enabled {
                        if let Some(replacement) = self.multiline_if_replacement(node) {
                            self.corrections.push(Correction {
                                start: node.location().start_offset(),
                                end: node.location().end_offset(),
                                replacement,
                                cop_name: self.cop.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
                    }

                    self.diagnostics.push(diagnostic);
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
                    let mut diagnostic = self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Favor a normal unless-statement over a modifier clause in a multiline statement.".to_string(),
                    );

                    if self.autocorrect_enabled {
                        if let Some(replacement) = self.multiline_unless_replacement(node) {
                            self.corrections.push(Correction {
                                start: node.location().start_offset(),
                                end: node.location().end_offset(),
                                replacement,
                                cop_name: self.cop.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
                    }

                    self.diagnostics.push(diagnostic);
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
    crate::cop_autocorrect_fixture_tests!(MultilineIfModifier, "cops/style/multiline_if_modifier");
}
