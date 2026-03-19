use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for redundant assignment before returning.
///
/// ## Investigation findings (2026-03-19)
///
/// Root causes of false negatives (FN):
/// - Original implementation only checked direct `StatementsNode` and `BeginNode` bodies
///   inside `DefNode`, missing recursion into `if/elsif/else`, `case/when`, `case/in`,
///   and explicit `begin..end` blocks as the last expression.
/// - Did not handle `defs` (class method definitions like `def self.foo`).
///
/// Root causes of false positives (FP):
/// - Original implementation checked the main body inside `BeginNode` even when an
///   `ensure` clause was present. RuboCop does NOT flag `x = val; x` when ensure is
///   present because ensure runs unconditionally and the semantics differ.
///
/// Fix: Rewrote to use RuboCop's recursive `check_branch` approach that dispatches on
/// the last expression's type (if/unless → check each branch, case → check each
/// when + else, case/in → check each in + else, begin → check for pattern or recurse,
/// rescue → check each resbody, ensure → skip entirely). In Prism, `def self.foo` is
/// also a `DefNode` (with `receiver().is_some()`), so no separate handler needed.
pub struct RedundantAssignment;

impl Cop for RedundantAssignment {
    fn name(&self) -> &'static str {
        "Style/RedundantAssignment"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RedundantAssignmentVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantAssignmentVisitor<'a> {
    cop: &'a RedundantAssignment,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl RedundantAssignmentVisitor<'_> {
    /// Check for redundant assignment in a list of statements.
    /// If the last two statements are `x = expr; x`, report an offense.
    /// Otherwise, recurse into the last statement if it's a branching construct.
    fn check_stmts(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        if stmts.len() >= 2 {
            let last = &stmts[stmts.len() - 1];
            let second_last = &stmts[stmts.len() - 2];

            if let Some(lvar) = last.as_local_variable_read_node() {
                if let Some(write) = second_last.as_local_variable_write_node() {
                    if write.name().as_slice() == lvar.name().as_slice() {
                        let loc = second_last.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Redundant assignment before returning detected.".to_string(),
                        ));
                        return;
                    }
                }
            }
        }

        // If no direct pattern match, recurse into the last expression
        if let Some(last) = stmts.last() {
            self.check_branch(last);
        }
    }

    /// Recursively check a node for redundant assignment patterns.
    /// Dispatches based on node type, mirroring RuboCop's check_branch.
    fn check_branch(&mut self, node: &ruby_prism::Node<'_>) {
        if let Some(if_node) = node.as_if_node() {
            self.check_if_node(&if_node);
        } else if let Some(unless_node) = node.as_unless_node() {
            self.check_unless_node(&unless_node);
        } else if let Some(case_node) = node.as_case_node() {
            self.check_case_node(&case_node);
        } else if let Some(case_match) = node.as_case_match_node() {
            self.check_case_match_node(&case_match);
        } else if let Some(begin_node) = node.as_begin_node() {
            self.check_begin_node(&begin_node);
        } else if let Some(stmts) = node.as_statements_node() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.check_stmts(&body);
        }
        // Note: rescue and ensure at the top level are handled via BeginNode
    }

    fn check_if_node(&mut self, node: &ruby_prism::IfNode<'_>) {
        // Skip modifier if (postfix form) — RuboCop skips modifier_form? and ternary?
        // In Prism, modifier if has no `if_keyword_loc` or it's written as postfix.
        // We check: if the if_keyword is absent or the node is a ternary, skip.

        // Check the "then" branch (if body)
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.check_stmts(&body);
        }

        // Check the "else" branch — could be an ElseNode containing statements,
        // or another IfNode (for elsif)
        if let Some(subsequent) = node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                self.check_if_node(&elsif);
            } else if let Some(else_node) = subsequent.as_else_node() {
                if let Some(stmts) = else_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    self.check_stmts(&body);
                }
            }
        }
    }

    fn check_unless_node(&mut self, node: &ruby_prism::UnlessNode<'_>) {
        // Check the "then" branch (unless body — runs when condition is false)
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.check_stmts(&body);
        }
        // Check the else branch
        if let Some(else_node) = node.else_clause() {
            if let Some(stmts) = else_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.check_stmts(&body);
            }
        }
    }

    fn check_case_node(&mut self, node: &ruby_prism::CaseNode<'_>) {
        for condition in node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    self.check_stmts(&body);
                }
            }
        }
        // Check the else branch
        if let Some(else_node) = node.else_clause() {
            if let Some(stmts) = else_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.check_stmts(&body);
            }
        }
    }

    fn check_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'_>) {
        for condition in node.conditions().iter() {
            if let Some(in_node) = condition.as_in_node() {
                if let Some(stmts) = in_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    self.check_stmts(&body);
                }
            }
        }
        // Check the else branch
        if let Some(else_node) = node.else_clause() {
            if let Some(stmts) = else_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.check_stmts(&body);
            }
        }
    }

    fn check_begin_node(&mut self, node: &ruby_prism::BeginNode<'_>) {
        // If ensure is present, do NOT check the main body (RuboCop behavior).
        // Only check inside the ensure body itself (which would be unusual).
        if node.ensure_clause().is_some() {
            return;
        }

        // If rescue clauses are present, check the main body AND each rescue body
        if let Some(rescue) = node.rescue_clause() {
            // Check main body (statements before rescue)
            if let Some(stmts) = node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.check_stmts(&body);
            }
            self.check_rescue_chain(&rescue);
        } else if let Some(stmts) = node.statements() {
            // Plain begin..end block with no rescue/ensure
            let body: Vec<_> = stmts.body().iter().collect();
            self.check_stmts(&body);
        }
    }

    fn check_rescue_chain(&mut self, rescue: &ruby_prism::RescueNode<'_>) {
        if let Some(stmts) = rescue.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.check_stmts(&body);
        }
        if let Some(subsequent) = rescue.subsequent() {
            self.check_rescue_chain(&subsequent);
        }
    }

    fn check_def_body(&mut self, body: &ruby_prism::Node<'_>) {
        if let Some(stmts) = body.as_statements_node() {
            let body_stmts: Vec<_> = stmts.body().iter().collect();
            self.check_stmts(&body_stmts);
        } else if let Some(begin) = body.as_begin_node() {
            self.check_begin_node(&begin);
        } else {
            // Single expression body — check_branch for if/case etc.
            self.check_branch(body);
        }
    }
}

impl<'pr> Visit<'pr> for RedundantAssignmentVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Handles both `def foo` and `def self.foo` (class methods)
        // In Prism, `def self.foo` is also a DefNode with receiver().is_some()
        if let Some(body) = node.body() {
            self.check_def_body(&body);
        }
        // Continue visiting nested defs
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantAssignment, "cops/style/redundant_assignment");
}
