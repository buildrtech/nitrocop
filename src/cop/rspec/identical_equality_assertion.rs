use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for equality assertions with identical expressions on both sides.
///
/// ## Investigation notes (2026-03)
///
/// **Root cause of FNs (4, all jruby):**
/// 1. `Obj.method` vs `Obj::method` for lowercase names — In Parser gem (used by
///    RuboCop), both produce `(send (const nil :Obj) :method)`, so `left == right`
///    is true. In Prism, the call_operator differs (`.` vs `::`). Fix: normalize
///    call_operator in AST fingerprint.
/// 2. `%i{}` vs `[]` — Both are empty arrays. Parser gem: `(array)` for both.
///    In Prism, source text differs. Fix: structural fingerprint for ArrayNode.
/// 3. `/[\§]/` vs `/[§]/` — `\§` is a no-op escape. Parser gem stores the
///    unescaped regex content, so they're equal. Fix: use `unescaped()` for regex.
pub struct IdenticalEqualityAssertion;

impl Cop for IdenticalEqualityAssertion {
    fn name(&self) -> &'static str {
        "RSpec/IdenticalEqualityAssertion"
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
        &[CALL_NODE]
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
        // Look for expect(X).to eq(X) / eql(X) / be(X)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Only flag `.to` (not `.not_to`)
        if method_name != b"to" {
            return;
        }

        // Receiver must be expect(X)
        let expect_call = match call.receiver() {
            Some(recv) => match recv.as_call_node() {
                Some(c) => c,
                None => return,
            },
            None => return,
        };

        if expect_call.name().as_slice() != b"expect" {
            return;
        }

        // Get the expect argument
        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let expect_arg_list: Vec<_> = expect_args.arguments().iter().collect();
        if expect_arg_list.len() != 1 {
            return;
        }

        let expect_arg = &expect_arg_list[0];

        // Get the matcher call (eq/eql/be)
        let matcher_args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let matcher_arg_list: Vec<_> = matcher_args.arguments().iter().collect();
        if matcher_arg_list.is_empty() {
            return;
        }

        let matcher_node = &matcher_arg_list[0];
        let matcher_call = match matcher_node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let matcher_name = matcher_call.name().as_slice();
        if matcher_name != b"eq" && matcher_name != b"eql" && matcher_name != b"be" {
            return;
        }

        if matcher_call.receiver().is_some() {
            return;
        }

        let matcher_inner_args = match matcher_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let inner_arg_list: Vec<_> = matcher_inner_args.arguments().iter().collect();
        if inner_arg_list.len() != 1 {
            return;
        }

        let matcher_arg = &inner_arg_list[0];

        // Compare AST structure of both expressions (not source text).
        // RuboCop uses `left == right` on Parser gem AST nodes, which compares
        // structure recursively. We build fingerprints that normalize surface
        // syntax differences (`.` vs `::`, `%i{}` vs `[]`, regex escapes).
        let mut expect_fp = Vec::new();
        let mut matcher_fp = Vec::new();
        ast_fingerprint(source.as_bytes(), expect_arg, &mut expect_fp);
        ast_fingerprint(source.as_bytes(), matcher_arg, &mut matcher_fp);

        if expect_fp == matcher_fp {
            let loc = expect_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Identical expressions on both sides of the equality may indicate a flawed test."
                    .to_string(),
            );
            if let Some(corrections) = corrections {
                corrections.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: "skip('TODO: avoid identical equality assertion')".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
            diagnostics.push(diagnostic);
        }
    }
}

/// Build a structural fingerprint for a node that normalizes surface syntax
/// differences to match RuboCop's Parser-gem AST equality semantics.
///
/// Key normalizations:
/// - CallNode: `.` and `::` call operators are treated identically
/// - ArrayNode: compared structurally (handles `%i{}` vs `[]`)
/// - RegularExpressionNode: uses `unescaped()` content (handles `\§` vs `§`)
/// - StringNode/SymbolNode: uses `unescaped()` content
/// - All other nodes: falls back to source text comparison
fn ast_fingerprint(bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    // CallNode: normalize call operator
    if let Some(call) = node.as_call_node() {
        out.extend_from_slice(b"C:");
        if let Some(recv) = call.receiver() {
            ast_fingerprint(bytes, &recv, out);
            // Normalize `.` and `::` to the same separator
            out.push(b'.');
        }
        out.extend_from_slice(call.name().as_slice());
        out.push(b'(');
        if let Some(args) = call.arguments() {
            for (i, arg) in args.arguments().iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                ast_fingerprint(bytes, &arg, out);
            }
        }
        out.push(b')');
        if let Some(block) = call.block() {
            out.push(b'{');
            ast_fingerprint(bytes, &block, out);
            out.push(b'}');
        }
        return;
    }

    // ArrayNode: structural comparison (handles %i{} vs [] etc.)
    if let Some(array) = node.as_array_node() {
        out.extend_from_slice(b"A:[");
        for (i, elem) in array.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            ast_fingerprint(bytes, &elem, out);
        }
        out.push(b']');
        return;
    }

    // HashNode: structural comparison
    if let Some(hash) = node.as_hash_node() {
        out.extend_from_slice(b"H:{");
        for (i, elem) in hash.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            ast_fingerprint(bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    // KeywordHashNode: same structure as HashNode (keyword args like `foo(a: 1)`)
    if let Some(hash) = node.as_keyword_hash_node() {
        out.extend_from_slice(b"H:{");
        for (i, elem) in hash.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            ast_fingerprint(bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    // AssocNode (hash pair): key => value
    if let Some(assoc) = node.as_assoc_node() {
        ast_fingerprint(bytes, &assoc.key(), out);
        out.extend_from_slice(b"=>");
        ast_fingerprint(bytes, &assoc.value(), out);
        return;
    }

    // RegularExpressionNode: use unescaped content + flags
    if let Some(regex) = node.as_regular_expression_node() {
        out.extend_from_slice(b"R:");
        out.extend_from_slice(regex.unescaped());
        // Include flags (e.g., /i, /m)
        let closing = regex.closing_loc().as_slice();
        if closing.len() > 1 {
            out.push(b'/');
            out.extend_from_slice(&closing[1..]);
        }
        return;
    }

    // StringNode: use unescaped content
    if let Some(string) = node.as_string_node() {
        out.extend_from_slice(b"S:");
        out.extend_from_slice(string.unescaped());
        return;
    }

    // SymbolNode: use unescaped content
    if let Some(sym) = node.as_symbol_node() {
        out.extend_from_slice(b"Y:");
        out.extend_from_slice(sym.unescaped());
        return;
    }

    // ConstantPathNode: Foo::Bar
    if let Some(cp) = node.as_constant_path_node() {
        out.extend_from_slice(b"CP:");
        if let Some(parent) = cp.parent() {
            ast_fingerprint(bytes, &parent, out);
        }
        out.extend_from_slice(b"::");
        if let Some(name) = cp.name() {
            out.extend_from_slice(name.as_slice());
        }
        return;
    }

    // ConstantReadNode: simple constant
    if let Some(cr) = node.as_constant_read_node() {
        out.extend_from_slice(b"CR:");
        out.extend_from_slice(cr.name().as_slice());
        return;
    }

    // Default: use source text for all other node types (integers, floats,
    // local variable reads, etc.)
    let loc = node.location();
    out.extend_from_slice(&bytes[loc.start_offset()..loc.end_offset()]);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IdenticalEqualityAssertion,
        "cops/rspec/identical_equality_assertion"
    );
    crate::cop_autocorrect_fixture_tests!(
        IdenticalEqualityAssertion,
        "cops/rspec/identical_equality_assertion"
    );
}
