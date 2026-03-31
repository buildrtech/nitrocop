use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantReturn;

struct ReturnOffense {
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    replacement: String,
}

impl Cop for RedundantReturn {
    fn name(&self) -> &'static str {
        "Style/RedundantReturn"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_multiple = config.get_bool("AllowMultipleReturnValues", false);
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        let mut offenses = Vec::new();
        check_terminal(source, &body, allow_multiple, &mut offenses);

        for offense in offenses {
            let mut diag = self.diagnostic(
                source,
                offense.line,
                offense.column,
                "Redundant `return` detected.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: offense.start,
                    end: offense.end,
                    replacement: offense.replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

/// Recursively check terminal positions for redundant `return` statements.
/// A terminal position is the last expression that would be implicitly returned.
fn check_terminal(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    allow_multiple: bool,
    offenses: &mut Vec<ReturnOffense>,
) {
    // StatementsNode: check the last statement
    if let Some(stmts) = node.as_statements_node() {
        if let Some(last) = stmts.body().last() {
            check_terminal(source, &last, allow_multiple, offenses);
        }
        return;
    }

    // ReturnNode: this is a redundant return in terminal position
    if let Some(ret_node) = node.as_return_node() {
        if allow_multiple {
            let arg_count = ret_node.arguments().map_or(0, |a| a.arguments().len());
            if arg_count > 1 {
                return;
            }
        }

        let replacement = return_replacement(source, &ret_node);
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        offenses.push(ReturnOffense {
            start: loc.start_offset(),
            end: loc.end_offset(),
            line,
            column,
            replacement,
        });
        return;
    }

    // IfNode: check terminal position in each branch
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        if let Some(subsequent) = if_node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                check_terminal(source, &elsif.as_node(), allow_multiple, offenses);
            } else if let Some(else_node) = subsequent.as_else_node()
                && let Some(stmts) = else_node.statements()
            {
                check_terminal_stmts(source, &stmts, allow_multiple, offenses);
            }
        }
        return;
    }

    // UnlessNode: check terminal position in each branch
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        if let Some(else_clause) = unless_node.else_clause()
            && let Some(stmts) = else_clause.statements()
        {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        return;
    }

    // CaseNode: check terminal position in each when/else branch
    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node()
                && let Some(stmts) = when_node.statements()
            {
                check_terminal_stmts(source, &stmts, allow_multiple, offenses);
            }
        }
        if let Some(else_clause) = case_node.else_clause()
            && let Some(stmts) = else_clause.statements()
        {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        return;
    }

    // BeginNode: check statements body and rescue clauses
    if let Some(begin_node) = node.as_begin_node() {
        // Check main body statements
        if let Some(stmts) = begin_node.statements() {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        // Check rescue clauses
        if let Some(rescue) = begin_node.rescue_clause() {
            check_rescue_terminal(source, &rescue, allow_multiple, offenses);
        }
        // Check else clause on begin/rescue/else
        if let Some(else_clause) = begin_node.else_clause()
            && let Some(stmts) = else_clause.statements()
        {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        return;
    }

    // RescueNode (implicit rescue on def body): check each rescue clause
    if let Some(rescue_node) = node.as_rescue_node() {
        // The rescue node's own statements
        if let Some(stmts) = rescue_node.statements() {
            check_terminal_stmts(source, &stmts, allow_multiple, offenses);
        }
        // Subsequent rescue clauses
        if let Some(subsequent) = rescue_node.subsequent() {
            check_rescue_terminal(source, &subsequent, allow_multiple, offenses);
        }
    }
}

fn return_replacement(source: &SourceFile, ret_node: &ruby_prism::ReturnNode<'_>) -> String {
    let Some(args) = ret_node.arguments() else {
        return "nil".to_string();
    };

    let args_vec: Vec<_> = args.arguments().iter().collect();
    if args_vec.is_empty() {
        return "nil".to_string();
    }

    let bytes = source.as_bytes();
    args_vec
        .iter()
        .map(|arg| {
            let loc = arg.location();
            String::from_utf8_lossy(&bytes[loc.start_offset()..loc.end_offset()]).to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Check the last statement in a StatementsNode as a terminal position.
fn check_terminal_stmts(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    allow_multiple: bool,
    offenses: &mut Vec<ReturnOffense>,
) {
    if let Some(last) = stmts.body().last() {
        check_terminal(source, &last, allow_multiple, offenses);
    }
}

/// Recursively check rescue clause chains for redundant returns.
fn check_rescue_terminal(
    source: &SourceFile,
    rescue: &ruby_prism::RescueNode<'_>,
    allow_multiple: bool,
    offenses: &mut Vec<ReturnOffense>,
) {
    if let Some(stmts) = rescue.statements() {
        check_terminal_stmts(source, &stmts, allow_multiple, offenses);
    }
    if let Some(subsequent) = rescue.subsequent() {
        check_rescue_terminal(source, &subsequent, allow_multiple, offenses);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(RedundantReturn, "cops/style/redundant_return");
    crate::cop_autocorrect_fixture_tests!(RedundantReturn, "cops/style/redundant_return");

    #[test]
    fn allow_multiple_return_values() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultipleReturnValues".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // `return x, y` should be allowed when AllowMultipleReturnValues is true
        let source = b"def foo\n  return x, y\nend\n";
        let diags = run_cop_full_with_config(&RedundantReturn, source, config);
        assert!(
            diags.is_empty(),
            "Should allow multiple return values when configured"
        );
    }

    #[test]
    fn disallow_multiple_return_values_by_default() {
        // `return x, y` should be flagged by default
        let source = b"def foo\n  return x, y\nend\n";
        let diags = run_cop_full(&RedundantReturn, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multiple return values by default"
        );
    }

    #[test]
    fn allow_multiple_still_flags_single_return() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultipleReturnValues".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // `return x` should still be flagged even with AllowMultipleReturnValues
        let source = b"def foo\n  return x\nend\n";
        let diags = run_cop_full_with_config(&RedundantReturn, source, config);
        assert_eq!(diags.len(), 1, "Single return should still be flagged");
    }
}
