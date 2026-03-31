use crate::cop::node_type::{BEGIN_NODE, DEF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingBodyOnMethodDefinition;

fn find_body_separator(bytes: &[u8], header_start: usize, body_start: usize) -> Option<usize> {
    if body_start > bytes.len() || header_start >= body_start {
        return None;
    }
    (header_start..body_start).find(|&i| bytes[i] == b';')
}

impl Cop for TrailingBodyOnMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnMethodDefinition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, DEF_NODE]
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
        if let Some(def_node) = node.as_def_node() {
            // Skip endless methods (def foo = ...)
            if def_node.equal_loc().is_some() {
                return;
            }

            let body = match def_node.body() {
                Some(b) => b,
                None => return,
            };

            // Method must be multiline (def on different line than end)
            let def_loc = def_node.def_keyword_loc();
            let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
            let end_loc = match def_node.end_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());
            if def_line == end_line {
                return;
            }

            // When body is a BeginNode (implicit begin wrapping rescue/ensure),
            // look at the first statement inside, not the BeginNode itself
            // (whose location may start on the def line in Prism).
            let (body_line, body_column) = if let Some(begin_node) = body.as_begin_node() {
                if let Some(stmts) = begin_node.statements() {
                    let stmts_body = stmts.body();
                    if let Some(first_stmt) = stmts_body.iter().next() {
                        let loc = first_stmt.location();
                        source.offset_to_line_col(loc.start_offset())
                    } else {
                        // No statements in begin body — check rescue clause
                        if let Some(rescue) = begin_node.rescue_clause() {
                            let loc = rescue.location();
                            source.offset_to_line_col(loc.start_offset())
                        } else {
                            return;
                        }
                    }
                } else {
                    // No statements — check rescue clause location
                    if let Some(rescue) = begin_node.rescue_clause() {
                        let loc = rescue.location();
                        source.offset_to_line_col(loc.start_offset())
                    } else {
                        return;
                    }
                }
            } else {
                let body_loc = body.location();
                source.offset_to_line_col(body_loc.start_offset())
            };

            if def_line == body_line {
                let mut diag = self.diagnostic(
                    source,
                    body_line,
                    body_column,
                    "Place the first line of a multi-line method definition's body on its own line."
                        .to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    let body_start = body.location().start_offset();
                    if let Some(semi_offset) =
                        find_body_separator(source.as_bytes(), def_loc.end_offset(), body_start)
                    {
                        let indent = " ".repeat(source.offset_to_line_col(def_loc.start_offset()).1 + 2);
                        corr.push(crate::correction::Correction {
                            start: semi_offset,
                            end: body_start,
                            replacement: format!("\n{indent}"),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
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
        TrailingBodyOnMethodDefinition,
        "cops/style/trailing_body_on_method_definition"
    );
    crate::cop_autocorrect_fixture_tests!(
        TrailingBodyOnMethodDefinition,
        "cops/style/trailing_body_on_method_definition"
    );
}
