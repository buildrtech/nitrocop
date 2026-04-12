use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantBegin;

impl Cop for RedundantBegin {
    fn name(&self) -> &'static str {
        "Style/RedundantBegin"
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
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RedundantBeginVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            pending_corrections: Vec::new(),
            autocorrect_enabled: corrections.is_some(),
            in_loop_body: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corrections) = corrections {
            corrections.extend(visitor.pending_corrections);
        }
    }
}

struct RedundantBeginVisitor<'a> {
    cop: &'a RedundantBegin,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    pending_corrections: Vec<crate::correction::Correction>,
    autocorrect_enabled: bool,
    /// Set when about to visit the body of a while/until begin-modifier loop.
    in_loop_body: bool,
}

impl RedundantBeginVisitor<'_> {
    /// Check if the begin body contains a RescueModifierNode (inline rescue).
    /// `begin X rescue Y end` has rescue as a modifier inside statements, not as
    /// the BeginNode's rescue_clause. The begin wrapper is NOT redundant here.
    fn body_has_rescue_modifier(begin_node: &ruby_prism::BeginNode<'_>) -> bool {
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                if child.as_rescue_modifier_node().is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a body (block, lambda, etc.) contains a redundant `begin` block.
    /// A `begin..rescue..end` or `begin..ensure..end` inside a block body is
    /// redundant when it's the only statement, because the block itself supports
    /// rescue/ensure directly.
    fn check_body_begin(&mut self, body: Option<ruby_prism::Node<'_>>) {
        let body = match body {
            Some(b) => b,
            None => return,
        };

        // The body is either a StatementsNode containing a single BeginNode,
        // or directly a BeginNode
        let begin_node = if let Some(b) = body.as_begin_node() {
            b
        } else if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                // Multiple statements — visit children and return
                for child in body_nodes.iter() {
                    self.visit(child);
                }
                return;
            }
            match body_nodes[0].as_begin_node() {
                Some(b) => b,
                None => {
                    self.visit(&body_nodes[0]);
                    return;
                }
            }
        } else {
            self.visit(&body);
            return;
        };

        // Must have an explicit `begin` keyword
        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => {
                // No explicit begin — visit children
                if let Some(stmts) = begin_node.statements() {
                    for child in stmts.body().iter() {
                        self.visit(&child);
                    }
                }
                if let Some(rescue) = begin_node.rescue_clause() {
                    self.visit_rescue_node(&rescue);
                }
                if let Some(ensure) = begin_node.ensure_clause() {
                    self.visit_ensure_node(&ensure);
                }
                return;
            }
        };

        let offset = begin_kw_loc.start_offset();
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        ));

        // Visit children for nested checks
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        if let Some(rescue) = begin_node.rescue_clause() {
            self.visit_rescue_node(&rescue);
        }
        if let Some(ensure) = begin_node.ensure_clause() {
            self.visit_ensure_node(&ensure);
        }
    }

    /// Check if an assignment value is a redundant `begin` block.
    /// `x = begin...end` or `x ||= begin...end` is redundant when:
    /// - The begin has an explicit `begin` keyword
    /// - There is only a single statement in the body
    /// - There are no rescue/ensure/else clauses
    fn check_assignment_begin(&mut self, value: &ruby_prism::Node<'_>) {
        let begin_node = match value.as_begin_node() {
            Some(b) => b,
            None => {
                // Continue visiting for nested structures
                self.visit(value);
                return;
            }
        };

        // Must have an explicit `begin` keyword
        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => {
                self.visit(value);
                return;
            }
        };

        // If it has rescue/ensure/else, the begin is NOT redundant
        if begin_node.rescue_clause().is_some()
            || begin_node.ensure_clause().is_some()
            || begin_node.else_clause().is_some()
        {
            // Visit children for nested checks
            if let Some(stmts) = begin_node.statements() {
                for child in stmts.body().iter() {
                    self.visit(&child);
                }
            }
            if let Some(rescue) = begin_node.rescue_clause() {
                self.visit_rescue_node(&rescue);
            }
            if let Some(ensure) = begin_node.ensure_clause() {
                self.visit_ensure_node(&ensure);
            }
            return;
        }

        // Must have exactly one statement in the body
        let stmts = match begin_node.statements() {
            Some(s) => s,
            None => return,
        };
        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            // Multiple statements — begin is not redundant in assignment context
            for child in body_nodes.iter() {
                self.visit(child);
            }
            return;
        }

        let offset = begin_kw_loc.start_offset();
        let (line, column) = self.source.offset_to_line_col(offset);
        let mut diagnostic = self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        );

        let replacement = std::str::from_utf8(body_nodes[0].location().as_slice())
            .unwrap_or("")
            .to_string();
        if self.autocorrect_enabled {
            self.pending_corrections
                .push(crate::correction::Correction {
                    start: begin_node.location().start_offset(),
                    end: begin_node.location().end_offset(),
                    replacement,
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
        }
        diagnostic.corrected = true;

        self.diagnostics.push(diagnostic);

        // Visit the begin body for nested checks
        for child in body_nodes.iter() {
            self.visit(child);
        }
    }
}

