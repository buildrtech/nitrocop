use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Extended corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=3, FN=0. All 3 FPs from files containing
/// invalid multibyte regex escapes that crash RuboCop's parser, causing all
/// other cops to be skipped. Not a cop logic issue. Fixed by adding the
/// affected files to `repo_excludes.json`.
pub struct InefficientHashSearch;

impl Cop for InefficientHashSearch {
    fn name(&self) -> &'static str {
        "Performance/InefficientHashSearch"
    }

    fn uses_node_check(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"include?" {
            return;
        }

        // inner_call must have an explicit receiver (e.g. `hash.keys`, not bare `keys`)
        // Bare `keys`/`values` without a receiver are often methods on non-Hash classes.
        if chain.inner_call.receiver().is_none() {
            return;
        }

        // inner_call must have no arguments (just `.keys` or `.values`)
        if chain.inner_call.arguments().is_some() {
            return;
        }

        let (message, replacement_method) = if chain.inner_method == b"keys" {
            ("Use `key?` instead of `keys.include?`.", "key?")
        } else if chain.inner_method == b"values" {
            ("Use `value?` instead of `values.include?`.", "value?")
        } else {
            return;
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(source, line, column, message.to_string());

        if let Some(ref mut corr) = corrections {
            let outer_call = match node.as_call_node() {
                Some(c) => c,
                None => return,
            };
            let first_arg = match outer_call
                .arguments()
                .and_then(|args| args.arguments().iter().next())
            {
                Some(a) => a,
                None => return,
            };
            let arg_loc = first_arg.location();
            let arg_source = source.byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "");

            let hash_recv = match chain.inner_call.receiver() {
                Some(r) => r,
                None => return,
            };
            let recv_loc = hash_recv.location();
            let recv_source = source.byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "");
            let call_op = chain
                .inner_call
                .call_operator_loc()
                .map(|op| {
                    source
                        .byte_slice(op.start_offset(), op.end_offset(), "")
                        .to_string()
                })
                .unwrap_or_else(|| ".".to_string());

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: format!("{recv_source}{call_op}{replacement_method}({arg_source})"),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        InefficientHashSearch,
        "cops/performance/inefficient_hash_search"
    );
    crate::cop_autocorrect_fixture_tests!(
        InefficientHashSearch,
        "cops/performance/inefficient_hash_search"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(InefficientHashSearch.supports_autocorrect());
    }
}
