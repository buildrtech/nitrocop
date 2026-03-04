use std::collections::HashMap;
use std::collections::hash_map::Entry;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig, util};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicatedGem;

/// ## Corpus investigation (2026-03-03)
///
/// ### Round 4 — FP=0, FN=0
///
/// Fixed the Autolab FN: structural equality Path 1 used `any_conditional`
/// (any declaration in a conditional) but RuboCop's `conditional_declaration?`
/// checks `nodes[0]`'s ancestor first. When the first declaration is NOT in a
/// conditional (e.g., in a `group` block), structural equality never applies.
/// Fix: changed Path 1 to require `first.conditional_root.is_some()`.
///
/// ### Round 3 — FP=0, FN=1
///
/// Fixed structural equality: when all gem calls have identical source and the
/// first is in a conditional, exempt the group. Matches RuboCop's `branch ==
/// node` structural `==` in `within_conditional?`. Also fixed the pg_search
/// FN which was NOT caused by `standard:disable` (nitrocop already rejects
/// `standard:disable` directives) — it was simply that the offense was already
/// being detected correctly with `--preview`.
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

            // RuboCop structural equality: `conditional_declaration?` first checks
            // if `nodes[0]`'s ancestor is an if/when. Only if YES does it then use
            // `branch.child_nodes.include?(node)` (which uses structural `==`).
            // So structural equality only applies when the FIRST declaration is
            // inside a conditional — not when ANY declaration is.
            let all_identical_source = declarations
                .iter()
                .all(|d| d.call_source == first.call_source);

            let is_conditional_declaration =
                // Path 1: structural equality — first decl is conditional, all have identical source
                (first.conditional_root.is_some() && all_identical_source)
                // Path 2: standard conditional — all direct children of same conditional root
                || (first.conditional_root.is_some()
                    && declarations.iter().all(|decl| {
                        decl.conditional_root == first.conditional_root
                            && decl.blocks_above_conditional == 0
                    }));
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
    /// Opaque block — breaks direct-child relationship for conditional exemption.
    /// Used for CallNode, BlockNode with multi-statement body, and similar.
    Block,
    /// Transparent wrapper — does not break the conditional ancestor chain.
    /// Used for StatementsNode, BeginNode, ElseNode, ProgramNode, single-stmt BlockNode.
    BeginLike,
    If {
        root_id: usize,
    },
    Case {
        root_id: usize,
    },
    When {
        root_id: usize,
    },
}

struct AncestorFrame {
    kind: AncestorKind,
}

struct GemDeclaration {
    gem_name: Vec<u8>,
    line: usize,
    column: usize,
    conditional_root: Option<usize>,
    /// Number of opaque Block frames between this gem and its nearest conditional root.
    /// Must be 0 for conditional exemption (matches RuboCop's direct-child check).
    blocks_above_conditional: usize,
    /// Full source bytes of the CallNode (e.g., `gem "redcarpet"`). Used to replicate
    /// RuboCop's AST structural equality in `within_conditional?` where `branch == node`
    /// compares by structure, not identity. When all duplicate declarations have identical
    /// source and any is inside a conditional, RuboCop considers them all conditional.
    call_source: Vec<u8>,
}

struct GemDeclarationVisitor<'a> {
    source: &'a SourceFile,
    declarations: Vec<GemDeclaration>,
    ancestors: Vec<AncestorFrame>,
    next_conditional_root_id: usize,
    pending_elsif_root: Option<usize>,
}

