use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/FindById cop.
///
/// ## Investigation findings (2026-03-16)
///
/// **Root cause of 2 FNs:** Pattern 2 (`find_by!(id: ...)`) and Pattern 1 (`find_by_id!(...)`)
/// required `call.receiver().is_some()`, but inside module-level class methods (e.g.
/// `extend`ed mixins like `external_id.rb`), these methods are called without an explicit
/// receiver — using implicit `self`. RuboCop's NodePattern uses `_` for the receiver slot which
/// matches `nil` (no receiver), so it fires regardless.
///
/// **Fix:** Removed the `call.receiver().is_some()` guard from both Pattern 1 and Pattern 2.
/// Pattern 3 (`where(id: ...).take!`) was unaffected: `as_method_chain` requires `take!` to
/// have a receiver (the `where(...)` call), so it already correctly handled both cases.
pub struct FindById;

fn sole_id_keyword_value<'a>(call: &ruby_prism::CallNode<'a>) -> Option<ruby_prism::Node<'a>> {
    let args = call.arguments()?;
    let all_args: Vec<_> = args.arguments().iter().collect();
    if all_args.len() != 1 {
        return None;
    }
    let kw = all_args[0].as_keyword_hash_node()?;
    let elements: Vec<_> = kw.elements().iter().collect();
    if elements.len() != 1 {
        return None;
    }
    let assoc = elements[0].as_assoc_node()?;
    let sym = assoc.key().as_symbol_node()?;
    if sym.unescaped() != b"id" {
        return None;
    }
    Some(assoc.value())
}

impl Cop for FindById {
    fn name(&self) -> &'static str {
        "Rails/FindById"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
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

        let name = call.name().as_slice();

        // Pattern 1: find_by_id!(id)
        // Fires with or without an explicit receiver (matches implicit self inside class methods).
        if name == b"find_by_id!" {
            if let Some(args) = call.arguments() {
                let all_args: Vec<_> = args.arguments().iter().collect();
                if let Some(id_value) = all_args.first() {
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `find` instead of `find_by_id!`.".to_string(),
                    );

                    if let Some(ref mut corr) = corrections {
                        let id_loc = id_value.location();
                        let id_src = source
                            .try_byte_slice(id_loc.start_offset(), id_loc.end_offset())
                            .unwrap_or("id");
                        corr.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: call.location().end_offset(),
                            replacement: format!("find({id_src})"),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }

                    diagnostics.push(diagnostic);
                }
            }
            return;
        }

        // Pattern 2: find_by!(id: value) — only when id is the sole argument.
        // Fires with or without an explicit receiver (matches implicit self inside class methods).
        if name == b"find_by!" {
            if let Some(id_value) = sole_id_keyword_value(&call) {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `find_by!`.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let id_loc = id_value.location();
                    let id_src = source
                        .try_byte_slice(id_loc.start_offset(), id_loc.end_offset())
                        .unwrap_or("id");
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: call.location().end_offset(),
                        replacement: format!("find({id_src})"),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
            return;
        }

        // Pattern 3: where(id: value).take!
        if name == b"take!" {
            let chain = match util::as_method_chain(node) {
                Some(c) => c,
                None => return,
            };
            if chain.inner_method != b"where" {
                return;
            }
            // Check that `where` has `id:` as the sole keyword arg
            if let Some(id_value) = sole_id_keyword_value(&chain.inner_call) {
                let loc = chain
                    .inner_call
                    .message_loc()
                    .unwrap_or(chain.inner_call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `where(id: ...).take!`.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let id_loc = id_value.location();
                    let id_src = source
                        .try_byte_slice(id_loc.start_offset(), id_loc.end_offset())
                        .unwrap_or("id");
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: call.location().end_offset(),
                        replacement: format!("find({id_src})"),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindById, "cops/rails/find_by_id");
    crate::cop_autocorrect_fixture_tests!(FindById, "cops/rails/find_by_id");
}
