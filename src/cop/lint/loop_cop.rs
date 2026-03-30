use crate::cop::node_type::{UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Loop;

fn body_source_from_statements(stmts: Option<ruby_prism::StatementsNode<'_>>) -> String {
    let Some(stmts) = stmts else {
        return String::new();
    };

    if stmts.body().len() == 1
        && let Some(begin_node) = stmts.body().iter().next().and_then(|n| n.as_begin_node())
        && let Some(inner) = begin_node.statements()
    {
        return std::str::from_utf8(inner.location().as_slice())
            .unwrap_or("")
            .to_string();
    }

    std::str::from_utf8(stmts.location().as_slice())
        .unwrap_or("")
        .to_string()
}

fn indent_body(body: &str) -> String {
    if body.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for line in body.lines() {
        out.push_str("  ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

impl Cop for Loop {
    fn name(&self) -> &'static str {
        "Lint/Loop"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[UNTIL_NODE, WHILE_NODE]
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
        // Check WhileNode for begin..end while form
        // Prism sets the PM_LOOP_FLAGS_BEGIN_MODIFIER flag for this pattern.
        if let Some(while_node) = node.as_while_node() {
            if while_node.is_begin_modifier() {
                let kw_loc = while_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Kernel#loop` with `break` rather than `begin/end/while(until)`."
                        .to_string(),
                );

                if let Some(corr) = corrections.as_mut() {
                    let body = body_source_from_statements(while_node.statements());
                    let cond = std::str::from_utf8(while_node.predicate().location().as_slice())
                        .unwrap_or("");
                    let body_with_newline = indent_body(&body);
                    let replacement =
                        format!("loop do\n{body_with_newline}  break unless {cond}\nend");
                    let loc = while_node.location();
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement,
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }

                diagnostics.push(diag);
            }
        }

        // Check UntilNode for begin..end until form
        if let Some(until_node) = node.as_until_node() {
            if until_node.is_begin_modifier() {
                let kw_loc = until_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Kernel#loop` with `break` rather than `begin/end/while(until)`."
                        .to_string(),
                );

                if let Some(corr) = corrections.as_mut() {
                    let body = body_source_from_statements(until_node.statements());
                    let cond = std::str::from_utf8(until_node.predicate().location().as_slice())
                        .unwrap_or("");
                    let body_with_newline = indent_body(&body);
                    let replacement = format!("loop do\n{body_with_newline}  break if {cond}\nend");
                    let loc = until_node.location();
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement,
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }

                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Loop, "cops/lint/loop_cop");
    crate::cop_autocorrect_fixture_tests!(Loop, "cops/lint/loop_cop");
}
