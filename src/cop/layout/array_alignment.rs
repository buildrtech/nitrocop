use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Layout/ArrayAlignment checks alignment of multi-line array literal elements
/// and rescue exception lists.
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause:** Prism wraps multi-assignment RHS values (`a, b = 1, 2`)
/// in an implicit `ArrayNode` with no `opening_loc`. The cop was checking these
/// implicit arrays as if they were array literals. Fixed by skipping arrays
/// where `opening_loc()` is `None`. RuboCop does the same via
/// `return if node.parent&.masgn_type?`.
///
/// **FN root cause:** RuboCop treats rescue exception lists (e.g.,
/// `rescue ArgumentError, RuntimeError`) as arrays for alignment purposes.
/// In Prism these are `RescueNode` with an `exceptions()` list, not `ArrayNode`.
/// Fixed by adding `RESCUE_NODE` handling that checks alignment of exception
/// classes spanning multiple lines.
pub struct ArrayAlignment;

/// Returns true if the byte at `offset` is the first non-whitespace character on its line.
fn begins_its_line(source: &SourceFile, offset: usize) -> bool {
    let (line, col) = source.offset_to_line_col(offset);
    if col == 0 {
        return true;
    }
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    line_bytes[..col].iter().all(|&b| b == b' ' || b == b'\t')
}

impl Cop for ArrayAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArrayAlignment"
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
        if let Some(array_node) = node.as_array_node() {
            self.check_array(source, &array_node, config, diagnostics);
        }
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RescueNode is not dispatched through visit_branch_node_enter in Prism,
        // so we need a dedicated visitor to find rescue nodes for exception list
        // alignment checking.
        let mut visitor = RescueVisitor {
            cop: self,
            source,
            config,
            diagnostics,
        };
        visitor.visit(&parse_result.node());
    }
}

struct RescueVisitor<'a> {
    cop: &'a ArrayAlignment,
    source: &'a SourceFile,
    config: &'a CopConfig,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RescueVisitor<'_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        self.cop
            .check_rescue_exceptions(self.source, node, self.config, self.diagnostics);
        ruby_prism::visit_rescue_node(self, node);
    }
}

impl ArrayAlignment {
    fn check_array(
        &self,
        source: &SourceFile,
        array_node: &ruby_prism::ArrayNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyle", "with_first_element");
        let indent_width = config.get_usize("IndentationWidth", 2);

        // Skip implicit arrays (no opening bracket) — these are RHS of
        // multi-assignments like `a, b = 1, 2` where Prism wraps values in
        // an ArrayNode with no `[`.
        if array_node.opening_loc().is_none() {
            return;
        }

        let elements = array_node.elements();
        if elements.len() < 2 {
            return;
        }

        let first = match elements.iter().next() {
            Some(e) => e,
            None => return,
        };
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        // For "with_fixed_indentation", expected column is array line indent + indent_width
        let expected_col = match style {
            "with_fixed_indentation" => {
                let open_loc = array_node.opening_loc().unwrap_or(first.location());
                let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
                let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
                crate::cop::util::indentation_of(open_line_bytes) + indent_width
            }
            _ => first_col, // "with_first_element" (default)
        };

        self.check_element_alignment(source, &elements, first_line, expected_col, diagnostics);
    }

    fn check_rescue_exceptions(
        &self,
        source: &SourceFile,
        rescue_node: &ruby_prism::RescueNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyle", "with_first_element");
        let indent_width = config.get_usize("IndentationWidth", 2);
        let exceptions = rescue_node.exceptions();
        if exceptions.len() < 2 {
            return;
        }

        let first = match exceptions.iter().next() {
            Some(e) => e,
            None => return,
        };
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        let expected_col = match style {
            "with_fixed_indentation" => {
                // Use the rescue keyword line's indentation + indent_width
                let rescue_line_bytes = source.lines().nth(first_line - 1).unwrap_or(b"");
                crate::cop::util::indentation_of(rescue_line_bytes) + indent_width
            }
            _ => first_col, // "with_first_element" (default)
        };

        self.check_element_alignment(source, &exceptions, first_line, expected_col, diagnostics);
    }

    fn check_element_alignment(
        &self,
        source: &SourceFile,
        elements: &ruby_prism::NodeList<'_>,
        first_line: usize,
        expected_col: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut last_checked_line = first_line;

        for elem in elements.iter().skip(1) {
            let start_offset = elem.location().start_offset();
            let (elem_line, elem_col) = source.offset_to_line_col(start_offset);
            // Only check the first element on each new line; subsequent elements
            // on the same line are just comma-separated and not alignment targets.
            if elem_line == last_checked_line {
                continue;
            }
            last_checked_line = elem_line;
            // Skip elements that are not the first non-whitespace token on their line.
            // E.g. in `}, {` the `{` follows a `}` and should not be checked.
            if !begins_its_line(source, start_offset) {
                continue;
            }
            if elem_col != expected_col {
                diagnostics.push(
                    self.diagnostic(
                        source,
                        elem_line,
                        elem_col,
                        "Align the elements of an array literal if they span more than one line."
                            .to_string(),
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ArrayAlignment, "cops/layout/array_alignment");

    #[test]
    fn rescue_exception_list_misaligned() {
        // rescue exceptions not aligned with first exception
        let source =
            b"begin\n  foo\nrescue ArgumentError,\n  RuntimeError,\n  TypeError => e\n  bar\nend\n";
        let diags = run_cop_full(&ArrayAlignment, source);
        assert_eq!(
            diags.len(),
            2,
            "should flag both misaligned rescue exceptions"
        );
    }

    #[test]
    fn rescue_exception_list_aligned() {
        // rescue exceptions aligned with first exception — no offense
        let source = b"begin\n  foo\nrescue ArgumentError,\n       RuntimeError,\n       TypeError => e\n  bar\nend\n";
        let diags = run_cop_full(&ArrayAlignment, source);
        assert!(
            diags.is_empty(),
            "aligned rescue exceptions should not be flagged"
        );
    }

    #[test]
    fn rescue_single_exception_no_offense() {
        let source = b"begin\n  foo\nrescue ArgumentError => e\n  bar\nend\n";
        let diags = run_cop_full(&ArrayAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn single_line_array_no_offense() {
        let source = b"x = [1, 2, 3]\n";
        let diags = run_cop_full(&ArrayAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_fixed_indentation_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("with_fixed_indentation".into()),
            )]),
            ..CopConfig::default()
        };
        // Elements at fixed indentation (2 spaces) should be accepted
        let src = b"x = [\n  1,\n  2\n]\n";
        let diags = run_cop_full_with_config(&ArrayAlignment, src, config.clone());
        assert!(
            diags.is_empty(),
            "with_fixed_indentation should accept 2-space indent"
        );

        // Elements aligned with first element at column 4 should be flagged
        let src2 = b"x = [1,\n     2]\n";
        let diags2 = run_cop_full_with_config(&ArrayAlignment, src2, config);
        assert_eq!(
            diags2.len(),
            1,
            "with_fixed_indentation should flag first-element alignment"
        );
    }
}
