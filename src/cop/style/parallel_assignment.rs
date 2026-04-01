use crate::cop::node_type::{ARRAY_NODE, MULTI_WRITE_NODE, SPLAT_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ParallelAssignment flags `a, b = 1, 2` style multi-assignment when
/// each target gets exactly one value and the assignment can be safely rewritten
/// as sequential assignments.
///
/// ## Swap exemption (FP fix, 183 FPs in corpus)
/// RuboCop exempts "swap" assignments where the RHS references any LHS target,
/// creating a dependency cycle that prevents safe reordering. Examples:
///   - `a, b = b, a`
///   - `array[i], array[j] = array[j], array[i]`
///   - `self[0], self[2] = self[2], self[0]`
///   - `min_x, max_x = max_x, min_x if min_x > max_x`
///
/// RuboCop implements this via topological sort with cycle detection
/// (`TSort::Cyclic`). We use a simpler approach: extract the source text of
/// each LHS target and each RHS element, then check if any RHS element's source
/// *exactly matches* any LHS target's source. If so, there is a dependency and
/// the assignment is allowed (it may be a swap or have order-dependent
/// semantics). We use exact equality rather than substring matching to avoid
/// false positives like `a, b = foo(), bar()` where `bar()` contains `a`.
///
/// ## Trailing-comma / ImplicitRestNode (FN fix, 10 FNs in corpus)
/// `@name, @config, @bulk, = name, config, bulk` has a trailing comma that
/// Prism represents as `ImplicitRestNode` in the `rest()` slot. The old code
/// skipped all multi-writes with `rest().is_some()`, but `ImplicitRestNode`
/// is not a real splat — it just means "discard extra RHS values". We now only
/// skip when `rest()` is a real `SplatNode` (i.e., `*var`).
pub struct ParallelAssignment;

impl Cop for ParallelAssignment {
    fn name(&self) -> &'static str {
        "Style/ParallelAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, MULTI_WRITE_NODE, SPLAT_NODE]
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
        // Look for multi-write nodes (parallel assignment: a, b = 1, 2)
        let multi_write = match node.as_multi_write_node() {
            Some(m) => m,
            None => return,
        };

        let targets: Vec<_> = multi_write.lefts().iter().collect();

        // Check if there are at least 2 targets
        if targets.len() < 2 {
            return;
        }

        // Skip if a real splat rest assignment is present (a, *b = ...)
        // but NOT for ImplicitRestNode (trailing comma: a, b, = ...)
        if let Some(rest) = multi_write.rest() {
            if rest.as_implicit_rest_node().is_none() {
                // It's a real SplatNode or other rest — skip
                return;
            }
        }

        // The value is the RHS. In Prism, for `a, b = 1, 2`, the value is an ArrayNode
        // with the implicit array of values. For `a, b = foo`, it's just a single node.
        let value = multi_write.value();

        // Check if RHS is an array node (implicit or explicit) with matching count
        if let Some(arr) = value.as_array_node() {
            let elements: Vec<_> = arr.elements().iter().collect();
            if elements.len() != targets.len() {
                return;
            }

            // Check no splat in elements
            if elements.iter().any(|e| e.as_splat_node().is_some()) {
                return;
            }

            // Check for swap pattern: if any RHS element references any LHS target,
            // the assignment has order-dependent semantics and should be allowed.
            if is_swap_assignment(source, &targets, &elements) {
                return;
            }

            let loc = multi_write.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Do not use parallel assignment.".to_string(),
            );

            if let Some(corrs) = corrections.as_mut() {
                let line_start = source.as_bytes()[..loc.start_offset()]
                    .iter()
                    .rposition(|&b| b == b'\n')
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let indent = String::from_utf8_lossy(&source.as_bytes()[line_start..loc.start_offset()]);

                let mut assignments = Vec::with_capacity(targets.len());
                for (target, element) in targets.iter().zip(elements.iter()) {
                    let target_loc = target.location();
                    let element_loc = element.location();
                    let Some(target_src) = source
                        .try_byte_slice(target_loc.start_offset(), target_loc.end_offset())
                    else {
                        return;
                    };
                    let Some(elem_src) = source
                        .try_byte_slice(element_loc.start_offset(), element_loc.end_offset())
                    else {
                        return;
                    };
                    assignments.push(format!("{}{} = {}", indent, target_src, elem_src));
                }

                corrs.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: assignments.join("\n"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

/// Check if any RHS element exactly matches any LHS target by source text,
/// indicating a swap or order-dependent assignment that should be allowed.
///
/// Uses exact string equality (not substring) to avoid false positives like
/// `a, b = foo(), bar()` where `bar()` contains `a` as a substring.
/// This catches:
/// - Simple swaps: `a, b = b, a`
/// - Indexed swaps: `arr[0], arr[1] = arr[1], arr[0]`
/// - Method swaps: `node.left, node.right = node.right, node.left`
fn is_swap_assignment(
    source: &SourceFile,
    targets: &[ruby_prism::Node<'_>],
    rhs_elements: &[ruby_prism::Node<'_>],
) -> bool {
    // Extract source text for each LHS target
    let lhs_texts: Vec<&str> = targets
        .iter()
        .filter_map(|t| {
            let loc = t.location();
            source.try_byte_slice(loc.start_offset(), loc.end_offset())
        })
        .collect();

    // Extract source text for each RHS element
    let rhs_texts: Vec<&str> = rhs_elements
        .iter()
        .filter_map(|e| {
            let loc = e.location();
            source.try_byte_slice(loc.start_offset(), loc.end_offset())
        })
        .collect();

    // If any RHS element's source text exactly matches any LHS target's source text,
    // the assignment has dependencies (potential swap). We use exact equality rather
    // than substring matching to avoid false positives like `a, b = foo(), bar()`
    // where `bar()` contains `a` as a substring but is unrelated.
    for rhs_text in &rhs_texts {
        for lhs_text in &lhs_texts {
            if !lhs_text.is_empty() && *rhs_text == *lhs_text {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ParallelAssignment, "cops/style/parallel_assignment");

    #[test]
    fn trailing_comma_lhs() {
        let diags = crate::testutil::run_cop_full_internal(
            &ParallelAssignment,
            b"@name, @config, @bulk, = name, config, bulk\n",
            CopConfig::default(),
            "test.rb",
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for trailing-comma LHS, got {}",
            diags.len()
        );
    }

    #[test]
    fn swap_not_flagged() {
        let diags = crate::testutil::run_cop_full_internal(
            &ParallelAssignment,
            b"a, b = b, a\n",
            CopConfig::default(),
            "test.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "Swap should not be flagged, got {} offenses",
            diags.len()
        );
    }

    #[test]
    fn indexed_swap_not_flagged() {
        let diags = crate::testutil::run_cop_full_internal(
            &ParallelAssignment,
            b"arr[0], arr[1] = arr[1], arr[0]\n",
            CopConfig::default(),
            "test.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "Indexed swap should not be flagged, got {} offenses",
            diags.len()
        );
    }
}
