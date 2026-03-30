use crate::cop::node_type::{ARRAY_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayIntersectWithSingleElement;

impl Cop for ArrayIntersectWithSingleElement {
    fn name(&self) -> &'static str {
        "Style/ArrayIntersectWithSingleElement"
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
        if method_name != "intersect?" {
            return;
        }

        if call.receiver().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Check if the argument is a single-element array literal
        if let Some(array_node) = arg_list[0].as_array_node() {
            let elements: Vec<_> = array_node.elements().iter().collect();
            if elements.len() == 1 {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `include?(element)` instead of `intersect?([element])`.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: "include?".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });

                    let elem_src = std::str::from_utf8(elements[0].location().as_slice())
                        .unwrap_or("")
                        .to_string();
                    if !elem_src.is_empty() {
                        let arg_loc = array_node.location();
                        corr.push(crate::correction::Correction {
                            start: arg_loc.start_offset(),
                            end: arg_loc.end_offset(),
                            replacement: elem_src,
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ArrayIntersectWithSingleElement,
        "cops/style/array_intersect_with_single_element"
    );
    crate::cop_autocorrect_fixture_tests!(
        ArrayIntersectWithSingleElement,
        "cops/style/array_intersect_with_single_element"
    );
}
