use crate::cop::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SubjectDeclaration;

impl Cop for SubjectDeclaration {
    fn name(&self) -> &'static str {
        "RSpec/SubjectDeclaration"
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
        &[CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE]
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

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();

        // Check for `let(:subject)` or `let!(:subject)` — should use `subject` directly
        if (method_name == b"let" || method_name == b"let!") && is_subject_name_arg(&call) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Use subject explicitly rather than using let".to_string(),
            );

            if let Some((start, end, replacement)) =
                replacement_for_subject_declaration(source, &call)
                && let Some(corrections) = corrections.as_deref_mut()
            {
                corrections.push(crate::correction::Correction {
                    start,
                    end,
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }

        // Check for `subject(:subject)` or `subject!(:subject)` — ambiguous
        if (method_name == b"subject" || method_name == b"subject!") && is_subject_name_arg(&call) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Ambiguous declaration of subject".to_string(),
            );

            if let Some((start, end, replacement)) =
                replacement_for_subject_declaration(source, &call)
                && let Some(corrections) = corrections.as_deref_mut()
            {
                corrections.push(crate::correction::Correction {
                    start,
                    end,
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

fn replacement_for_subject_declaration(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Option<(usize, usize, String)> {
    let args = call.arguments()?;
    let selector = call.message_loc()?;
    let method = call.name().as_slice();
    let replacement = if method == b"let" {
        "subject"
    } else if method == b"let!" {
        "subject!"
    } else if method == b"subject" {
        "subject"
    } else if method == b"subject!" {
        "subject!"
    } else {
        return None;
    };

    let mut end = args.location().end_offset();
    if end < source.as_bytes().len() && source.as_bytes()[end] == b')' {
        end += 1;
    }

    Some((selector.start_offset(), end, replacement.to_string()))
}

/// Check if the first argument to a call is `:subject` or `'subject'` (or `subject!` variants).
fn is_subject_name_arg(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    for arg in args.arguments().iter() {
        if arg.as_keyword_hash_node().is_some() {
            continue;
        }
        if let Some(sym) = arg.as_symbol_node() {
            let val = sym.unescaped();
            return val == b"subject" || val == b"subject!";
        }
        if let Some(s) = arg.as_string_node() {
            let val = s.unescaped();
            return val == b"subject" || val == b"subject!";
        }
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SubjectDeclaration, "cops/rspec/subject_declaration");
    crate::cop_autocorrect_fixture_tests!(SubjectDeclaration, "cops/rspec/subject_declaration");
}
