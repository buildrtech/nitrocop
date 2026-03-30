use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MethodDefParentheses;

impl Cop for MethodDefParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodDefParentheses"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "require_parentheses");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Only apply to methods with parameters
        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Check if there are actual parameters
        if params.requireds().is_empty()
            && params.optionals().is_empty()
            && params.rest().is_none()
            && params.posts().is_empty()
            && params.keywords().is_empty()
            && params.keyword_rest().is_none()
            && params.block().is_none()
        {
            return;
        }

        let has_parens = def_node.lparen_loc().is_some();

        match enforced_style {
            "require_parentheses" | "require_no_parentheses_except_multiline" if !has_parens => {
                // RuboCop points at the arguments (parameters), not the `def` keyword
                let params_loc = params.location();
                let (line, column) = source.offset_to_line_col(params_loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `def` with parentheses when there are parameters.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    let name_end = def_node.name_loc().end_offset();
                    corr.push(crate::correction::Correction {
                        start: name_end,
                        end: params_loc.start_offset(),
                        replacement: "(".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    corr.push(crate::correction::Correction {
                        start: params_loc.end_offset(),
                        end: params_loc.end_offset(),
                        replacement: ")".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
            "require_no_parentheses" if has_parens => {
                // RuboCop points at the args node including parens — use lparen_loc
                let start = def_node
                    .lparen_loc()
                    .map_or_else(|| params.location().start_offset(), |lp| lp.start_offset());
                let (line, column) = source.offset_to_line_col(start);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `def` without parentheses.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    if let Some(lp) = def_node.lparen_loc() {
                        corr.push(crate::correction::Correction {
                            start: lp.start_offset(),
                            end: lp.end_offset(),
                            replacement: "".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                    if let Some(rp) = def_node.rparen_loc() {
                        corr.push(crate::correction::Correction {
                            start: rp.start_offset(),
                            end: rp.end_offset(),
                            replacement: "".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodDefParentheses, "cops/style/method_def_parentheses");
    crate::cop_autocorrect_fixture_tests!(
        MethodDefParentheses,
        "cops/style/method_def_parentheses"
    );
}