impl<'pr> Visit<'pr> for RedundantBeginVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let body = match node.body() {
            Some(b) => b,
            None => return,
        };

        // The body might be a BeginNode directly or a StatementsNode containing
        // a single BeginNode
        let begin_node = if let Some(b) = body.as_begin_node() {
            b
        } else if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                // Continue visiting children for nested defs/begins
                for child in body_nodes.iter() {
                    self.visit(child);
                }
                return;
            }
            match body_nodes[0].as_begin_node() {
                Some(b) => b,
                None => {
                    self.visit(&body_nodes[0]);
                    return;
                }
            }
        } else {
            self.visit(&body);
            return;
        };

        // Must have an explicit `begin` keyword
        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => {
                // Visit the begin body for nested checks
                if let Some(stmts) = begin_node.statements() {
                    for child in stmts.body().iter() {
                        self.visit(&child);
                    }
                }
                return;
            }
        };

        let offset = begin_kw_loc.start_offset();
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        ));

        // Visit the begin body for nested checks
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // A standalone `begin...end` without rescue/ensure/else is redundant —
        // the begin/end wrapper serves no purpose. This covers top-level,
        // conditional-body, and other non-method/block/assignment contexts.
        // Exceptions: begin...end inside while/until (do-while loops), and
        // begin with inline rescue modifier (RescueModifierNode in body).
        if let Some(begin_kw_loc) = node.begin_keyword_loc() {
            if node.rescue_clause().is_none()
                && node.ensure_clause().is_none()
                && node.else_clause().is_none()
                && !self.in_loop_body
                && !Self::body_has_rescue_modifier(node)
            {
                let offset = begin_kw_loc.start_offset();
                let (line, column) = self.source.offset_to_line_col(offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Redundant `begin` block detected.".to_string(),
                ));
            }
        }
        self.in_loop_body = false;

        // Continue visiting children to find nested begin nodes (e.g. nested defs)
        if let Some(stmts) = node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        if let Some(rescue) = node.rescue_clause() {
            self.visit_rescue_node(&rescue);
        }
        if let Some(ensure) = node.ensure_clause() {
            self.visit_ensure_node(&ensure);
        }
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        // `begin...end while cond` is a do-while loop. The begin is NOT
        // redundant — it's what makes it a post-condition loop. Set flag
        // so visit_begin_node skips the redundancy check.
        self.in_loop_body = true;
        if let Some(stmts) = node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        self.in_loop_body = false;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        // `begin...end until cond` — same as while, the begin is NOT redundant.
        self.in_loop_body = true;
        if let Some(stmts) = node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        self.in_loop_body = false;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Skip brace blocks ({ }) — only do..end blocks support implicit
        // begin/rescue. Ruby syntax doesn't allow bare rescue in { } blocks.
        if node.opening_loc().as_slice() == b"{" {
            // Still need to visit children for nested checks
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            return;
        }
        self.check_body_begin(node.body());
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // RuboCop skips stabby lambdas entirely — they don't support implicit
        // begin/rescue in their body (even with do..end form).
        // Still visit children for nested checks.
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_or_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_or_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_or_write_node(&mut self, node: &ruby_prism::ConstantOrWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBegin, "cops/style/redundant_begin");
    crate::cop_autocorrect_fixture_tests!(RedundantBegin, "cops/style/redundant_begin");
}
