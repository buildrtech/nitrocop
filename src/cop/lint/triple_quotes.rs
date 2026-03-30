use crate::cop::node_type::INTERPOLATED_STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TripleQuotes;

impl Cop for TripleQuotes {
    fn name(&self) -> &'static str {
        "Lint/TripleQuotes"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_STRING_NODE]
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
        let interp = match node.as_interpolated_string_node() {
            Some(n) => n,
            None => return,
        };

        // Empty string pieces are the extra adjacent literals created by triple-quote syntax.
        let mut empty_parts: Vec<ruby_prism::Node<'_>> = interp
            .parts()
            .iter()
            .filter(|part| {
                part.as_string_node()
                    .is_some_and(|s| s.unescaped().is_empty())
            })
            .collect();

        if empty_parts.is_empty() {
            return;
        }

        // Check if the source starts with 3+ quote characters
        let loc = interp.location();
        let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let quote_count = src.iter().take_while(|&&b| b == b'"' || b == b'\'').count();

        if quote_count < 3 {
            return;
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Triple quotes found. Did you mean to use a heredoc?".to_string(),
        );

        if let Some(corr) = corrections.as_mut() {
            // If all parts are empty literals, keep one so the resulting source remains a string.
            if empty_parts.len() == interp.parts().len() {
                empty_parts.remove(0);
            }

            for part in empty_parts {
                let part_loc = part.location();
                corr.push(crate::correction::Correction {
                    start: part_loc.start_offset(),
                    end: part_loc.end_offset(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
            diag.corrected = true;
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TripleQuotes, "cops/lint/triple_quotes");
    crate::cop_autocorrect_fixture_tests!(TripleQuotes, "cops/lint/triple_quotes");

    #[test]
    fn skip_in_heredoc() {
        let source = b"x = <<~RUBY\n  \"\"\"\n  foo\n  \"\"\"\nRUBY\n";
        let diags = crate::testutil::run_cop_full(&TripleQuotes, source);
        assert!(
            diags.is_empty(),
            "Should not fire on triple quotes inside heredoc"
        );
    }
}
