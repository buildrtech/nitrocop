/// Checks the indentation of the first line of the right-hand-side of a multi-line assignment.
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause (30 FPs):** The cop was using line indentation (`indentation_of`) as the
/// base for expected RHS column, but RuboCop uses the column position of the assignment
/// variable itself. For embedded assignments like `if upload = \n Upload.find_by(...)`,
/// the line indentation includes the `if` keyword, making the base too small and falsely
/// flagging properly-indented RHS. Fixed by using `name_col` (variable column) as the base.
///
/// **FN root cause (94 FNs):** The cop only handled simple write nodes (`*WriteNode`) but
/// missed operator assignments (`+=`, `-=` via `*OperatorWriteNode`), or-assignments
/// (`||=` via `*OrWriteNode`), and-assignments (`&&=` via `*AndWriteNode`),
/// multi-assignments (`a, b = ...` via `MultiWriteNode`), constant path writes
/// (`Module::CONST = ...` via `ConstantPathWriteNode`), setter calls (`obj.x = val`,
/// `hash[key] = val` via `CallNode`), and compound setter/index assignments
/// (`obj.x ||= val`, `hash[key] += val` via `Call*WriteNode`/`Index*WriteNode`). All added.
use crate::cop::node_type::{
    CALL_AND_WRITE_NODE, CALL_NODE, CALL_OPERATOR_WRITE_NODE, CALL_OR_WRITE_NODE,
    CLASS_VARIABLE_AND_WRITE_NODE, CLASS_VARIABLE_OPERATOR_WRITE_NODE,
    CLASS_VARIABLE_OR_WRITE_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_AND_WRITE_NODE,
    CONSTANT_OPERATOR_WRITE_NODE, CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_AND_WRITE_NODE,
    CONSTANT_PATH_OPERATOR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE, CONSTANT_PATH_WRITE_NODE,
    CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_AND_WRITE_NODE, GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
    GLOBAL_VARIABLE_OR_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, INDEX_AND_WRITE_NODE,
    INDEX_OPERATOR_WRITE_NODE, INDEX_OR_WRITE_NODE, INSTANCE_VARIABLE_AND_WRITE_NODE,
    INSTANCE_VARIABLE_OPERATOR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE,
    INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_AND_WRITE_NODE,
    LOCAL_VARIABLE_OPERATOR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE,
    MULTI_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AssignmentIndentation;

impl AssignmentIndentation {
    fn check_write(
        &self,
        source: &SourceFile,
        name_offset: usize,
        value: &ruby_prism::Node<'_>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let (name_line, name_col) = source.offset_to_line_col(name_offset);
        let value_loc = value.location();
        let (value_line, value_col) = source.offset_to_line_col(value_loc.start_offset());

        // Only check when RHS is on a different line
        if value_line == name_line {
            return Vec::new();
        }

        // Use the column of the assignment variable as the base, not the line indentation.
        // This correctly handles embedded assignments like `if x = \n value` where the
        // line indentation includes the `if` keyword but the expected indent is relative
        // to the variable name position.
        let expected = name_col + width;

        if value_col != expected {
            return vec![
                self.diagnostic(
                    source,
                    value_line,
                    value_col,
                    "Indent the first line of the right-hand-side of a multi-line assignment."
                        .to_string(),
                ),
            ];
        }

        Vec::new()
    }
}

impl Cop for AssignmentIndentation {
    fn name(&self) -> &'static str {
        "Layout/AssignmentIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            // Simple writes
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            // Operator writes (+=, -=, etc.)
            CLASS_VARIABLE_OPERATOR_WRITE_NODE,
            CONSTANT_OPERATOR_WRITE_NODE,
            GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
            INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
            LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
            // Or writes (||=)
            CLASS_VARIABLE_OR_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            // And writes (&&=)
            CLASS_VARIABLE_AND_WRITE_NODE,
            CONSTANT_AND_WRITE_NODE,
            GLOBAL_VARIABLE_AND_WRITE_NODE,
            INSTANCE_VARIABLE_AND_WRITE_NODE,
            LOCAL_VARIABLE_AND_WRITE_NODE,
            // Multi-write (a, b = ...)
            MULTI_WRITE_NODE,
            // Constant path writes (Module::CONST = ...)
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_PATH_AND_WRITE_NODE,
            CONSTANT_PATH_OPERATOR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            // Setter calls (obj.x = val, hash[key] = val)
            CALL_NODE,
            CALL_AND_WRITE_NODE,
            CALL_OPERATOR_WRITE_NODE,
            CALL_OR_WRITE_NODE,
            // Index compound writes (hash[key] ||= val, etc.)
            INDEX_AND_WRITE_NODE,
            INDEX_OPERATOR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let width = config.get_usize("IndentationWidth", 2);

        // Simple writes
        if let Some(n) = node.as_local_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Operator writes (+=, -=, *=, etc.)
        if let Some(n) = node.as_local_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Or writes (||=)
        if let Some(n) = node.as_local_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // And writes (&&=)
        if let Some(n) = node.as_local_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_instance_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_class_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_global_variable_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.name_loc().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Multi-write (a, b = ...)
        if let Some(n) = node.as_multi_write_node() {
            // Use the start of the whole multi-write node (first target) as the base
            diagnostics.extend(self.check_write(
                source,
                n.location().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Constant path writes (Module::CONST = ...)
        if let Some(n) = node.as_constant_path_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_path_operator_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_path_or_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                &n.value(),
                width,
            ));
        }

        if let Some(n) = node.as_constant_path_and_write_node() {
            diagnostics.extend(self.check_write(
                source,
                n.target().location().start_offset(),
                &n.value(),
                width,
            ));
        }

        // Setter calls (obj.x = val, hash[key] = val)
        if let Some(n) = node.as_call_node() {
            let name = n.name();
            let name_bytes = name.as_slice();
            // Only handle setter methods: name ends with '=' but is not ==, !=, ===, <=>, >=, <=
            if name_bytes.ends_with(b"=")
                && name_bytes != b"=="
                && name_bytes != b"!="
                && name_bytes != b"==="
                && name_bytes != b"<=>"
                && name_bytes != b">="
                && name_bytes != b"<="
            {
                if let Some(args) = n.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if let Some(last_arg) = arg_list.last() {
                        // Base is the receiver (or node start if no receiver)
                        let base_offset = if let Some(recv) = n.receiver() {
                            recv.location().start_offset()
                        } else {
                            n.location().start_offset()
                        };
                        diagnostics.extend(self.check_write(source, base_offset, last_arg, width));
                    }
                }
            }
        }

        // Call compound writes (obj.x ||= val, obj.x &&= val, obj.x += val)
        if let Some(n) = node.as_call_or_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(source, base_offset, &n.value(), width));
        }

        if let Some(n) = node.as_call_and_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(source, base_offset, &n.value(), width));
        }

        if let Some(n) = node.as_call_operator_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(source, base_offset, &n.value(), width));
        }

        // Index compound writes (hash[key] ||= val, hash[key] &&= val, hash[key] += val)
        if let Some(n) = node.as_index_or_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(source, base_offset, &n.value(), width));
        }

        if let Some(n) = node.as_index_and_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(source, base_offset, &n.value(), width));
        }

        if let Some(n) = node.as_index_operator_write_node() {
            let base_offset = if let Some(recv) = n.receiver() {
                recv.location().start_offset()
            } else {
                n.location().start_offset()
            };
            diagnostics.extend(self.check_write(source, base_offset, &n.value(), width));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(AssignmentIndentation, "cops/layout/assignment_indentation");

    #[test]
    fn single_line_assignment_ignored() {
        let source = b"x = 1\n";
        let diags = run_cop_full(&AssignmentIndentation, source);
        assert!(diags.is_empty());
    }
}
