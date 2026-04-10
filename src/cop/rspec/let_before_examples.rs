use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_let,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=32, FN=4.
///
/// FP root cause: `it_behaves_like`/`include_examples` calls with inline blocks
/// were treated as "first examples seen", so following top-level `let` declarations
/// were flagged. RuboCop only treats include calls without inline blocks as example
/// inclusions for this cop's ordering rule.
///
/// Fix: count example-inclusion calls only when they do not have blocks.
/// Inline-block forms are setup wrappers and should not trigger ordering offenses.
///
/// ## Corpus investigation (2026-03-18)
///
/// Remaining FN=4: all in avo-hq, `RSpec.feature` example group.
/// Root cause: when receiver was `RSpec`, only `describe` was matched as an
/// example group method. `RSpec.feature`, `RSpec.context`, etc. were ignored.
/// Fix: use `is_rspec_example_group()` for receiver-qualified calls too.
pub struct LetBeforeExamples;

impl Cop for LetBeforeExamples {
    fn name(&self) -> &'static str {
        "RSpec/LetBeforeExamples"
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

        // Check for example group calls (including ::RSpec.describe).
        // Exclude shared groups (shared_examples, shared_context, etc.) — RuboCop's
        // `example_group_with_body?` only matches ExampleGroups (describe/context/feature),
        // not SharedGroups, so let ordering inside shared_examples is allowed.
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
                && !is_shared_group(method_name)
        } else {
            is_rspec_example_group(method_name) && !is_shared_group(method_name)
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

        let mut seen_example = false;
        let mut first_example_start: Option<usize> = None;
        let mut corrected_one = false;
        let mut has_example_include = false;

        let mut corrections = corrections;
        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();
                if c.receiver().is_none() {
                    // Only count actual examples and non-shared example groups
                    // (with blocks) as "seen example". Shared groups
                    // (shared_examples, shared_context) don't count.
                    let is_example_or_group_with_block = (is_rspec_example(name)
                        || is_non_shared_example_group(name))
                        && c.block().is_some();
                    if is_example_include(name) {
                        has_example_include = true;
                    }

                    if is_example_or_group_with_block
                        || (is_example_include(name) && c.block().is_none())
                    {
                        seen_example = true;
                        let stmt_loc = stmt.location();
                        let (stmt_line, _) = source.offset_to_line_col(stmt_loc.start_offset());
                        let line_start = source.line_start_offset(stmt_line);
                        let insert_start = if source
                            .try_byte_slice(line_start, stmt_loc.start_offset())
                            .is_some_and(|s| s.trim().is_empty())
                        {
                            line_start
                        } else {
                            stmt_loc.start_offset()
                        };
                        first_example_start.get_or_insert(insert_start);
                    } else if seen_example && is_rspec_let(name) {
                        let let_name = std::str::from_utf8(name).unwrap_or("let");
                        let loc = stmt.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Move `{let_name}` before the examples in the group."),
                        );

                        if !has_example_include
                            && !corrected_one
                            && let Some(insert_at) = first_example_start
                            && let Some(corrections) = &mut corrections
                            && let Some((remove_start, remove_end, moved_text)) =
                                movable_statement_text(source, &stmt)
                            && insert_at <= remove_start
                            && let Some(between_text) = source.try_byte_slice(insert_at, remove_start)
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
                }
            }
        }
    }
}

/// Check if a method name is an RSpec example group but NOT a shared group.
/// Shared groups (shared_examples, shared_examples_for, shared_context) don't
/// count as "examples seen" for the LetBeforeExamples cop because they define
/// reusable code, not actual test groups.
fn is_non_shared_example_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
    )
}

fn is_shared_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"shared_examples" | b"shared_examples_for" | b"shared_context"
    )
}

fn is_example_include(name: &[u8]) -> bool {
    name == b"include_examples" || name == b"it_behaves_like" || name == b"it_should_behave_like"
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
    crate::cop_fixture_tests!(LetBeforeExamples, "cops/rspec/let_before_examples");

    #[test]
    fn supports_autocorrect() {
        assert!(LetBeforeExamples.supports_autocorrect());
    }

    #[test]
    fn autocorrect_moves_let_before_first_example() {
        crate::testutil::assert_cop_autocorrect(
            &LetBeforeExamples,
            b"RSpec.describe User do\n  it { is_expected.to be_valid }\n  let(:foo) { bar }\nend\n",
            b"RSpec.describe User do\n  let(:foo) { bar }\n  it { is_expected.to be_valid }\nend\n",
        );
    }
}
