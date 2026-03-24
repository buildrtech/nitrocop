use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (FP=119→96→0, FN=454→0):
///
/// FP fix 1 (FP=119): Heredoc concatenation (e.g., `<<EOM + code`) — RuboCop doesn't flag
/// because heredocs can't be converted to interpolation. Fixed by checking opening `<<`.
///
/// FP fix 2 (FP=96): Percent literal concatenation (e.g., `config + %[...]`, `header + %{...}`).
/// In Prism, percent literals without interpolation parse as StringNode, but in Parser they're
/// dstr (not str_type?). RuboCop's `str_type?` matcher excludes dstr, so it doesn't flag these.
/// Fixed by checking if the StringNode's opening starts with `%`.
///
/// FN fix (FN=454): Two root causes:
/// 1. Multiline skip was too broad — skipped all multiline `str + str` regardless of where `+`
///    appeared. RuboCop only skips "line-end concatenation" where `+\s*\n` pattern exists (the `+`
///    is at the end of the line). With backslash continuation (`"str" \` + newline + `"str"`), the
///    `+` is at the start of the next line, so RuboCop flags it. Fixed by checking for `+\s*\n`.
/// 2. Dedup was inverted — skipped outer nodes when receiver was a concat chain, meaning only the
///    innermost was flagged. But inner nodes often get skipped by line-end-concat check while the
///    middle/outer nodes (with CallNode receivers, not str_type?) should still fire. Changed to
///    skip inner nodes when they're part of a larger chain (argument-side dedup).
///
/// FN/FP fix (FN=18273→?, dedup rewrite):
/// Root cause: dedup only checked immediate receiver for `is_string_concat`, not the full
/// receiver chain. In chains like `((user.name + ' <') + user.email) + '>'`, the outer node
/// fired because its receiver `(user.name + ' <') + user.email` was NOT string_concat (neither
/// side was str), so dedup didn't suppress it. Meanwhile the inner also fired → duplicate.
/// And in chains where no direct string-concat node existed at the `+` level being checked,
/// nothing fired at all → FN. Fix: walk the full receiver chain to find if any inner `+` call
/// would fire (is_string_concat AND NOT line-end-concat AND NOT heredoc). Fire only from the
/// innermost qualifying node. Also match RuboCop's `find_topmost_plus_node` / `collect_parts`
/// approach: conservative mode checks leftmost part of entire chain, not just immediate receiver.
pub struct StringConcatenation;

impl StringConcatenation {
    fn is_string_literal(node: &ruby_prism::Node<'_>) -> bool {
        // Only match plain StringNode (str_type? in RuboCop), NOT InterpolatedStringNode (dstr).
        // RuboCop's node matcher uses str_type? which excludes dstr, so `foo + "#{bar}"`
        // is not flagged when neither side is a plain string literal.
        // Also exclude percent literals (%[...], %{...}, %(...), %Q[...], %q[...]) — in Prism
        // these are StringNode but in Parser they're dstr (not str_type?).
        if let Some(s) = node.as_string_node() {
            if let Some(opening) = s.opening_loc() {
                let slice = opening.as_slice();
                // Exclude heredocs (opening starts with <<) and percent literals (opening starts with %)
                if slice.starts_with(b"<<") || slice.starts_with(b"%") {
                    return false;
                }
            }
            return true;
        }
        false
    }

    /// Check if a node is a heredoc. In Prism, heredocs are StringNode or
    /// InterpolatedStringNode whose opening starts with `<<`. RuboCop does not flag
    /// concatenation involving heredocs because they can't be converted to interpolation.
    fn is_heredoc(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(s) = node.as_string_node() {
            return s
                .opening_loc()
                .is_some_and(|loc| loc.as_slice().starts_with(b"<<"));
        }
        if let Some(s) = node.as_interpolated_string_node() {
            return s
                .opening_loc()
                .is_some_and(|loc| loc.as_slice().starts_with(b"<<"));
        }
        false
    }

    /// Check if this is a `+` call with exactly one argument and a receiver.
    fn is_plus_call(call: &ruby_prism::CallNode<'_>) -> bool {
        if call.name().as_slice() != b"+" {
            return false;
        }
        if let Some(args) = call.arguments() {
            let count = args.arguments().iter().count();
            return count == 1 && call.receiver().is_some();
        }
        false
    }

