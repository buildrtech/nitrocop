use crate::cop::node_type::{ARRAY_NODE, SPLAT_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayCoercion;

impl Cop for ArrayCoercion {
    fn name(&self) -> &'static str {
        "Style/ArrayCoercion"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, SPLAT_NODE]
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
        // Pattern 1: [*var] - splat into array with single element
        if let Some(array_node) = node.as_array_node() {
            // Skip implicit arrays (e.g., RHS of multi-write `a, b = *x`)
            if array_node.opening_loc().is_none() {
                return;
            }
            let elements: Vec<_> = array_node.elements().iter().collect();
            if elements.len() == 1 {
                if let Some(splat) = elements[0].as_splat_node() {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `Array(variable)` instead of `[*variable]`.".to_string(),
                    );

                    if let Some(ref mut corr) = corrections {
                        if let Some(expr) = splat.expression() {
                            let expr_src =
                                std::str::from_utf8(expr.location().as_slice()).unwrap_or("");
                            if !expr_src.is_empty() {
                                corr.push(crate::correction::Correction {
                                    start: loc.start_offset(),
                                    end: loc.end_offset(),
                                    replacement: format!("Array({expr_src})"),
                                    cop_name: self.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }
                        }
                    }

                    diagnostics.push(diag);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayCoercion, "cops/style/array_coercion");
    crate::cop_autocorrect_fixture_tests!(ArrayCoercion, "cops/style/array_coercion");
}
