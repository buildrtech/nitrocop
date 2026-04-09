use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TimesMap;

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

impl Cop for TimesMap {
    fn name(&self) -> &'static str {
        "Performance/TimesMap"
    }

    fn uses_node_check(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"times"
            || (chain.outer_method != b"map" && chain.outer_method != b"collect")
        {
            return;
        }

        // `times` must have a receiver (e.g. `n.times.map`). Bare `times.map`
        // means `times` is a local variable or method, not Integer#times.
        if chain.inner_call.receiver().is_none() {
            return;
        }

        // Integer#times takes no arguments. Other classes (e.g. Fabricate, Factory)
        // define .times(n, factory) which should not be flagged.
        if chain.inner_call.arguments().is_some() {
            return;
        }

        // RuboCop only flags `times.map` when map/collect has a block (either
        // `{ }` / `do..end` or a block_pass like `&method(:foo)`). Without a
        // block, `times.map` returns an Enumerator and is not an offense.
        let outer_call = node.as_call_node().unwrap();
        let Some(block) = outer_call.block() else {
            return;
        };

        // Skip safe-navigation (`&.`) chains.
        if outer_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
            || chain
                .inner_call
                .call_operator_loc()
                .is_some_and(|op| op.as_slice() == b"&.")
        {
            return;
        }

        let outer_name = std::str::from_utf8(chain.outer_method).unwrap_or("map");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `Array.new` with a block instead of `times.{outer_name}`."),
        );

        if let Some(ref mut corr) = corrections
            && let Some(count_receiver) = chain.inner_call.receiver()
        {
            let count_src = node_source(source, &count_receiver);
            let mut arg_parts = Vec::new();
            if let Some(args) = outer_call.arguments() {
                for arg in args.arguments().iter() {
                    arg_parts.push(node_source(source, &arg));
                }
            }

            let replacement = if block.as_block_argument_node().is_some() {
                arg_parts.push(node_source(source, &block));
                let tail = if arg_parts.is_empty() {
                    String::new()
                } else {
                    format!(", {}", arg_parts.join(", "))
                };
                format!("Array.new({count_src}{tail})")
            } else {
                let tail = if arg_parts.is_empty() {
                    String::new()
                } else {
                    format!(", {}", arg_parts.join(", "))
                };
                let block_src = node_source(source, &block);
                format!("Array.new({count_src}{tail}) {block_src}")
            };

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(TimesMap, "cops/performance/times_map");
    crate::cop_autocorrect_fixture_tests!(TimesMap, "cops/performance/times_map");

    #[test]
    fn supports_autocorrect() {
        assert!(TimesMap.supports_autocorrect());
    }
}
