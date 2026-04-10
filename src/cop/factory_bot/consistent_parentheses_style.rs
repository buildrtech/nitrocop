use ruby_prism::Visit;

use crate::cop::factory_bot::{FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE, is_factory_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-12)
///
/// Corpus oracle (run 22998123880) reported FP=0, FN=7. All 7 FN in
/// scinote-eln/scinote-web, a newly added corpus repo.
///
/// Root cause: `parent_is_ambiguous` propagated recursively through the entire
/// AST subtree. RuboCop checks only the immediate parent via `node.parent.type`.
///
/// FN fix 1 (4 FN): `visit_parentheses_node` only cleared ambiguity for
/// `AmbiguityKind::Array`. Parenthesized expressions map to `begin` in Parser
/// (not in AMBIGUOUS_TYPES), so they should always clear ambiguity. Pattern:
/// `archived_by: (create :user)` — factory call in parens inside assoc value.
///
/// FN fix 2 (3 FN): `visit_if_node` propagated ambiguity to ALL descendants.
/// Factory calls inside `lvasgn` inside `if` body have parent `lvasgn` in
/// Parser (not `if`), so not ambiguous. Pattern: `if foo; x = create :sym; end`.
/// Single-statement if bodies with a CallNode as the sole statement are treated
/// as ambiguous (matching Parser's direct-child behavior), but when the statement
/// is something else (lvasgn, etc.), the body is non-ambiguous.
///
/// ## Corpus investigation (2026-03-13)
///
/// Corpus oracle reported FP=1, FN=0. FP in decidim/decidim:
/// `create :organization` as sole statement in an `else` branch.
///
/// FP fix: `visit_if_node` subsequent (else) handling set `parent_is_ambiguous = false`
/// unconditionally. In Parser AST, else-branch content is a direct child of `if`
/// (ambiguous), same as the then-branch. Applied same single-statement-CallNode
/// ambiguity logic to the else branch via `as_else_node()` check.
///
/// ## Corpus investigation (2026-03-19)
///
/// Corpus oracle reported FP=0, FN=2. Both FN in decko-commons/decko.
/// Factory calls (`create`) inside lambda bodies `-> { ... }` nested within
/// ambiguous contexts (hash values, method arguments) were incorrectly skipped.
///
/// FN fix: Added `visit_lambda_node` override to clear `parent_is_ambiguous`.
/// Lambda bodies are independent contexts (like block bodies and parentheses),
/// so factory calls inside them should not inherit ambiguity from outer context.
pub struct ConsistentParenthesesStyle;

impl Cop for ConsistentParenthesesStyle {
    fn name(&self) -> &'static str {
        "FactoryBot/ConsistentParenthesesStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "require_parentheses");
        let explicit_only = config.get_bool("ExplicitOnly", false);

