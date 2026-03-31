use crate::cop::node_type::BLOCK_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus conformance: 62.5% (5 matches, 0 FP, 3 FN).
///
/// All 3 FNs are caused by a RuboCop bug in `argument_tokens`: when a chained
/// block expression has an inner block with a single-param trailing comma
/// (`|name,|`) and an outer block with 2+ params (`|name, constant|`), RuboCop's
/// `tokens_within(node)` for the outer block includes the inner block's tokens.
/// The `pipes.select` then picks the first two `|` characters (from the inner
/// block), causing `trailing_comma?` to check the inner block's params instead
/// of the outer block's. Combined with `arg_count > 1` from the outer block,
/// this incorrectly flags the inner block's single-param trailing comma.
///
/// Affected corpus patterns (all identical root cause):
/// - `sort_by { |name,| name }.map do |name, constant|` (ffi)
/// - `.select { |k,| ... }.each do |k, v|` (openproject)
/// - `sort_by do |day,| day end.reverse_each do |day, entries|` (rdoc)
///
/// Isolated `|name,|` (single param) is correctly NOT flagged by RuboCop.
/// nitrocop is correct here; the FNs are RuboCop false positives.
pub struct TrailingCommaInBlockArgs;

impl Cop for TrailingCommaInBlockArgs {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInBlockArgs"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
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
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let params = match block.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        // Count the number of block parameters. A single-parameter block with
        // trailing comma (|a,|) is semantically meaningful — it destructures and
        // discards extra block arguments. Only flag when there are multiple params.
        // Prism represents the trailing comma as an ImplicitRestNode on the rest
        // field, so we must exclude it from the count.
        if let Some(inner_params) = block_params.parameters() {
            let has_explicit_rest = inner_params
                .rest()
                .is_some_and(|r| r.as_implicit_rest_node().is_none());
            let param_count = inner_params.requireds().iter().count()
                + inner_params.optionals().iter().count()
                + inner_params.posts().iter().count()
                + inner_params.keywords().iter().count()
                + usize::from(has_explicit_rest)
                + usize::from(inner_params.keyword_rest().is_some());
            if param_count <= 1 {
                return;
            }
        } else {
            return;
        }

        // Check the source for a trailing comma before |
        let close_loc = match block_params.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Look at bytes before the closing |
        let bytes = source.as_bytes();
        let close_offset = close_loc.start_offset();
        if close_offset == 0 {
            return;
        }

        // Scan backwards for trailing comma (skip whitespace)
        let mut pos = close_offset - 1;
        while pos > 0 && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n') {
            pos -= 1;
        }

        if bytes[pos] == b',' {
            let (line, column) = source.offset_to_line_col(pos);
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Useless trailing comma present in block arguments.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                let mut remove_end = pos + 1;
                while remove_end < close_offset
                    && (bytes[remove_end] == b' ' || bytes[remove_end] == b'\t')
                {
                    remove_end += 1;
                }
                corr.push(crate::correction::Correction {
                    start: pos,
                    end: remove_end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TrailingCommaInBlockArgs,
        "cops/style/trailing_comma_in_block_args"
    );
    crate::cop_autocorrect_fixture_tests!(
        TrailingCommaInBlockArgs,
        "cops/style/trailing_comma_in_block_args"
    );
}
