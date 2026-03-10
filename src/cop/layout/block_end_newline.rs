use crate::cop::node_type::{BLOCK_NODE, LAMBDA_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=15.
///
/// FN=15: Multiline lambdas like `-> { ... }` and `-> do ... end` were missed
/// because Prism exposes them as `LAMBDA_NODE`, while the cop only visited
/// `BLOCK_NODE`. The fix widens the visitor set to include lambdas and keeps
/// RuboCop's `; end` / `; }` escape so newly-covered lambda bodies do not
/// regress into false positives.
pub struct BlockEndNewline;

impl Cop for BlockEndNewline {
    fn name(&self) -> &'static str {
        "Layout/BlockEndNewline"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, LAMBDA_NODE]
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
        let (opening_loc, closing_loc) = if let Some(block_node) = node.as_block_node() {
            (block_node.opening_loc(), block_node.closing_loc())
        } else if let Some(lambda_node) = node.as_lambda_node() {
            (lambda_node.opening_loc(), lambda_node.closing_loc())
        } else {
            return;
        };

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Single line block — no offense
        if open_line == close_line {
            return;
        }

        // Check if `end` or `}` begins its line (only whitespace before it)
        let bytes = source.as_bytes();
        let mut pos = closing_loc.start_offset();
        while pos > 0 && bytes[pos - 1] != b'\n' {
            pos -= 1;
        }

        // Check if everything from line start to closing is whitespace
        let before_close = &bytes[pos..closing_loc.start_offset()];
        let begins_line = before_close.iter().all(|&b| b == b' ' || b == b'\t');

        if begins_line || begins_with_semicolon(before_close) {
            return;
        }

        diagnostics.push(self.diagnostic(
            source,
            close_line,
            close_col,
            format!(
                "Expression at {}, {} should be on its own line.",
                close_line,
                close_col + 1
            ),
        ));
    }
}

fn begins_with_semicolon(before_close: &[u8]) -> bool {
    before_close
        .iter()
        .find(|&&b| b != b' ' && b != b'\t')
        == Some(&b';')
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BlockEndNewline, "cops/layout/block_end_newline");
}
