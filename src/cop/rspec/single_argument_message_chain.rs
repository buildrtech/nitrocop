use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SingleArgumentMessageChain;

impl Cop for SingleArgumentMessageChain {
    fn name(&self) -> &'static str {
        "RSpec/SingleArgumentMessageChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, STRING_NODE, SYMBOL_NODE]
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

        let method = call.name().as_slice();
        let replacement = if method == b"receive_message_chain" {
            "receive"
        } else if method == b"stub_chain" {
            "stub"
        } else {
            return;
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();

        let is_single_arg = if arg_list.len() == 1 {
            let arg = &arg_list[0];
            // Single symbol or single string (without dots for strings)
            if arg.as_symbol_node().is_some() {
                true
            } else if let Some(s) = arg.as_string_node() {
                // Multi-part string like "one.two" should not be flagged
                !s.unescaped().contains(&b'.')
            } else if let Some(arr) = arg.as_array_node() {
                // Single-element array
                arr.elements().iter().count() == 1
            } else {
                false
            }
        } else {
            false
        };

        if !is_single_arg {
            return;
        }

        let method_str = std::str::from_utf8(method).unwrap_or("receive_message_chain");
        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{replacement}` instead of calling `{method_str}` with a single argument."
            ),
        );

        if let Some(ref mut corr) = corrections
            && let Some(selector_loc) = call.message_loc()
        {
            corr.push(crate::correction::Correction {
                start: selector_loc.start_offset(),
                end: selector_loc.end_offset(),
                replacement: replacement.to_string(),
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
    crate::cop_fixture_tests!(
        SingleArgumentMessageChain,
        "cops/rspec/single_argument_message_chain"
    );
    crate::cop_autocorrect_fixture_tests!(
        SingleArgumentMessageChain,
        "cops/rspec/single_argument_message_chain"
    );
}