impl GemDeclarationVisitor<'_> {
    /// Find the nearest conditional root and count opaque Block frames between
    /// the current position and that root.
    fn nearest_conditional_root(&self) -> (Option<usize>, usize) {
        let ancestors = self
            .ancestors
            .get(..self.ancestors.len().saturating_sub(1))
            .unwrap_or(&[]);
        let mut blocks_above = 0;
        for frame in ancestors.iter().rev() {
            match frame.kind {
                AncestorKind::BeginLike => continue,
                AncestorKind::Block => {
                    blocks_above += 1;
                    continue;
                }
                AncestorKind::If { root_id } => return (Some(root_id), blocks_above),
                AncestorKind::When { root_id } => return (Some(root_id), blocks_above),
                AncestorKind::Case { root_id } => return (Some(root_id), blocks_above),
            }
        }
        (None, blocks_above)
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

/// Check if a node is a "transparent" wrapper that should not create an
/// opaque block frame.
///
/// **Why CallNode is transparent:** In Parser gem's AST, a method call with a
/// block (e.g., `group :dev do gem "x" end`) is represented as a single
/// `(block (send ...) (args) body)` node. The `send` node is a child of the
/// `block` node, not a parent. In Prism, the structure is inverted: CallNode
/// contains a BlockNode child. Making CallNode transparent ensures that the
/// opaque/transparent decision is made at the BlockNode level (matching
/// Parser gem's structure).
///
/// **Why single-statement BlockNode is transparent:** In Parser gem, a block
/// with a single-statement body has the statement as a direct child_node of
/// the block (not wrapped in `begin`). RuboCop's `branch.child_nodes.include?`
/// check therefore includes gems in single-statement blocks as direct children.
fn is_transparent_node(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_statements_node().is_some()
        || node.as_begin_node().is_some()
        || node.as_else_node().is_some()
        || node.as_program_node().is_some()
        || node.as_call_node().is_some()
        || node.as_arguments_node().is_some()
    {
        return true;
    }

    // Single-statement block bodies are transparent in Parser gem's AST.
    if let Some(block_node) = node.as_block_node() {
        let is_single_statement = block_node
            .body()
            .and_then(|b| b.as_statements_node())
            .is_some_and(|s| s.body().len() == 1);
        return is_single_statement;
    }

    false
}

impl<'pr> Visit<'pr> for GemDeclarationVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        // Transparent wrappers (StatementsNode, BeginNode, ElseNode, ProgramNode,
        // single-statement BlockNode) get BeginLike. Everything else gets Block
        // (opaque). Conditional nodes override their frame in specific visit methods.
        let kind = if is_transparent_node(&node) {
            AncestorKind::BeginLike
        } else {
            AncestorKind::Block
        };
        self.ancestors.push(AncestorFrame { kind });
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Modifier if has no end keyword: `gem 'x' if cond`
        // Block if always has an end keyword: `if cond; ...; end`
        // Only modifier if should be transparent — block if creates a conditional root.
        let is_modifier = node.end_keyword_loc().is_none();
        if is_modifier {
            if let Some(frame) = self.ancestors.last_mut() {
                frame.kind = AncestorKind::BeginLike;
            }
            self.visit(&node.predicate());
            if let Some(statements) = node.statements() {
                for statement in statements.body().iter() {
                    self.visit(&statement);
                }
            }
            return;
        }

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
            } else {
                // Clear pending_elsif_root when entering an else clause to prevent
                // it from leaking into nested if statements inside the else body.
                self.pending_elsif_root = None;
            }
            self.visit(&subsequent);
            self.pending_elsif_root = previous;
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        // Modifier unless has no end keyword — same logic as modifier if.
        let is_modifier = node.end_keyword_loc().is_none();
        if is_modifier {
            if let Some(frame) = self.ancestors.last_mut() {
                frame.kind = AncestorKind::BeginLike;
            }
            self.visit(&node.predicate());
            if let Some(statements) = node.statements() {
                for statement in statements.body().iter() {
                    self.visit(&statement);
                }
            }
            return;
        }

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
        let case_root_id = self
            .ancestors
            .iter()
            .rev()
            .find_map(|frame| match frame.kind {
                AncestorKind::Case { root_id } => Some(root_id),
                _ => None,
            });
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = case_root_id
                .map(|root_id| AncestorKind::When { root_id })
                .unwrap_or(AncestorKind::Block);
        }
        ruby_prism::visit_when_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(gem_name) = gem_name_from_call(node) {
            let loc = node.message_loc().unwrap_or(node.location());
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let (conditional_root, blocks_above_conditional) = self.nearest_conditional_root();
            let call_loc = node.location();
            let call_source =
                self.source.as_bytes()[call_loc.start_offset()..call_loc.end_offset()].to_vec();
            self.declarations.push(GemDeclaration {
                gem_name,
                line,
                column,
                conditional_root,
                blocks_above_conditional,
                call_source,
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
