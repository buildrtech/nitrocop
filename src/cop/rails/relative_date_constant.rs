use crate::cop::node_type::{CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RelativeDateConstant;

/// RuboCop's RELATIVE_DATE_METHODS: methods that produce relative times when
/// chained on a duration or date. These only evaluate once when assigned to
/// a constant, so the constant becomes stale.
const RELATIVE_DATE_METHODS: &[&[u8]] = &[
    b"since",
    b"from_now",
    b"after",
    b"ago",
    b"until",
    b"before",
    b"yesterday",
    b"tomorrow",
];

impl Cop for RelativeDateConstant {
    fn name(&self) -> &'static str {
        "Rails/RelativeDateConstant"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE]
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
        let value = if let Some(cw) = node.as_constant_write_node() {
            cw.value()
        } else if let Some(cpw) = node.as_constant_path_write_node() {
            cpw.value()
        } else {
            return;
        };

        // Check if the value contains a relative date/time call
        // RuboCop checks: `(send _ $RELATIVE_DATE_METHODS)` anywhere in the
        // value subtree, skipping block nodes.
        let mut finder = RelativeDateFinder { found: false };
        finder.visit(&value);

        if finder.found {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Do not assign relative dates to constants.".to_string(),
            );

            if let Some(cw) = node.as_constant_write_node()
                && let Some(ref mut corr) = corrections
            {
                let const_name = String::from_utf8_lossy(cw.name().as_slice()).to_lowercase();
                let value = cw.value();
                let value_loc = value.location();
                let value_src =
                    source.byte_slice(value_loc.start_offset(), value_loc.end_offset(), "");
                let indent = " ".repeat(column);
                let replacement =
                    format!("def self.{const_name}\n{indent}  {value_src}\n{indent}end");
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
}

struct RelativeDateFinder {
    found: bool,
}

impl<'a> Visit<'a> for RelativeDateFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'a>) {
        if self.found {
            return;
        }

        let method_name = node.name().as_slice();
        // Match any call to a relative date method on any receiver
        if RELATIVE_DATE_METHODS.contains(&method_name) && node.receiver().is_some() {
            self.found = true;
            return;
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }

    // Skip block nodes — RuboCop does `return if node.any_block_type?`
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'a>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'a>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RelativeDateConstant, "cops/rails/relative_date_constant");
    crate::cop_autocorrect_fixture_tests!(
        RelativeDateConstant,
        "cops/rails/relative_date_constant"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(RelativeDateConstant.supports_autocorrect());
    }
}
