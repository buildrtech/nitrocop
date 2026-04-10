use ruby_prism::Visit;

use crate::cop::node_type::{BEGIN_NODE, RETURN_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnsureReturn;

struct ReturnInfo {
    start_offset: usize,
    has_args: bool,
}

struct ReturnFinder {
    found: Vec<ReturnInfo>,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if let Some(ret) = node.as_return_node() {
            self.found.push(ReturnInfo {
                start_offset: ret.location().start_offset(),
                has_args: ret.arguments().is_some(),
            });
        }
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if let Some(ret) = node.as_return_node() {
            self.found.push(ReturnInfo {
                start_offset: ret.location().start_offset(),
                has_args: ret.arguments().is_some(),
            });
        }
    }
}

impl Cop for EnsureReturn {
    fn name(&self) -> &'static str {
        "Lint/EnsureReturn"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, RETURN_NODE]
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
        // EnsureNode is visited via visit_begin_node's specific method,
        // not via the generic visit() dispatch. So we match BeginNode
        // and check its ensure_clause.
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let ensure_node = match begin_node.ensure_clause() {
            Some(n) => n,
            None => return,
        };

        let statements = match ensure_node.statements() {
            Some(s) => s,
            None => return,
        };

        let mut finder = ReturnFinder { found: vec![] };
        for stmt in statements.body().iter() {
            finder.visit(&stmt);
        }

        for ret in finder.found {
            let (line, column) = source.offset_to_line_col(ret.start_offset);
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Do not return from an `ensure` block.".to_string(),
            );

            if ret.has_args
                && let Some(ref mut corrs) = corrections
            {
                let mut end = ret.start_offset.saturating_add(6); // "return"
                if source
                    .as_bytes()
                    .get(end)
                    .is_some_and(|b| b.is_ascii_whitespace())
                {
                    end = end.saturating_add(1);
                }
                corrs.push(crate::correction::Correction {
                    start: ret.start_offset,
                    end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnsureReturn, "cops/lint/ensure_return");
    crate::cop_autocorrect_fixture_tests!(EnsureReturn, "cops/lint/ensure_return");
}
