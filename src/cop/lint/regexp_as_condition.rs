use crate::cop::node_type::MATCH_LAST_LINE_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RegexpAsCondition;

impl Cop for RegexpAsCondition {
    fn name(&self) -> &'static str {
        "Lint/RegexpAsCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[MATCH_LAST_LINE_NODE]
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
        // MatchLastLineNode is what Prism creates for bare regexp in conditions
        let match_node = match node.as_match_last_line_node() {
            Some(n) => n,
            None => return,
        };

        let loc = match_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly."
                .to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let regexp_src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
            if !regexp_src.is_empty() {
                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: format!("{regexp_src} =~ $_"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RegexpAsCondition, "cops/lint/regexp_as_condition");
    crate::cop_autocorrect_fixture_tests!(RegexpAsCondition, "cops/lint/regexp_as_condition");
}
