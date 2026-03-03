use std::collections::HashSet;

use crate::cop::node_type::{ASSOC_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Returns true if a node is a pure literal (no method calls or variable references).
/// Matches RuboCop's behavior: only flag duplicate keys when the key is entirely literal.
fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_symbol_node().is_some()
        || node.as_string_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_source_encoding_node().is_some()
    {
        return true;
    }

    // Regular expression without interpolation
    if node.as_regular_expression_node().is_some() {
        return true;
    }

    // Parenthesized expression: (expr) - literal if body is literal
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                return body_stmts.len() == 1 && is_literal(&body_stmts[0]);
            }
        }
        return false;
    }

    // Array literal: [a, b, c] - literal if all elements are literal
    if let Some(array) = node.as_array_node() {
        return array.elements().iter().all(|e| is_literal(&e));
    }

    // Hash literal: { a: 1, b: 2 } - literal if all keys and values are literal
    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal(&assoc.key()) && is_literal(&assoc.value())
            } else {
                false // splat is not literal
            }
        });
    }

    // Range: (1..10) or (1...) - literal if both endpoints are literal
    if let Some(range) = node.as_range_node() {
        let left_ok = range.left().as_ref().is_none_or(|n| is_literal(n));
        let right_ok = range.right().as_ref().is_none_or(|n| is_literal(n));
        return left_ok && right_ok;
    }

    // Unary operators: !true, -1 - literal if operand is literal
    // Binary operators on literals: (false && true), (false <=> true)
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"!" || name == b"-@" || name == b"+@")
            && call.arguments().is_none()
            && call.receiver().is_some()
        {
            return is_literal(&call.receiver().unwrap());
        }
        if let Some(recv) = call.receiver() {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 && call.block().is_none() {
                    return is_literal(&recv) && is_literal(&arg_list[0]);
                }
            }
        }
        return false;
    }

    // `and` / `or` keywords: (x and y), (x or y) - literal if both sides are literal
    if let Some(and_node) = node.as_and_node() {
        return is_literal(&and_node.left()) && is_literal(&and_node.right());
    }
    if let Some(or_node) = node.as_or_node() {
        return is_literal(&or_node.left()) && is_literal(&or_node.right());
    }

    // Interpolated string: "#{2}" is literal if all parts are literal
    if let Some(interp_str) = node.as_interpolated_string_node() {
        return interp_str.parts().iter().all(|part| {
            if part.as_string_node().is_some() {
                true
            } else if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    body.len() == 1 && is_literal(&body[0])
                } else {
                    false
                }
            } else {
                false
            }
        });
    }

    // Interpolated regex: /#{2}/ is literal if all parts are literal
    if let Some(interp_re) = node.as_interpolated_regular_expression_node() {
        return interp_re.parts().iter().all(|part| {
            if part.as_string_node().is_some() {
                true
            } else if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    body.len() == 1 && is_literal(&body[0])
                } else {
                    false
                }
            } else {
                false
            }
        });
    }

    // Constant reads (KEY, Foo::BAR) are considered literal by RuboCop
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        return true;
    }

    false
}

pub struct DuplicateHashKey;

impl Cop for DuplicateHashKey {
    fn name(&self) -> &'static str {
        "Lint/DuplicateHashKey"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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
        let elements = if let Some(hash_node) = node.as_hash_node() {
            hash_node.elements()
        } else if let Some(kw_hash) = node.as_keyword_hash_node() {
            kw_hash.elements()
        } else {
            return;
        };

        let mut seen = HashSet::new();

        for element in elements.iter() {
            let assoc = match element.as_assoc_node() {
                Some(a) => a,
                None => continue, // skip AssocSplatNode (**)
            };

            let key = assoc.key();

            if !is_literal(&key) {
                continue;
            }

            let key_loc = key.location();
            let key_text = key_loc.as_slice();

            if !seen.insert(key_text.to_vec()) {
                let (line, column) = source.offset_to_line_col(key_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Duplicated key in hash literal.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateHashKey, "cops/lint/duplicate_hash_key");
}
