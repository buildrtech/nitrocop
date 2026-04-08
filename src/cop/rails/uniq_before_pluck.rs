/// Rails/UniqBeforePluck — flag `pluck(...).uniq` and suggest `distinct.pluck(...)`.
///
/// ## Root cause (2026-03)
/// Original implementation used `CONSTANT_PATH_NODE` / `CONSTANT_READ_NODE` as interested
/// node types and tried to walk *up* from the model constant to find a `.pluck(...).uniq`
/// chain via `as_method_chain`. This never worked because `as_method_chain` expects a
/// `CallNode` as input — a constant read node is not a call node, so it always returned
/// `None` and produced zero offenses.
///
/// ## Fix (2026-03, round 1)
/// Switched to `CALL_NODE` as the interested type.  On every `CallNode`, check whether the
/// method name is `uniq` or `uniq!` (no block arguments), and whether the receiver is also a
/// `CallNode` whose method name is `pluck`.  In conservative mode (default), additionally
/// require that the root receiver of the `pluck` call is a constant (model class), not an
/// instance variable or a chained scope/association.
///
/// ## Fix (2026-03, round 2) — false positives from `uniq!`
/// RuboCop uses `RESTRICT_ON_SEND = %i[uniq].freeze` — it only triggers on `uniq`, NOT on
/// `uniq!`. The round-1 implementation also flagged `uniq!`, causing 2 false positives in the
/// corpus. Fixed by only checking for `uniq` (not `uniq!`).
///
/// ## Fix (2026-03, round 3) — false positives from block bodies
/// RuboCop's NodePattern includes `!^any_block` which means the parent of the `pluck.uniq`
/// send node must NOT be a block node. In Parser AST, when `pluck.uniq` is the sole body
/// expression of a block (e.g., `cache { Model.pluck(:name).uniq }`), its parent IS the
/// block node, so it's excluded. Converted from `check_node` to `check_source` with a
/// visitor that tracks `in_block_body` to replicate this behavior. This eliminated 2 FPs
/// in the corpus (both in discourse's `lib/svg_sprite.rb`).
///
/// Offense is reported at the `uniq` selector location (matching RuboCop's
/// `node.loc.selector`), i.e., `message_loc()` of the `uniq` call node.
use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UniqBeforePluck;

impl Cop for UniqBeforePluck {
    fn name(&self) -> &'static str {
        "Rails/UniqBeforePluck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "conservative");
        let mut visitor = UniqBeforePluckVisitor {
            cop: self,
            source,
            conservative: style == "conservative",
            in_block_body: false,
            diagnostics: Vec::new(),
            corrections,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct UniqBeforePluckVisitor<'a, 'src> {
    cop: &'a UniqBeforePluck,
    source: &'src SourceFile,
    conservative: bool,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
    /// True when we're visiting statements that are the direct body of a block.
    /// In Parser AST, a single-statement block body has the statement as a direct
    /// child of the block node, so `!^any_block` excludes it. For multi-statement
    /// bodies, the parent is `begin` (not block), so they ARE flagged.
    in_block_body: bool,
    diagnostics: Vec<Diagnostic>,
}

impl UniqBeforePluckVisitor<'_, '_> {
    fn check_uniq_call(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Only interested in `uniq` method calls (not `uniq!`).
        if node.name().as_slice() != b"uniq" {
            return;
        }

        // uniq must not have a block argument
        if node.block().is_some() {
            return;
        }

        // The receiver of `uniq` must be a `pluck(...)` call
        let receiver = match node.receiver() {
            Some(r) => r,
            None => return,
        };
        let pluck_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if pluck_call.name().as_slice() != b"pluck" {
            return;
        }

        // In conservative mode, only flag if the root receiver of pluck is a constant
        if self.conservative {
            let pluck_receiver = match pluck_call.receiver() {
                Some(r) => r,
                None => return,
            };
            let is_const = pluck_receiver.as_constant_read_node().is_some()
                || pluck_receiver.as_constant_path_node().is_some();
            if !is_const {
                return;
            }
        }

        // Skip if this is a direct block body (!^any_block in RuboCop)
        if self.in_block_body {
            return;
        }

        // Report at the `uniq` selector (message_loc)
        let loc = node.message_loc().unwrap_or_else(|| node.location());
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `distinct` before `pluck`.".to_string(),
        );

        if let Some(ref mut corr) = self.corrections {
            let uniq_selector = node.message_loc().unwrap_or_else(|| node.location());
            corr.push(crate::correction::Correction {
                start: pluck_call.location().end_offset(),
                end: uniq_selector.end_offset(),
                replacement: String::new(),
                cop_name: self.cop.name(),
                cop_index: 0,
            });

            if let Some(dot) = pluck_call.call_operator_loc() {
                corr.push(crate::correction::Correction {
                    start: dot.start_offset(),
                    end: dot.start_offset(),
                    replacement: ".distinct".to_string(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
            } else {
                corr.push(crate::correction::Correction {
                    start: pluck_call.location().start_offset(),
                    end: pluck_call.location().start_offset(),
                    replacement: "distinct.".to_string(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
            }

            diagnostic.corrected = true;
        }

        self.diagnostics.push(diagnostic);
    }
}

impl<'pr> Visit<'pr> for UniqBeforePluckVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_uniq_call(node);

        // Visit children normally (receiver, arguments)
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit_arguments_node(&args);
        }
        // Visit block — block body children get in_block_body set in visit_block_node
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Visit parameters normally
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        // Visit body with in_block_body tracking.
        // In Parser AST, !^any_block excludes single-statement block bodies because
        // the statement is a direct child of the block. For multi-statement bodies,
        // the parent is `begin` (not block). In Prism, the body is always a
        // StatementsNode. We check if it has exactly one statement — if so, set
        // in_block_body=true for that statement.
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let stmt_list: Vec<_> = stmts.body().iter().collect();
                if stmt_list.len() == 1 {
                    let saved = self.in_block_body;
                    self.in_block_body = true;
                    self.visit(&stmt_list[0]);
                    self.in_block_body = saved;
                } else {
                    // Multi-statement body: parent is `begin` in Parser AST, not block
                    self.visit(&body);
                }
            } else {
                self.visit(&body);
            }
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambda is also a block type in RuboCop's `any_block`
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let stmt_list: Vec<_> = stmts.body().iter().collect();
                if stmt_list.len() == 1 {
                    let saved = self.in_block_body;
                    self.in_block_body = true;
                    self.visit(&stmt_list[0]);
                    self.in_block_body = saved;
                } else {
                    self.visit(&body);
                }
            } else {
                self.visit(&body);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(UniqBeforePluck, "cops/rails/uniq_before_pluck");

    fn config_with_style(style: &str) -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String(style.to_string()),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn autocorrects_model_pluck_uniq() {
        crate::testutil::assert_cop_autocorrect(
            &UniqBeforePluck,
            b"Model.pluck(:name).uniq\n",
            b"Model.distinct.pluck(:name)\n",
        );
    }

    #[test]
    fn autocorrects_receiverless_pluck_uniq_in_aggressive_style() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &UniqBeforePluck,
            b"pluck(:name).uniq\n",
            b"distinct.pluck(:name)\n",
            config_with_style("aggressive"),
        );
    }
}
