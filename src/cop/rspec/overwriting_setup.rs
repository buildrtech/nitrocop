use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashSet;

/// RSpec/OverwritingSetup: Flag duplicate `let`/`subject` declarations with the same name.
pub struct OverwritingSetup;

impl Cop for OverwritingSetup {
    fn name(&self) -> &'static str {
        "RSpec/OverwritingSetup"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let mut seen_names: HashSet<Vec<u8>> = HashSet::new();
        let mut corrections = corrections;

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_some() {
                    continue;
                }
                let m = c.name().as_slice();
                let is_let = m == b"let" || m == b"let!";
                let is_subject = m == b"subject" || m == b"subject!";

                if !is_let && !is_subject {
                    continue;
                }

                // Match RuboCop's `(block (send nil? {let/subject} ...) ...)` shape:
                // setup declarations must have a real block body.
                if c.block().and_then(|b| b.as_block_node()).is_none() {
                    continue;
                }

                let var_name = if is_subject && c.arguments().is_none() {
                    // Unnamed subject
                    Some(b"subject".to_vec())
                } else {
                    extract_let_name(&c)
                };

                if let Some(vn) = var_name {
                    if !seen_names.insert(vn.clone()) {
                        let loc = c.location();
                        let (line, col) = source.offset_to_line_col(loc.start_offset());
                        let name_str = std::str::from_utf8(&vn).unwrap_or("?");
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            col,
                            format!("`{}` is already defined.", name_str),
                        );
                        if let Some(corrections) = corrections.as_deref_mut() {
                            let (start, end) = if let Some(msg_loc) = c.message_loc() {
                                (msg_loc.start_offset(), msg_loc.end_offset())
                            } else {
                                (loc.start_offset(), loc.end_offset())
                            };
                            corrections.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: "skip".to_string(),
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
}

fn extract_let_name(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return None;
    }
    let first = &arg_list[0];
    if let Some(sym) = first.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = first.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

fn is_example_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(OverwritingSetup, "cops/rspec/overwriting_setup");

    #[test]
    fn autocorrect_rewrites_duplicate_setup_selector() {
        crate::testutil::assert_cop_autocorrect(
            &OverwritingSetup,
            b"RSpec.describe User do\n  let(:a) { 1 }\n  let(:a) { 2 }\nend\n",
            b"RSpec.describe User do\n  let(:a) { 1 }\n  skip(:a) { 2 }\nend\n",
        );
    }
}
