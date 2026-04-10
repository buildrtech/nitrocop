use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Default style is `have_received` — flags `expect(...).to receive(...)`.
///
/// Corpus investigation (46 FN, 0 FP): FNs caused by compound expectations
/// like `expect(foo).to receive(:bar).and receive(:baz)` where the second
/// `receive` call is nested as an argument to `.and`/`.or` rather than in the
/// receiver chain. RuboCop uses `def_node_search` (recursive subtree search)
/// to find ALL `receive`/`have_received` calls, while nitrocop previously only
/// walked the receiver chain. Fixed by implementing recursive subtree search
/// matching RuboCop's `receive_message` node search behavior.
pub struct MessageSpies;

impl Cop for MessageSpies {
    fn name(&self) -> &'static str {
        "RSpec/MessageSpies"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Config: EnforcedStyle — "have_received" (default) or "receive"
        let enforced_style = config.get_str("EnforcedStyle", "have_received");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check receiver is `expect(...)`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if recv_call.name().as_slice() != b"expect" || recv_call.receiver().is_some() {
            return;
        }

        // Check that the matcher argument contains `receive` or `have_received`.
        // Use recursive search (matching RuboCop's def_node_search :receive_message)
        // to find ALL receive/have_received calls in the argument subtree.
        // This handles compound expectations like:
        //   expect(foo).to receive(:bar).and receive(:baz)
        // where the second `receive` is an argument to `.and`, not in the
        // receiver chain.
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let target_name = if enforced_style == "receive" {
            b"have_received" as &[u8]
        } else {
            b"receive" as &[u8]
        };

        let mut found = Vec::new();
        for arg in args.arguments().iter() {
            find_matcher_calls(&arg, target_name, &mut found);
        }

        let (msg, replacement) = if enforced_style == "receive" {
            (
                "Prefer `receive` for setting message expectations.",
                "receive",
            )
        } else {
            (
                "Prefer `have_received` for setting message expectations. Setup the object as a spy using `allow` or `instance_spy`.",
                "have_received",
            )
        };

        for (start_offset, end_offset) in found {
            let (line, column) = source.offset_to_line_col(start_offset);
            let mut diagnostic = self.diagnostic(source, line, column, msg.to_string());

            if let Some(ref mut corrs) = corrections {
                corrs.push(crate::correction::Correction {
                    start: start_offset,
                    end: end_offset,
                    replacement: replacement.to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

/// Recursively search a node subtree for `(send nil? target_name ...)` calls,
/// matching RuboCop's `def_node_search :receive_message` behavior.
/// Handles compound expectations like `receive(:a).and receive(:b)` where
/// the second `receive` is an argument to `.and`/`.or`.
fn find_matcher_calls(
    node: &ruby_prism::Node<'_>,
    target_name: &[u8],
    out: &mut Vec<(usize, usize)>,
) {
    if let Some(call) = node.as_call_node() {
        // Check if this is a bare `receive(...)` or `have_received(...)` call
        if call.name().as_slice() == target_name
            && call.receiver().is_none()
            && let Some(selector) = call.message_loc()
        {
            out.push((selector.start_offset(), selector.end_offset()));
        }
        // Recurse into receiver
        if let Some(recv) = call.receiver() {
            find_matcher_calls(&recv, target_name, out);
        }
        // Recurse into arguments
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_matcher_calls(&arg, target_name, out);
            }
        }
        // Recurse into block body if present
        if let Some(block) = call.block() {
            if let Some(body) = block.as_block_node().and_then(|b| b.body()) {
                find_matcher_calls(&body, target_name, out);
            }
        }
    } else if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_matcher_calls(&child, target_name, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageSpies, "cops/rspec/message_spies");
    crate::cop_autocorrect_fixture_tests!(MessageSpies, "cops/rspec/message_spies");

    #[test]
    fn compound_receive_produces_two_offenses() {
        let source = b"expect(foo).to receive(:bar).and receive(:baz)\n";
        let diags = crate::testutil::run_cop_full(&MessageSpies, source);
        assert_eq!(
            diags.len(),
            2,
            "compound receive should produce 2 offenses: {:?}",
            diags
        );
        // First at `receive(:bar)` column 15
        assert_eq!(diags[0].location.column, 15);
        // Second at `receive(:baz)` column 33
        assert_eq!(diags[1].location.column, 33);
    }

    #[test]
    fn receive_style_flags_have_received() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("receive".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).to have_received(:bar)\n";
        let diags = crate::testutil::run_cop_full_with_config(&MessageSpies, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("receive"));
    }

    #[test]
    fn receive_style_autocorrects_have_received_to_receive() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_autocorrect_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("receive".into()),
            )]),
            ..CopConfig::default()
        };

        assert_cop_autocorrect_with_config(
            &MessageSpies,
            b"expect(foo).to have_received(:bar)\n",
            b"expect(foo).to receive(:bar)\n",
            config,
        );
    }
}
