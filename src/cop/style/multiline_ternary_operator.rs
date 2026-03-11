use crate::cop::node_type::IF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for ternaries broken across operator boundaries.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=5, FN=0.
///
/// The false positives were ternaries whose `?` and `:` stayed inline, but one
/// branch expression contained a multiline hash or method call:
/// `condition ? { ...multiline... } : {}`. RuboCop does not treat those as
/// multiline ternary operators. The original Prism port compared the ternary
/// node's overall start and end lines, which overmatched any multiline child
/// expression.
///
/// Fix: only register an offense when the condition itself is multiline, the
/// true branch starts on a later line than the condition ends, or the false
/// branch starts on a later line than the true branch ends. That tracks
/// line breaks around the ternary operators instead of descendant formatting.
pub struct MultilineTernaryOperator;

impl Cop for MultilineTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/MultilineTernaryOperator"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must be a ternary (no if_keyword_loc)
        if if_node.if_keyword_loc().is_some() {
            return;
        }

        if !breaks_across_ternary_operators(source, &if_node) {
            return;
        }

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid multi-line ternary operators, use `if` or `unless` instead.".to_string(),
        ));
    }
}

fn breaks_across_ternary_operators(source: &SourceFile, if_node: &ruby_prism::IfNode<'_>) -> bool {
    let predicate = if_node.predicate().location();
    let (predicate_start_line, _) = source.offset_to_line_col(predicate.start_offset());
    let (predicate_end_line, _) =
        source.offset_to_line_col(predicate.end_offset().saturating_sub(1));

    if predicate_start_line != predicate_end_line {
        return true;
    }

    let if_branch = match if_node.statements() {
        Some(branch) => branch.location(),
        None => return false,
    };
    let (if_branch_start_line, _) = source.offset_to_line_col(if_branch.start_offset());
    if predicate_end_line != if_branch_start_line {
        return true;
    }

    let else_branch = match if_node.subsequent() {
        Some(node) => match node.as_else_node().and_then(|branch| branch.statements()) {
            Some(branch) => branch.location(),
            None => return false,
        },
        None => return false,
    };
    let (if_branch_end_line, _) =
        source.offset_to_line_col(if_branch.end_offset().saturating_sub(1));
    let (else_branch_start_line, _) = source.offset_to_line_col(else_branch.start_offset());
    if_branch_end_line != else_branch_start_line
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultilineTernaryOperator,
        "cops/style/multiline_ternary_operator"
    );
}
