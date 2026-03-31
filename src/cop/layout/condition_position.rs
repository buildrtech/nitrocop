use crate::cop::node_type::{IF_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ConditionPosition;

fn push_condition_position_offense(
    cop: &dyn Cop,
    source: &SourceFile,
    keyword_end: usize,
    predicate_start: usize,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (pred_line, pred_col) = source.offset_to_line_col(predicate_start);
    let mut diagnostic = cop.diagnostic(source, pred_line, pred_col, message);
    if let Some(corrections) = corrections.as_mut() {
        corrections.push(Correction {
            start: keyword_end,
            end: predicate_start,
            replacement: " ".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diagnostic.corrected = true;
    }
    diagnostics.push(diagnostic);
}

impl Cop for ConditionPosition {
    fn name(&self) -> &'static str {
        "Layout/ConditionPosition"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let keyword = if kw_loc.as_slice() == b"if" {
                "if"
            } else if kw_loc.as_slice() == b"unless" {
                "unless"
            } else {
                // elsif — keyword_loc is "elsif"; end_keyword_loc is None
                // for elsif nodes (the end belongs to the outermost if)
                "elsif"
            };
            // Skip modifier form (postfix if/unless) — no `end` keyword and
            // not an elsif (which also lacks end_keyword_loc).
            if if_node.end_keyword_loc().is_none() && keyword != "elsif" {
                return;
            }
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = if_node.predicate();
            let (pred_line, _) = source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                push_condition_position_offense(
                    self,
                    source,
                    kw_loc.end_offset(),
                    predicate.location().start_offset(),
                    format!("Place the condition on the same line as `{keyword}`."),
                    diagnostics,
                    &mut corrections,
                );
            }
        } else if let Some(while_node) = node.as_while_node() {
            // Skip modifier form (postfix while) — no closing `end` keyword
            if while_node.closing_loc().is_none() {
                return;
            }
            let kw_loc = while_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = while_node.predicate();
            let (pred_line, _) = source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                push_condition_position_offense(
                    self,
                    source,
                    kw_loc.end_offset(),
                    predicate.location().start_offset(),
                    "Place the condition on the same line as `while`.".to_string(),
                    diagnostics,
                    &mut corrections,
                );
            }
        } else if let Some(until_node) = node.as_until_node() {
            // Skip modifier form (postfix until) — no closing `end` keyword
            if until_node.closing_loc().is_none() {
                return;
            }
            let kw_loc = until_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = until_node.predicate();
            let (pred_line, _) = source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                push_condition_position_offense(
                    self,
                    source,
                    kw_loc.end_offset(),
                    predicate.location().start_offset(),
                    "Place the condition on the same line as `until`.".to_string(),
                    diagnostics,
                    &mut corrections,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ConditionPosition, "cops/layout/condition_position");
    crate::cop_autocorrect_fixture_tests!(ConditionPosition, "cops/layout/condition_position");

    #[test]
    fn inline_if_no_offense() {
        let source = b"x = 1 if true\n";
        let diags = run_cop_full(&ConditionPosition, source);
        assert!(diags.is_empty());
    }
}
