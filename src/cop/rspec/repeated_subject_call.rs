use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/RepeatedSubjectCall: Flag calling `subject` multiple times in the same example
/// when at least one call is inside a block (expect { subject }).
///
/// ## FP fix (2026-03): align with RuboCop's `detect_offense` filtering
///
/// RuboCop only flags a repeated `subject` call when ALL three conditions hold:
/// 1. `subject` is NOT chained (`subject.foo` — skip, subject is a receiver)
/// 2. `subject`'s parent is NOT a send/call node (e.g., `expect(subject)` — skip,
///    subject is an argument)
/// 3. `subject` is inside an `expect { ... }` block (not just any block)
///
/// The previous implementation only tracked `in_block` (any block) and didn't
/// check condition 2. This caused 42 FPs from patterns like:
///   `expect(subject).to be_allowed(resource)` inside `.each` loops —
///   subject as an argument was incorrectly counted as a flaggable call.
///
/// ## FN fix (2026-03): track named subject aliases
///
/// RSpec allows `subject(:name) { ... }` which creates a method alias for `subject`.
/// The cop only matched literal `subject` calls but not named aliases. All 17 FNs
/// were from named subjects like `subject(:job)` where the example calls `job`.
/// Refactored from `check_node` to `check_source` with a visitor that collects
/// `subject(:name)` definitions per example group scope, then uses the alias set
/// when scanning examples for repeated calls.
///
/// ## FP fix (2026-03-20): always group subject calls by name
///
/// When multiple named subjects exist in scope (e.g., `subject(:metric)` at an outer
/// group and `subject(:track)` at an inner group), calls to different subject names
/// were being lumped together and counted as "repeated". For example, in:
///   `metric.track(value); expect { track }.to change { metric.values }`
/// the chained `metric` call (from `metric.values`) was counted as the "first" subject
/// call, making the bare `track` call appear as the "second" — triggering a false
/// positive even though `track` only appears once. Fixed by always grouping subject
/// calls by name (matching RuboCop's per-`method_name` tracking in
/// `detect_offenses_in_example`).
///
/// ## FN fix (2026-03-20): handle ConstantPathNode for subject usage
///
/// When a named subject is used as a constant path parent (e.g., `mod::Params` where
/// `mod` is `subject(:mod)`), the `mod` call is inside a `ConstantPathNode`. The
/// previous implementation only recursed into `CallNode` children, missing subject
/// calls nested inside constant paths. Added `ConstantPathNode` handling in
/// `collect_subject_calls` to recurse into the parent node. The subject call is NOT
/// marked as chained (it's a constant path parent, not a method chain receiver).
pub struct RepeatedSubjectCall;

impl Cop for RepeatedSubjectCall {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedSubjectCall"
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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match parse_result.node().as_program_node() {
            Some(p) => p,
            None => return,
        };
        let body = program.statements();
        let mut corrections = corrections;
        for stmt in body.body().iter() {
            find_example_groups(
                source,
                &stmt,
                &[b"subject".to_vec()],
                diagnostics,
                corrections.as_deref_mut(),
                self,
            );
        }
    }
}

/// Check if a call has an RSpec receiver (e.g., `RSpec.describe`).
fn is_rspec_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"RSpec";
        }
        if let Some(cp) = recv.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"RSpec";
                }
            }
        }
    }
    false
}

/// Recursively find example groups in the AST and process them.
fn find_example_groups(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    inherited_subjects: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
    cop: &RepeatedSubjectCall,
) {
    let mut corrections = corrections;
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_eg = (call.receiver().is_none() && is_rspec_example_group(name))
            || (is_rspec_receiver(&call) && is_rspec_example_group(name));

        if is_eg {
            if let Some(block) = call.block() {
                if let Some(bn) = block.as_block_node() {
                    process_example_group(
                        source,
                        &bn,
                        inherited_subjects,
                        diagnostics,
                        corrections.as_deref_mut(),
                        cop,
                    );
                }
            }
            return;
        }
    }

    // Unwrap module/class/begin nodes to find top-level groups
    if let Some(module_node) = node.as_module_node() {
        if let Some(body) = module_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for child in stmts.body().iter() {
                    find_example_groups(
                        source,
                        &child,
                        inherited_subjects,
                        diagnostics,
                        corrections.as_deref_mut(),
                        cop,
                    );
                }
            }
        }
        return;
    }
    if let Some(class_node) = node.as_class_node() {
        if let Some(body) = class_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for child in stmts.body().iter() {
                    find_example_groups(
                        source,
                        &child,
                        inherited_subjects,
                        diagnostics,
                        corrections.as_deref_mut(),
                        cop,
                    );
                }
            }
        }
        return;
    }
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                find_example_groups(
                    source,
                    &child,
                    inherited_subjects,
                    diagnostics,
                    corrections.as_deref_mut(),
                    cop,
                );
            }
        }
    }
}

/// Process an example group block: collect subject names, check examples, recurse into nested groups.
fn process_example_group(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
    inherited_subjects: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
    cop: &RepeatedSubjectCall,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    // Collect subject names defined in this scope
    let mut subject_names: Vec<Vec<u8>> = inherited_subjects.to_vec();
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if (name == b"subject" || name == b"subject!") && call.receiver().is_none() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        if let Some(sym) = arg_list[0].as_symbol_node() {
                            let alias = sym.unescaped().to_vec();
                            if !subject_names.contains(&alias) {
                                subject_names.push(alias);
                            }
                        }
                    }
                }
            }
        }
    }

    // Process statements
    let mut corrections = corrections;
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            let call_name = call.name().as_slice();

            // Check examples
            if is_rspec_example(call_name) && call.receiver().is_none() {
                if let Some(block) = call.block() {
                    if let Some(bn) = block.as_block_node() {
                        if let Some(body) = bn.body() {
                            check_example_body(
                                source,
                                &body,
                                &subject_names,
                                diagnostics,
                                corrections.as_deref_mut(),
                                cop,
                            );
                        }
                    }
                }
            }

            // Recurse into nested example groups
            if is_rspec_example_group(call_name) && call.receiver().is_none() {
                if let Some(block) = call.block() {
                    if let Some(bn) = block.as_block_node() {
                        process_example_group(
                            source,
                            &bn,
                            &subject_names,
                            diagnostics,
                            corrections.as_deref_mut(),
                            cop,
                        );
                    }
                }
            }
        }
    }
}

