use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodDefinitionBraceLayout;

impl Cop for MultilineMethodDefinitionBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodDefinitionBraceLayout"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must have explicit parentheses
        let lparen_loc = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return,
        };
        let rparen_loc = match def_node.rparen_loc() {
            Some(loc) => loc,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let (open_line, _) = source.offset_to_line_col(lparen_loc.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(rparen_loc.start_offset());

        // Only check multiline parameter lists
        if open_line == close_line {
            return;
        }

        // Find the first and last parameter locations
        let mut first_offset: Option<usize> = None;
        let mut last_end_offset: Option<usize> = None;

        // Collect all parameter locations from requireds (Node type)
        let requireds: Vec<ruby_prism::Node<'_>> = params.requireds().iter().collect();
        for p in &requireds {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Optionals
        let optionals: Vec<ruby_prism::Node<'_>> = params.optionals().iter().collect();
        for p in &optionals {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Rest
        if let Some(p) = params.rest() {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Posts
        let posts: Vec<ruby_prism::Node<'_>> = params.posts().iter().collect();
        for p in &posts {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Keywords
        let keywords: Vec<ruby_prism::Node<'_>> = params.keywords().iter().collect();
        for p in &keywords {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Keyword rest
        if let Some(p) = params.keyword_rest() {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Block parameter
        if let Some(p) = params.block() {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        let first_off = match first_offset {
            Some(o) => o,
            None => return,
        };
        let last_end = match last_end_offset {
            Some(o) => o,
            None => return,
        };

        let (first_param_line, _) = source.offset_to_line_col(first_off);
        let (last_param_line, _) = source.offset_to_line_col(last_end.saturating_sub(1));

        let open_same_as_first = open_line == first_param_line;
        let close_same_as_last = close_line == last_param_line;

        let closing_start = rparen_loc.start_offset();
        let closing_end = rparen_loc.end_offset();
        let opening_line_start = source.line_start_offset(open_line);
        let opening_indent = source.as_bytes()[opening_line_start..]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();

        let mut emit = |message: &str, want_same_line: bool| {
            let mut diagnostic =
                self.diagnostic(source, close_line, close_col, message.to_string());

            if let Some(corrections) = corrections.as_mut() {
                if want_same_line {
                    let between = &source.as_bytes()[last_end..closing_start];
                    if between
                        .iter()
                        .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
                    {
                        corrections.push(Correction {
                            start: last_end,
                            end: closing_end,
                            replacement: ")".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                } else {
                    corrections.push(Correction {
                        start: last_end,
                        end: closing_start,
                        replacement: format!("\n{}", " ".repeat(opening_indent)),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
            }

            diagnostics.push(diagnostic);
        };

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    emit(
                        "Closing method definition brace must be on the same line as the last parameter when opening brace is on the same line as the first parameter.",
                        true,
                    );
                }
                if !open_same_as_first && close_same_as_last {
                    emit(
                        "Closing method definition brace must be on the line after the last parameter when opening brace is on a separate line from the first parameter.",
                        false,
                    );
                }
            }
            "new_line" => {
                if close_same_as_last {
                    emit(
                        "Closing method definition brace must be on the line after the last parameter.",
                        false,
                    );
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    emit(
                        "Closing method definition brace must be on the same line as the last parameter.",
                        true,
                    );
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineMethodDefinitionBraceLayout,
        "cops/layout/multiline_method_definition_brace_layout"
    );
    crate::cop_autocorrect_fixture_tests!(
        MultilineMethodDefinitionBraceLayout,
        "cops/layout/multiline_method_definition_brace_layout"
    );
}
