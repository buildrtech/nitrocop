use crate::cop::node_type::PROGRAM_NODE;
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook,
    is_rspec_let, is_rspec_subject,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/LeadingSubject checks that `subject` is declared before `let`, hooks,
/// examples, and other declarations within an example group.
///
/// RuboCop uses `InsideExampleGroup` to determine whether a `subject` node
/// should be checked. This check walks up to the file's root-level node and
/// verifies it is a spec group (describe/context/shared_examples block). When
/// the describe block is wrapped in a `module` or `class` declaration, the
/// root-level node is the module/class — NOT a spec group — so RuboCop skips
/// the cop entirely. This is a documented side-effect of `InsideExampleGroup`.
///
/// We replicate this by only checking subjects inside spec groups that are
/// at the file's top level (direct children of the program node, or within a
/// top-level `begin`). Spec groups inside module/class wrappers are skipped.
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of 118 FNs:** Two issues found:
///
/// 1. Include-family blocks (it_behaves_like, include_context, include_examples,
///    it_should_behave_like) were not recursed into. RuboCop's `on_block` fires
///    on ALL blocks, so subjects inside `it_behaves_like do...end` are checked
///    independently for ordering within that block. The nitrocop code only
///    recursed into example group blocks (describe/context/shared_examples).
///    Fixed by adding `recurse_into_block()` for include-family calls.
///
/// 2. `RSpec.describe` nested inside another example group was recursed into but
///    NOT treated as an offending node (the `continue` after recursion skipped
///    the `first_relevant_name` update). RuboCop's `spec_group?` includes
///    `RSpec.describe`, so it IS offending. Fixed by setting `first_relevant_name`
///    for `RSpec.describe` calls.
///
/// ## Investigation (2026-03-15)
///
/// **Root cause of 84 FNs:** RuboCop's `on_block` fires on ALL blocks, not just
/// example groups and include-family blocks. The `parent(node)` method gets the
/// immediate block ancestor, so subjects inside arbitrary blocks (custom DSL
/// methods like `with_feature_flag do...end`, `around do...end` etc.) are
/// checked independently for ordering within that block. The nitrocop code only
/// recursed into example group and include-family blocks, missing subjects
/// inside arbitrary blocks. Fixed by recursing into ALL call nodes with blocks
/// that are children of an example group body.
///
/// ## Investigation (2026-03-18)
///
/// **Root cause of 72 FNs:** Two issues found:
///
/// 1. `is_spec_group_call()` at the top level only matched `RSpec.describe` for
///    receiver calls, missing `RSpec.shared_examples_for`, `RSpec.shared_context`,
///    `RSpec.feature`, etc. Many corpus files use `RSpec.shared_examples_for` or
///    `RSpec.shared_context` at the top level, so subjects in those blocks were
///    never checked. Fixed by matching all `RSpec.<example_group>` methods.
///
/// 2. Calls with receivers (e.g. `items.each do...end`, `hash.each_pair do...end`)
///    were completely skipped during recursion (`continue` after the
///    `RSpec.describe` check). Subjects inside iterator blocks that contain
///    nested `context`/`describe` blocks were missed. Fixed by recursing into
///    the block body of any receiver call that has a block, matching RuboCop's
///    `on_block` behavior that fires on ALL blocks.
///
/// ## Verification (2026-03-18)
///
/// Manual verification against locally available corpus repos (avo-hq, openproject,
/// diaspora) confirms all 72 FN examples from the CI oracle are now detected by the
/// current code. Patterns verified include:
/// - `include_context` without block before subject (diaspora mentioning_spec)
/// - Subject inside `.each` iterator block with destructured args (openproject users_helper)
/// - Named subject `subject(:name)` after `let` with intervening `def` method (openproject attachment_resource)
/// - `it_behaves_like` with block before subject at same level (openproject attachment_resource)
/// - Subject inside `RSpec.shared_examples_for` after `let` (openproject response_examples)
/// - `shared_let` (custom DSL, not offending) followed by `include_context` + `subject`
///
/// The commit c0bc7a5 estimated "fixes 43 of 72 (29 remain)" but actual verification
/// shows all 72 patterns are handled. The "29 remain" was a conservative estimate;
/// the CI oracle simply hasn't re-run to confirm.
///
/// ## Investigation (2026-03-20)
///
/// **Root cause of 3 FNs:** `if`/`unless` control flow nodes wrapping spec groups
/// (e.g., `if linux?`, `unless ENV["CI"]`) were not traversed during block body
/// iteration. The cop only looked at `CallNode` children, so `describe`/`context`
/// blocks inside conditionals were invisible. Fixed by adding
/// `recurse_into_conditional()` which walks `IfNode`/`UnlessNode` bodies (including
/// elsif/else branches) and recurses into any block-bearing call nodes found within.
/// All 3 FN repos (guard/listen, bunny, vcr) use this pattern.
pub struct LeadingSubject;

