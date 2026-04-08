use crate::cop::node_type::{CALL_NODE, HASH_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-25)
///
/// Corpus oracle reported FP=4, FN=0.
///
/// FP=4: Fixed by only flagging empty hash when it's the last argument
/// and a description/subject argument precedes it. Cases like `describe({})`
/// where `{}` is the subject (first arg) or `example(name, {}, caller)` where
/// `{}` is a middle argument are not metadata and should not be flagged.
pub struct EmptyMetadata;

impl Cop for EmptyMetadata {
    fn name(&self) -> &'static str {
        "RSpec/EmptyMetadata"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, HASH_NODE]
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
        // Detect empty metadata hash `{}` in example groups/examples
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check if this is an RSpec method (example group or example, including RSpec.describe,
        // RSpec.shared_context, etc.)
        let is_rspec = if call.receiver().is_none() {
            is_rspec_example_group(method_name) || is_rspec_example(method_name)
        } else if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && (is_rspec_example_group(method_name) || is_rspec_example(method_name))
        } else {
            false
        };

        if !is_rspec {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Note: keyword_hash_node (keyword args) intentionally not handled —
        // empty metadata is specifically the `{}` hash literal form, not keyword args.
        //
        // Only flag empty hash when it's the LAST argument and there's at least
        // one preceding argument (the description/subject). This avoids FPs like
        // `describe({})` where `{}` is the subject, or `example(name, {}, caller)`
        // where `{}` is a middle positional argument.
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() >= 2 {
            if let Some(hash) = arg_list.last().and_then(|a| a.as_hash_node()) {
                if hash.elements().iter().count() == 0 {
                    let loc = hash.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid empty metadata hash.".to_string(),
                    );

                    if let Some(ref mut corr) = corrections {
                        let bytes = source.as_bytes();
                        let mut start = loc.start_offset();
                        let end = loc.end_offset();

                        // Match RuboCop behavior by removing surrounding comma/space on the left
                        while start > 0 && bytes[start - 1].is_ascii_whitespace() {
                            start -= 1;
                        }
                        if start > 0 && bytes[start - 1] == b',' {
                            start -= 1;
                            while start > 0 && bytes[start - 1].is_ascii_whitespace() {
                                start -= 1;
                            }
                        }

                        corr.push(crate::correction::Correction {
                            start,
                            end,
                            replacement: "".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyMetadata, "cops/rspec/empty_metadata");
    crate::cop_autocorrect_fixture_tests!(EmptyMetadata, "cops/rspec/empty_metadata");
}