/// Check an example body for repeated subject calls.
///
/// Always groups calls by subject name (matching RuboCop's per-method_name tracking
/// in `detect_offenses_in_example`). This prevents false positives when multiple
/// named subjects coexist (e.g., `subject(:metric)` and `subject(:track)`) — a
/// chained `metric.values` call should not count as a repeat of `track`.
fn check_example_body(
    source: &SourceFile,
    body: &ruby_prism::Node<'_>,
    subject_names: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
    cop: &RepeatedSubjectCall,
) {
    let mut subject_calls: Vec<SubjectCall> = Vec::new();
    collect_subject_calls(
        source,
        body,
        false,
        false,
        subject_names,
        &mut subject_calls,
    );

    if subject_calls.len() <= 1 {
        return;
    }

    // Group calls by name and check each group independently.
    // RuboCop uses `subjects_used[call.method_name]` which is inherently per-name.
    let unique_names: Vec<&[u8]> = {
        let mut names: Vec<&[u8]> = subject_calls.iter().map(|c| c.name.as_slice()).collect();
        names.sort_unstable();
        names.dedup();
        names
    };
    let mut corrections = corrections;
    for name in unique_names {
        let calls_for_name: Vec<&SubjectCall> =
            subject_calls.iter().filter(|c| c.name == name).collect();
        if calls_for_name.len() <= 1 {
            continue;
        }
        flag_repeated_calls(
            &calls_for_name,
            source,
            diagnostics,
            corrections.as_deref_mut(),
            cop,
        );
    }
}

/// Flag the 2nd+ subject calls that are in expect blocks and not chained/args.
fn flag_repeated_calls(
    calls: &[&SubjectCall],
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
    cop: &RepeatedSubjectCall,
) {
    let mut seen_first = false;
    let mut corrections = corrections;
    for call in calls {
        if !seen_first {
            seen_first = true;
            continue;
        }
        if call.in_expect_block && !call.is_chained && !call.is_arg_of_call {
            let mut diagnostic = cop.diagnostic(
                source,
                call.line,
                call.col,
                "Calls to subject are memoized, this block is misleading".to_string(),
            );
            if let Some(corrections) = corrections.as_deref_mut() {
                corrections.push(crate::correction::Correction {
                    start: call.start,
                    end: call.end,
                    replacement: "skip('TODO: avoid repeated subject call')".to_string(),
                    cop_name: cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
            diagnostics.push(diagnostic);
        }
    }
}

struct SubjectCall {
    name: Vec<u8>,
    line: usize,
    col: usize,
    start: usize,
    end: usize,
    in_expect_block: bool,
    is_chained: bool,
    is_arg_of_call: bool,
}

fn collect_subject_calls(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    in_expect_block: bool,
    is_receiver: bool,
    subject_names: &[Vec<u8>],
    results: &mut Vec<SubjectCall>,
) {
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_subject_calls(
                source,
                &stmt,
                in_expect_block,
                false,
                subject_names,
                results,
            );
        }
        return;
    }

    // Handle ConstantPathNode (e.g., `mod::Params`) — the parent of a constant path
    // may be a subject call (e.g., `mod` is a named subject). RuboCop's recursive
    // `def_node_search :subject_calls` finds `(send nil? %)` inside constant paths.
    // The subject call here is NOT chained (parent is a const, not a send).
    if let Some(const_path) = node.as_constant_path_node() {
        if let Some(parent) = const_path.parent() {
            collect_subject_calls(
                source,
                &parent,
                in_expect_block,
                false,
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();

        // Check if this is a bare subject call (no receiver) matching any known subject name.
        if call.receiver().is_none() && subject_names.iter().any(|s| s == name) {
            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            results.push(SubjectCall {
                name: name.to_vec(),
                line,
                col,
                start: loc.start_offset(),
                end: loc.end_offset(),
                in_expect_block,
                is_chained: is_receiver,
                is_arg_of_call: false, // will be set by caller
            });
        }

        // Recurse into receiver, marking it as a receiver context
        if let Some(recv) = call.receiver() {
            collect_subject_calls(source, &recv, in_expect_block, true, subject_names, results);
        }

        // Check arguments — subject here is an argument to this call
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                let before = results.len();
                collect_subject_calls(source, &arg, in_expect_block, false, subject_names, results);
                // Mark any subject calls found in args as arguments of a call
                for r in &mut results[before..] {
                    r.is_arg_of_call = true;
                }
            }
        }

        // Check the call's own block — track if this is an `expect` block
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    let is_expect =
                        name == b"expect" || (in_expect_block && call.receiver().is_none());
                    collect_subject_calls(
                        source,
                        &body,
                        in_expect_block || is_expect,
                        false,
                        subject_names,
                        results,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedSubjectCall, "cops/rspec/repeated_subject_call");
    crate::cop_autocorrect_fixture_tests!(RepeatedSubjectCall, "cops/rspec/repeated_subject_call");
}
