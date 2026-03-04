use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Flags stubbing methods on `subject`. The object under test should not be stubbed.
/// Detects: allow(subject_name).to receive(...), expect(subject_name).to receive(...)
///
/// Investigation notes (corpus FP=571, FN=27):
/// Root cause of FPs:
/// 1. Missing TopLevelGroup scoping: RuboCop's `TopLevelGroup#top_level_nodes` only
///    processes describe/context blocks at the file's top level. When `require "spec_helper"`
///    appears alongside a `module Foo` wrapper, the AST root is a `begin` node whose children
///    are `[require_call, module_node]`. The module is not a spec group, so it's skipped.
///    Our cop was processing ALL describe/context blocks regardless of nesting.
/// 2. Local variable reads: RuboCop's `(send nil? %)` pattern only matches method calls,
///    not local variable reads. When `subject = Foo.new` shadows the RSpec subject,
///    `allow(subject).to receive(...)` uses a local variable, not the subject method.
///    Our `extract_simple_name` was matching both CallNode and LocalVariableReadNode.
///
/// Root cause of FNs (27):
/// Multi-line `do...end` block arguments on receive chains like
/// `expect(subject).to receive(:method).and_wrap_original do |orig| ... end`
/// are not detected because the block wraps the whole chain differently in the AST.
pub struct SubjectStub;

impl Cop for SubjectStub {
    fn name(&self) -> &'static str {
        "RSpec/SubjectStub"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match parse_result.node().as_program_node() {
            Some(p) => p,
            None => return,
        };
        let body = program.statements();
        let stmts: Vec<_> = body.body().iter().collect();

        if stmts.len() == 1 {
            // Single top-level statement: unwrap module/class wrappers to find spec groups.
            self.find_top_level_groups(&stmts[0], source, diagnostics);
        } else {
            // Multiple top-level statements (e.g., `require "spec_helper"` + `describe Foo`):
            // Only check direct children for spec groups, do NOT unwrap modules/classes.
            // This matches RuboCop's TopLevelGroup `:begin` branch.
            for stmt in &stmts {
                self.check_direct_spec_group(stmt, source, diagnostics);
            }
        }
    }
}

impl SubjectStub {
    /// Check if a single node is a top-level spec group and process it.
    /// Does NOT recurse into module/class — used for the `:begin` case.
    fn check_direct_spec_group(
        &self,
        node: &ruby_prism::Node<'_>,
        source: &SourceFile,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(call) = node.as_call_node() {
            if let Some(block) = call.block() {
                if let Some(bn) = block.as_block_node() {
                    let name = call.name().as_slice();
                    let is_eg = call.receiver().is_none() && is_rspec_example_group(name);
                    let is_rspec_describe =
                        is_rspec_receiver(&call) && is_rspec_example_group(name);
                    if is_eg || is_rspec_describe {
                        let mut subject_names: Vec<Vec<u8>> = Vec::new();
                        subject_names.push(b"subject".to_vec());
                        collect_subject_stub_offenses(
                            source,
                            bn,
                            &mut subject_names,
                            diagnostics,
                            self,
                        );
                    }
                }
            }
        }
    }

    /// Recursively find top-level spec groups, unwrapping module/class/begin nodes.
    /// Used when there's a single top-level construct.
    fn find_top_level_groups(
        &self,
        node: &ruby_prism::Node<'_>,
        source: &SourceFile,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check if this node is a spec group call
        if let Some(call) = node.as_call_node() {
            if let Some(block) = call.block() {
                if let Some(bn) = block.as_block_node() {
                    let name = call.name().as_slice();
                    let is_eg = call.receiver().is_none() && is_rspec_example_group(name);
                    let is_rspec_describe =
                        is_rspec_receiver(&call) && is_rspec_example_group(name);
                    if is_eg || is_rspec_describe {
                        let mut subject_names: Vec<Vec<u8>> = Vec::new();
                        subject_names.push(b"subject".to_vec());
                        collect_subject_stub_offenses(
                            source,
                            bn,
                            &mut subject_names,
                            diagnostics,
                            self,
                        );
                        return;
                    }
                }
            }
        }

        // Unwrap module nodes
        if let Some(module_node) = node.as_module_node() {
            if let Some(body) = module_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for child in stmts.body().iter() {
                        self.find_top_level_groups(&child, source, diagnostics);
                    }
                }
            }
            return;
        }

        // Unwrap class nodes
        if let Some(class_node) = node.as_class_node() {
            if let Some(body) = class_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for child in stmts.body().iter() {
                        self.find_top_level_groups(&child, source, diagnostics);
                    }
                }
            }
            return;
        }

        // Unwrap begin nodes
        if let Some(begin_node) = node.as_begin_node() {
            if let Some(stmts) = begin_node.statements() {
                for child in stmts.body().iter() {
                    self.find_top_level_groups(&child, source, diagnostics);
                }
            }
        }
    }
}

