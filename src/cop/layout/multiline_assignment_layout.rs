use crate::cop::node_type::{
    BEGIN_NODE, BLOCK_NODE, CASE_MATCH_NODE, CASE_NODE, CLASS_NODE, CLASS_VARIABLE_OR_WRITE_NODE,
    CLASS_VARIABLE_WRITE_NODE, CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE,
    CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_OR_WRITE_NODE,
    GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE,
    INSTANCE_VARIABLE_WRITE_NODE, LAMBDA_NODE, LOCAL_VARIABLE_OR_WRITE_NODE,
    LOCAL_VARIABLE_WRITE_NODE, MODULE_NODE, UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=13, FN=39,806.
///
/// FP=13: not investigated in this pass.
///
/// FN root causes investigated:
/// - Prism represents `||=` as `*_or_write` nodes. The cop only subscribed to
///   plain `*_write` nodes, so memoized multiline assignments like
///   `memoized ||= begin ... end` were skipped entirely.
/// - Prism keeps `do/end` and `{}` blocks on `CallNode` itself. The cop only
///   treated `BlockNode`/`LambdaNode` as supported `block` RHS values, so
///   assignments like `result = fetch_records do ... end` were missed.
///
/// Fix applied:
/// - Added `*_or_write` and constant-path assignment node handling.
/// - Treat block-bearing `CallNode` values as supported `block` assignments.
///
/// Remaining likely gaps:
/// - Setter/index assignment shapes (`foo.bar =`, `hash[:key] =`) still are not
///   modeled here.
/// - Multi-assignment (`masgn`) support from RuboCop is still absent.
pub struct MultilineAssignmentLayout;

/// Check if a node represents one of the supported types for this cop.
fn is_supported_type(node: &ruby_prism::Node<'_>, supported_types: &[String]) -> bool {
    for t in supported_types {
        let matches = match t.as_str() {
            "if" => node.as_if_node().is_some() || node.as_unless_node().is_some(),
            "case" => node.as_case_node().is_some() || node.as_case_match_node().is_some(),
            "class" => node.as_class_node().is_some(),
            "module" => node.as_module_node().is_some(),
            "kwbegin" => node.as_begin_node().is_some(),
            "block" => {
                node.as_block_node().is_some()
                    || node.as_lambda_node().is_some()
                    || node
                        .as_call_node()
                        .is_some_and(|call| call.block().is_some())
            }
            _ => false,
        };
        if matches {
            return true;
        }
    }
    false
}

/// Find the assignment operator byte offset by scanning backwards from the RHS.
/// This catches both `=` and `||=` forms while preferring the last operator
/// before the value start.
fn find_eq_offset(
    source: &SourceFile,
    assignment_start: usize,
    value_start: usize,
) -> Option<usize> {
    let bytes = source.as_bytes();
    let end = value_start.min(bytes.len());
    for i in (assignment_start..end).rev() {
        if bytes[i] != b'=' {
            continue;
        }

        // Skip comparison operators like `==`/`===` by ignoring both sides.
        if i + 1 < end && bytes[i + 1] == b'=' {
            continue;
        }
        if i > assignment_start && bytes[i - 1] == b'=' {
            continue;
        }

        return Some(i);
    }
    None
}

fn assignment_start_and_value<'a>(
    node: &'a ruby_prism::Node<'a>,
) -> Option<(usize, ruby_prism::Node<'a>)> {
    if let Some(asgn) = node.as_local_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_instance_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_path_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_class_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_global_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_local_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_instance_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_path_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_class_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else {
        node.as_global_variable_or_write_node()
            .map(|asgn| (asgn.location().start_offset(), asgn.value()))
    }
}

impl Cop for MultilineAssignmentLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineAssignmentLayout"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BEGIN_NODE,
            BLOCK_NODE,
            CASE_MATCH_NODE,
            CASE_NODE,
            CLASS_NODE,
            CLASS_VARIABLE_OR_WRITE_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LAMBDA_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            MODULE_NODE,
            UNLESS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "new_line");
        let supported_types = config
            .get_string_array("SupportedTypes")
            .unwrap_or_else(|| {
                vec![
                    "block".to_string(),
                    "case".to_string(),
                    "class".to_string(),
                    "if".to_string(),
                    "kwbegin".to_string(),
                    "module".to_string(),
                ]
            });

        let (assignment_start, value) = match assignment_start_and_value(node) {
            Some(parts) => parts,
            None => return,
        };

        if !is_supported_type(&value, &supported_types) {
            return;
        }

        let (value_start_line, _) = source.offset_to_line_col(value.location().start_offset());
        let (value_end_line, _) =
            source.offset_to_line_col(value.location().end_offset().saturating_sub(1));

        // Only check multi-line RHS
        if value_start_line == value_end_line {
            return;
        }

        let eq_offset =
            match find_eq_offset(source, assignment_start, value.location().start_offset()) {
                Some(o) => o,
                None => return,
            };

        let (eq_line, _) = source.offset_to_line_col(eq_offset);
        let same_line = eq_line == value_start_line;
        let (node_line, node_col) = source.offset_to_line_col(node.location().start_offset());

        match enforced_style {
            "new_line" => {
                if same_line {
                    let mut diagnostic = self.diagnostic(
                        source,
                        node_line,
                        node_col,
                        "Right hand side of multi-line assignment is on the same line as the assignment operator `=`.".to_string(),
                    );

                    if let Some(corrections) = corrections.as_deref_mut() {
                        let indent = leading_indent(source, eq_line).unwrap_or(0)
                            + config.get_usize("IndentationWidth", 2);
                        corrections.push(Correction {
                            start: eq_offset + 1,
                            end: value.location().start_offset(),
                            replacement: format!("\n{}", " ".repeat(indent)),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }

                    diagnostics.push(diagnostic);
                }
            }
            "same_line" => {
                if !same_line {
                    diagnostics.push(self.diagnostic(
                        source,
                        node_line,
                        node_col,
                        "Right hand side of multi-line assignment is not on the same line as the assignment operator `=`.".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

fn leading_indent(source: &SourceFile, line: usize) -> Option<usize> {
    let lines: Vec<&[u8]> = source.lines().collect();
    let raw = *lines.get(line.checked_sub(1)?)?;
    Some(raw.iter().take_while(|&&b| b == b' ' || b == b'\t').count())
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineAssignmentLayout,
        "cops/layout/multiline_assignment_layout"
    );
    crate::cop_autocorrect_fixture_tests!(
        MultilineAssignmentLayout,
        "cops/layout/multiline_assignment_layout"
    );
}
