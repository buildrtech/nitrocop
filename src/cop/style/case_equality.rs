use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE,
    REGULAR_EXPRESSION_NODE, SELF_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/CaseEquality: Avoid the case equality operator `===`.
///
/// Investigation: RuboCop unconditionally skips `===` when the receiver is a
/// non-module-name constant (ALL_CAPS like `NUMERIC_PATTERN`), regardless of
/// `AllowOnConstant`. Only PascalCase constants (e.g., `String`, `Integer`)
/// are subject to `AllowOnConstant`. This was causing 42 FPs across 27 repos
/// on patterns like `NUMERIC_PATTERN === timezone`.
///
/// Second fix: `receiver_constant_name()` was returning hardcoded "QualifiedPath"
/// for all `ConstantPathNode` receivers, which always passed `is_module_name()`.
/// Fixed to extract the actual last-segment name via `cp.name()`. This resolves
/// 21 FPs on patterns like `Constants::ATOM_UNSAFE === str` and `URI::HTTPS === @uri`
/// where the last segment is ALL_CAPS (not a module name).
pub struct CaseEquality;

impl CaseEquality {
    /// Extract the constant name from a receiver node (ConstantReadNode or ConstantPathNode).
    fn receiver_constant_name(node: &ruby_prism::Node<'_>) -> Option<String> {
        if let Some(c) = node.as_constant_read_node() {
            return Some(String::from_utf8_lossy(c.name().as_slice()).into_owned());
        }
        if let Some(cp) = node.as_constant_path_node() {
            // For qualified constants like Foo::Bar, extract the last segment name.
            // RuboCop checks the last segment: URI::HTTPS is ALL_CAPS (not a module name),
            // while URI::Generic is PascalCase (a module name).
            if let Some(name) = cp.name() {
                return Some(String::from_utf8_lossy(name.as_slice()).into_owned());
            }
            return None;
        }
        None
    }

    /// A "module name" constant has at least one lowercase ASCII letter (PascalCase).
    /// ALL_CAPS_CONSTANTS like NUMERIC_PATTERN are not module names.
    fn is_module_name(name: &str) -> bool {
        name.bytes().any(|b| b.is_ascii_lowercase())
    }
}

impl Cop for CaseEquality {
    fn name(&self) -> &'static str {
        "Style/CaseEquality"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            REGULAR_EXPRESSION_NODE,
            SELF_NODE,
        ]
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
        let allow_on_constant = config.get_bool("AllowOnConstant", false);
        let allow_on_self_class = config.get_bool("AllowOnSelfClass", false);

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call_node.name().as_slice() != b"===" {
            return;
        }

        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        // Skip regexp receivers (Performance/RegexpMatch handles those)
        if receiver.as_regular_expression_node().is_some()
            || receiver.as_interpolated_regular_expression_node().is_some()
        {
            return;
        }

        // RuboCop unconditionally skips constants that are not "module names"
        // (i.e., ALL_CAPS like NUMERIC_PATTERN). Only PascalCase constants
        // (like String, Integer) are subject to the AllowOnConstant setting.
        if let Some(const_name) = Self::receiver_constant_name(&receiver) {
            if !Self::is_module_name(&const_name) {
                return;
            }
            if allow_on_constant {
                return;
            }
        }

        // AllowOnSelfClass: self.class === something
        if allow_on_self_class {
            if let Some(recv_call) = receiver.as_call_node() {
                if recv_call.name().as_slice() == b"class" {
                    if let Some(inner_recv) = recv_call.receiver() {
                        if inner_recv.as_self_node().is_some() {
                            return;
                        }
                    }
                }
            }
        }

        let msg_loc = call_node
            .message_loc()
            .unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of the case equality operator `===`.".to_string(),
        );

        if let Some(args) = call_node.arguments() {
            if let Some(rhs) = args.arguments().iter().next() {
                let rhs_src = std::str::from_utf8(rhs.location().as_slice()).unwrap_or("");
                let lhs_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
                let replacement = if receiver.as_constant_read_node().is_some()
                    || receiver.as_constant_path_node().is_some()
                {
                    Some(format!("{rhs_src}.is_a?({lhs_src})"))
                } else if let Some(recv_call) = receiver.as_call_node() {
                    if recv_call.name().as_slice() == b"class"
                        && recv_call
                            .receiver()
                            .is_some_and(|inner| inner.as_self_node().is_some())
                    {
                        Some(format!("{rhs_src}.is_a?({lhs_src})"))
                    } else {
                        None
                    }
                } else if lhs_src.starts_with('(')
                    && lhs_src.ends_with(')')
                    && lhs_src.contains("..")
                {
                    Some(format!("{lhs_src}.include?({rhs_src})"))
                } else {
                    None
                };

                if let Some(replacement) = replacement {
                    if let Some(ref mut corr) = corrections {
                        let loc = call_node.location();
                        corr.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: loc.end_offset(),
                            replacement,
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }
            }
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseEquality, "cops/style/case_equality");
    crate::cop_autocorrect_fixture_tests!(CaseEquality, "cops/style/case_equality");
}
