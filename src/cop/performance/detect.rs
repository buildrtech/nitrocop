use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

const CANDIDATE_METHODS: &[&[u8]] = &[b"select", b"find_all", b"filter"];

pub struct Detect;

impl Cop for Detect {
    fn name(&self) -> &'static str {
        "Performance/Detect"
    }

    fn uses_node_check(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();

        // Determine the second_method type: first, last, or [0]/[-1]
        let (is_last, is_index) = match outer_method {
            b"first" => (false, false),
            b"last" => (true, false),
            b"[]" => {
                // Must have exactly one integer argument: 0 or -1
                let args = match outer_call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return;
                }
                let arg_text = std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("");
                match arg_text {
                    "0" => (false, true),
                    "-1" => (true, true),
                    _ => return,
                }
            }
            _ => return,
        };

        // Skip safe-navigation on outer call (e.g., items&.select { ... }&.first)
        // RuboCop's NodePattern uses `(send ...)` not `(csend ...)`, so &. chains don't match.
        if outer_call
            .call_operator_loc()
            .is_some_and(|loc| loc.as_slice() == b"&.")
        {
            return;
        }

        // For first/last, must have NO arguments (first(n) / last(n) should not flag)
        if !is_index && outer_call.arguments().is_some() {
            return;
        }

        // Get the inner call (receiver of the outer call)
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner_method = inner_call.name().as_slice();

        // Inner method must be select, find_all, or filter
        if !CANDIDATE_METHODS.contains(&inner_method) {
            return;
        }

        // The inner call must have a block or block_pass
        let inner_block = match inner_call.block() {
            Some(b) => b,
            None => return,
        };

        // RuboCop's Parser gem has separate `block` and `numblock` node types.
        // `numblock` (used for _1/_2 numbered params and Ruby 3.4 `it`) returns
        // false for `block_type?`, causing RuboCop to skip these chains.
        // Match that behavior: skip when the block uses numbered or it params.
        if let Some(block_node) = inner_block.as_block_node() {
            if let Some(params) = block_node.parameters() {
                if params.as_numbered_parameters_node().is_some()
                    || params.as_it_parameters_node().is_some()
                {
                    return;
                }
            }
        }

        // Check if the inner call's receiver is `lazy` with its own receiver
        // e.g., `adapter.lazy.select { }.first` should not flag
        // but `lazy.select { }.first` (bare lazy without receiver) should flag
        if let Some(inner_receiver) = inner_call.receiver() {
            if let Some(lazy_call) = inner_receiver.as_call_node() {
                if lazy_call.name().as_slice() == b"lazy" && lazy_call.receiver().is_some() {
                    return;
                }
            }
        }

        let inner_method_str = std::str::from_utf8(inner_method).unwrap_or("select");
        let prefer = if is_last { "reverse.detect" } else { "detect" };
        let msg = if is_index {
            let idx = if is_last { -1 } else { 0 };
            format!("Use `{prefer}` instead of `{inner_method_str}[{idx}]`.")
        } else {
            let outer = if is_last { "last" } else { "first" };
            format!("Use `{prefer}` instead of `{inner_method_str}.{outer}`.")
        };

        // Report at the inner call's method name (e.g., `select`), matching RuboCop's
        // `receiver.loc.selector` behavior, not the start of the entire expression.
        let inner_msg_loc = inner_call.message_loc().unwrap_or(inner_call.location());
        let (line, column) = source.offset_to_line_col(inner_msg_loc.start_offset());
        let mut diagnostic = self.diagnostic(source, line, column, msg);

        if let Some(ref mut corr) = corrections {
            let inner_receiver = inner_call.receiver().expect("validated by earlier guards");
            let recv_loc = inner_receiver.location();
            let recv_source = source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
            let dot = inner_call
                .call_operator_loc()
                .map(|op| source.byte_slice(op.start_offset(), op.end_offset(), "."))
                .unwrap_or(".");
            let tail = source.byte_slice(
                inner_msg_loc.end_offset(),
                inner_call.location().end_offset(),
                "",
            );
            corr.push(crate::correction::Correction {
                start: outer_call.location().start_offset(),
                end: outer_call.location().end_offset(),
                replacement: format!("{recv_source}{dot}{prefer}{tail}"),
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
    crate::cop_fixture_tests!(Detect, "cops/performance/detect");
    crate::cop_autocorrect_fixture_tests!(Detect, "cops/performance/detect");

    #[test]
    fn supports_autocorrect() {
        assert!(Detect.supports_autocorrect());
    }
}
