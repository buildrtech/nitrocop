use crate::cop::node_type::{IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for nested modifier conditionals/loops.
///
/// ## Investigation findings
/// FN root cause: only handled `IfNode` and `UnlessNode` as outer/inner modifiers.
/// `WhileNode` and `UntilNode` can also be modifier forms (no `end` keyword) and
/// participate in nested modifier combinations like `something if a while b`.
/// Fix: added WHILE_NODE/UNTIL_NODE to interested_node_types and inner body checks.
pub struct NestedModifier;

impl Cop for NestedModifier {
    fn name(&self) -> &'static str {
        "Style/NestedModifier"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, WHILE_NODE, UNTIL_NODE]
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
        let (outer_if_unless, body_node) = if let Some(if_node) = node.as_if_node() {
            if if_node.end_keyword_loc().is_some() {
                return;
            }
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let kw_bytes = kw_loc.as_slice();
            if kw_bytes != b"if" && kw_bytes != b"unless" {
                return;
            }
            (
                Some((
                    String::from_utf8_lossy(kw_loc.as_slice()).to_string(),
                    if_node.predicate(),
                )),
                if_node.statements(),
            )
        } else if let Some(unless_node) = node.as_unless_node() {
            if unless_node.end_keyword_loc().is_some() {
                return;
            }
            (
                Some(("unless".to_string(), unless_node.predicate())),
                unless_node.statements(),
            )
        } else if let Some(while_node) = node.as_while_node() {
            if while_node.closing_loc().is_some() {
                return;
            }
            (None, while_node.statements())
        } else if let Some(until_node) = node.as_until_node() {
            if until_node.closing_loc().is_some() {
                return;
            }
            (None, until_node.statements())
        } else {
            return;
        };

        let stmts = match body_node {
            Some(s) => s,
            None => return,
        };

        let body: Vec<_> = stmts.body().iter().collect();
        if body.len() != 1 {
            return;
        }

        if let Some(inner_if) = body[0].as_if_node() {
            if inner_if.end_keyword_loc().is_some() {
                return;
            }
            if let Some(inner_kw) = inner_if.if_keyword_loc() {
                let inner_bytes = inner_kw.as_slice();
                if inner_bytes == b"if" || inner_bytes == b"unless" {
                    let (line, column) = source.offset_to_line_col(inner_kw.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid using nested modifiers.".to_string(),
                    );

                    if let (Some(corrs), Some((outer_kw, outer_predicate))) =
                        (corrections.as_deref_mut(), outer_if_unless.as_ref())
                    {
                        let (start, end, replacement) = build_if_unless_rewrite(
                            source,
                            outer_kw,
                            outer_predicate,
                            inner_kw,
                            &inner_if.predicate(),
                        );
                        corrs.push(crate::correction::Correction {
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

        if let Some(inner_unless) = body[0].as_unless_node() {
            if inner_unless.end_keyword_loc().is_some() {
                return;
            }
            let inner_kw = inner_unless.keyword_loc();
            if inner_kw.as_slice() == b"unless" {
                let (line, column) = source.offset_to_line_col(inner_kw.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid using nested modifiers.".to_string(),
                );

                if let (Some(corrs), Some((outer_kw, outer_predicate))) =
                    (corrections.as_deref_mut(), outer_if_unless.as_ref())
                {
                    let (start, end, replacement) = build_if_unless_rewrite(
                        source,
                        outer_kw,
                        outer_predicate,
                        inner_kw,
                        &inner_unless.predicate(),
                    );
                    corrs.push(crate::correction::Correction {
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

        if let Some(inner_while) = body[0].as_while_node() {
            if inner_while.closing_loc().is_some() {
                return;
            }
            let inner_kw = inner_while.keyword_loc();
            let (line, column) = source.offset_to_line_col(inner_kw.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid using nested modifiers.".to_string(),
            ));
        }

        if let Some(inner_until) = body[0].as_until_node() {
            if inner_until.closing_loc().is_some() {
                return;
            }
            let inner_kw = inner_until.keyword_loc();
            let (line, column) = source.offset_to_line_col(inner_kw.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid using nested modifiers.".to_string(),
            ));
        }
    }
}

fn build_if_unless_rewrite(
    source: &SourceFile,
    outer_keyword: &str,
    outer_predicate: &ruby_prism::Node<'_>,
    inner_kw_loc: ruby_prism::Location<'_>,
    inner_predicate: &ruby_prism::Node<'_>,
) -> (usize, usize, String) {
    let left = String::from_utf8_lossy(
        &source.as_bytes()
            [outer_predicate.location().start_offset()..outer_predicate.location().end_offset()],
    )
    .to_string();
    let mut right = String::from_utf8_lossy(
        &source.as_bytes()
            [inner_predicate.location().start_offset()..inner_predicate.location().end_offset()],
    )
    .to_string();

    let inner_kw = String::from_utf8_lossy(inner_kw_loc.as_slice()).to_string();
    if outer_keyword != inner_kw {
        right = format!("!{}", right);
    }

    let operator = if outer_keyword == "if" { "&&" } else { "||" };
    let replacement = format!("{} {} {} {}", outer_keyword, left, operator, right);

    (
        inner_kw_loc.start_offset(),
        outer_predicate.location().end_offset(),
        replacement,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedModifier, "cops/style/nested_modifier");
    crate::cop_autocorrect_fixture_tests!(NestedModifier, "cops/style/nested_modifier");
}
