use std::collections::HashMap;
use std::collections::hash_map::Entry;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig, util};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicatedGem;

impl Cop for DuplicatedGem {
    fn name(&self) -> &'static str {
        "Bundler/DuplicatedGem"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
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
        let mut visitor = GemDeclarationVisitor {
            source,
            declarations: Vec::new(),
            ancestors: Vec::new(),
            next_conditional_root_id: 1,
            pending_elsif_root: None,
        };
        visitor.visit(&parse_result.node());

        let mut grouped: HashMap<Vec<u8>, Vec<GemDeclaration>> = HashMap::new();
        for declaration in visitor.declarations {
            match grouped.entry(declaration.gem_name.clone()) {
                Entry::Occupied(mut occupied) => occupied.get_mut().push(declaration),
                Entry::Vacant(vacant) => {
                    vacant.insert(vec![declaration]);
                }
            }
        }

        for declarations in grouped.into_values() {
            if declarations.len() < 2 {
                continue;
            }

            let first = &declarations[0];
            let is_conditional_declaration = first.conditional_root.is_some()
                && declarations
                    .iter()
                    .all(|decl| decl.conditional_root == first.conditional_root);
            if is_conditional_declaration {
                continue;
            }

            let gem_name = String::from_utf8_lossy(&first.gem_name);
            for duplicate in declarations.iter().skip(1) {
                diagnostics.push(self.diagnostic(
                    source,
                    duplicate.line,
                    duplicate.column,
                    format!(
                        "Gem `{}` requirements already given on line {} of the Gemfile.",
                        gem_name, first.line
                    ),
                ));
            }
        }
    }
}

#[derive(Clone, Copy)]
enum AncestorKind {
    Other,
    BeginLike,
    If { root_id: usize },
    Case { root_id: usize },
    When { root_id: usize },
}

struct AncestorFrame {
    kind: AncestorKind,
}

struct GemDeclaration {
    gem_name: Vec<u8>,
    line: usize,
    column: usize,
    conditional_root: Option<usize>,
}

struct GemDeclarationVisitor<'a> {
    source: &'a SourceFile,
    declarations: Vec<GemDeclaration>,
    ancestors: Vec<AncestorFrame>,
    next_conditional_root_id: usize,
    pending_elsif_root: Option<usize>,
}

impl GemDeclarationVisitor<'_> {
    fn nearest_conditional_root(&self) -> Option<usize> {
        let ancestors = self
            .ancestors
            .get(..self.ancestors.len().saturating_sub(1))
            .unwrap_or(&[]);
        for frame in ancestors.iter().rev() {
            match frame.kind {
                // Prism wraps branch bodies in `StatementsNode`; Ruby AST uses `begin`.
                AncestorKind::BeginLike => continue,
                AncestorKind::If { root_id } => return Some(root_id),
                AncestorKind::When { root_id } => return Some(root_id),
                AncestorKind::Case { root_id } => return Some(root_id),
                AncestorKind::Other => return None,
            }
        }
        None
    }

    fn allocate_conditional_root_id(&mut self) -> usize {
        let id = self.next_conditional_root_id;
        self.next_conditional_root_id += 1;
        id
    }
}

fn gem_name_from_call(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    if call.receiver().is_some() || call.name().as_slice() != b"gem" {
        return None;
    }
    let first_arg = util::first_positional_arg(call)?;
    util::string_value(&first_arg)
}

impl<'pr> Visit<'pr> for GemDeclarationVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let kind = if node.as_begin_node().is_some()
            || node.as_parentheses_node().is_some()
            || node.as_statements_node().is_some()
            || node.as_else_node().is_some()
        {
            AncestorKind::BeginLike
        } else {
            AncestorKind::Other
        };
        self.ancestors.push(AncestorFrame { kind });
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let root_id = self
            .pending_elsif_root
            .unwrap_or_else(|| self.allocate_conditional_root_id());
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = AncestorKind::If { root_id };
        }

        self.visit(&node.predicate());
        if let Some(statements) = node.statements() {
            for statement in statements.body().iter() {
                self.visit(&statement);
            }
        }
        if let Some(subsequent) = node.subsequent() {
            let previous = self.pending_elsif_root;
            if subsequent.as_if_node().is_some() {
                self.pending_elsif_root = Some(root_id);
            }
            self.visit(&subsequent);
            self.pending_elsif_root = previous;
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let root_id = self.allocate_conditional_root_id();
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = AncestorKind::If { root_id };
        }

        self.visit(&node.predicate());
        if let Some(statements) = node.statements() {
            for statement in statements.body().iter() {
                self.visit(&statement);
            }
        }
        if let Some(else_clause) = node.else_clause() {
            if let Some(statements) = else_clause.statements() {
                for statement in statements.body().iter() {
                    self.visit(&statement);
                }
            }
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let root_id = self.allocate_conditional_root_id();
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = AncestorKind::Case { root_id };
        }
        ruby_prism::visit_case_node(self, node);
    }

    fn visit_when_node(&mut self, node: &ruby_prism::WhenNode<'pr>) {
        let case_root_id = self.ancestors.iter().rev().find_map(|frame| match frame.kind {
            AncestorKind::Case { root_id } => Some(root_id),
            _ => None,
        });
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = case_root_id
                .map(|root_id| AncestorKind::When { root_id })
                .unwrap_or(AncestorKind::Other);
        }
        ruby_prism::visit_when_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(gem_name) = gem_name_from_call(node) {
            let loc = node.message_loc().unwrap_or(node.location());
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let conditional_root = self.nearest_conditional_root();
            self.declarations.push(GemDeclaration {
                gem_name,
                line,
                column,
                conditional_root,
            });
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedGem, "cops/bundler/duplicated_gem");
}
