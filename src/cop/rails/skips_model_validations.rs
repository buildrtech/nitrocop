use crate::cop::node_type::{
    ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, HASH_NODE,
    KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/SkipsModelValidations — flags methods that skip ActiveRecord validations.
///
/// ## Investigation (2026-03-10)
///
/// Root causes of FP/FN vs RuboCop:
///
/// 1. **FN: receiver-required check (major, ~1600 FN)** — Our code returned early when
///    `call.receiver().is_none()`, but RuboCop flags bare calls (implicit self) like
///    `insert(attrs, returning: false)`. Removed the receiver check entirely.
///
/// 2. **FN: diagnostic location on wrong line** — Used `node.location()` (entire call
///    expression starting at receiver) instead of `call.message_loc()` (method name only).
///    For multi-line calls like `User\n  .touch`, this reported line 1 instead of line 2,
///    creating phantom FP on line 1 and FN on line 2.
///
/// 3. **FP: `good_insert?` logic inversion** — Our code flagged insert calls if ANY hash
///    key was `:returning`/`:unique_by`. RuboCop's pattern skips (doesn't flag) if ANY key
///    is NOT those AR-specific keys. Fixed to match: only flag if ALL keys are AR-specific.
///
/// 4. **FN: missing `touch_all`** — Not in SKIP_METHODS fallback list. Added it.
///
/// 5. **FP: `FileUtils` constant path check too loose** — Exempted `Foo::FileUtils.touch`
///    but RuboCop only exempts `FileUtils.touch` and `::FileUtils.touch`. Fixed to check
///    that ConstantPathNode has nil parent (cbase).
///
/// 6. **FN: `good_touch?` should only match `send`, not `csend`** — RuboCop's pattern uses
///    `(send ...)` not `(call ...)`, so `obj&.touch(true)` is NOT exempted. Added check for
///    safe navigation operator.
///
/// ## Investigation (2026-03-15)
///
/// **FP root cause (5 FP):** RuboCop disable comments using `Rails::SkipsModelValidations`
/// (double-colon separator) were not being honored — nitrocop only recognized `Rails/SkipsModelValidations`
/// (slash separator). Fixed in `src/parse/directives.rs` by normalizing `::` to `/` when
/// parsing disable directives. This is a general fix affecting all cops.
pub struct SkipsModelValidations;

const SKIP_METHODS: &[&[u8]] = &[
    b"update_attribute",
    b"touch",
    b"touch_all",
    b"update_column",
    b"update_columns",
    b"update_all",
    b"toggle!",
    b"increment!",
    b"decrement!",
    b"insert",
    b"insert!",
    b"insert_all",
    b"insert_all!",
    b"upsert",
    b"upsert_all",
    b"increment_counter",
    b"decrement_counter",
    b"update_counters",
];

impl Cop for SkipsModelValidations {
    fn name(&self) -> &'static str {
        "Rails/SkipsModelValidations"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
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
        let forbidden = config.get_string_array("ForbiddenMethods");
        let allowed = config.get_string_array("AllowedMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let method_name = call.name().as_slice();
        let method_str = std::str::from_utf8(method_name).unwrap_or("");

        // Use ForbiddenMethods if configured, otherwise fall back to hardcoded list
        let is_forbidden = if let Some(ref list) = forbidden {
            list.iter().any(|m| m == method_str)
        } else {
            SKIP_METHODS.contains(&method_name)
        };

        if !is_forbidden {
            return;
        }

        // Skip if method is in AllowedMethods
        if let Some(ref list) = allowed {
            if list.iter().any(|m| m == method_str) {
                return;
            }
        }

        // RuboCop: METHODS_WITH_ARGUMENTS — skip if the method is in this list
        // and has no arguments (e.g. `User.toggle!` with no args).
        let methods_with_args: &[&[u8]] = &[
            b"decrement!",
            b"decrement_counter",
            b"increment!",
            b"increment_counter",
            b"insert",
            b"insert!",
            b"insert_all",
            b"insert_all!",
            b"toggle!",
            b"update_all",
            b"update_attribute",
            b"update_column",
            b"update_columns",
            b"update_counters",
            b"upsert",
            b"upsert_all",
        ];
        if methods_with_args.contains(&method_name) && call.arguments().is_none() {
            return;
        }

        // RuboCop: good_insert? — for insert/insert!, skip when the call looks like
        // String#insert or Array#insert rather than ActiveRecord insert.
        // Pattern: (call _ {:insert :insert!} _ { !(hash ...) | (hash <(pair (sym !{:returning :unique_by}) _) ...>) } ...)
        // This means: skip if 2+ args AND second arg is not a hash, OR is a hash where
        // at least one key is NOT :returning/:unique_by (i.e., not purely AR-specific keys).
        // Uses `call` (matches both send and csend).
        if method_name == b"insert" || method_name == b"insert!" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() >= 2 {
                    let second = &arg_list[1];
                    let is_good = if let Some(hash) = second.as_hash_node() {
                        // It's a hash — "good" (not AR) if at least one key is NOT :returning/:unique_by
                        hash_has_non_ar_key(hash.elements().iter())
                    } else if let Some(kw_hash) = second.as_keyword_hash_node() {
                        kw_hash_has_non_ar_key(kw_hash.elements().iter())
                    } else {
                        true // Not a hash at all — not an AR insert (e.g., String#insert)
                    };
                    if is_good {
                        return;
                    }
                }
            }
        }

        // RuboCop: good_touch? — FileUtils.touch or _.touch(boolean)
        // Uses `send` (NOT `call`), so safe navigation `&.touch(true)` is NOT exempted.
        let is_safe_nav = call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.");
        if method_name == b"touch" && !is_safe_nav {
            if let Some(recv) = call.receiver() {
                // (send (const {nil? cbase} :FileUtils) :touch ...) — only bare or top-level
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"FileUtils" {
                        return;
                    }
                }
                if let Some(cp) = recv.as_constant_path_node() {
                    // Only match ::FileUtils (cbase — parent is nil), not Foo::FileUtils
                    if cp.parent().is_none() {
                        if let Some(name) = cp.name() {
                            if name.as_slice() == b"FileUtils" {
                                return;
                            }
                        }
                    }
                }
            }
            // (send _ :touch boolean) — touch with a single boolean argument
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    let first = &arg_list[0];
                    if first.as_true_node().is_some() || first.as_false_node().is_some() {
                        return;
                    }
                }
            }
        }

        // Report at method name location (node.loc.selector in RuboCop)
        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = format!("Avoid using `{}` because it skips validations.", method_str);
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

/// Check if a hash has at least one key that is NOT :returning or :unique_by.
fn hash_has_non_ar_key<'a>(elements: impl Iterator<Item = ruby_prism::Node<'a>>) -> bool {
    for elem in elements {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                let name: &[u8] = sym.unescaped();
                if name != b"returning" && name != b"unique_by" {
                    return true;
                }
            } else {
                // Non-symbol key — not an AR-specific key
                return true;
            }
        }
    }
    false
}

/// Check if a keyword hash has at least one key that is NOT :returning or :unique_by.
fn kw_hash_has_non_ar_key<'a>(elements: impl Iterator<Item = ruby_prism::Node<'a>>) -> bool {
    for elem in elements {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                let name: &[u8] = sym.unescaped();
                if name != b"returning" && name != b"unique_by" {
                    return true;
                }
            } else {
                // Non-symbol key — not an AR-specific key
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SkipsModelValidations, "cops/rails/skips_model_validations");
}