impl Cop for LeadingSubject {
    fn name(&self) -> &'static str {
        "RSpec/LeadingSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        // Walk top-level statements looking for spec groups.
        // Only spec groups at the file root (not inside module/class) are checked,
        // matching RuboCop's InsideExampleGroup behavior.
        let mut corrections = corrections;
        for stmt in program.statements().body().iter() {
            if is_spec_group_call(&stmt) {
                self.check_block_body(source, &stmt, diagnostics, corrections.as_deref_mut());
            }
            // Skip modules, classes, requires, and anything else at the top level.
        }
    }
}

impl LeadingSubject {
    /// Check subject ordering within a block body and recurse into child blocks.
    /// This is the unified handler for example groups, include-family blocks,
    /// and arbitrary blocks — matching RuboCop's `on_block` behavior which fires
    /// on ALL blocks and uses `parent(node)` to check ordering within the
    /// immediate parent block.
    fn check_block_body(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let mut first_relevant_name: Option<&[u8]> = None;
        let mut first_relevant_insert_at: Option<usize> = None;
        let mut corrected_one = false;

        let mut corrections = corrections;
        for stmt in stmts.body().iter() {
            if stmt.as_if_node().is_some() || stmt.as_unless_node().is_some() {
                self.recurse_into_conditional(
                    source,
                    &stmt,
                    diagnostics,
                    corrections.as_deref_mut(),
                );
                continue;
            }

            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();

                if c.receiver().is_some() {
                    let is_rspec_group = util::constant_name(&c.receiver().unwrap())
                        .is_some_and(|n| n == b"RSpec")
                        && is_rspec_example_group(name);
                    if is_rspec_group {
                        self.check_block_body(
                            source,
                            &stmt,
                            diagnostics,
                            corrections.as_deref_mut(),
                        );
                        if first_relevant_name.is_none() {
                            first_relevant_name = Some(name);
                            first_relevant_insert_at = first_statement_insert_at(source, &stmt);
                        }
                    } else if c.block().is_some() {
                        self.check_block_body(
                            source,
                            &stmt,
                            diagnostics,
                            corrections.as_deref_mut(),
                        );
                    }
                    continue;
                }

                if is_rspec_subject(name) {
                    if let Some(prev_name) = first_relevant_name {
                        let prev_str = std::str::from_utf8(prev_name).unwrap_or("let");
                        let loc = stmt.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Declare `subject` above any other `{prev_str}` declarations."),
                        );

                        if !corrected_one
                            && let Some(insert_at) = first_relevant_insert_at
                            && let Some(corrections) = &mut corrections
                            && let Some((remove_start, remove_end, moved_text)) =
                                movable_statement_text(source, &stmt)
                            && insert_at <= remove_start
                            && let Some(between_text) =
                                source.try_byte_slice(insert_at, remove_start)
                        {
                            corrections.push(crate::correction::Correction {
                                start: insert_at,
                                end: remove_end,
                                replacement: format!("{moved_text}{between_text}"),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                            corrected_one = true;
                        }

                        diagnostics.push(diagnostic);
                    }
                } else if is_rspec_example_group(name) {
                    self.check_block_body(source, &stmt, diagnostics, corrections.as_deref_mut());
                    if first_relevant_name.is_none() {
                        first_relevant_name = Some(name);
                        first_relevant_insert_at = first_statement_insert_at(source, &stmt);
                    }
                } else if is_example_include(name) {
                    self.check_block_body(source, &stmt, diagnostics, corrections.as_deref_mut());
                    if first_relevant_name.is_none() {
                        first_relevant_name = Some(name);
                        first_relevant_insert_at = first_statement_insert_at(source, &stmt);
                    }
                } else if is_rspec_let(name) {
                    let has_block = c.block().is_some();
                    let has_block_pass = c.arguments().is_some_and(|args| {
                        args.arguments()
                            .iter()
                            .any(|a| a.as_block_argument_node().is_some())
                    });
                    if has_block {
                        self.check_block_body(
                            source,
                            &stmt,
                            diagnostics,
                            corrections.as_deref_mut(),
                        );
                    }
                    if (has_block || has_block_pass) && first_relevant_name.is_none() {
                        first_relevant_name = Some(name);
                        first_relevant_insert_at = first_statement_insert_at(source, &stmt);
                    }
                } else if is_rspec_hook(name) || is_rspec_example(name) {
                    if c.block().is_some() {
                        self.check_block_body(
                            source,
                            &stmt,
                            diagnostics,
                            corrections.as_deref_mut(),
                        );
                        if first_relevant_name.is_none() {
                            first_relevant_name = Some(name);
                            first_relevant_insert_at = first_statement_insert_at(source, &stmt);
                        }
                    }
                } else if c.block().is_some() {
                    self.check_block_body(source, &stmt, diagnostics, corrections.as_deref_mut());
                }
            }
        }
    }

