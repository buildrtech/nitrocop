use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{has_keyword_arg, is_dsl_call, keyword_arg_pair_start_offset};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DelegateAllowBlank;

impl Cop for DelegateAllowBlank {
    fn name(&self) -> &'static str {
        "Rails/DelegateAllowBlank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        if !is_dsl_call(&call, b"delegate") {
            return;
        }

        if !has_keyword_arg(&call, b"allow_blank") {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "`allow_blank` is not a valid option for `delegate`. Did you mean `allow_nil`?"
                .to_string(),
        );

        if let Some(ref mut corr) = corrections
            && let Some(start) = keyword_arg_pair_start_offset(&call, b"allow_blank")
        {
            corr.push(crate::correction::Correction {
                start,
                end: start + "allow_blank".len(),
                replacement: "allow_nil".to_string(),
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
    crate::cop_fixture_tests!(DelegateAllowBlank, "cops/rails/delegate_allow_blank");

    #[test]
    fn autocorrects_allow_blank_to_allow_nil() {
        crate::testutil::assert_cop_autocorrect(
            &DelegateAllowBlank,
            b"delegate :name, to: :client, allow_blank: true\n",
            b"delegate :name, to: :client, allow_nil: true\n",
        );
    }
}
