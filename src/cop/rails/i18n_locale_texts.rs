use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/I18nLocaleTexts: Enforces use of I18n and locale files instead of locale-specific strings.
///
/// ## Investigation (2026-03-15)
///
/// **FP root cause (3 FPs):** All from `flash[:error] = "string"` in the Autolab repo where
/// `flash` is a local variable (assigned earlier in the method). RuboCop's pattern only matches
/// `(send nil? :flash)` (method call), NOT `(lvar :flash)` (local variable read). Nitrocop was
/// incorrectly matching local variable reads of `flash` in `is_flash_receiver`.
/// **Fix:** Remove local variable `flash` handling from `is_flash_receiver`.
///
/// **FN root cause (8 FNs):** RuboCop uses `def_node_search` which recursively searches the
/// entire AST subtree of call arguments for matching pairs. Nitrocop only searched at specific
/// nesting depths. Missed patterns:
/// - `validates :title, message: "text"` (top-level `:message` keyword, not nested in hash)
/// - `redirect_to url(notice: "text")` (`:notice`/`:alert` nested inside URL helper call args)
/// - `redirect_to path, flash: { notice: "text" }` (`:notice`/`:alert` inside `flash:` hash)
/// - `mail({ subject: "text" }.merge!(hash))` (`:subject` in hash literal arg, not keyword arg)
///
/// **Fix:** Implement recursive AST search for `(pair (sym :key) str)` patterns,
/// matching RuboCop's `def_node_search` behavior.
pub struct I18nLocaleTexts;

const MSG: &str = "Move locale texts to the locale files in the `config/locales` directory.";

/// Check if a node is a plain string literal (not a symbol, not interpolated).
fn is_string_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
}

/// Recursively search a node's subtree for `(pair (sym :key) str)` patterns.
/// Mirrors RuboCop's `def_node_search` which walks the entire AST subtree.
fn find_pairs_recursive<'a>(
    node: &ruby_prism::Node<'a>,
    key: &[u8],
    results: &mut Vec<ruby_prism::Node<'a>>,
) {
    // Check if this node is an assoc (pair) with matching key and string value
    if let Some(assoc) = node.as_assoc_node() {
        if let Some(sym) = assoc.key().as_symbol_node() {
            if sym.unescaped() == key && is_string_literal(&assoc.value()) {
                results.push(assoc.value());
                return; // Don't recurse further into this pair
            }
        }
        // Recurse into assoc value (could contain nested hashes)
        find_pairs_recursive(&assoc.value(), key, results);
        return;
    }

    // KeywordHashNode: recurse into elements
    if let Some(kw) = node.as_keyword_hash_node() {
        for elem in kw.elements().iter() {
            find_pairs_recursive(&elem, key, results);
        }
        return;
    }

    // HashNode: recurse into elements
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            find_pairs_recursive(&elem, key, results);
        }
        return;
    }

    // CallNode: recurse into receiver and arguments
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            find_pairs_recursive(&recv, key, results);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_pairs_recursive(&arg, key, results);
            }
        }
        return;
    }

    // ArgumentsNode: recurse into each argument
    if let Some(args) = node.as_arguments_node() {
        for arg in args.arguments().iter() {
            find_pairs_recursive(&arg, key, results);
        }
    }
}

impl Cop for I18nLocaleTexts {
    fn name(&self) -> &'static str {
        "Rails/I18nLocaleTexts"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        let method_name = call.name().as_slice();

        match method_name {
            b"validates" => {
                // Recursively search for (pair (sym :message) str) anywhere in args
                let mut results = Vec::new();
                find_pairs_recursive(node, b"message", &mut results);
                for val in results {
                    let loc = val.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
                return;
            }
            b"redirect_to" | b"redirect_back" => {
                // Recursively search for (pair (sym :notice/:alert) str) anywhere in args
                for key in &[b"notice" as &[u8], b"alert"] {
                    let mut results = Vec::new();
                    find_pairs_recursive(node, key, &mut results);
                    for val in results {
                        let loc = val.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                    }
                }
                return;
            }
            b"mail" => {
                // Recursively search for (pair (sym :subject) str) anywhere in args
                let mut results = Vec::new();
                find_pairs_recursive(node, b"subject", &mut results);
                for val in results {
                    let loc = val.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
            }
            _ => {}
        }

        // Check flash[:notice] = "string" or flash.now[:notice] = "string"
        // This is `[]=` call on `flash` or `flash.now`
        if method_name == b"[]=" {
            if let Some(receiver) = call.receiver() {
                let is_flash = is_flash_receiver(&receiver);
                if is_flash {
                    // The last argument is the assigned value
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 2 && is_string_literal(&arg_list[1]) {
                            let loc = arg_list[1].location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                MSG.to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Check if a node is `flash` or `flash.now` (method call only, not local variable).
/// RuboCop's pattern matches `(send nil? :flash)` and `(send (send nil? :flash) :now)`,
/// which only matches `flash` as a method call (implicit receiver). When `flash` is
/// assigned as a local variable, RuboCop does not flag it.
fn is_flash_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // Direct `flash` call
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"flash" && call.receiver().is_none() {
            return true;
        }
        // `flash.now`
        if call.name().as_slice() == b"now" {
            if let Some(recv) = call.receiver() {
                if let Some(inner_call) = recv.as_call_node() {
                    if inner_call.name().as_slice() == b"flash" && inner_call.receiver().is_none() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(I18nLocaleTexts, "cops/rails/i18n_locale_texts");
}
