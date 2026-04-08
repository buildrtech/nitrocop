use crate::cop::node_type::{
    ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereMissing;

/// Information about a call in a method chain.
#[allow(dead_code)]
struct ChainCallInfo {
    name: Vec<u8>,
    start_offset: usize,
    end_offset: usize,
    msg_offset: usize,
    assoc_name: Option<Vec<u8>>,    // For left_joins(:assoc) calls
    where_nil_assocs: Vec<Vec<u8>>, // For where(assoc: { id: nil }) — which table names matched
    where_pair_count: usize,         // Top-level hash pair count in where(...)
}

/// Walk a method chain and collect info about each call.
fn collect_chain_info(node: &ruby_prism::Node<'_>) -> Vec<ChainCallInfo> {
    let mut infos = Vec::new();
    collect_chain_info_inner(node, &mut infos);
    infos
}

fn collect_chain_info_inner(node: &ruby_prism::Node<'_>, infos: &mut Vec<ChainCallInfo>) {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return,
    };

    let name = call.name().as_slice().to_vec();
    let start_offset = call.location().start_offset();
    let end_offset = call.location().end_offset();
    let msg_offset = call
        .message_loc()
        .map(|l| l.start_offset())
        .unwrap_or(start_offset);

    let assoc_name = left_joins_assoc(&call);
    let (where_nil_assocs, where_pair_count) = if name == b"where" {
        extract_where_nil_assocs(&call)
    } else {
        (Vec::new(), 0)
    };

    infos.push(ChainCallInfo {
        name,
        start_offset,
        end_offset,
        msg_offset,
        assoc_name,
        where_nil_assocs,
        where_pair_count,
    });

    if let Some(recv) = call.receiver() {
        collect_chain_info_inner(&recv, infos);
    }
}

/// Extract association table names from `where(assocs: { id: nil })` patterns
/// and count top-level where hash pairs.
fn extract_where_nil_assocs(call: &ruby_prism::CallNode<'_>) -> (Vec<Vec<u8>>, usize) {
    let mut assocs = Vec::new();
    let mut pair_count = 0;
    let args = match call.arguments() {
        Some(a) => a,
        None => return (assocs, pair_count),
    };
    for arg in args.arguments().iter() {
        let kw = match arg.as_keyword_hash_node() {
            Some(k) => k,
            None => continue,
        };
        for elem in kw.elements().iter() {
            let assoc_node = match elem.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };
            let key = match assoc_node.key().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            pair_count += 1;
            let value = assoc_node.value();
            let has_nil_id = if let Some(hash) = value.as_hash_node() {
                hash_has_id_nil(&hash)
            } else if let Some(kw_hash) = value.as_keyword_hash_node() {
                keyword_hash_has_id_nil(&kw_hash)
            } else {
                false
            };
            if has_nil_id {
                assocs.push(key.unescaped().to_vec());
            }
        }
    }
    (assocs, pair_count)
}

