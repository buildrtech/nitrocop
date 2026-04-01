use crate::cop::node_type::{CALL_NODE, DEF_NODE, HASH_NODE, KEYWORD_HASH_NODE, PARENTHESES_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Count leading whitespace characters (spaces and tabs) as columns.
/// Unlike `util::indentation_of()` which only counts spaces, this counts both
/// spaces and tabs as 1 column each, matching `offset_to_line_col()`'s character
/// counting and RuboCop's `processed_source.line_indentation()`.
fn leading_whitespace_columns(line: &[u8]) -> usize {
    line.iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

fn corrected_close_paren_diagnostic(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    close_line: usize,
    close_col: usize,
    expected_col: usize,
    message: String,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> Diagnostic {
    let mut diagnostic = cop.diagnostic(source, close_line, close_col, message);
    if let Some(corrections) = corrections.as_mut() {
        let line_start = source.line_start_offset(close_line);
        corrections.push(Correction {
            start: line_start,
            end: line_start + close_col,
            replacement: " ".repeat(expected_col),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diagnostic.corrected = true;
    }
    diagnostic
}

/// Corpus investigation (2026-03-16)
///
/// FP root cause #1 (5 FPs from loomio): Tab-indented code. `indentation_of()` only
/// counts spaces, returning 0 for tab-prefixed lines. `offset_to_line_col()` counts
/// tabs as 1 character each. This mismatch caused the cop to compute expected=0 for
/// tab-indented closing parens. Fix: use `leading_whitespace_columns()` which counts
/// both spaces and tabs, matching RuboCop's `line_indentation()`.
///
/// FP root cause #2 (2 FPs from puppetlabs/puppet): When the first argument is an
/// empty hash `{}`, expanding its children produces an empty `element_columns` vec.
/// Rust's `.all()` returns true (vacuously) on empty iterators, so the cop treated
/// it as "all aligned" and required `)` to align with `(`. But RuboCop's `[].uniq.one?`
/// returns false, going to the else branch (line indentation). Fix: check that
/// `element_columns` is non-empty before treating it as "all aligned".
pub struct ClosingParenthesisIndentation;

impl Cop for ClosingParenthesisIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingParenthesisIndentation"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            DEF_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            PARENTHESES_NODE,
        ]
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
        // Handle method calls with parentheses
        if let Some(call) = node.as_call_node() {
            if let (Some(open_loc), Some(close_loc)) = (call.opening_loc(), call.closing_loc()) {
                if close_loc.as_slice() == b")" {
                    let (_, node_col) = source.offset_to_line_col(node.location().start_offset());
                    diagnostics.extend(check_parens(
                        source,
                        self,
                        open_loc,
                        close_loc,
                        call.arguments(),
                        node_col,
                        config,
                        corrections.as_mut().map(|c| &mut **c),
                    ));
                    return;
                }
            }
            return;
        }

        // Handle grouped expressions: (expr)
        // In Parser gem these are `begin` nodes; in Prism they are ParenthesesNode.
        if let Some(parens) = node.as_parentheses_node() {
            let open_loc = parens.opening_loc();
            let close_loc = parens.closing_loc();
            if close_loc.as_slice() == b")" {
                diagnostics.extend(check_grouped_parens(
                    source,
                    self,
                    open_loc,
                    close_loc,
                    parens.body(),
                    config,
                    corrections.as_mut().map(|c| &mut **c),
                ));
            }
            return;
        }

        // Handle method definitions with parenthesized parameters
        if let Some(def_node) = node.as_def_node() {
            let lparen = def_node.lparen_loc();
            let rparen = def_node.rparen_loc();
            if let (Some(open_loc), Some(close_loc)) = (lparen, rparen) {
                diagnostics.extend(check_def_parens(
                    source,
                    self,
                    open_loc,
                    close_loc,
                    def_node.parameters(),
                    config,
                    corrections.as_mut().map(|c| &mut **c),
                ));
            }
        }
    }
}

fn check_parens(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
    node_col: usize,
    config: &CopConfig,
    mut corrections: Option<&mut Vec<Correction>>,
) -> Vec<Diagnostic> {
    let (open_line, open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    if !util::begins_its_line(source, close_loc.start_offset()) || close_line == open_line {
        return Vec::new();
    }

    let args = match arguments {
        Some(a) => a,
        None => {
            let open_line_indent = util::line_at(source, open_line)
                .map(leading_whitespace_columns)
                .unwrap_or(0);
            if close_col != open_line_indent && close_col != open_col && close_col != node_col {
                return vec![corrected_close_paren_diagnostic(
                    source,
                    cop,
                    close_line,
                    close_col,
                    open_line_indent,
                    format!("Indent `)` to column {} (not {}).", open_line_indent, close_col),
                    &mut corrections,
                )];
            }
            return Vec::new();
        }
    };

    let first_arg = match args.arguments().iter().next() {
        Some(a) => a,
        None => return Vec::new(),
    };
    let (first_arg_line, _) = source.offset_to_line_col(first_arg.location().start_offset());
    let indent_width = config.get_usize("IndentationWidth", 2);

    if first_arg_line > open_line {
        let first_arg_line_indent = util::line_at(source, first_arg_line)
            .map(leading_whitespace_columns)
            .unwrap_or(0);
        let expected = first_arg_line_indent.saturating_sub(indent_width);
        if close_col != expected {
            return vec![corrected_close_paren_diagnostic(
                source,
                cop,
                close_line,
                close_col,
                expected,
                format!("Indent `)` to column {} (not {}).", expected, close_col),
                &mut corrections,
            )];
        }
        return Vec::new();
    }

    let first_arg = args.arguments().iter().next().unwrap();
    let element_columns: Vec<usize> =
        if first_arg.as_keyword_hash_node().is_some() || first_arg.as_hash_node().is_some() {
            let pairs: Vec<ruby_prism::Node<'_>> = if let Some(kh) = first_arg.as_keyword_hash_node() {
                kh.elements().iter().collect()
            } else if let Some(h) = first_arg.as_hash_node() {
                h.elements().iter().collect()
            } else {
                vec![]
            };
            pairs
                .iter()
                .map(|p| source.offset_to_line_col(p.location().start_offset()).1)
                .collect()
        } else {
            args.arguments()
                .iter()
                .map(|a| source.offset_to_line_col(a.location().start_offset()).1)
                .collect()
        };

    let all_aligned = !element_columns.is_empty() && element_columns.iter().all(|&c| c == element_columns[0]);
    if all_aligned {
        if close_col != open_col {
            return vec![corrected_close_paren_diagnostic(
                source,
                cop,
                close_line,
                close_col,
                open_col,
                "Align `)` with `(`.".to_string(),
                &mut corrections,
            )];
        }
        return Vec::new();
    }

    let open_line_indent = util::line_at(source, open_line)
        .map(leading_whitespace_columns)
        .unwrap_or(0);
    let first_arg_line_indent = util::line_at(source, first_arg_line)
        .map(leading_whitespace_columns)
        .unwrap_or(0);
    if close_col != first_arg_line_indent && close_col != open_line_indent {
        return vec![corrected_close_paren_diagnostic(
            source,
            cop,
            close_line,
            close_col,
            open_line_indent,
            format!("Indent `)` to column {} (not {}).", open_line_indent, close_col),
            &mut corrections,
        )];
    }

    Vec::new()
}

fn check_def_parens(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    params: Option<ruby_prism::ParametersNode<'_>>,
    config: &CopConfig,
    mut corrections: Option<&mut Vec<Correction>>,
) -> Vec<Diagnostic> {
    let (open_line, open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    if !util::begins_its_line(source, close_loc.start_offset()) || close_line == open_line {
        return Vec::new();
    }

    let params = match params {
        Some(p) => p,
        None => {
            let open_line_indent = util::line_at(source, open_line)
                .map(leading_whitespace_columns)
                .unwrap_or(0);
            if close_col != open_line_indent && close_col != open_col {
                return vec![corrected_close_paren_diagnostic(
                    source,
                    cop,
                    close_line,
                    close_col,
                    open_line_indent,
                    format!("Indent `)` to column {} (not {}).", open_line_indent, close_col),
                    &mut corrections,
                )];
            }
            return Vec::new();
        }
    };

    let first_param = params
        .requireds()
        .iter()
        .next()
        .or_else(|| params.optionals().iter().next())
        .or_else(|| params.posts().iter().next())
        .or_else(|| params.keywords().iter().next());

    let first_param_offset = first_param.as_ref().map(|p| p.location().start_offset());
    let rest_offset = params.rest().map(|r| r.location().start_offset());
    let keyword_rest_offset = params.keyword_rest().map(|kr| kr.location().start_offset());
    let block_offset = params.block().map(|b| b.location().start_offset());

    let earliest_offset = [first_param_offset, rest_offset, keyword_rest_offset, block_offset]
        .into_iter()
        .flatten()
        .min();
    let earliest_offset = match earliest_offset {
        Some(o) => o,
        None => return Vec::new(),
    };

    let (first_param_line, _) = source.offset_to_line_col(earliest_offset);
    let indent_width = config.get_usize("IndentationWidth", 2);

    if first_param_line > open_line {
        let first_param_line_indent = util::line_at(source, first_param_line)
            .map(leading_whitespace_columns)
            .unwrap_or(0);
        let expected = first_param_line_indent.saturating_sub(indent_width);
        if close_col != expected {
            return vec![corrected_close_paren_diagnostic(
                source,
                cop,
                close_line,
                close_col,
                expected,
                format!("Indent `)` to column {} (not {}).", expected, close_col),
                &mut corrections,
            )];
        }
        return Vec::new();
    }

    let param_columns: Vec<usize> = collect_def_param_columns(source, &params);
    let all_aligned = !param_columns.is_empty() && param_columns.iter().all(|&c| c == param_columns[0]);

    if all_aligned {
        if close_col != open_col {
            return vec![corrected_close_paren_diagnostic(
                source,
                cop,
                close_line,
                close_col,
                open_col,
                "Align `)` with `(`.".to_string(),
                &mut corrections,
            )];
        }
        return Vec::new();
    }

    let open_line_indent = util::line_at(source, open_line)
        .map(leading_whitespace_columns)
        .unwrap_or(0);
    let first_param_line_indent = util::line_at(source, first_param_line)
        .map(leading_whitespace_columns)
        .unwrap_or(0);
    if close_col != first_param_line_indent && close_col != open_line_indent {
        return vec![corrected_close_paren_diagnostic(
            source,
            cop,
            close_line,
            close_col,
            open_line_indent,
            format!("Indent `)` to column {} (not {}).", open_line_indent, close_col),
            &mut corrections,
        )];
    }

    Vec::new()
}

/// Collect column positions for all def parameters.
fn collect_def_param_columns(
    source: &SourceFile,
    params: &ruby_prism::ParametersNode<'_>,
) -> Vec<usize> {
    let mut columns = Vec::new();
    for p in params.requireds().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    for p in params.optionals().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    for p in params.posts().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    for p in params.keywords().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    if let Some(r) = params.rest() {
        let (_, col) = source.offset_to_line_col(r.location().start_offset());
        columns.push(col);
    }
    if let Some(kr) = params.keyword_rest() {
        let (_, col) = source.offset_to_line_col(kr.location().start_offset());
        columns.push(col);
    }
    if let Some(b) = params.block() {
        let (_, col) = source.offset_to_line_col(b.location().start_offset());
        columns.push(col);
    }
    columns
}

/// Check closing parenthesis indentation for grouped expressions: `(expr)`.
/// In RuboCop's Parser gem, `(expr)` produces a `begin` node handled by `on_begin`.
/// `on_begin` calls `check(node, node.children)` — the children of the begin node
/// are the expressions inside the parens.
fn check_grouped_parens(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    body: Option<ruby_prism::Node<'_>>,
    config: &CopConfig,
    mut corrections: Option<&mut Vec<Correction>>,
) -> Vec<Diagnostic> {
    let (open_line, open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    if !util::begins_its_line(source, close_loc.start_offset()) || close_line == open_line {
        return Vec::new();
    }

    let body = match body {
        Some(b) => b,
        None => {
            let open_line_indent = util::line_at(source, open_line)
                .map(leading_whitespace_columns)
                .unwrap_or(0);
            if close_col != open_col && close_col != open_line_indent {
                return vec![corrected_close_paren_diagnostic(
                    source,
                    cop,
                    close_line,
                    close_col,
                    open_line_indent,
                    format!("Indent `)` to column {} (not {}).", open_line_indent, close_col),
                    &mut corrections,
                )];
            }
            return Vec::new();
        }
    };

    let first_element = if let Some(stmts) = body.as_statements_node() {
        match stmts.body().iter().next() {
            Some(n) => n,
            None => return Vec::new(),
        }
    } else {
        body
    };

    let (first_elem_line, _) = source.offset_to_line_col(first_element.location().start_offset());
    let indent_width = config.get_usize("IndentationWidth", 2);

    if first_elem_line > open_line {
        let first_elem_line_indent = util::line_at(source, first_elem_line)
            .map(leading_whitespace_columns)
            .unwrap_or(0);
        let expected = first_elem_line_indent.saturating_sub(indent_width);
        if close_col != expected {
            return vec![corrected_close_paren_diagnostic(
                source,
                cop,
                close_line,
                close_col,
                expected,
                format!("Indent `)` to column {} (not {}).", expected, close_col),
                &mut corrections,
            )];
        }
        return Vec::new();
    }

    let open_line_indent = util::line_at(source, open_line)
        .map(leading_whitespace_columns)
        .unwrap_or(0);
    if close_col != open_col && close_col != open_line_indent {
        return vec![corrected_close_paren_diagnostic(
            source,
            cop,
            close_line,
            close_col,
            open_line_indent,
            format!("Indent `)` to column {} (not {}).", open_line_indent, close_col),
            &mut corrections,
        )];
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        ClosingParenthesisIndentation,
        "cops/layout/closing_parenthesis_indentation"
    );
    crate::cop_autocorrect_fixture_tests!(
        ClosingParenthesisIndentation,
        "cops/layout/closing_parenthesis_indentation"
    );
}
