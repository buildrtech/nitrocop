use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/MultilineArrayLineBreaks — each item in a multi-line array must start
/// on a separate line.
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of 1,119 FPs:** The `AllowMultilineFinalElement` config option was
/// read but ignored (stored in `_allow_multiline_final` with underscore prefix).
/// When `AllowMultilineFinalElement: true`, the last element of a multiline array
/// is allowed to span multiple lines without triggering an offense. Many corpus
/// projects enable this option.
///
/// **Additional bug:** The `all_on_same_line?` guard compared bracket positions
/// (`open_line == close_line`) instead of element positions. RuboCop checks
/// whether all elements occupy the same line range, not whether the brackets do.
/// This caused false positives on arrays like `[\n  1, 2, 3,\n]` where elements
/// are on one line but brackets span multiple.
///
/// **Fix:** Rewrote to match the RuboCop `MultilineElementLineBreaks` mixin:
/// 1. `all_on_same_line?` guard checks element line ranges, not bracket lines
/// 2. `AllowMultilineFinalElement` changes the guard to compare first.start_line
///    vs last.start_line (ignoring the last element's span)
/// 3. Uses `last_seen_line` tracking algorithm (only updates on non-offending
///    elements) matching RuboCop exactly
pub struct MultilineArrayLineBreaks;

impl Cop for MultilineArrayLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineArrayLineBreaks"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        // Skip implicit arrays (no brackets — e.g. multiple assignment RHS)
        if array.opening_loc().is_none() || array.closing_loc().is_none() {
            return;
        }

        let elements: Vec<ruby_prism::Node<'_>> = array.elements().iter().collect();
        if elements.len() < 2 {
            return;
        }

        // RuboCop's all_on_same_line? guard — checks elements, not brackets
        let first_start_line = source
            .offset_to_line_col(elements[0].location().start_offset())
            .0;
        let last = elements.last().unwrap();

        if allow_multiline_final {
            // ignore_last: true — check first.first_line == last.first_line
            // All elements start on the same line; last element may span multiple lines
            let last_start_line = source.offset_to_line_col(last.location().start_offset()).0;
            if first_start_line == last_start_line {
                return;
            }
        } else {
            // Default: check first.first_line == last.last_line
            // All elements fit entirely on the same line
            let last_end_line = source
                .offset_to_line_col(last.location().end_offset().saturating_sub(1))
                .0;
            if first_start_line == last_end_line {
                return;
            }
        }

        // Track last_line of the most recent non-offending element (matches RuboCop's
        // last_seen_line algorithm). When an element is flagged, last_seen_line is NOT
        // updated, so subsequent elements are compared against the last "good" element.
        let mut last_seen_line: isize = -1;
        for elem in &elements {
            let (start_line, start_col) = source.offset_to_line_col(elem.location().start_offset());
            if last_seen_line >= start_line as isize {
                diagnostics.push(self.diagnostic(
                    source,
                    start_line,
                    start_col,
                    "Each item in a multi-line array must start on a separate line.".to_string(),
                ));
            } else {
                let end_line = source
                    .offset_to_line_col(elem.location().end_offset().saturating_sub(1))
                    .0;
                last_seen_line = end_line as isize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full_with_config, run_cop_full_with_config};
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        MultilineArrayLineBreaks,
        "cops/layout/multiline_array_line_breaks"
    );

    #[test]
    fn allow_multiline_final_element_ignores_multiline_last_hash() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        // Last element is a multiline hash — should be allowed
        assert_cop_no_offenses_full_with_config(
            &MultilineArrayLineBreaks,
            b"[1, 2, 3, {\n  a: 1\n}]\n",
            config,
        );
    }

    #[test]
    fn allow_multiline_final_element_still_flags_non_last() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        // Non-last elements on same line should still be flagged
        let diags = run_cop_full_with_config(
            &MultilineArrayLineBreaks,
            b"[1, 2, 3, {\n  a: 1\n}, 4]\n",
            config,
        );

        // 2, 3, and { are all on same line as 1 → 3 offenses
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn allow_multiline_final_element_no_offense_when_each_on_own_line() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        assert_cop_no_offenses_full_with_config(
            &MultilineArrayLineBreaks,
            b"[\n  1,\n  2,\n  foo(\n    bar\n  )\n]\n",
            config,
        );
    }
}
