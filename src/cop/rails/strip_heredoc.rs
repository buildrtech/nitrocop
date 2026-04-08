use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StripHeredoc;

impl Cop for StripHeredoc {
    fn name(&self) -> &'static str {
        "Rails/StripHeredoc"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"strip_heredoc" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Only flag when the direct receiver is a heredoc.
        // In Prism, heredocs are StringNode or InterpolatedStringNode with opening starting with "<<".
        let opening_loc = if let Some(s) = receiver.as_string_node() {
            s.opening_loc()
                .filter(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
        } else if let Some(s) = receiver.as_interpolated_string_node() {
            s.opening_loc()
                .filter(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
        } else {
            None
        };

        let Some(opening_loc) = opening_loc else {
            return;
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use squiggly heredoc (`<<~`) instead of `strip_heredoc`.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let opening =
                source.byte_slice(opening_loc.start_offset(), opening_loc.end_offset(), "");
            let opening_str = if opening.starts_with("<<-") || opening.starts_with("<<~") {
                format!("<<~{}", &opening[3..])
            } else {
                format!("<<~{}", &opening[2..])
            };

            corr.push(crate::correction::Correction {
                start: opening_loc.start_offset(),
                end: opening_loc.end_offset(),
                replacement: opening_str,
                cop_name: self.name(),
                cop_index: 0,
            });

            if let Some(dot_loc) = call.call_operator_loc()
                && let Some(selector_loc) = call.message_loc()
            {
                corr.push(crate::correction::Correction {
                    start: dot_loc.start_offset(),
                    end: selector_loc.end_offset(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            } else if let Some(selector_loc) = call.message_loc() {
                corr.push(crate::correction::Correction {
                    start: selector_loc.start_offset(),
                    end: selector_loc.end_offset(),
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
    crate::cop_fixture_tests!(StripHeredoc, "cops/rails/strip_heredoc");

    #[test]
    fn autocorrects_dash_heredoc_strip_call() {
        crate::testutil::assert_cop_autocorrect(
            &StripHeredoc,
            b"<<-EOS.strip_heredoc\n  some text\nEOS\n",
            b"<<~EOS\n  some text\nEOS\n",
        );
    }

    #[test]
    fn autocorrects_plain_heredoc_strip_call() {
        crate::testutil::assert_cop_autocorrect(
            &StripHeredoc,
            b"<<EOS.strip_heredoc\n  some text\nEOS\n",
            b"<<~EOS\n  some text\nEOS\n",
        );
    }
}
