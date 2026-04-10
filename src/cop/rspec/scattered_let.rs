use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_let, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ScatteredLet checks for let/let! declarations scattered across an example group.
///
/// ## Corpus investigation (2026-03-18) — FN=5
///
/// Root cause: when receiver was `RSpec`, only `describe` was matched as an
/// example group. `RSpec.feature`, `RSpec.context`, etc. were not recognized.
/// 4 of 5 FNs were from avo-hq using `RSpec.feature`; 1 from rubocop-rspec's
/// smoke test file (excluded by AllCops.Exclude in that project).
/// Fix: use `is_rspec_example_group()` for receiver-qualified calls too,
/// matching the same pattern used in LetBeforeExamples (commit 7158fd2b).
///
/// FP root cause (43 FPs): The cop was running inside shared_examples/shared_examples_for/
/// shared_context blocks. RuboCop's `example_group_with_body?` matcher only matches
/// ExampleGroups (describe/context/feature), NOT SharedGroups. Fixed by skipping
/// shared group method names.
///
/// ## Corpus investigation (2026-03-14)
///
/// Previously excluded `let :name, &proc` (BlockArgumentNode) form, thinking RuboCop's
/// `let?` only matched BlockNode. But RuboCop's actual `let?` pattern matches both:
///   `(block (send nil? {:let :let!}) ...)` AND `(send nil? {:let :let!} _ block_pass)`.
/// Restored block_pass recognition (2026-03-20) to fix FN=2 on rubocop-rspec's
/// `weird_rspec_spec.rb` where `let(:foo, &bar)` was not counted as a let declaration.
///
/// ## Corpus investigation (2026-03-24) — FP=1
///
/// FP=1: Infrastructure issue (plugin detection). que-rb uses minitest, not RSpec.
/// No cop logic fix needed.
///
/// que-rb/que `spec/que/connection_spec.rb:19`: `let :fresh_connection, &NEW_PG_CONNECTION`
/// flagged as scattered let. The que repo uses minitest (not RSpec) but has
/// `describe`/`let`/`around` DSL from minitest-hooks that looks like RSpec.
/// The repo does NOT have rubocop-rspec in its Gemfile, so RuboCop cannot load
/// rubocop-rspec and skips all RSpec cops. nitrocop has them compiled in and
/// runs them regardless, causing the asymmetry. This is an infrastructure-level
/// issue (plugin detection), not a cop logic bug. nitrocop's detection is correct
/// for actual RSpec files.
pub struct ScatteredLet;

impl Cop for ScatteredLet {
    fn name(&self) -> &'static str {
        "RSpec/ScatteredLet"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for example group calls (including ::RSpec.describe)
        // Skip shared groups (shared_examples, shared_examples_for, shared_context)
        // — RuboCop's example_group_with_body? only matches ExampleGroups, not SharedGroups.
        if is_rspec_shared_group(method_name) {
            return;
        }

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
                && !is_rspec_shared_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return;
        }

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

        // Track if we've seen a non-let statement after the initial let block
        let mut seen_non_let = false;
        let mut in_let_group = false;
        let mut first_let_insert_after: Option<usize> = None;
        let mut corrected_one = false;

        let mut corrections = corrections;
        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();
                // RuboCop's `let?` pattern matches two forms:
                // 1. (block (send nil? {:let :let!}) ...) — regular block
                // 2. (send nil? {:let :let!} _ block_pass) — &proc argument
                let has_block_node = c.block().is_some_and(|b| b.as_block_node().is_some());
                // In Prism, `&proc` (block-pass) is stored in `call.block()` as BlockArgumentNode
                let has_block_pass = c
                    .block()
                    .is_some_and(|b| b.as_block_argument_node().is_some());
                if c.receiver().is_none()
                    && is_rspec_let(name)
                    && (has_block_node || has_block_pass)
                {
                    if seen_non_let {
                        // This let is after a non-let statement
                        let loc = stmt.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            "Group all let/let! blocks in the example group together.".to_string(),
                        );

                        if !corrected_one
                            && let Some(anchor) = first_let_insert_after
                            && let Some(corrections) = &mut corrections
                            && let Some((remove_start, remove_end, moved_text)) =
                                movable_statement_text(source, &stmt)
                            && anchor <= remove_start
                            && let Some(between_text) = source.try_byte_slice(anchor, remove_start)
                        {
                            corrections.push(crate::correction::Correction {
                                start: anchor,
                                end: remove_end,
                                replacement: format!("{moved_text}{between_text}"),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                            corrected_one = true;
                        }

                        diagnostics.push(diagnostic);
                    } else {
                        in_let_group = true;
                        if first_let_insert_after.is_none() {
                            let mut end = stmt.location().end_offset();
                            if source.as_bytes().get(end).copied() == Some(b'\n') {
                                end += 1;
                            }
                            first_let_insert_after = Some(end);
                        }
                    }
                    continue;
                }
            }

            if in_let_group {
                seen_non_let = true;
            }
        }
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
    crate::cop_fixture_tests!(ScatteredLet, "cops/rspec/scattered_let");

    #[test]
    fn supports_autocorrect() {
        assert!(ScatteredLet.supports_autocorrect());
    }

    #[test]
    fn autocorrect_moves_first_scattered_let_after_first_let() {
        crate::testutil::assert_cop_autocorrect(
            &ScatteredLet,
            b"RSpec.describe User do\n  let(:a) { a }\n  it { expect(subject.foo).to eq(a) }\n  let(:b) { b }\nend\n",
            b"RSpec.describe User do\n  let(:a) { a }\n  let(:b) { b }\n  it { expect(subject.foo).to eq(a) }\nend\n",
        );
    }
}
