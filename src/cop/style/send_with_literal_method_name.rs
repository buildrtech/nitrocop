use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus FP=9→0: All 9 FPs were `public_send(:[], N)` — operator method names like `[]`, `+`,
/// `*`, etc. were incorrectly treated as valid direct-call targets. RuboCop only considers
/// identifier methods matching `/[a-z_][a-z0-9_]*[!?]?/i` as replaceable. Removed the
/// OPERATOR_METHODS allowlist so operators fall through to the identifier regex (which rejects them).
pub struct SendWithLiteralMethodName;

/// Valid Ruby method name: starts with letter/underscore, contains alphanumerics/underscores,
/// optionally ends with ! or ?
fn is_valid_ruby_method_name(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check for reserved words that cannot be used as direct method calls
    const RESERVED_WORDS: &[&[u8]] = &[
        b"BEGIN",
        b"END",
        b"alias",
        b"and",
        b"begin",
        b"break",
        b"case",
        b"class",
        b"def",
        b"defined?",
        b"do",
        b"else",
        b"elsif",
        b"end",
        b"ensure",
        b"false",
        b"for",
        b"if",
        b"in",
        b"module",
        b"next",
        b"nil",
        b"not",
        b"or",
        b"redo",
        b"rescue",
        b"retry",
        b"return",
        b"self",
        b"super",
        b"then",
        b"true",
        b"undef",
        b"unless",
        b"until",
        b"when",
        b"while",
        b"yield",
    ];
    if RESERVED_WORDS.contains(&name) {
        return false;
    }

    // Match /\A[a-zA-Z_][a-zA-Z0-9_]*[!?]?\z/
    let first = name[0];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    let last = *name.last().unwrap();
    let check_end = if last == b'!' || last == b'?' {
        &name[1..name.len() - 1]
    } else {
        &name[1..]
    };

    check_end
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

impl Cop for SendWithLiteralMethodName {
    fn name(&self) -> &'static str {
        "Style/SendWithLiteralMethodName"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
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
        let allow_send = config.get_bool("AllowSend", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();

        // Check for public_send, __send__, or send
        // When AllowSend is true (default), only public_send is flagged.
        // When AllowSend is false, send and __send__ are also flagged.
        let is_target =
            name == b"public_send" || (!allow_send && (name == b"__send__" || name == b"send"));

        if !is_target {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a static symbol or string with a valid Ruby method name.
        // Setter methods (ending in =) can't be converted because behavior differs.
        // Names with special chars (hyphens, dots, brackets, etc.) require send/public_send.
        // Reserved words (class, if, end, etc.) can't be used as direct method calls.
        let is_valid_literal = if let Some(sym) = arg_list[0].as_symbol_node() {
            let name = sym.unescaped();
            !name.ends_with(b"=") && is_valid_ruby_method_name(name)
        } else if let Some(s) = arg_list[0].as_string_node() {
            let content = s.unescaped();
            !content.ends_with(b"=") && is_valid_ruby_method_name(content)
        } else {
            false
        };

        if !is_valid_literal {
            return;
        }

        let method_name = if let Some(sym) = arg_list[0].as_symbol_node() {
            std::str::from_utf8(sym.unescaped())
                .ok()
                .map(|s| s.to_string())
        } else if let Some(s) = arg_list[0].as_string_node() {
            std::str::from_utf8(s.unescaped())
                .ok()
                .map(|s| s.to_string())
        } else {
            None
        };

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Use a direct method call instead of `send` with a literal method name.".to_string(),
        );

        if let (Some(corr), Some(method_name)) = (corrections.as_mut(), method_name) {
            // Skip block forms with an explicit block body (`do ... end` / `{ ... }`).
            // Keep corrections local to call syntax transformations only.
            if call.block().is_none_or(|b| b.as_block_node().is_none()) {
                let mut replacement = String::new();
                if let Some(receiver) = call.receiver() {
                    let recv_loc = receiver.location();
                    replacement.push_str(source.byte_slice(
                        recv_loc.start_offset(),
                        recv_loc.end_offset(),
                        "",
                    ));
                    if let Some(op) = call.call_operator_loc() {
                        replacement.push_str(std::str::from_utf8(op.as_slice()).unwrap_or("."));
                    } else {
                        replacement.push('.');
                    }
                }
                replacement.push_str(&method_name);

                let mut tail_args: Vec<String> = arg_list[1..]
                    .iter()
                    .map(|arg| {
                        let arg_loc = arg.location();
                        source
                            .byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "")
                            .to_string()
                    })
                    .collect();

                if let Some(block_arg) = call.block().and_then(|b| b.as_block_argument_node()) {
                    let b_loc = block_arg.location();
                    tail_args.push(
                        source
                            .byte_slice(b_loc.start_offset(), b_loc.end_offset(), "")
                            .to_string(),
                    );
                }

                if !tail_args.is_empty() {
                    replacement.push('(');
                    replacement.push_str(&tail_args.join(", "));
                    replacement.push(')');
                }

                let call_loc = call.location();
                corr.push(crate::correction::Correction {
                    start: call_loc.start_offset(),
                    end: call_loc.end_offset(),
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SendWithLiteralMethodName,
        "cops/style/send_with_literal_method_name"
    );
    crate::cop_autocorrect_fixture_tests!(
        SendWithLiteralMethodName,
        "cops/style/send_with_literal_method_name"
    );
}
