use crate::cop::factory_bot::{FACTORY_BOT_DEFAULT_INCLUDE, is_factory_bot_receiver};
use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IdSequence;

impl Cop for IdSequence {
    fn name(&self) -> &'static str {
        "FactoryBot/IdSequence"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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

        if call.name().as_slice() != b"sequence" {
            return;
        }

        // Receiver must be nil or FactoryBot
        match call.receiver() {
            None => {}
            Some(recv) => {
                if !is_factory_bot_receiver(&recv) {
                    return;
                }
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be :id symbol
        let first = &arg_list[0];
        let is_id = first
            .as_symbol_node()
            .is_some_and(|s| s.unescaped() == b"id");

        if !is_id {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Do not create a sequence for an id attribute".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let (line, _) = source.offset_to_line_col(loc.start_offset());
            let line_start = source.line_start_offset(line);
            let mut remove_start = loc.start_offset();
            if source
                .try_byte_slice(line_start, loc.start_offset())
                .is_some_and(|s| s.trim().is_empty())
            {
                remove_start = line_start;
            }

            let mut remove_end = loc.end_offset();
            if source.as_bytes().get(remove_end).copied() == Some(b'\n') {
                remove_end += 1;
            }

            corr.push(crate::correction::Correction {
                start: remove_start,
                end: remove_end,
                replacement: String::new(),
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
    crate::cop_fixture_tests!(IdSequence, "cops/factorybot/id_sequence");

    #[test]
    fn supports_autocorrect() {
        assert!(IdSequence.supports_autocorrect());
    }

    #[test]
    fn autocorrect_removes_id_sequence_line() {
        crate::testutil::assert_cop_autocorrect(
            &IdSequence,
            b"FactoryBot.define do\n  factory :post do\n    sequence :id\n    title { \"x\" }\n  end\nend\n",
            b"FactoryBot.define do\n  factory :post do\n    title { \"x\" }\n  end\nend\n",
        );
    }
}
