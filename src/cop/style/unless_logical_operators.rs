use crate::cop::node_type::{AND_NODE, OR_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnlessLogicalOperators;

impl Cop for UnlessLogicalOperators {
    fn name(&self) -> &'static str {
        "Style/UnlessLogicalOperators"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, OR_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut corrections = corrections;
        let enforced_style = config.get_str("EnforcedStyle", "forbid_mixed_logical_operators");

        let unless_node = match node.as_unless_node() {
            Some(u) => u,
            None => return,
        };

        let predicate = unless_node.predicate();

        match enforced_style {
            "forbid_logical_operators" => {
                // Flag any logical operators in unless conditions
                if contains_logical_operator(&predicate) {
                    let (line, column) =
                        source.offset_to_line_col(unless_node.keyword_loc().start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use logical operators in `unless` conditions.".to_string(),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
                        let loc = unless_node.location();
                        corrections.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: loc.end_offset(),
                            replacement: "nil".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                }
            }
            _ => {
                // Flag mixed logical operators (both && and ||)
                if contains_mixed_logical_operators(&predicate) {
                    let (line, column) =
                        source.offset_to_line_col(unless_node.keyword_loc().start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use mixed logical operators in `unless` conditions.".to_string(),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
                        let loc = unless_node.location();
                        corrections.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: loc.end_offset(),
                            replacement: "nil".to_string(),
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

fn contains_logical_operator(node: &ruby_prism::Node<'_>) -> bool {
    node.as_and_node().is_some() || node.as_or_node().is_some()
}

/// Check if the condition has mixed logical operators at the same structural level.
/// Matches RuboCop's `or_with_and?` and `and_with_or?` node patterns which only
/// check direct children, plus `mixed_precedence_and?`/`mixed_precedence_or?` which
/// check for mixing `&&` with `and` or `||` with `or`.
fn contains_mixed_logical_operators(node: &ruby_prism::Node<'_>) -> bool {
    or_with_and(node)
        || and_with_or(node)
        || mixed_precedence_and(node)
        || mixed_precedence_or(node)
}

/// An OR node whose direct left or right child is an AND node.
/// e.g. `a && b || c` parses as `(or (and a b) c)`.
fn or_with_and(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(or_node) = node.as_or_node() {
        let left = or_node.left();
        let right = or_node.right();
        if left.as_and_node().is_some() || right.as_and_node().is_some() {
            return true;
        }
        // Recurse into OR children that are also OR nodes (chained ||)
        or_with_and(&left) || or_with_and(&right)
    } else {
        false
    }
}

/// An AND node whose direct left or right child is an OR node.
/// e.g. `a || b && c` parses as `(and (or a b) c)`.
fn and_with_or(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(and_node) = node.as_and_node() {
        let left = and_node.left();
        let right = and_node.right();
        if left.as_or_node().is_some() || right.as_or_node().is_some() {
            return true;
        }
        // Recurse into AND children that are also AND nodes (chained &&)
        and_with_or(&left) || and_with_or(&right)
    } else {
        false
    }
}

/// Check for mixing `&&` with `and` operators.
fn mixed_precedence_and(node: &ruby_prism::Node<'_>) -> bool {
    let mut ops = Vec::new();
    collect_and_operators(node, &mut ops);
    if ops.len() < 2 {
        return false;
    }
    // Mixed if not all symbolic (&&) and not all keyword (and)
    !(ops.iter().all(|&s| s) || ops.iter().all(|&s| !s))
}

/// Check for mixing `||` with `or` operators.
fn mixed_precedence_or(node: &ruby_prism::Node<'_>) -> bool {
    let mut ops = Vec::new();
    collect_or_operators(node, &mut ops);
    if ops.len() < 2 {
        return false;
    }
    !(ops.iter().all(|&s| s) || ops.iter().all(|&s| !s))
}

fn collect_and_operators(node: &ruby_prism::Node<'_>, ops: &mut Vec<bool>) {
    if let Some(and_node) = node.as_and_node() {
        let is_symbolic = and_node.operator_loc().as_slice() == b"&&";
        ops.push(is_symbolic);
        collect_and_operators(&and_node.left(), ops);
        collect_and_operators(&and_node.right(), ops);
    }
}

fn collect_or_operators(node: &ruby_prism::Node<'_>, ops: &mut Vec<bool>) {
    if let Some(or_node) = node.as_or_node() {
        let is_symbolic = or_node.operator_loc().as_slice() == b"||";
        ops.push(is_symbolic);
        collect_or_operators(&or_node.left(), ops);
        collect_or_operators(&or_node.right(), ops);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        UnlessLogicalOperators,
        "cops/style/unless_logical_operators"
    );

    #[test]
    fn autocorrect_replaces_offending_unless_with_nil() {
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("forbid_logical_operators".into()),
            )]),
            ..CopConfig::default()
        };
        crate::testutil::assert_cop_autocorrect_with_config(
            &UnlessLogicalOperators,
            b"unless a && b\n  work\nend\n",
            b"nil\n",
            config,
        );
    }
}
