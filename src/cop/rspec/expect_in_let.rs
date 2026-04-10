use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ExpectInLet: flags `expect`, `is_expected`, `expect_any_instance_of` inside let blocks.
///
/// **Root cause of 70 FNs (0 FP):** The `find_expects_in_node` recursive search only handled
/// `StatementsNode` and `CallNode` receiver chains. It did not recurse into `IfNode`, `BlockNode`,
/// `UnlessNode`, `CaseNode`, `BeginNode`, `AndNode`, `OrNode`, or any other container nodes.
/// This meant any `expect` call nested inside control flow, iterator blocks, or logical operators
/// within a let body was missed.
///
/// **Fix:** Expanded `find_expects_in_node` to recurse into all common container node types,
/// matching RuboCop's `def_node_search` deep traversal behavior.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=0, FN=11 (7 from toptal/chewy, 2 from pakyow, 2 from shoes4).
///
/// FN=11: All FNs were nested `expect` calls inside outer `expect` blocks within
/// let bodies, e.g., `let(:expectation) { expect { expect { ... }.to ... } }`.
/// The function returned early after finding the outer `expect`, preventing
/// detection of inner nested `expect` calls. RuboCop's `def_node_search` finds
/// ALL matching nodes in the subtree. Fix: continue recursion after reporting
/// an offense instead of returning early.
///
/// ## Corpus investigation (2026-03-20)
///
/// FP=0, FN=4 (2 from pakyow/pakyow, 2 from shoes/shoes4).
///
/// **pakyow FN=2:** `let :error do ... rescue => error; expect(error)... end` —
/// `BeginNode` handler only recursed into `statements()` (the body before rescue),
/// not `rescue_clause()`. The `expect` was inside the rescue clause body.
/// Fix: recurse into `begin_node.rescue_clause()` and `ensure_clause()`.
///
/// **shoes4 FN=2:** `let(:klazz) do Class.new(Base) { def visit_me; expect(...); end } end` —
/// `expect` inside `DefNode` (method definition) within a class body within a let block.
/// `find_expects_in_node` did not handle `DefNode`. Fix: add `DefNode` recursion.
pub struct ExpectInLet;

/// Expectation methods to flag inside let blocks.
const EXPECT_METHODS: &[&[u8]] = &[b"expect", b"is_expected", b"expect_any_instance_of"];

impl Cop for ExpectInLet {
    fn name(&self) -> &'static str {
        "RSpec/ExpectInLet"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if method_name != b"let" && method_name != b"let!" {
            return;
        }

        // Check the block body for expect calls
        let block_raw = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let mut hits = Vec::new();
        find_expects_in_node(&body, &mut hits);

        for hit in hits {
            let method_str = std::str::from_utf8(hit.method.as_slice()).unwrap_or("expect");
            let (line, column) = source.offset_to_line_col(hit.start);
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                format!("Do not use `{method_str}` in let"),
            );

            if let Some(corrections) = corrections.as_deref_mut() {
                corrections.push(crate::correction::Correction {
                    start: hit.start,
                    end: hit.end,
                    replacement: "skip".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

struct ExpectHit {
    method: Vec<u8>,
    start: usize,
    end: usize,
}

fn find_expects_in_node(node: &ruby_prism::Node<'_>, hits: &mut Vec<ExpectHit>) {
    if let Some(call) = node.as_call_node()
        && call.receiver().is_none()
        && EXPECT_METHODS.contains(&call.name().as_slice())
    {
        if let Some(selector) = call.message_loc() {
            hits.push(ExpectHit {
                method: call.name().as_slice().to_vec(),
                start: selector.start_offset(),
                end: selector.end_offset(),
            });
        }
    }

    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_expects_in_node(&child, hits);
        }
        return;
    }
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            find_expects_in_node(&recv, hits);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_expects_in_node(&arg, hits);
            }
        }
        if let Some(block) = call.block() {
            find_expects_in_node(&block, hits);
        }
        return;
    }
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            find_expects_in_node(&body, hits);
        }
        return;
    }
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_expects_in_node(&subsequent, hits);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        if let Some(else_clause) = unless_node.else_clause() {
            find_expects_in_node(&else_clause.as_node(), hits);
        }
        return;
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            find_expects_in_node(&cond, hits);
        }
        if let Some(else_clause) = case_node.else_clause() {
            find_expects_in_node(&else_clause.as_node(), hits);
        }
        return;
    }
    if let Some(when_node) = node.as_when_node() {
        if let Some(stmts) = when_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            find_expects_in_node(&rescue_clause.as_node(), hits);
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            find_expects_in_node(&ensure_clause.as_node(), hits);
        }
        return;
    }
    if let Some(rescue_node) = node.as_rescue_node() {
        if let Some(stmts) = rescue_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        if let Some(consequent) = rescue_node.subsequent() {
            find_expects_in_node(&consequent.as_node(), hits);
        }
        return;
    }
    if let Some(rescue_mod) = node.as_rescue_modifier_node() {
        find_expects_in_node(&rescue_mod.expression(), hits);
        find_expects_in_node(&rescue_mod.rescue_expression(), hits);
        return;
    }
    if let Some(ensure_node) = node.as_ensure_node() {
        if let Some(stmts) = ensure_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(in_node) = node.as_in_node() {
        if let Some(stmts) = in_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(case_match) = node.as_case_match_node() {
        for cond in case_match.conditions().iter() {
            find_expects_in_node(&cond, hits);
        }
        if let Some(else_clause) = case_match.else_clause() {
            find_expects_in_node(&else_clause.as_node(), hits);
        }
        return;
    }
    if let Some(parentheses) = node.as_parentheses_node() {
        if let Some(body) = parentheses.body() {
            find_expects_in_node(&body, hits);
        }
        return;
    }
    if let Some(for_node) = node.as_for_node() {
        if let Some(stmts) = for_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(while_node) = node.as_while_node() {
        if let Some(stmts) = while_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        if let Some(stmts) = until_node.statements() {
            find_expects_in_node(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(and_node) = node.as_and_node() {
        find_expects_in_node(&and_node.left(), hits);
        find_expects_in_node(&and_node.right(), hits);
        return;
    }
    if let Some(or_node) = node.as_or_node() {
        find_expects_in_node(&or_node.left(), hits);
        find_expects_in_node(&or_node.right(), hits);
        return;
    }
    if let Some(def_node) = node.as_def_node() {
        if let Some(body) = def_node.body() {
            find_expects_in_node(&body, hits);
        }
        return;
    }
    if let Some(lambda) = node.as_lambda_node()
        && let Some(body) = lambda.body()
    {
        find_expects_in_node(&body, hits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectInLet, "cops/rspec/expect_in_let");
    crate::cop_autocorrect_fixture_tests!(ExpectInLet, "cops/rspec/expect_in_let");
}
