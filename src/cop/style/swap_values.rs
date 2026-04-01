use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SwapValues;

impl Cop for SwapValues {
    fn name(&self) -> &'static str {
        "Style/SwapValues"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = SwapVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corrections) = _corrections {
            corrections.extend(visitor.corrections);
        }
    }
}

struct SwapVisitor<'a> {
    cop: &'a SwapValues,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
}

impl<'pr> Visit<'pr> for SwapVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let stmts: Vec<_> = node.body().iter().collect();

        let mut i = 0;
        while i + 2 < stmts.len() {
            // Pattern: tmp = a; a = b; b = tmp
            let Some(w1) = stmts[i].as_local_variable_write_node() else {
                i += 1;
                continue;
            };
            let Some(w2) = stmts[i + 1].as_local_variable_write_node() else {
                i += 1;
                continue;
            };
            let Some(w3) = stmts[i + 2].as_local_variable_write_node() else {
                i += 1;
                continue;
            };

            let tmp_name = w1.name().as_slice();
            let w1_value = w1.value();
            let a_value = get_lvar_name(&w1_value);
            let b_name = w2.name().as_slice();
            let w2_value = w2.value();
            let b_value = get_lvar_name(&w2_value);
            let c_name = w3.name().as_slice();
            let w3_value = w3.value();
            let c_value = get_lvar_name(&w3_value);

            let (Some(a_val), Some(b_val), Some(c_val)) = (a_value, b_value, c_value) else {
                i += 1;
                continue;
            };

            // Pattern: tmp = a; a = b; b = tmp
            // w1: tmp_name = a_val  (save a into tmp)
            // w2: b_name = b_val    (assign b's value to a)
            // w3: c_name = c_val    (restore tmp into b)
            // Conditions:
            //   b_name == a_val (second writes to the saved variable)
            //   c_name == b_val (third writes to the source variable)
            //   c_val == tmp_name (third reads from temp)
            //   b_val != tmp_name (second doesn't read temp)
            if b_name == a_val && c_name == b_val && c_val == tmp_name && b_val != tmp_name {
                let loc = stmts[i].location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Replace this swap with `{}, {} = {}, {}`.",
                        String::from_utf8_lossy(b_name),
                        String::from_utf8_lossy(c_name),
                        String::from_utf8_lossy(c_name),
                        String::from_utf8_lossy(b_name),
                    ),
                );

                let replacement = format!(
                    "{}, {} = {}, {}",
                    String::from_utf8_lossy(b_name),
                    String::from_utf8_lossy(c_name),
                    String::from_utf8_lossy(c_name),
                    String::from_utf8_lossy(b_name),
                );
                self.corrections.push(crate::correction::Correction {
                    start: w1.location().start_offset(),
                    end: w3.location().end_offset(),
                    replacement,
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
                self.diagnostics.push(diagnostic);

                i += 3;
                continue;
            }

            i += 1;
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

fn get_lvar_name<'a>(node: &'a ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(lv) = node.as_local_variable_read_node() {
        Some(lv.name().as_slice())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SwapValues, "cops/style/swap_values");
    crate::cop_autocorrect_fixture_tests!(SwapValues, "cops/style/swap_values");
}
