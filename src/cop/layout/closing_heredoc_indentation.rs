use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct ClosingHeredocIndentation;

impl Cop for ClosingHeredocIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingHeredocIndentation"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = HeredocVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            pending_corrections: Vec::new(),
            argument_indent: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corr) = corrections {
            corr.extend(visitor.pending_corrections);
        }
    }
}

struct HeredocVisitor<'a> {
    cop: &'a ClosingHeredocIndentation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    pending_corrections: Vec<Correction>,
    /// When a heredoc is a direct argument to a method call (or chained call),
    /// this holds the indentation of the outermost call in the chain.
    /// Mirrors RuboCop's `argument_indentation_correct?` + `find_node_used_heredoc_argument`.
    argument_indent: Option<usize>,
}

impl HeredocVisitor<'_> {
    fn check_heredoc(
        &mut self,
        opening_loc: ruby_prism::Location<'_>,
        closing_loc: ruby_prism::Location<'_>,
    ) {
        let bytes = self.source.as_bytes();
        let opening = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        // Must be a heredoc
        if !opening.starts_with(b"<<") {
            return;
        }

        // Skip simple heredocs (<<FOO without - or ~) since they have no indentation control
        let after_arrows = &opening[2..];
        if !after_arrows.starts_with(b"~") && !after_arrows.starts_with(b"-") {
            return;
        }

        // Get indentation of the opening line
        let opening_line_indent = line_indent(self.source, opening_loc.start_offset());

        // Get indentation of the closing line
        let closing_line_indent = line_indent(self.source, closing_loc.start_offset());

        // If opening and closing indentation match, no offense
        if opening_line_indent == closing_line_indent {
            return;
        }

        // If the heredoc is a direct argument to a method call (or chained call),
        // check whether the closing indentation matches the outermost call's
        // indentation (RuboCop argument_indentation_correct? logic).
        if let Some(arg_indent) = self.argument_indent {
            if closing_line_indent == arg_indent {
                return;
            }
        }

        // Build the diagnostic message
        let (opening_line_num, _) = self.source.offset_to_line_col(opening_loc.start_offset());
        let lines: Vec<&[u8]> = self.source.lines().collect();
        let empty: &[u8] = b"";
        let opening_line_text = lines.get(opening_line_num - 1).unwrap_or(&empty);
        let opening_trimmed = std::str::from_utf8(opening_line_text).unwrap_or("").trim();

        let closing_line_text = &bytes[closing_loc.start_offset()..closing_loc.end_offset()];
        let closing_trimmed = std::str::from_utf8(closing_line_text).unwrap_or("").trim();

        // Find the start of the actual delimiter text (skip leading whitespace)
        let close_content_offset = closing_loc.start_offset()
            + closing_line_text
                .iter()
                .take_while(|&&b| b == b' ' || b == b'\t')
                .count();
        let (close_line, close_col) = self.source.offset_to_line_col(close_content_offset);

        let message = if self.argument_indent.is_some() {
            format!(
                "`{}` is not aligned with `{}` or beginning of method definition.",
                closing_trimmed, opening_trimmed
            )
        } else {
            format!(
                "`{}` is not aligned with `{}`.",
                closing_trimmed, opening_trimmed
            )
        };

        self.diagnostics.push(
            self.cop
                .diagnostic(self.source, close_line, close_col, message),
        );

        let expected_indent = self.argument_indent.unwrap_or(opening_line_indent);
        let line_start = self.source.line_start_offset(close_line);
        self.pending_corrections.push(Correction {
            start: line_start,
            end: close_content_offset,
            replacement: " ".repeat(expected_indent),
            cop_name: self.cop.name(),
            cop_index: 0,
        });
    }
}

