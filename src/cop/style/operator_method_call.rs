use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/OperatorMethodCall — flags redundant dot before binary operator methods.
///
/// Investigation (2026-03-15): 61 FPs, mostly from xiki repo patterns like `Tree.<<(result)`
/// and `Image.>> dest`. Root cause: RuboCop's `on_send` returns early when the receiver is
/// a constant (`node.receiver.const_type?`), because removing the dot before an operator
/// on a constant creates parsing ambiguity (e.g., `Tree << result` could be a heredoc).
/// Also excludes splat/kwsplat/forwarded args (`INVALID_SYNTAX_ARG_TYPES`), since removing
/// the dot would produce invalid syntax.
///
/// Fix: Added constant-receiver check and invalid-argument-type check to match RuboCop behavior.
///
/// Investigation (2026-03-15): 18 remaining FPs from parenthesized operator calls nested
/// inside other method calls, e.g. `expect(one.==(two))`, `assert_equal 0, @c2.<=>(@c2)`.
/// RuboCop's `method_call_with_parenthesized_arg?` skips when the operator call is
/// parenthesized AND its parent is another send node. Without parent pointers in Prism,
/// we use two text-based heuristics:
/// A. After closing paren: `)` or `,` means nested in another call's argument list
/// B. Before receiver: `(` or `,` means the operator call is inside another call's args
/// This catches both parenthesized (`expect(foo.==(bar))`) and non-parenthesized
/// (`assert_equal 0, foo.<=>(bar)`) outer calls.
///
/// Fix: Extended post-closing-paren check to also skip `)` or `,`, and added pre-receiver
/// check scanning backwards for `(` or `,`.
///
/// Remaining FPs (~3): cases where the operator call is an argument to another operator
/// (e.g., `should == bd.%(x)`) or a no-paren unary call (e.g., `assert_nil @c2.<=>(x)`).
/// These are rare and would require AST parent pointers to fix.
pub struct OperatorMethodCall;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"<<",
    b">>", b"|", b"&", b"^",
];

impl Cop for OperatorMethodCall {
    fn name(&self) -> &'static str {
        "Style/OperatorMethodCall"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be an operator method
        if !OPERATOR_METHODS.contains(&method_bytes) {
            return;
        }

        // Must have a receiver, and receiver must not be a constant
        // RuboCop skips const_type? receivers (e.g., `Tree.<<(result)`)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        {
            return;
        }

        // Must have a dot call operator (redundant dot before operator)
        let call_op = match call.call_operator_loc() {
            Some(op) => op,
            None => return,
        };

        if call_op.as_slice() != b"." {
            return;
        }

        // Must have exactly one argument (binary operator)
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
            // Skip splat, kwsplat, forwarded args — removing dot would be
            // invalid syntax (RuboCop's INVALID_SYNTAX_ARG_TYPES)
            let arg = &arg_list[0];
            if arg.as_splat_node().is_some() || arg.as_assoc_splat_node().is_some() {
                return;
            }
            // kwsplat may also appear inside a keyword_hash_node wrapper
            if let Some(kh) = arg.as_keyword_hash_node() {
                if kh
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
            }
        } else {
            // Unary operator with dot is also wrong but less common
            // Only flag binary operators
            return;
        }

        // Skip `foo.-(bar).baz` pattern and `expect(foo.==(bar))` pattern:
        // RuboCop's `method_call_with_parenthesized_arg?` skips when:
        // 1. The operator call is parenthesized AND chained (used as receiver), OR
        // 2. The operator call is parenthesized AND nested inside another method call's arguments
        // Without parent pointers, we detect nesting two ways:
        // A. Check AFTER the closing paren: `.`/`&.` (chain), `)` or `,` (nested in call args)
        // B. Check BEFORE the receiver: `(` or `,` means we're inside another call's argument list
        //    This catches cases like `assert_equal 0, @c2.<=>(@c2)` where `)` is at end of line
        if call.opening_loc().is_some() {
            if let Some(close) = call.closing_loc() {
                let src = source.as_bytes();

                // Check A: what follows the closing paren
                let end_off = close.start_offset() + close.as_slice().len();
                let mut pos = end_off;
                while pos < src.len()
                    && (src[pos] == b' '
                        || src[pos] == b'\t'
                        || src[pos] == b'\n'
                        || src[pos] == b'\r')
                {
                    pos += 1;
                }
                if pos < src.len() {
                    let ch = src[pos];
                    // Dot/safe-nav → chaining: `foo.-(bar).baz`
                    if ch == b'.' || (pos + 1 < src.len() && ch == b'&' && src[pos + 1] == b'.') {
                        return;
                    }
                    // Closing paren or comma → nested in another call: `expect(foo.==(bar))`
                    if ch == b')' || ch == b',' {
                        return;
                    }
                }

                // Check B: what precedes the receiver (scan backwards, skip whitespace)
                // Catches `assert_equal 0, @c2.<=>(@c2)` where `,` is before receiver
                let recv_start = receiver.location().start_offset();
                if recv_start > 0 {
                    let mut rpos = recv_start - 1;
                    while rpos > 0
                        && (src[rpos] == b' '
                            || src[rpos] == b'\t'
                            || src[rpos] == b'\n'
                            || src[rpos] == b'\r')
                    {
                        rpos -= 1;
                    }
                    let prev_ch = src[rpos];
                    if prev_ch == b'(' || prev_ch == b',' {
                        return;
                    }
                }
            }
        }

        let (line, column) = source.offset_to_line_col(call_op.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Redundant dot detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OperatorMethodCall, "cops/style/operator_method_call");
}
