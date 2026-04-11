use crate::cop::node_type::{
    LOCAL_VARIABLE_READ_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Investigation (2026-03-03)
///
/// Found 1 FP: `def send!(name: name)` pattern. This IS a genuine circular
/// reference (Ruby warns about it too). RuboCop doesn't flag it because the
/// project's style gem likely disables this cop. Not a cop logic bug — this is
/// a config resolution issue where nitrocop may not be loading the effective
/// `Enabled: false` from the inherited gem config.
pub struct CircularArgumentReference;

impl Cop for CircularArgumentReference {
    fn name(&self) -> &'static str {
        "Lint/CircularArgumentReference"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            LOCAL_VARIABLE_READ_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            OPTIONAL_PARAMETER_NODE,
        ]
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
        let mut corrections = corrections;

        // Check optional keyword arguments: def foo(bar: bar)
        if let Some(kwopt) = node.as_optional_keyword_parameter_node() {
            let param_name = kwopt.name().as_slice();
            let value = kwopt.value();
            if let Some(ref_offset) = find_circular_ref(param_name, &value) {
                let (line, column) = source.offset_to_line_col(ref_offset);
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Circular argument reference - `{}`.",
                        std::str::from_utf8(param_name).unwrap_or("?")
                    ),
                );
                if let Some(corrections) = corrections.as_deref_mut() {
                    corrections.push(crate::correction::Correction {
                        start: ref_offset,
                        end: ref_offset + param_name.len(),
                        replacement: "nil".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
                diagnostics.push(diagnostic);
            }
            return;
        }

        // Check optional positional arguments: def foo(bar = bar)
        if let Some(optarg) = node.as_optional_parameter_node() {
            let param_name = optarg.name().as_slice();
            let value = optarg.value();
            if let Some(ref_offset) = find_circular_ref(param_name, &value) {
                let (line, column) = source.offset_to_line_col(ref_offset);
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Circular argument reference - `{}`.",
                        std::str::from_utf8(param_name).unwrap_or("?")
                    ),
                );
                if let Some(corrections) = corrections.as_deref_mut() {
                    corrections.push(crate::correction::Correction {
                        start: ref_offset,
                        end: ref_offset + param_name.len(),
                        replacement: "nil".to_string(),
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

/// Returns the start offset of the circular reference node, or None.
fn find_circular_ref(param_name: &[u8], value: &ruby_prism::Node<'_>) -> Option<usize> {
    // Direct reference: def foo(x = x) where value is a local variable read
    if let Some(lvar) = value.as_local_variable_read_node() {
        if lvar.name().as_slice() == param_name {
            return Some(lvar.location().start_offset());
        }
        return None;
    }
    // Nested assignment: def foo(pie = pie = pie) — the outer value is a
    // LocalVariableWriteNode whose own value may contain the circular ref.
    if let Some(lvar_write) = value.as_local_variable_write_node() {
        return find_circular_ref(param_name, &lvar_write.value());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        CircularArgumentReference,
        "cops/lint/circular_argument_reference"
    );

    #[test]
    fn autocorrect_replaces_circular_reference_with_nil() {
        crate::testutil::assert_cop_autocorrect(
            &CircularArgumentReference,
            b"def cook(dry_ingredients = dry_ingredients)\n  dry_ingredients.reduce(&:+)\nend\n",
            b"def cook(dry_ingredients = nil)\n  dry_ingredients.reduce(&:+)\nend\n",
        );
    }
}