fn collect_subject_stub_offenses(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    subject_names: &mut Vec<Vec<u8>>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    // First pass: collect subject names defined in this scope
    let scope_start = subject_names.len();
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if (name == b"subject" || name == b"subject!") && call.receiver().is_none() {
                // Check if it has a name argument: subject(:foo)
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        if let Some(sym) = arg_list[0].as_symbol_node() {
                            subject_names.push(sym.unescaped().to_vec());
                        }
                    }
                }
            }
        }
    }

    // Second pass: check for stubs on subject names and recurse into nested groups
    for stmt in stmts.body().iter() {
        check_for_subject_stubs(source, &stmt, subject_names, diagnostics, cop);
    }

    // Restore subject names for this scope (don't leak child-scope subjects to siblings)
    subject_names.truncate(scope_start);
    // But re-add the implicit "subject"
    if !subject_names.contains(&b"subject".to_vec()) {
        subject_names.push(b"subject".to_vec());
    }
}

fn check_for_subject_stubs(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    subject_names: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    if let Some(call) = node.as_call_node() {
        // Check for allow(subject_name).to receive(...) or expect(subject_name).to receive(...)
        let method = call.name().as_slice();
        if method == b"to" || method == b"not_to" || method == b"to_not" {
            // Check if the argument involves `receive`
            if has_receive_matcher(&call) || has_have_received_matcher(&call) {
                // Check receiver is allow/expect(subject_name) or is_expected
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        let recv_method = recv_call.name().as_slice();
                        if recv_method == b"is_expected" && recv_call.receiver().is_none() {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(cop.diagnostic(
                                source,
                                line,
                                column,
                                "Do not stub methods of the object under test.".to_string(),
                            ));
                            return;
                        }
                        if (recv_method == b"allow" || recv_method == b"expect")
                            && recv_call.receiver().is_none()
                        {
                            if let Some(args) = recv_call.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if !arg_list.is_empty() {
                                    // Only match method calls (send nil?), NOT local variable reads.
                                    // RuboCop's `(send nil? %)` pattern only matches method sends.
                                    let arg_name = extract_method_name(&arg_list[0]);
                                    if let Some(name) = arg_name {
                                        if subject_names.iter().any(|s| s == &name) {
                                            let loc = node.location();
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(
                                                cop.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Do not stub methods of the object under test."
                                                        .to_string(),
                                                ),
                                            );
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Recurse into nested blocks (before, it, context, etc.)
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                let call_name = call.name().as_slice();
                if is_rspec_example_group(call_name) {
                    // Nested example group — create new scope with inherited subject names
                    let mut child_names = subject_names.to_vec();
                    collect_subject_stub_offenses(source, bn, &mut child_names, diagnostics, cop);
                } else {
                    // Non-example-group block (before, it, specify, def, etc.)
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                check_for_subject_stubs(
                                    source,
                                    &s,
                                    subject_names,
                                    diagnostics,
                                    cop,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Check def nodes for subject stubs too
    if let Some(def_node) = node.as_def_node() {
        if let Some(body) = def_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for s in stmts.body().iter() {
                    check_for_subject_stubs(source, &s, subject_names, diagnostics, cop);
                }
            }
        }
    }
}

fn has_receive_matcher(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if contains_receive_call(&arg) {
                return true;
            }
        }
    }
    false
}

fn has_have_received_matcher(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if contains_have_received_call(&arg) {
                return true;
            }
        }
    }
    false
}

fn contains_receive_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"receive" || name == b"receive_messages" || name == b"receive_message_chain")
            && call.receiver().is_none()
        {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return contains_receive_call(&recv);
        }
        // Check arguments too (e.g., `all(receive(:baz))`)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if contains_receive_call(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

fn contains_have_received_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"have_received" && call.receiver().is_none() {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return contains_have_received_call(&recv);
        }
    }
    false
}

/// Extract the name of a receiverless method call. Only matches `CallNode` with
/// no receiver and no arguments (i.e., `(send nil? :name)` in RuboCop terms).
/// Does NOT match local variable reads — RuboCop's message_expectation? matcher
/// uses `(send nil? %)` which only matches method sends, not lvar reads.
fn extract_method_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.arguments().is_none() {
            return Some(call.name().as_slice().to_vec());
        }
    }
    None
}

/// Check if the receiver of a CallNode is `RSpec` (simple constant) or `::RSpec`.
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SubjectStub, "cops/rspec/subject_stub");
}
