use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MapCompact;

impl Cop for MapCompact {
    fn name(&self) -> &'static str {
        "Performance/MapCompact"
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

        if chain.outer_method != b"compact" {
            return;
        }

        let inner = chain.inner_method;
        if inner != b"map" && inner != b"collect" {
            return;
        }

        // RuboCop's pattern only matches map/collect with NO method arguments.
        // e.g. `Parallel.map(items) { ... }.compact` should NOT be flagged
        // because map has an explicit argument — it's not Enumerable#map.
        if chain.inner_call.arguments().is_some() {
            return;
        }

        // The inner call should have a block (either { } / do..end or &:symbol)
        let block = match chain.inner_call.block() {
            Some(b) => b,
            None => return,
        };

        // RuboCop's pattern matches (block ... (args ...) _) which excludes numblock/itblock.
        // In Prism, numbered-parameter and it-parameter blocks use special parameter nodes.
        if let Some(block_node) = block.as_block_node() {
            if let Some(params) = block_node.parameters() {
                if params.as_numbered_parameters_node().is_some()
                    || params.as_it_parameters_node().is_some()
                {
                    return;
                }
            }
        }

        // RuboCop's pattern matches (block_pass (sym _)) — only &:symbol.
        // Skip &method(:foo), &variable, etc.
        if let Some(block_arg) = block.as_block_argument_node() {
            if let Some(expr) = block_arg.expression() {
                if expr.as_symbol_node().is_none() {
                    return;
                }
            }
        }

        // Report at the inner method selector (map/collect), matching RuboCop
        let msg_loc = match chain.inner_call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `filter_map` instead.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: msg_loc.start_offset(),
                end: msg_loc.end_offset(),
                replacement: "filter_map".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });

            let outer_call = node.as_call_node().unwrap();
            if let Some(dot_loc) = outer_call.call_operator_loc()
                && let Some(outer_msg_loc) = outer_call.message_loc()
            {
                corr.push(crate::correction::Correction {
                    start: dot_loc.start_offset(),
                    end: outer_msg_loc.end_offset(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }

            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapCompact, "cops/performance/map_compact");
    crate::cop_autocorrect_fixture_tests!(MapCompact, "cops/performance/map_compact");

    #[test]
    fn supports_autocorrect() {
        assert!(MapCompact.supports_autocorrect());
    }
}