/// Check if a call is `left_joins(:assoc)` or `left_outer_joins(:assoc)`.
/// Returns the association name as bytes if matched.
fn left_joins_assoc<'a>(call: &ruby_prism::CallNode<'a>) -> Option<Vec<u8>> {
    let name = call.name().as_slice();
    if name != b"left_joins" && name != b"left_outer_joins" {
        return None;
    }
    let args = call.arguments()?;
    let mut arg_iter = args.arguments().iter();
    let first = arg_iter.next()?;
    // Require exactly one arg.
    if arg_iter.next().is_some() {
        return None;
    }
    // Must be a simple symbol argument, not a hash like `left_joins(foo: :bar)`
    let sym = first.as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

fn hash_has_id_nil(hash: &ruby_prism::HashNode<'_>) -> bool {
    for elem in hash.elements().iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                if sym.unescaped() == b"id" && assoc.value().as_nil_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

fn keyword_hash_has_id_nil(hash: &ruby_prism::KeywordHashNode<'_>) -> bool {
    for elem in hash.elements().iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                if sym.unescaped() == b"id" && assoc.value().as_nil_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

impl Cop for WhereMissing {
    fn name(&self) -> &'static str {
        "Rails/WhereMissing"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            NIL_NODE,
            SYMBOL_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        // minimum_target_rails_version 6.1
        if !config.rails_version_at_least(6.1) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Only process chains whose current (outermost) call could participate
        // in the offense pair. Other outer calls (e.g., `order`, `select`) never
        // report here due the index-0 pair check below.
        let current_name = call.name().as_slice();
        if current_name != b"left_joins"
            && current_name != b"left_outer_joins"
            && current_name != b"where"
        {
            return;
        }

        let chain = collect_chain_info(node);
        if chain.is_empty() {
            return;
        }

        // Find left_joins calls
        let left_joins_info: Vec<(usize, &Vec<u8>)> = chain
            .iter()
            .enumerate()
            .filter_map(|(i, info)| info.assoc_name.as_ref().map(|a| (i, a)))
            .collect();

        if left_joins_info.is_empty() {
            return;
        }

        for (lj_idx, assoc_name) in &left_joins_info {
            // Build both singular and plural table names for matching
            let mut plural = (*assoc_name).clone();
            plural.push(b's');

            for (i, info) in chain.iter().enumerate() {
                if i == *lj_idx {
                    continue;
                }

                // Check if this is a where call with matching nil-id assoc
                if !info.where_nil_assocs.iter().any(|a| {
                    a.as_slice() == assoc_name.as_slice() || a.as_slice() == plural.as_slice()
                }) {
                    continue;
                }

                // Only fire if the closer-to-root element of the pair is at index 0
                // (the current node). This prevents outer chain calls from duplicating.
                let outermost_idx = (*lj_idx).min(i);
                if outermost_idx != 0 {
                    continue;
                }

                let max_idx = (*lj_idx).max(i);
                let has_separator = (outermost_idx + 1..max_idx).any(|j| {
                    let n = &chain[j].name;
                    n == b"or" || n == b"and"
                });

                if has_separator {
                    continue;
                }

                let lj_info = &chain[*lj_idx];
                let where_idx = if chain[i].name == b"where" {
                    i
                } else if chain[*lj_idx].name == b"where" {
                    *lj_idx
                } else {
                    continue;
                };
                let where_info = &chain[where_idx];

                let (line, column) = source.offset_to_line_col(lj_info.msg_offset);
                let assoc_str = std::str::from_utf8(assoc_name).unwrap_or("assoc");
                let method_name = std::str::from_utf8(&lj_info.name).unwrap_or("left_joins");

                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use `where.missing(:{assoc_str})` instead of `{method_name}(:{assoc_str}).where({assoc_str}s: {{ id: nil }})`."
                    ),
                );

                // Conservative autocorrect: only when where(...) contains exactly one
                // top-level key/value pair (the nil-association condition).
                if where_info.where_pair_count == 1 {
                    if let Some(ref mut corr) = corrections {
                        let (start, end) = if where_idx < *lj_idx {
                            (lj_info.msg_offset, where_info.end_offset)
                        } else {
                            (where_info.msg_offset, lj_info.end_offset)
                        };
                        corr.push(crate::correction::Correction {
                            start,
                            end,
                            replacement: format!("where.missing(:{assoc_str})"),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                }

                diagnostics.push(diagnostic);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::assert_cop_autocorrect_with_config;
    use std::collections::HashMap;

    crate::cop_rails_fixture_tests!(WhereMissing, "cops/rails/where_missing", 6.1);

    fn where_missing_config() -> CopConfig {
        CopConfig {
            options: HashMap::from([
                (
                    "TargetRailsVersion".to_string(),
                    serde_yml::Value::Number(serde_yml::value::Number::from(6.1)),
                ),
                (
                    "__RailtiesInLockfile".to_string(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn supports_autocorrect() {
        assert!(WhereMissing.supports_autocorrect());
    }

    #[test]
    fn autocorrect_fixture() {
        assert_cop_autocorrect_with_config(
            &WhereMissing,
            include_bytes!("../../../tests/fixtures/cops/rails/where_missing/offense.rb"),
            include_bytes!("../../../tests/fixtures/cops/rails/where_missing/corrected.rb"),
            where_missing_config(),
        );
    }
}
