use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for useless constant scoping under `private` access modifier.
/// Private constants must be defined using `private_constant`, not `private`.
///
/// ## Corpus investigation (FN=41, FP=0)
/// Root cause 1: only handled `ConstantWriteNode` (e.g., `CONST = value`), missed
/// `ConstantPathWriteNode` for qualified assignments like `self::CONST = value`.
/// Fix: added `as_constant_path_write_node()` handling alongside `as_constant_write_node()`.
///
/// ## Corpus investigation (FN=40, FP=0)
/// Root cause 2: only visited `ClassNode` and `ModuleNode` bodies, missing
/// `SingletonClassNode` (`class << self`, 27 FNs), `BlockNode` (DSL blocks, 12 FNs),
/// `IfNode`/`ElseNode` branches (2 FNs), and top-level program scope (1 FN).
/// Fix: replaced per-node-type visitors with `visit_statements_node` to check all
/// statement lists uniformly, matching RuboCop's `on_casgn` which fires everywhere.
pub struct UselessConstantScoping;

impl Cop for UselessConstantScoping {
    fn name(&self) -> &'static str {
        "Lint/UselessConstantScoping"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = ConstScopingVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ConstScopingVisitor<'a, 'src, 'corr> {
    cop: &'a UselessConstantScoping,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'corr mut Vec<crate::correction::Correction>>,
}

impl<'pr> Visit<'pr> for ConstScopingVisitor<'_, '_, '_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        self.check_statements(node);
        ruby_prism::visit_statements_node(self, node);
    }
}

impl ConstScopingVisitor<'_, '_, '_> {
    fn check_statements(&mut self, stmts: &ruby_prism::StatementsNode<'_>) {
        let body_nodes: Vec<_> = stmts.body().iter().collect();

        // Track private modifier and constant assignments
        let mut seen_private = false;
        let mut private_constant_names: Vec<Vec<u8>> = Vec::new();

        // First pass: collect private_constant names
        for node in &body_nodes {
            if let Some(call) = node.as_call_node() {
                if call.name().as_slice() == b"private_constant" && call.receiver().is_none() {
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if let Some(sym) = arg.as_symbol_node() {
                                private_constant_names.push(sym.unescaped().to_vec());
                            }
                        }
                    }
                }
            }
        }

        // Second pass: check for constants after private modifier
        let mut corrected_consts: HashSet<Vec<u8>> = HashSet::new();
        for node in &body_nodes {
            if let Some(call) = node.as_call_node() {
                if call.name().as_slice() == b"private"
                    && call.receiver().is_none()
                    && call.arguments().is_none()
                {
                    seen_private = true;
                    continue;
                }
            }

            if seen_private {
                if let Some(casgn) = node.as_constant_write_node() {
                    let const_name = casgn.name().as_slice();
                    // Check if this constant has a private_constant call
                    if !private_constant_names
                        .iter()
                        .any(|n| n.as_slice() == const_name)
                    {
                        let loc = casgn.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Useless `private` access modifier for constant scope.".to_string(),
                        );
                        if let Some(corrections) = self.corrections.as_deref_mut() {
                            if corrected_consts.insert(const_name.to_vec()) {
                                let insertion = format!(
                                    "\nprivate_constant :{}",
                                    String::from_utf8_lossy(const_name)
                                );
                                corrections.push(crate::correction::Correction {
                                    start: loc.end_offset(),
                                    end: loc.end_offset(),
                                    replacement: insertion,
                                    cop_name: self.cop.name(),
                                    cop_index: 0,
                                });
                            }
                            diagnostic.corrected = true;
                        }
                        self.diagnostics.push(diagnostic);
                    }
                }

                // Handle qualified constant assignments like `self::CONST = value`
                if let Some(cpw) = node.as_constant_path_write_node() {
                    let const_name = cpw.target().name().map(|n| n.as_slice()).unwrap_or(b"");
                    if !private_constant_names
                        .iter()
                        .any(|n| n.as_slice() == const_name)
                    {
                        let loc = cpw.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Useless `private` access modifier for constant scope.".to_string(),
                        );
                        if let Some(corrections) = self.corrections.as_deref_mut() {
                            if corrected_consts.insert(const_name.to_vec()) {
                                let insertion = format!(
                                    "\nprivate_constant :{}",
                                    String::from_utf8_lossy(const_name)
                                );
                                corrections.push(crate::correction::Correction {
                                    start: loc.end_offset(),
                                    end: loc.end_offset(),
                                    replacement: insertion,
                                    cop_name: self.cop.name(),
                                    cop_index: 0,
                                });
                            }
                            diagnostic.corrected = true;
                        }
                        self.diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessConstantScoping, "cops/lint/useless_constant_scoping");

    #[test]
    fn autocorrect_adds_private_constant_after_assignment() {
        crate::testutil::assert_cop_autocorrect(
            &UselessConstantScoping,
            b"class Foo\n  private\n  PRIVATE_CONST = 42\nend\n",
            b"class Foo\n  private\n  PRIVATE_CONST = 42\nprivate_constant :PRIVATE_CONST\nend\n",
        );
    }
}
