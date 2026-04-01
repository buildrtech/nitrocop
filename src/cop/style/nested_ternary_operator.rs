use crate::cop::node_type::{ELSE_NODE, IF_NODE, PARENTHESES_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NestedTernaryOperator;

/// Check if an IfNode is a ternary operator (no if_keyword_loc in Prism)
fn is_ternary(if_node: &ruby_prism::IfNode<'_>) -> bool {
    if_node.if_keyword_loc().is_none()
}

/// Find ternary nodes within a node, recursing into parentheses
fn find_nested_ternary(node: &ruby_prism::Node<'_>, source: &SourceFile) -> Vec<(usize, usize)> {
    let mut results = Vec::new();
    if let Some(if_node) = node.as_if_node() {
        if is_ternary(&if_node) {
            let loc = if_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            results.push((line, column));
        }
    }
    // Recurse into parentheses
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if let Some(stmts) = body.as_statements_node() {
                for stmt in stmts.body().iter() {
                    results.extend(find_nested_ternary(&stmt, source));
                }
            }
        }
    }
    results
}

fn single_statement_source(
    source: &SourceFile,
    statements: ruby_prism::StatementsNode<'_>,
) -> Option<String> {
    let body: Vec<_> = statements.body().iter().collect();
    if body.len() != 1 {
        return None;
    }
    let node = &body[0];
    Some(
        source
            .byte_slice(node.location().start_offset(), node.location().end_offset(), "")
            .to_string(),
    )
}

fn remove_wrapping_parentheses(expr: String) -> String {
    let trimmed = expr.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        expr
    }
}

fn replacement_for_ternary(if_node: &ruby_prism::IfNode<'_>, source: &SourceFile) -> Option<String> {
    let predicate = if_node.predicate();
    let if_statements = if_node.statements()?;
    let subsequent = if_node.subsequent()?;

    let if_branch = remove_wrapping_parentheses(single_statement_source(source, if_statements)?);

    let else_branch = if let Some(else_node) = subsequent.as_else_node() {
        single_statement_source(source, else_node.statements()?)?
    } else if let Some(else_if) = subsequent.as_if_node() {
        source
            .byte_slice(
                else_if.location().start_offset(),
                else_if.location().end_offset(),
                "",
            )
            .to_string()
    } else {
        return None;
    };

    let cond_src = source.byte_slice(
        predicate.location().start_offset(),
        predicate.location().end_offset(),
        "",
    );

    let (_, column) = source.offset_to_line_col(if_node.location().start_offset());
    let base_indent = " ".repeat(column.saturating_sub(1));
    let body_indent = format!("{base_indent}  ");

    Some(format!(
        "if {cond_src}\n{body_indent}{if_branch}\n{base_indent}else\n{body_indent}{else_branch}\n{base_indent}end"
    ))
}

impl Cop for NestedTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/NestedTernaryOperator"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ELSE_NODE, IF_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must be a ternary
        if !is_ternary(&if_node) {
            return;
        }

        let mut offenses = Vec::new();

        // Check if_branch for nested ternaries
        if let Some(if_branch) = if_node.statements() {
            for stmt in if_branch.body().iter() {
                for (line, column) in find_nested_ternary(&stmt, source) {
                    offenses.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
                    ));
                }
            }
        }

        // Check else_branch for nested ternaries
        if let Some(else_clause) = if_node.subsequent() {
            let else_node: ruby_prism::Node<'_> = else_clause;
            if let Some(else_n) = else_node.as_else_node() {
                if let Some(stmts) = else_n.statements() {
                    for stmt in stmts.body().iter() {
                        for (line, column) in find_nested_ternary(&stmt, source) {
                            offenses.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
                            ));
                        }
                    }
                }
            }
            // Also check if the subsequent is itself an IfNode (ternary in else position without else keyword)
            if let Some(sub_if) = else_node.as_if_node() {
                if is_ternary(&sub_if) {
                    let loc = sub_if.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    offenses.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
                    ));
                }
            }
        }

        if !offenses.is_empty() {
            if let Some(corrections) = corrections {
                if let Some(replacement) = replacement_for_ternary(&if_node, source) {
                    let loc = if_node.location();
                    corrections.push(Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement,
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
            }
            diagnostics.extend(offenses);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedTernaryOperator, "cops/style/nested_ternary_operator");
    crate::cop_autocorrect_fixture_tests!(
        NestedTernaryOperator,
        "cops/style/nested_ternary_operator"
    );
}
