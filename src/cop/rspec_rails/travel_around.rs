use ruby_prism::Visit;

use crate::cop::node_type::{
    BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-20)
///
/// Extended corpus oracle reported FP=3, FN=0.
///
/// FP=3: All from `travel_to(time, &ex)` — travel method with both a time
/// argument and a block pass. RuboCop's `extract_travel_with_block_pass` pattern
/// `(send _ TRAVEL (block_pass $lvar))` requires the block_pass to be the ONLY
/// child (no other arguments). `travel_to(time, &ex)` has an extra argument
/// and is not trivially convertible to a `before` block.
/// Fixed by only flagging block_pass pattern when `node.arguments().is_none()`.
pub struct TravelAround;

const TRAVEL_METHODS: &[&[u8]] = &[b"freeze_time", b"travel", b"travel_to"];

impl Cop for TravelAround {
    fn name(&self) -> &'static str {
        "RSpecRails/TravelAround"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            BLOCK_NODE,
            CALL_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
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
        // We look for `around` blocks and then check their body for travel patterns.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"around" || call.receiver().is_some() {
            return;
        }

        // Check for around(:all) or around(:suite) - those are exempt
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if let Some(sym) = arg.as_symbol_node() {
                    let sym_name = sym.unescaped();
                    if sym_name == b"all" || sym_name == b"suite" {
                        return;
                    }
                }
            }
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

        // Recursively search for travel calls anywhere in the around block body
        let mut finder = TravelFinder {
            offsets: Vec::new(),
        };
        finder.visit(&body);

        for offset in finder.offsets {
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer to travel in `before` rather than `around`.".to_string(),
            ));
        }

        // Conservative autocorrect baseline (RuboCop-aligned pattern 2 only):
        // around do |example|
        //   freeze_time(&example)
        // end
        // => before { freeze_time }
        if let Some(ref mut corr) = corrections
            && let Some(stmts) = body.as_statements_node()
        {
            let stmt_list: Vec<_> = stmts.body().iter().collect();
            if stmt_list.len() == 1
                && let Some(travel_call) = stmt_list[0].as_call_node()
                && TRAVEL_METHODS.contains(&travel_call.name().as_slice())
                && travel_call.receiver().is_none()
                && travel_call.arguments().is_none()
                && travel_call
                    .block()
                    .and_then(|b| b.as_block_argument_node())
                    .is_some()
            {
                let method =
                    std::str::from_utf8(travel_call.name().as_slice()).unwrap_or("freeze_time");
                corr.push(crate::correction::Correction {
                    start: call.location().start_offset(),
                    end: block_node.location().end_offset(),
                    replacement: format!("before {{ {method} }}"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                if let Some(last) = diagnostics.last_mut() {
                    last.corrected = true;
                }
            }
        }
    }
}

struct TravelFinder {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for TravelFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        if TRAVEL_METHODS.contains(&name) && node.receiver().is_none() {
            if let Some(travel_block) = node.block() {
                // Pattern 1: travel_method do ... example.run ... end
                if let Some(block_node) = travel_block.as_block_node() {
                    if let Some(travel_body) = block_node.body() {
                        if let Some(stmts) = travel_body.as_statements_node() {
                            let stmt_list: Vec<_> = stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(run_call) = stmt_list[0].as_call_node() {
                                    if run_call.name().as_slice() == b"run" {
                                        self.offsets.push(node.location().start_offset());
                                    }
                                }
                            }
                        }
                    }
                }

                // Pattern 2: travel_method(&example) — only when there are no other arguments.
                // RuboCop's pattern `(send _ TRAVEL (block_pass $lvar))` requires block_pass
                // to be the only child. `travel_to(time, &ex)` is NOT flagged because the
                // time argument makes it non-trivially convertible to a `before` block.
                if travel_block.as_block_argument_node().is_some() && node.arguments().is_none() {
                    self.offsets.push(node.location().start_offset());
                }
            }
        }

        // Continue visiting children to find nested travel calls
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TravelAround, "cops/rspecrails/travel_around");

    #[test]
    fn supports_autocorrect() {
        assert!(TravelAround.supports_autocorrect());
    }

    #[test]
    fn autocorrects_simple_block_pass_form_to_before() {
        crate::testutil::assert_cop_autocorrect(
            &TravelAround,
            b"around do |example|\n  freeze_time(&example)\nend\n",
            b"before { freeze_time }\n",
        );
    }
}
