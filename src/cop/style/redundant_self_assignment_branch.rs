use crate::cop::node_type::{IF_NODE, LOCAL_VARIABLE_WRITE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/RedundantSelfAssignmentBranch
///
/// Checks for places where conditional branch makes redundant self-assignment.
///
/// RuboCop only detects local variable assignments (not instance/class/global vars)
/// because replacing those with nil could change state across methods.
///
/// ## Conditions for offense
/// - LHS is a local variable assignment (`LocalVariableWriteNode`)
/// - RHS is an if/else expression (NOT case/when, NOT ternary)
/// - No `elsif` branch present
/// - Neither branch has multiple statements
/// - One branch is a bare read of the same local variable
///
/// ## Historical FP causes
/// - Flagging case/when expressions (RuboCop only handles if/else)
/// - Flagging if/elsif/else chains
/// - Flagging branches with multiple statements
/// - Reporting offense on the whole assignment instead of the self-assignment branch
/// - Flagging ternary expressions (`a ? b : a`) — RuboCop's `use_if_and_else_branch?`
///   returns false for ternaries (`!expression.ternary? || !expression.else?` = false).
///   Found via corpus FP in linguist repo's sinatra.rb sample.
///
/// ## Historical FN causes
/// - None expected — RuboCop only handles local variables, same as this cop.
pub struct RedundantSelfAssignmentBranch;

impl Cop for RedundantSelfAssignmentBranch {
    fn name(&self) -> &'static str {
        "Style/RedundantSelfAssignmentBranch"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, LOCAL_VARIABLE_WRITE_NODE]
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
        let write = match node.as_local_variable_write_node() {
            Some(w) => w,
            None => return,
        };

        let var_name = write.name().as_slice();
        let value = write.value();

        // Only handle if/else expressions — NOT case/when or ternary
        let if_node = match value.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Skip ternary expressions (a ? b : c) — RuboCop only flags if/else form
        if if_node.if_keyword_loc().is_none() {
            return;
        }

        self.check_if_node(source, &if_node, var_name, diagnostics, corrections);
    }
}

impl RedundantSelfAssignmentBranch {
    fn check_if_node(
        &self,
        source: &SourceFile,
        if_node: &ruby_prism::IfNode<'_>,
        var_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Get the if-branch statements
        let if_stmts = if_node.statements();

        // Get the else/subsequent branch
        let subsequent = match if_node.subsequent() {
            Some(s) => s,
            None => return, // no else branch — skip
        };

        // If subsequent is another IfNode (elsif), skip entirely
        if subsequent.as_if_node().is_some() {
            return;
        }

        // Must be an ElseNode
        let else_node = match subsequent.as_else_node() {
            Some(e) => e,
            None => return,
        };

        let else_stmts = else_node.statements();

        // Check for multiple statements in either branch
        if has_multiple_statements(&if_stmts) || has_multiple_statements(&else_stmts) {
            return;
        }

        // Check if the if-branch is a self-assignment
        if is_single_var_read(&if_stmts, var_name) {
            if let Some(stmts) = &if_stmts {
                let body: Vec<_> = stmts.body().iter().collect();
                if let Some(read_node) = body.first() {
                    let loc = read_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove the self-assignment branch.".to_string(),
                    );
                    if let Some(replacement) =
                        build_replacement(source, if_node, &else_stmts, "unless")
                    {
                        if let Some(corrections) = corrections.as_mut() {
                            let if_loc = if_node.location();
                            corrections.push(crate::correction::Correction {
                                start: if_loc.start_offset(),
                                end: if_loc.end_offset(),
                                replacement,
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
                    }
                    diagnostics.push(diagnostic);
                }
            }
            return;
        }

        // Check if the else-branch is a self-assignment
        if is_single_var_read(&else_stmts, var_name) {
            if let Some(stmts) = &else_stmts {
                let body: Vec<_> = stmts.body().iter().collect();
                if let Some(read_node) = body.first() {
                    let loc = read_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove the self-assignment branch.".to_string(),
                    );
                    if let Some(replacement) = build_replacement(source, if_node, &if_stmts, "if") {
                        if let Some(corrections) = corrections.as_mut() {
                            let if_loc = if_node.location();
                            corrections.push(crate::correction::Correction {
                                start: if_loc.start_offset(),
                                end: if_loc.end_offset(),
                                replacement,
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
                    }
                    diagnostics.push(diagnostic);
                }
            }
        }
    }
}

fn build_replacement(
    source: &SourceFile,
    if_node: &ruby_prism::IfNode<'_>,
    opposite_branch: &Option<ruby_prism::StatementsNode<'_>>,
    keyword: &str,
) -> Option<String> {
    let condition_loc = if_node.predicate().location();
    let condition_src = std::str::from_utf8(
        &source.as_bytes()[condition_loc.start_offset()..condition_loc.end_offset()],
    )
    .ok()?
    .trim();

    let assignment_value = opposite_branch
        .as_ref()
        .and_then(|stmts| stmts.body().iter().next())
        .map(|node| {
            let loc = node.location();
            std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()])
                .ok()
                .map(|s| s.trim().to_string())
        })
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "nil".to_string());

    Some(format!("{assignment_value} {keyword} {condition_src}"))
}

fn has_multiple_statements(stmts: &Option<ruby_prism::StatementsNode<'_>>) -> bool {
    if let Some(s) = stmts {
        let body: Vec<_> = s.body().iter().collect();
        body.len() > 1
    } else {
        false
    }
}

fn is_single_var_read(stmts: &Option<ruby_prism::StatementsNode<'_>>, var_name: &[u8]) -> bool {
    if let Some(s) = stmts {
        let body: Vec<_> = s.body().iter().collect();
        body.len() == 1 && is_same_var(&body[0], var_name)
    } else {
        false
    }
}

fn is_same_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        return lv.name().as_slice() == var_name;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSelfAssignmentBranch,
        "cops/style/redundant_self_assignment_branch"
    );
    crate::cop_autocorrect_fixture_tests!(
        RedundantSelfAssignmentBranch,
        "cops/style/redundant_self_assignment_branch"
    );
}
