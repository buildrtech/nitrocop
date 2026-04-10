use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/WhereNotWithMultipleConditions
///
/// ## Investigation (2026-03-15): FP=2, FN=2
///
/// Location mismatch: nitrocop was reporting at `node.location()` (start of the entire
/// chain expression), while RuboCop reports at `node.receiver.loc.selector` (the `where`
/// keyword in `.where.not`). For multiline chains, the `where.not` call appears on a
/// different line than the chain start, causing FP at the chain start line and FN at
/// the `where` keyword line.
///
/// Fix: Report at `chain.inner_call.message_loc()` (the `where` keyword location).
pub struct WhereNotWithMultipleConditions;

fn hash_has_multiple_pairs(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(hash) = node.as_hash_node() {
        let pairs: Vec<_> = hash.elements().iter().collect();
        if pairs.len() >= 2 {
            return true;
        }
        // Check for nested hash with multiple pairs
        if pairs.len() == 1 {
            if let Some(assoc) = pairs[0].as_assoc_node() {
                let val = assoc.value();
                if let Some(inner_hash) = val.as_hash_node() {
                    let inner_pairs: Vec<_> = inner_hash.elements().iter().collect();
                    return inner_pairs.len() >= 2;
                }
            }
        }
        return false;
    }
    if let Some(kw_hash) = node.as_keyword_hash_node() {
        let pairs: Vec<_> = kw_hash.elements().iter().collect();
        if pairs.len() >= 2 {
            return true;
        }
        if pairs.len() == 1 {
            if let Some(assoc) = pairs[0].as_assoc_node() {
                let val = assoc.value();
                if let Some(inner_hash) = val.as_hash_node() {
                    let inner_pairs: Vec<_> = inner_hash.elements().iter().collect();
                    return inner_pairs.len() >= 2;
                }
            }
        }
    }
    false
}

impl Cop for WhereNotWithMultipleConditions {
    fn name(&self) -> &'static str {
        "Rails/WhereNotWithMultipleConditions"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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
        let chain = match util::as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        // outer must be `not`, inner must be `where`
        if chain.outer_method != b"not" || chain.inner_method != b"where" {
            return;
        }

        // The `not` call must have hash arguments with multiple conditions
        let call = node.as_call_node().unwrap();
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        if !hash_has_multiple_pairs(&arg_list[0]) {
            return;
        }

        // RuboCop reports offense starting at the `where` keyword (node.receiver.loc.selector),
        // not at the start of the entire chain expression.
        let where_call = &chain.inner_call;
        let loc = where_call
            .message_loc()
            .unwrap_or_else(|| where_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use a SQL statement instead of `where.not` with multiple conditions.".to_string(),
        );

        // Conservative baseline autocorrect: only split top-level multi-pair hash arguments
        // into chained `.where.not(<single_pair>)` calls.
        // Skip nested-hash forms (`where.not(posts: { ... })`) to avoid broad rewrites.
        if let Some(corrections) = corrections.as_deref_mut()
            && let Some(receiver) = call.receiver()
        {
            let direct_pairs: Vec<ruby_prism::Node<'_>> = if let Some(hash) = arg_list[0].as_hash_node() {
                hash.elements().iter().filter(|n| n.as_assoc_node().is_some()).collect()
            } else if let Some(kw_hash) = arg_list[0].as_keyword_hash_node() {
                kw_hash.elements().iter().filter(|n| n.as_assoc_node().is_some()).collect()
            } else {
                Vec::new()
            };

            if direct_pairs.len() >= 2 {
                let receiver_loc = receiver.location();
                let receiver_source = source.byte_slice(
                    receiver_loc.start_offset(),
                    receiver_loc.end_offset(),
                    "",
                );

                let mut pair_sources = Vec::with_capacity(direct_pairs.len());
                for pair in direct_pairs {
                    let pair_loc = pair.location();
                    pair_sources.push(source.byte_slice(
                        pair_loc.start_offset(),
                        pair_loc.end_offset(),
                        "",
                    ));
                }

                let mut replacement = format!("{receiver_source}.not({})", pair_sources[0]);
                for pair in pair_sources.iter().skip(1) {
                    replacement.push_str(&format!(".where.not({pair})"));
                }

                corrections.push(crate::correction::Correction {
                    start: call.location().start_offset(),
                    end: call.location().end_offset(),
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        WhereNotWithMultipleConditions,
        "cops/rails/where_not_with_multiple_conditions"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(WhereNotWithMultipleConditions.supports_autocorrect());
    }

    #[test]
    fn autocorrect_splits_top_level_pairs_into_chained_where_not_calls() {
        crate::testutil::assert_cop_autocorrect(
            &WhereNotWithMultipleConditions,
            b"User.where.not(trashed: true, role: 'admin')\n",
            b"User.where.not(trashed: true).where.not(role: 'admin')\n",
        );
    }

    #[test]
    fn nested_hash_offense_remains_uncorrected_in_baseline() {
        let input = b"User.joins(:posts).where.not(posts: { trashed: true, title: 'Rails' })\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(
            &WhereNotWithMultipleConditions,
            input,
        );
        assert_eq!(diags.len(), 1);
        assert!(!diags[0].corrected);
        assert!(corrections.is_empty());
    }
}
