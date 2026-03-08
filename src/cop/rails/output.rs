use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// FP=312, FN=59. Biggest FP source: `p { "..." }` in Phlex/Markaby views where
/// `p` is an HTML `<p>` tag builder, not Kernel#p. RuboCop skips calls with blocks
/// (`node.block_node`), block_pass args, and hash args. Fixed by adding block and
/// argument type checks.
pub struct Output;

const OUTPUT_METHODS: &[&[u8]] = &[b"puts", b"print", b"p", b"pp"];

impl Cop for Output {
    fn name(&self) -> &'static str {
        "Rails/Output"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &[
            "**/app/**/*.rb",
            "**/config/**/*.rb",
            "db/**/*.rb",
            "**/lib/**/*.rb",
        ]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        if !OUTPUT_METHODS.contains(&name) {
            return;
        }

        // RuboCop: skip if call has a block (e.g. `p { "HTML" }` in Phlex views)
        // or a block_pass argument (e.g. `p(&:to_s)`)
        if call.block().is_some() {
            return;
        }

        // RuboCop: skip if any argument is a hash or block_pass
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not write to stdout. Use Rails logger instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Output, "cops/rails/output");
}