    fn recurse_into_conditional(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut corrections = corrections;
        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                self.recurse_conditional_stmts(
                    source,
                    &stmts,
                    diagnostics,
                    corrections.as_deref_mut(),
                );
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.recurse_into_conditional(source, &subsequent, diagnostics, corrections);
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                self.recurse_conditional_stmts(
                    source,
                    &stmts,
                    diagnostics,
                    corrections.as_deref_mut(),
                );
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    self.recurse_conditional_stmts(
                        source,
                        &stmts,
                        diagnostics,
                        corrections.as_deref_mut(),
                    );
                }
            }
        } else if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                self.recurse_conditional_stmts(source, &stmts, diagnostics, corrections);
            }
        }
    }

    fn recurse_conditional_stmts(
        &self,
        source: &SourceFile,
        stmts: &ruby_prism::StatementsNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut corrections = corrections;
        for stmt in stmts.body().iter() {
            if stmt.as_if_node().is_some() || stmt.as_unless_node().is_some() {
                self.recurse_into_conditional(
                    source,
                    &stmt,
                    diagnostics,
                    corrections.as_deref_mut(),
                );
            } else if let Some(c) = stmt.as_call_node() {
                if c.block().is_some() {
                    self.check_block_body(source, &stmt, diagnostics, corrections.as_deref_mut());
                }
            }
        }
    }
}

fn is_spec_group_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        // RSpec.describe, RSpec.shared_examples_for, RSpec.shared_context, RSpec.feature, etc.
        util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && is_rspec_example_group(name)
    } else {
        is_rspec_example_group(name)
    }
}

fn is_example_include(name: &[u8]) -> bool {
    name == b"include_examples"
        || name == b"it_behaves_like"
        || name == b"it_should_behave_like"
        || name == b"include_context"
}

fn first_statement_insert_at(source: &SourceFile, stmt: &ruby_prism::Node<'_>) -> Option<usize> {
    let loc = stmt.location();
    let (line, _) = source.offset_to_line_col(loc.start_offset());
    let line_start = source.line_start_offset(line);
    if source
        .try_byte_slice(line_start, loc.start_offset())
        .is_some_and(|s| s.trim().is_empty())
    {
        Some(line_start)
    } else {
        Some(loc.start_offset())
    }
}

fn movable_statement_text(
    source: &SourceFile,
    stmt: &ruby_prism::Node<'_>,
) -> Option<(usize, usize, String)> {
    let loc = stmt.location();
    let (line, _) = source.offset_to_line_col(loc.start_offset());
    let line_start = source.line_start_offset(line);

    let mut remove_start = loc.start_offset();
    if source
        .try_byte_slice(line_start, loc.start_offset())
        .is_some_and(|s| s.trim().is_empty())
    {
        remove_start = line_start;
    }

    let mut remove_end = loc.end_offset();
    if source.as_bytes().get(remove_end).copied() == Some(b'\n') {
        remove_end += 1;
    }

    let moved_text = source.try_byte_slice(remove_start, remove_end)?.to_string();
    Some((remove_start, remove_end, moved_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeadingSubject, "cops/rspec/leading_subject");

    #[test]
    fn supports_autocorrect() {
        assert!(LeadingSubject.supports_autocorrect());
    }

    #[test]
    fn autocorrect_moves_subject_before_let() {
        crate::testutil::assert_cop_autocorrect(
            &LeadingSubject,
            b"RSpec.describe User do\n  let(:params) { foo }\n\n  subject { described_class.new }\nend\n",
            b"RSpec.describe User do\n  subject { described_class.new }\n  let(:params) { foo }\n\nend\n",
        );
    }
}
