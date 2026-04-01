use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, DEF_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE,
    REQUIRED_KEYWORD_PARAMETER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct KeywordParametersOrder;

impl Cop for KeywordParametersOrder {
    fn name(&self) -> &'static str {
        "Style/KeywordParametersOrder"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            DEF_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            REQUIRED_KEYWORD_PARAMETER_NODE,
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
        // Check both def and block parameters
        let parameters = if let Some(def_node) = node.as_def_node() {
            def_node.parameters()
        } else if let Some(block_node) = node.as_block_node() {
            if let Some(params) = block_node.parameters() {
                if let Some(bp) = params.as_block_parameters_node() {
                    bp.parameters()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let parameters = match parameters {
            Some(p) => p,
            None => return,
        };

        // Check keyword parameters order: required keywords should come before optional keywords
        let keywords: Vec<_> = parameters.keywords().iter().collect();
        let mut seen_required = false;
        let mut have_optional_before_required = false;

        // First pass: check if there are any required keywords after optional ones
        for kw in keywords.iter().rev() {
            if kw.as_required_keyword_parameter_node().is_some() {
                seen_required = true;
            } else if kw.as_optional_keyword_parameter_node().is_some() && seen_required {
                have_optional_before_required = true;
                break;
            }
        }

        if !have_optional_before_required {
            return;
        }

        // Second pass: report each optional keyword that appears before a required keyword
        seen_required = false;
        let mut offense_count = 0usize;
        for kw in keywords.iter().rev() {
            if kw.as_required_keyword_parameter_node().is_some() {
                seen_required = true;
            } else if kw.as_optional_keyword_parameter_node().is_some() && seen_required {
                let loc = kw.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Place optional keyword parameters at the end of the parameters list."
                            .to_string(),
                    ),
                );
                offense_count += 1;
            }
        }

        if offense_count == 0 {
            return;
        }

        if let Some(corrections) = corrections {
            let first_kw = match keywords.first() {
                Some(kw) => kw,
                None => return,
            };
            let last_kw = match keywords.last() {
                Some(kw) => kw,
                None => return,
            };

            let first_line = source
                .offset_to_line_col(first_kw.location().start_offset())
                .0;
            let last_line = source.offset_to_line_col(last_kw.location().end_offset()).0;
            if first_line != last_line {
                return;
            }

            let mut required_src: Vec<&str> = Vec::new();
            let mut optional_src: Vec<&str> = Vec::new();
            for kw in &keywords {
                let loc = kw.location();
                let kw_src = match std::str::from_utf8(
                    &source.as_bytes()[loc.start_offset()..loc.end_offset()],
                ) {
                    Ok(s) => s.trim(),
                    Err(_) => return,
                };
                if kw.as_required_keyword_parameter_node().is_some() {
                    required_src.push(kw_src);
                } else if kw.as_optional_keyword_parameter_node().is_some() {
                    optional_src.push(kw_src);
                }
            }

            let mut reordered: Vec<&str> = Vec::new();
            reordered.extend(required_src);
            reordered.extend(optional_src);
            if reordered.is_empty() {
                return;
            }

            let replacement = reordered.join(", ");
            corrections.push(crate::correction::Correction {
                start: first_kw.location().start_offset(),
                end: last_kw.location().end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });

            for diag in diagnostics.iter_mut().rev().take(offense_count) {
                diag.corrected = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        KeywordParametersOrder,
        "cops/style/keyword_parameters_order"
    );
    crate::cop_autocorrect_fixture_tests!(
        KeywordParametersOrder,
        "cops/style/keyword_parameters_order"
    );
}
