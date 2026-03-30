use crate::cop::node_type::{ARRAY_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ConcatArrayLiterals;

impl Cop for ConcatArrayLiterals {
    fn name(&self) -> &'static str {
        "Style/ConcatArrayLiterals"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "concat" {
            return;
        }

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // All arguments must be array literals
        let all_arrays = arg_list.iter().all(|arg| arg.as_array_node().is_some());
        if !all_arrays {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let msg = "Use `push` with elements as arguments instead of `concat` with array brackets.";
        let mut diag = self.diagnostic(source, line, column, msg.to_string());

        if let Some(ref mut corr) = corrections {
            let receiver_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
            let operator = call
                .call_operator_loc()
                .and_then(|op| std::str::from_utf8(op.as_slice()).ok())
                .unwrap_or(".");

            let mut elements = Vec::new();
            for arg in arg_list {
                let Some(array_node) = arg.as_array_node() else {
                    return;
                };
                for element in array_node.elements().iter() {
                    let Ok(elem_src) = std::str::from_utf8(element.location().as_slice()) else {
                        return;
                    };
                    elements.push(elem_src.to_string());
                }
            }

            if !receiver_src.is_empty() {
                let replacement = format!("{receiver_src}{operator}push({})", elements.join(", "));
                let call_loc = call.location();
                corr.push(crate::correction::Correction {
                    start: call_loc.start_offset(),
                    end: call_loc.end_offset(),
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConcatArrayLiterals, "cops/style/concat_array_literals");
    crate::cop_autocorrect_fixture_tests!(ConcatArrayLiterals, "cops/style/concat_array_literals");
}
