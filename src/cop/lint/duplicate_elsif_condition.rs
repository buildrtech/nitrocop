use std::collections::HashSet;

use crate::cop::node_type::IF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicateElsifCondition;

fn predicate_autocorrect_safe(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
}

impl Cop for DuplicateElsifCondition {
    fn name(&self) -> &'static str {
        "Lint/DuplicateElsifCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Only process top-level if (not elsif nodes visited separately)
        // The walker visits all IfNodes including elsif ones, so we need
        // to check this is a top-level if by checking there's an if_keyword
        let kw_loc = if_node.if_keyword_loc();
        if kw_loc.is_none() {
            return;
        }
        let kw_slice = kw_loc.unwrap().as_slice();
        if kw_slice != b"if" && kw_slice != b"unless" {
            return;
        }

        let mut seen = HashSet::new();

        // Add the first condition
        let first_cond = if_node.predicate().location().as_slice().to_vec();
        seen.insert(first_cond);

        // Walk elsif chain
        let mut subsequent = if_node.subsequent();
        while let Some(sub) = subsequent {
            if let Some(elsif) = sub.as_if_node() {
                let cond_text = elsif.predicate().location().as_slice().to_vec();
                if !seen.insert(cond_text) {
                    let loc = elsif.predicate().location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicate `elsif` condition detected.".to_string(),
                    );

                    // Conservative autocorrect: only remove a duplicate elsif when
                    // it is the final elsif branch (no trailing elsif/else) and the
                    // predicate is a side-effect-free variable/constant/literal read.
                    if let Some(corrections) = corrections.as_deref_mut() {
                        if elsif.subsequent().is_none() && predicate_autocorrect_safe(&elsif.predicate()) {
                            let mut start = elsif.location().start_offset();
                            if start > 0 && source.as_bytes()[start - 1] == b'\n' {
                                start -= 1;
                            }

                            let end = elsif
                                .end_keyword_loc()
                                .map(|l| l.start_offset())
                                .unwrap_or_else(|| elsif.location().end_offset());

                            corrections.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: "\n".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }
                    }

                    diagnostics.push(diagnostic);
                }
                subsequent = elsif.subsequent();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DuplicateElsifCondition,
        "cops/lint/duplicate_elsif_condition"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(DuplicateElsifCondition.supports_autocorrect());
    }

    #[test]
    fn autocorrect_removes_last_duplicate_elsif_with_safe_predicate() {
        crate::testutil::assert_cop_autocorrect(
            &DuplicateElsifCondition,
            b"if @flag\n  :a\nelsif @other\n  :b\nelsif @flag\n  :c\nend\n",
            b"if @flag\n  :a\nelsif @other\n  :b\nend\n",
        );
    }

    #[test]
    fn does_not_autocorrect_duplicate_elsif_with_trailing_else() {
        let (_diagnostics, corrections) = crate::testutil::run_cop_autocorrect(
            &DuplicateElsifCondition,
            b"if @flag\n  :a\nelsif @flag\n  :b\nelse\n  :c\nend\n",
        );
        assert!(corrections.is_empty());
    }
}