        let mut visitor = ParenStyleVisitor {
            source,
            cop: self,
            style,
            explicit_only,
            diagnostics: Vec::new(),
            corrections,
            parent_is_ambiguous: false,
            ambiguity_kind: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ParenStyleVisitor<'s, 'c> {
    source: &'s SourceFile,
    cop: &'s ConsistentParenthesesStyle,
    style: &'s str,
    explicit_only: bool,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'c mut Vec<crate::correction::Correction>>,
    parent_is_ambiguous: bool,
    ambiguity_kind: Option<AmbiguityKind>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AmbiguityKind {
    Call,
    Array,
    Assoc,
    AndOr,
    If,
}

impl<'s, 'c> ParenStyleVisitor<'s, 'c> {
    fn check_factory_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !FACTORY_BOT_METHODS.contains(&method_name) {
            return;
        }

        if !is_factory_call(call.receiver(), self.explicit_only) {
            return;
        }

        // Skip if parent is an ambiguous context (send, pair, array, and, or, if)
        if self.parent_is_ambiguous {
            return;
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a symbol, string, send, or local variable
        let first_arg = &arg_list[0];
        let valid_first_arg = first_arg.as_symbol_node().is_some()
            || first_arg.as_string_node().is_some()
            || first_arg.as_call_node().is_some()
            || first_arg.as_local_variable_read_node().is_some();

        if !valid_first_arg {
            return;
        }

        // `generate` with more than 1 argument is excluded
        if method_name == "generate" && arg_list.len() > 1 {
            return;
        }

        let has_parens = call.opening_loc().is_some();

        if self.style == "require_parentheses" && !has_parens {
            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer method call with parentheses".to_string(),
            );

            if let Some(ref mut corrections) = self.corrections {
                let first_arg_loc = first_arg.location();
                let args_loc = args.location();
                corrections.push(crate::correction::Correction {
                    start: msg_loc.end_offset(),
                    end: first_arg_loc.start_offset(),
                    replacement: "(".to_string(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                corrections.push(crate::correction::Correction {
                    start: args_loc.end_offset(),
                    end: args_loc.end_offset(),
                    replacement: ")".to_string(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            self.diagnostics.push(diagnostic);
        }

        if self.style == "omit_parentheses" && has_parens {
            let call_loc = call.location();
            let (call_line, _) = self.source.offset_to_line_col(call_loc.start_offset());
            let first_arg_loc = first_arg.location();
            let (arg_line, _) = self.source.offset_to_line_col(first_arg_loc.start_offset());

            if call_line != arg_line {
                return;
            }

            if has_value_omission_hash(&arg_list) {
                return;
            }

            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer method call without parentheses".to_string(),
            );

            if let Some(ref mut corrections) = self.corrections {
                if let (Some(open), Some(close)) = (call.opening_loc(), call.closing_loc()) {
                    corrections.push(crate::correction::Correction {
                        start: open.start_offset(),
                        end: open.end_offset(),
                        replacement: " ".to_string(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    corrections.push(crate::correction::Correction {
                        start: close.start_offset(),
                        end: close.end_offset(),
                        replacement: String::new(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }

            self.diagnostics.push(diagnostic);
        }
    }
}

impl<'pr> Visit<'pr> for ParenStyleVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_factory_call(node);

        // Visit receiver (not ambiguous)
        if let Some(recv) = node.receiver() {
            let saved = self.parent_is_ambiguous;
            let saved_kind = self.ambiguity_kind;
            self.parent_is_ambiguous = false;
            self.ambiguity_kind = None;
            self.visit(&recv);
            self.parent_is_ambiguous = saved;
            self.ambiguity_kind = saved_kind;
        }

        // Visit arguments — children here have a CallNode (send) parent = ambiguous
        if let Some(args) = node.arguments() {
            let saved = self.parent_is_ambiguous;
            let saved_kind = self.ambiguity_kind;
            self.parent_is_ambiguous = true;
            self.ambiguity_kind = Some(AmbiguityKind::Call);
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
            self.parent_is_ambiguous = saved;
            self.ambiguity_kind = saved_kind;
        }

        // Visit block (not ambiguous — block body is independent context)
        if let Some(block) = node.block() {
            let saved = self.parent_is_ambiguous;
            let saved_kind = self.ambiguity_kind;
            self.parent_is_ambiguous = false;
            self.ambiguity_kind = None;
            self.visit(&block);
            self.parent_is_ambiguous = saved;
            self.ambiguity_kind = saved_kind;
        }
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;
        self.parent_is_ambiguous = true;
        self.ambiguity_kind = Some(AmbiguityKind::Array);
        ruby_prism::visit_array_node(self, node);
        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;
        self.parent_is_ambiguous = true;
        self.ambiguity_kind = Some(AmbiguityKind::Assoc);
        ruby_prism::visit_assoc_node(self, node);
        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;
        self.parent_is_ambiguous = true;
        self.ambiguity_kind = Some(AmbiguityKind::AndOr);
        ruby_prism::visit_and_node(self, node);
        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;
        self.parent_is_ambiguous = true;
        self.ambiguity_kind = Some(AmbiguityKind::AndOr);
        ruby_prism::visit_or_node(self, node);
        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;

        // Predicate: ambiguous (factory call in if condition has parent `if` in Parser)
        self.parent_is_ambiguous = true;
        self.ambiguity_kind = Some(AmbiguityKind::If);
        self.visit(&node.predicate());

        // Body: In Parser, single-statement if bodies have the statement as a
        // direct child of `if` (no `begin` wrapper). Multi-statement bodies wrap
        // in `begin`. In Prism, StatementsNode always interposes. We match
        // Parser by treating single-statement bodies as ambiguous only when the
        // statement itself is a CallNode (the factory call IS the if child).
        // When it's something else (lvasgn, etc.), the factory call inside has
        // a non-ambiguous parent in Parser too (e.g., lvasgn).
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() == 1 && body[0].as_call_node().is_some() {
                self.parent_is_ambiguous = true;
                self.ambiguity_kind = Some(AmbiguityKind::If);
            } else {
                self.parent_is_ambiguous = false;
                self.ambiguity_kind = None;
            }
            for stmt in &body {
                self.visit(stmt);
            }
        }

        // Subsequent (elsif/else): same treatment as the body.
        // In Parser, else-branch content is a direct child of `if`, so a sole
        // CallNode statement in the else body has parent `if` (ambiguous).
        if let Some(sub) = node.subsequent() {
            if let Some(else_node) = sub.as_else_node() {
                if let Some(stmts) = else_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if body.len() == 1 && body[0].as_call_node().is_some() {
                        self.parent_is_ambiguous = true;
                        self.ambiguity_kind = Some(AmbiguityKind::If);
                    } else {
                        self.parent_is_ambiguous = false;
                        self.ambiguity_kind = None;
                    }
                    for stmt in &body {
                        self.visit(stmt);
                    }
                }
            } else {
                // elsif — handled recursively by visit_if_node
                self.parent_is_ambiguous = false;
                self.ambiguity_kind = None;
                self.visit(&sub);
            }
        }

        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;
        // Lambda bodies are independent contexts (like blocks and parentheses).
        // Factory calls inside `-> { create :sym }` are not ambiguous even when
        // the lambda itself appears inside an ambiguous context (assoc, call args).
        self.parent_is_ambiguous = false;
        self.ambiguity_kind = None;
        ruby_prism::visit_lambda_node(self, node);
        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        let saved_kind = self.ambiguity_kind;
        // Parenthesized expressions map to `begin` in Parser AST, which is NOT
        // in RuboCop's AMBIGUOUS_TYPES. Always clear ambiguity: `(create :user)`
        // inside an assoc value or or-expression is not ambiguous.
        self.parent_is_ambiguous = false;
        self.ambiguity_kind = None;
        ruby_prism::visit_parentheses_node(self, node);
        self.parent_is_ambiguous = saved;
        self.ambiguity_kind = saved_kind;
    }
}

/// Check if any argument is a hash with value omission (Ruby 3.1+ `name:` syntax).
fn has_value_omission_hash(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        if let Some(hash) = arg.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ConsistentParenthesesStyle,
        "cops/factorybot/consistent_parentheses_style"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(ConsistentParenthesesStyle.supports_autocorrect());
    }

    #[test]
    fn autocorrects_require_parentheses_style() {
        crate::testutil::assert_cop_autocorrect(
            &ConsistentParenthesesStyle,
            b"create :user\n",
            b"create(:user)\n",
        );
    }

    #[test]
    fn autocorrects_omit_parentheses_style() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("omit_parentheses".to_string()),
            )]),
            ..CopConfig::default()
        };

        crate::testutil::assert_cop_autocorrect_with_config(
            &ConsistentParenthesesStyle,
            b"create(:user)\n",
            b"create :user\n",
            config,
        );
    }
}