    /// Check if this `+` call is a string concatenation (at least one side is a string literal)
    fn is_string_concat(call: &ruby_prism::CallNode<'_>) -> bool {
        if !Self::is_plus_call(call) {
            return false;
        }
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(receiver) = call.receiver() {
                // Either side must be a string literal
                return Self::is_string_literal(&receiver) || Self::is_string_literal(&arg_list[0]);
            }
        }
        false
    }

    /// Check if this is a line-end concatenation: both sides are string literals, the
    /// expression spans multiple lines, and the `+` is at the end of a line (followed
    /// by whitespace and newline). Matches RuboCop's `line_end_concatenation?` which
    /// checks `node.source.match?(/\+\s*\n/)`.
    fn is_line_end_concatenation(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> bool {
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return false,
        };
        let args = match call.arguments() {
            Some(a) => a,
            None => return false,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return false;
        }

        // Both sides must be string literals
        if !Self::is_string_literal(&receiver) || !Self::is_string_literal(&arg_list[0]) {
            return false;
        }

        // Must be multiline
        let (recv_line, _) = source.offset_to_line_col(receiver.location().start_offset());
        let (arg_line, _) = source.offset_to_line_col(arg_list[0].location().start_offset());
        if recv_line == arg_line {
            return false;
        }

        // The `+` must be at the end of a line (followed by optional whitespace and newline).
        // Extract the source text between receiver end and argument start.
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return false,
        };
        let plus_offset = msg_loc.start_offset();
        let arg_start = arg_list[0].location().start_offset();
        // Check bytes after the `+` up to the argument
        let src = source.as_bytes();
        if plus_offset < arg_start.min(src.len()) {
            let between = &src[plus_offset + 1..arg_start.min(src.len())];
            // Must contain a newline (meaning `+` is at end of line, not start of next line)
            return between.contains(&b'\n');
        }
        false
    }

    /// Check if any `+` call in the receiver chain would independently fire
    /// (is_string_concat AND NOT line_end_concatenation AND NOT heredoc-involved).
    /// This is used for dedup: if an inner node will fire, the outer should not.
    fn has_inner_firing_node(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if Self::is_plus_call(&recv_call) {
                    // Check if this receiver `+` call would fire
                    if Self::is_string_concat(&recv_call)
                        && !Self::is_line_end_concatenation(source, &recv_call)
                        && !Self::chain_has_heredoc(&recv_call)
                    {
                        return true;
                    }
                    // Recurse: check deeper in the chain
                    return Self::has_inner_firing_node(source, &recv_call);
                }
            }
        }
        false
    }

    /// Check if any part in the `+` chain involves a heredoc.
    fn chain_has_heredoc(call: &ruby_prism::CallNode<'_>) -> bool {
        // Check immediate receiver and argument
        if let Some(receiver) = call.receiver() {
            if Self::is_heredoc(&receiver) {
                return true;
            }
            // Recurse through receiver chain
            if let Some(recv_call) = receiver.as_call_node() {
                if Self::is_plus_call(&recv_call) && Self::chain_has_heredoc(&recv_call) {
                    return true;
                }
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if Self::is_heredoc(&arg) {
                    return true;
                }
            }
        }
        false
    }

    /// Find the leftmost (deepest) non-`+` part of the chain. Used for conservative mode.
    fn leftmost_part<'a>(call: &ruby_prism::CallNode<'a>) -> Option<ruby_prism::Node<'a>> {
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if Self::is_plus_call(&recv_call) {
                    return Self::leftmost_part(&recv_call);
                }
            }
            return Some(receiver);
        }
        None
    }
}

impl Cop for StringConcatenation {
    fn name(&self) -> &'static str {
        "Style/StringConcatenation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if !Self::is_string_concat(&call) {
            return;
        }

        // Skip line-end concatenation where both sides are string literals, the
        // expression spans multiple lines, and the `+` is at the end of a line.
        // This is handled by Style/LineEndConcatenation instead.
        if Self::is_line_end_concatenation(source, &call) {
            return;
        }

        // Skip concatenation involving heredocs anywhere in the chain
        if Self::chain_has_heredoc(&call) {
            return;
        }

        // Dedup chains: if any inner `+` call in the receiver chain would
        // independently fire (is_string_concat, not line-end-concat, not heredoc),
        // skip this node. The inner one will fire at the same start position.
        // This matches RuboCop's behavior of reporting one offense per chain.
        if Self::has_inner_firing_node(source, &call) {
            return;
        }

        // Conservative mode: check if the leftmost part of the entire chain is a
        // string literal. RuboCop walks up to the topmost `+` node, collects all
        // parts, and checks `parts.first.str_type?`.
        let mode = config.get_str("Mode", "aggressive");
        if mode == "conservative" {
            if let Some(leftmost) = Self::leftmost_part(&call) {
                if !Self::is_string_literal(&leftmost) {
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer string interpolation to string concatenation.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringConcatenation, "cops/style/string_concatenation");
}
