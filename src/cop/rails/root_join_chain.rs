use crate::cop::node_type::CALL_NODE;
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::VecDeque;

pub struct RootJoinChain;

impl Cop for RootJoinChain {
    fn name(&self) -> &'static str {
        "Rails/RootJoinChain"
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

        if call.name().as_slice() != b"join" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if recv_call.name().as_slice() != b"join" {
            return;
        }

        if !chain_starts_with_rails_root(recv_call) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use a single `join` with multiple arguments instead of chaining.".to_string(),
        );

        if !has_chained_join_after(source, loc.end_offset())
            && let Some((base_call, all_args)) = flatten_join_chain(call, source)
            && all_args.len() > 1
            && let Some(ref mut corr) = corrections
        {
            let base_loc = base_call.location();
            let base_src = source
                .byte_slice(base_loc.start_offset(), base_loc.end_offset(), "Rails.root")
                .to_string();
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: format!("{base_src}.join({})", all_args.join(", ")),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

fn chain_starts_with_rails_root(call: ruby_prism::CallNode<'_>) -> bool {
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    if let Some(recv_call) = recv.as_call_node() {
        if recv_call.name().as_slice() == b"join" {
            return chain_starts_with_rails_root(recv_call);
        }
        if recv_call.name().as_slice() == b"root" || recv_call.name().as_slice() == b"public_path" {
            if let Some(r) = recv_call.receiver() {
                return util::constant_name(&r) == Some(b"Rails");
            }
        }
    }

    false
}

fn flatten_join_chain<'a>(
    call: ruby_prism::CallNode<'a>,
    source: &SourceFile,
) -> Option<(ruby_prism::CallNode<'a>, Vec<String>)> {
    let mut args: VecDeque<String> = VecDeque::new();
    let mut current = call;

    loop {
        if current.name().as_slice() != b"join" {
            return None;
        }

        if let Some(arguments) = current.arguments() {
            let mut call_args = Vec::new();
            for arg in arguments.arguments().iter() {
                let loc = arg.location();
                call_args.push(
                    source
                        .byte_slice(loc.start_offset(), loc.end_offset(), "")
                        .to_string(),
                );
            }
            for arg in call_args.into_iter().rev() {
                args.push_front(arg);
            }
        }

        let recv = current.receiver()?.as_call_node()?;
        if recv.name().as_slice() == b"join" {
            current = recv;
            continue;
        }

        if (recv.name().as_slice() == b"root" || recv.name().as_slice() == b"public_path")
            && recv
                .receiver()
                .is_some_and(|r| util::constant_name(&r) == Some(b"Rails"))
        {
            return Some((recv, args.into_iter().collect()));
        }

        return None;
    }
}

fn has_chained_join_after(source: &SourceFile, end_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut i = end_offset;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    bytes.get(i) == Some(&b'.')
        && bytes
            .get(i + 1..i + 5)
            .is_some_and(|slice| slice == b"join")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootJoinChain, "cops/rails/root_join_chain");
    crate::cop_autocorrect_fixture_tests!(RootJoinChain, "cops/rails/root_join_chain");

    #[test]
    fn supports_autocorrect() {
        assert!(RootJoinChain.supports_autocorrect());
    }
}
