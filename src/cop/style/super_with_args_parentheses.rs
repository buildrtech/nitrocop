use crate::cop::node_type::SUPER_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN=1: Kamal calls `super &block` in a module override. Prism stores the
/// block pass on `SuperNode#block()` rather than in `arguments()`, so the cop
/// previously treated that form as zero-arity `super` and missed the offense.
pub struct SuperWithArgsParentheses;

impl Cop for SuperWithArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/SuperWithArgsParentheses"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[SUPER_NODE]
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
        let super_node = match node.as_super_node() {
            Some(s) => s,
            None => return,
        };

        // RuboCop also requires parentheses for block-pass-only forms like
        // `super &block`.
        if super_node.arguments().is_none() && super_node.block().is_none() {
            return;
        }

        // Check if parentheses are missing
        if super_node.lparen_loc().is_some() {
            return;
        }

        let loc = super_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Use parentheses for `super` with arguments.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let keyword_loc = super_node.keyword_loc();
            let arg_start = super_node
                .arguments()
                .map(|a| a.location().start_offset())
                .or_else(|| super_node.block().map(|b| b.location().start_offset()));
            let arg_end = super_node
                .block()
                .map(|b| b.location().end_offset())
                .or_else(|| super_node.arguments().map(|a| a.location().end_offset()));

            if let (Some(arg_start), Some(arg_end)) = (arg_start, arg_end) {
                corr.push(crate::correction::Correction {
                    start: keyword_loc.end_offset(),
                    end: arg_start,
                    replacement: "(".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                corr.push(crate::correction::Correction {
                    start: arg_end,
                    end: arg_end,
                    replacement: ")".to_string(),
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
    crate::cop_fixture_tests!(
        SuperWithArgsParentheses,
        "cops/style/super_with_args_parentheses"
    );
    crate::cop_autocorrect_fixture_tests!(
        SuperWithArgsParentheses,
        "cops/style/super_with_args_parentheses"
    );
}
