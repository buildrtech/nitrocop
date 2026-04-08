use crate::cop::node_type::{CALL_NODE, IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SafeNavigationWithBlank;

fn check_safe_blank_predicate(
    source: &SourceFile,
    predicate: &ruby_prism::Node<'_>,
    cop: &SafeNavigationWithBlank,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) -> Option<crate::diagnostic::Diagnostic> {
    let call = predicate.as_call_node()?;

    if call.name().as_slice() != b"blank?" {
        return None;
    }

    // `blank?` takes no arguments — if arguments are present, skip.
    if call.arguments().is_some() {
        return None;
    }

    // Check for safe navigation operator (&.)
    let call_op = call.call_operator_loc()?;

    let op_bytes = &source.as_bytes()[call_op.start_offset()..call_op.end_offset()];
    if op_bytes != b"&." {
        return None;
    }

    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    let mut diagnostic = cop.diagnostic(
        source,
        line,
        column,
        "Avoid calling `blank?` with the safe navigation operator in conditionals.".to_string(),
    );

    if let Some(corr) = corrections.as_deref_mut() {
        corr.push(crate::correction::Correction {
            start: call_op.start_offset(),
            end: call_op.end_offset(),
            replacement: ".".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diagnostic.corrected = true;
    }

    Some(diagnostic)
}

impl Cop for SafeNavigationWithBlank {
    fn name(&self) -> &'static str {
        "Rails/SafeNavigationWithBlank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, IF_NODE, UNLESS_NODE]
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
        // Check if nodes
        if let Some(if_node) = node.as_if_node() {
            let predicate = if_node.predicate();
            if let Some(diagnostic) =
                check_safe_blank_predicate(source, &predicate, self, &mut corrections)
            {
                diagnostics.push(diagnostic);
            }
            return;
        }

        // Check unless nodes
        if let Some(unless_node) = node.as_unless_node() {
            let predicate = unless_node.predicate();
            if let Some(diagnostic) =
                check_safe_blank_predicate(source, &predicate, self, &mut corrections)
            {
                diagnostics.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationWithBlank,
        "cops/rails/safe_navigation_with_blank"
    );
    crate::cop_autocorrect_fixture_tests!(
        SafeNavigationWithBlank,
        "cops/rails/safe_navigation_with_blank"
    );
}
