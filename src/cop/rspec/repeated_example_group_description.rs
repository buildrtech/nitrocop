use crate::cop::node_type::{PROGRAM_NODE, STATEMENTS_NODE};
use crate::cop::util::is_rspec_example_group;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/RepeatedExampleGroupDescription: Flag example groups with identical descriptions.
///
/// ## Investigation findings (2026-03-11)
///
/// Root cause of 119 FNs: the cop only handled `ProgramNode` (top-level) and `CallNode`
/// (example group blocks) as parent contexts. RuboCop's `on_begin` fires on ANY `begin`
/// node (equivalent to Prism's `StatementsNode`), which also includes module/class bodies,
/// method bodies, and any other compound statement context.
///
/// Fix: switched to handling `StatementsNode` directly, which covers all contexts where
/// sibling example groups can appear (top-level program, module/class bodies, block bodies).
///
/// Also added:
/// - `skip_or_pending` filtering: groups containing bare `skip` or `pending` calls are
///   excluded from duplicate checking (matches RuboCop's SkipOrPending mixin).
/// - Empty description filtering: groups with no arguments are excluded.
///
/// ## Corpus investigation (FN=79, 2026-03-15)
///
/// FN=79: description_signature used raw source bytes for comparison, causing
/// single-quoted vs double-quoted strings to be treated as different descriptions.
/// RuboCop's AST node equality treats "foo" and 'foo' identically (both are str nodes).
/// Fixed by normalizing StringNode/SymbolNode to their unescaped values.
pub struct RepeatedExampleGroupDescription;

impl Cop for RepeatedExampleGroupDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExampleGroupDescription"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        crate::cop::util::RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE, STATEMENTS_NODE]
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
        // Handle ProgramNode (top-level) and StatementsNode (module/class/block bodies).
        // ProgramNode is needed because Prism may not visit the top-level StatementsNode
        // as a separate node in the walk.
        let stmts: Vec<ruby_prism::Node<'_>> = if let Some(program) = node.as_program_node() {
            program.statements().body().iter().collect()
        } else if let Some(stmts_node) = node.as_statements_node() {
            stmts_node.body().iter().collect()
        } else {
            return;
        };

        // RuboCop's several_example_groups? requires at least 2 example groups
        let example_group_count = stmts.iter().filter(|s| is_example_group_call(s)).count();
        if example_group_count < 2 {
            return;
        }

        #[allow(clippy::type_complexity)] // internal collection used only in this function
        let mut desc_map: HashMap<Vec<u8>, Vec<(usize, usize, Vec<u8>, usize, usize)>> =
            HashMap::new();

        for stmt in &stmts {
            let call = match extract_example_group_call(stmt) {
                Some(c) => c,
                None => continue,
            };

            // Skip groups with skip/pending inside the block
            if has_skip_or_pending_in_block(stmt) {
                continue;
            }

            // Skip groups with no description (empty_description?)
            if call.arguments().is_none() {
                continue;
            }

            let name = call.name().as_slice();

            // Extract the description signature (all args)
            let desc_sig = match description_signature(source, &call) {
                Some(s) => s,
                None => continue,
            };

            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let first_arg = call
                .arguments()
                .and_then(|a| a.arguments().iter().next())
                .map(|arg| arg.location());
            let (arg_start, arg_end) = if let Some(arg_loc) = first_arg {
                (arg_loc.start_offset(), arg_loc.end_offset())
            } else {
                (loc.start_offset(), loc.end_offset())
            };
            desc_map.entry(desc_sig).or_default().push((
                line,
                col,
                name.to_vec(),
                arg_start,
                arg_end,
            ));
        }

        let mut corrections = corrections;
        for locs in desc_map.values() {
            if locs.len() > 1 {
                for (idx, (line, col, group_name, start, end)) in locs.iter().enumerate() {
                    let other_lines: Vec<String> = locs
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != idx)
                        .map(|(_, (l, _, _, _, _))| l.to_string())
                        .collect();
                    let group_type = std::str::from_utf8(group_name).unwrap_or("describe");
                    let display_type = group_type
                        .strip_prefix('f')
                        .or(group_type.strip_prefix('x'))
                        .unwrap_or(group_type);
                    let msg = format!(
                        "Repeated {} block description on line(s) [{}]",
                        display_type,
                        other_lines.join(", ")
                    );
                    let mut diagnostic = self.diagnostic(source, *line, *col, msg);
                    if let Some(corrections) = corrections.as_deref_mut() {
                        corrections.push(crate::correction::Correction {
                            start: *start,
                            end: *end,
                            replacement: "'TODO: unique group description'".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                }
            }
        }
    }
}

/// Check if a node is an example group call (with or without receiver).
fn is_example_group_call(node: &ruby_prism::Node<'_>) -> bool {
    extract_example_group_call(node).is_some()
}

/// Extract the CallNode from a node if it's an example group call.
/// The node might be a bare CallNode or a CallNode with a block.
fn extract_example_group_call<'a>(
    node: &'a ruby_prism::Node<'a>,
) -> Option<ruby_prism::CallNode<'a>> {
    let call = node.as_call_node()?;
    if is_rspec_group_for_desc(&call) {
        // Must have a block to be an example group
        if call.block().is_some() {
            return Some(call);
        }
    }
    None
}

/// Check if a block-level example group contains `skip` or `pending` calls.
/// Matches RuboCop's `skip_or_pending_inside_block?` pattern:
///   (block <(send nil? {:skip :pending} ...) ...>)
fn has_skip_or_pending_in_block(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let block = match call.block() {
        Some(b) => b,
        None => return false,
    };
    let block_node = match block.as_block_node() {
        Some(b) => b,
        None => return false,
    };
    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    for stmt in stmts.body().iter() {
        if let Some(inner_call) = stmt.as_call_node() {
            let name = inner_call.name().as_slice();
            if (name == b"skip" || name == b"pending") && inner_call.receiver().is_none() {
                return true;
            }
        }
    }
    false
}

fn description_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None;
    }

    let mut sig: Vec<u8> = Vec::new();

    // Normalize the first argument (description string/symbol) to its unescaped value.
    // RuboCop compares AST node equality where "foo" and 'foo' produce the same (str "foo") node.
    let first = &arg_list[0];
    if let Some(s) = first.as_string_node() {
        sig.extend_from_slice(b"str:");
        sig.extend_from_slice(s.unescaped());
    } else if let Some(sym) = first.as_symbol_node() {
        sig.extend_from_slice(b"sym:");
        sig.extend_from_slice(sym.unescaped());
    } else {
        // Constants, interpolated strings, etc: use source text
        let loc = first.location();
        sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
    }

    // Metadata args: use source text (symbols here won't have quote style differences)
    for arg in &arg_list[1..] {
        sig.push(b',');
        let loc = arg.location();
        sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
    }

    Some(sig)
}

fn is_rspec_group_for_desc(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context" {
        return false;
    }
    if !is_rspec_example_group(name) {
        return false;
    }
    match call.receiver() {
        None => true,
        Some(recv) => {
            if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"RSpec"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.name().is_some_and(|n| n.as_slice() == b"RSpec") && cp.parent().is_none()
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        RepeatedExampleGroupDescription,
        "cops/rspec/repeated_example_group_description"
    );
    crate::cop_autocorrect_fixture_tests!(
        RepeatedExampleGroupDescription,
        "cops/rspec/repeated_example_group_description"
    );
}