impl<'pr> Visit<'pr> for HeredocVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let saved = self.argument_indent;

        // Visit the receiver (if any) with no argument context change —
        // the receiver is not "an argument" of this call.
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }

        // Visit arguments: set argument_indent to the outermost call's indent.
        // If we're already inside an argument context (nested calls), keep the
        // outer one. Otherwise, use this call's line indent.
        if let Some(arguments) = node.arguments() {
            let outer_indent = self
                .argument_indent
                .unwrap_or_else(|| line_indent(self.source, node.location().start_offset()));
            self.argument_indent = Some(outer_indent);
            self.visit(&arguments.as_node());
            self.argument_indent = saved;
        }

        // Visit the block (if any) with argument context cleared —
        // heredocs inside a block body are NOT arguments.
        if let Some(block) = node.block() {
            self.argument_indent = None;
            self.visit(&block);
            self.argument_indent = saved;
        }
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let (Some(opening), Some(closing)) = (node.opening_loc(), node.closing_loc()) {
            self.check_heredoc(opening, closing);
        }
        ruby_prism::visit_string_node(self, node);
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let (Some(opening), Some(closing)) = (node.opening_loc(), node.closing_loc()) {
            self.check_heredoc(opening, closing);
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_interpolated_x_string_node(
        &mut self,
        node: &ruby_prism::InterpolatedXStringNode<'pr>,
    ) {
        self.check_heredoc(node.opening_loc(), node.closing_loc());
        ruby_prism::visit_interpolated_x_string_node(self, node);
    }

    fn visit_x_string_node(&mut self, node: &ruby_prism::XStringNode<'pr>) {
        self.check_heredoc(node.opening_loc(), node.closing_loc());
        ruby_prism::visit_x_string_node(self, node);
    }
}

/// Get the indentation (leading spaces) of the line containing the given offset.
fn line_indent(source: &SourceFile, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut indent = 0;
    while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
        indent += 1;
    }
    indent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        ClosingHeredocIndentation,
        "cops/layout/closing_heredoc_indentation"
    );
    crate::cop_autocorrect_fixture_tests!(
        ClosingHeredocIndentation,
        "cops/layout/closing_heredoc_indentation"
    );

    #[test]
    fn heredoc_as_argument_aligned_to_outermost_call() {
        let source = b"expect($stdout.string)\n  .to eq(<<~RESULT)\n    content here\nRESULT\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offenses for heredoc argument aligned to outermost call, got: {:?}",
            diags,
        );
    }

    #[test]
    fn heredoc_in_block_body_flags_offense() {
        let source = b"get '/foo' do\n    <<-EOHTML\n    <html></html>\nEOHTML\nend\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected offense for heredoc in block body with wrong closing indent, got: {:?}",
            diags,
        );
    }

    #[test]
    fn heredoc_in_block_body_aligned_no_offense() {
        let source = b"get '/foo' do\n  <<-EOHTML\n  <html></html>\n  EOHTML\nend\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offenses for heredoc in block body with correct closing indent, got: {:?}",
            diags,
        );
    }

    #[test]
    fn heredoc_argument_aligned_to_method_call() {
        // closing aligned with include_examples (indent 0), not with <<-EOS (indent 17)
        let source = b"include_examples :offense,\n                 <<-EOS\n  bar\nEOS\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offenses for argument heredoc aligned to outermost call, got: {:?}",
            diags,
        );
    }

    #[test]
    fn heredoc_argument_with_strip_indent() {
        let source =
            b"include_examples :offense,\n                 <<-EOS.strip_indent\n  bar\nEOS\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offenses for argument heredoc with .strip_indent, got: {:?}",
            diags,
        );
    }

    #[test]
    fn heredoc_argument_msg_arg_format() {
        // closing at indent 4 matches neither the opening (indent 17) nor the call (indent 0)
        let source = b"include_examples :offense,\n                 <<-EOS\n  bar\n    EOS\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0]
                .message
                .contains("or beginning of method definition"),
            "Expected MSG_ARG format for argument heredoc, got: {}",
            diags[0].message,
        );
    }
}
