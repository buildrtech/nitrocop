use crate::cop::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OneLineConditional;

impl Cop for OneLineConditional {
    fn name(&self) -> &'static str {
        "Style/OneLineConditional"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
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
        let _always_multiline = config.get_bool("AlwaysCorrectToMultiline", false);

        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            if kw_loc.as_slice() != b"if"
                || if_node.end_keyword_loc().is_none()
                || if_node.then_keyword_loc().is_none()
                || if_node.subsequent().is_none()
            {
                return;
            }

            let loc = if_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return;
            }

            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs."
                    .to_string(),
            );

            if let Some(corr) = corrections.as_mut() {
                if let (Some(if_expr), Some(else_expr)) = (
                    first_statement_source(source, if_node.statements()),
                    if_node
                        .subsequent()
                        .and_then(|s| s.as_else_node())
                        .and_then(|e| first_statement_source(source, e.statements())),
                ) {
                    let pred = if_node.predicate().location();
                    let predicate = source.byte_slice(pred.start_offset(), pred.end_offset(), "");
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: format!("{predicate} ? {if_expr} : {else_expr}"),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
            }

            diagnostics.push(diag);
        }

        if let Some(unless_node) = node.as_unless_node() {
            if unless_node.keyword_loc().as_slice() != b"unless"
                || unless_node.end_keyword_loc().is_none()
                || unless_node.then_keyword_loc().is_none()
                || unless_node.else_clause().is_none()
            {
                return;
            }

            let loc = unless_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return;
            }

            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Favor the ternary operator (`?:`) over single-line `unless/then/else/end` constructs."
                    .to_string(),
            );

            if let Some(corr) = corrections.as_mut() {
                if let (Some(if_expr), Some(else_expr)) = (
                    first_statement_source(source, unless_node.statements()),
                    unless_node
                        .else_clause()
                        .and_then(|e| first_statement_source(source, e.statements())),
                ) {
                    let pred = unless_node.predicate().location();
                    let predicate = source.byte_slice(pred.start_offset(), pred.end_offset(), "");
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: format!("{predicate} ? {else_expr} : {if_expr}"),
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

fn first_statement_source(
    source: &SourceFile,
    statements: Option<ruby_prism::StatementsNode<'_>>,
) -> Option<String> {
    let stmts = statements?;
    let mut iter = stmts.body().iter();
    let first = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    let loc = first.location();
    Some(
        source
            .byte_slice(loc.start_offset(), loc.end_offset(), "")
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OneLineConditional, "cops/style/one_line_conditional");
    crate::cop_autocorrect_fixture_tests!(OneLineConditional, "cops/style/one_line_conditional");
}
