use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::node_type::{BEGIN_NODE, CASE_MATCH_NODE, CASE_NODE, IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that there are no repeated bodies within `if/unless`, `case-when`,
/// `case-in` and `rescue` constructs.
///
/// ## Root cause analysis (528 FN, 0 FP at 79.1% match rate)
///
/// The original implementation was missing several branch types and config options:
///
/// 1. **rescue branches** - `begin/rescue` constructs were completely unhandled.
///    RuboCop's `on_rescue` checks rescue clause bodies and the else clause.
///    Fixed by handling `BEGIN_NODE` and walking `rescue_clause()` / `subsequent()` chain.
///
/// 2. **case-in (pattern matching)** - `CaseMatchNode` / `InNode` were not handled.
///    Fixed by adding `CASE_MATCH_NODE` to interested types and `check_case_match_branches`.
///
/// 3. **unless** - `UnlessNode` is separate from `IfNode` in Prism. Was not handled.
///    Fixed by adding `UNLESS_NODE` and treating it like a 2-branch if/else.
///
/// 4. **ternary** - Ternary operators parse as `IfNode` with `if_keyword_loc() == None`.
///    Were already handled by the `IfNode` path, but the offense location was wrong.
///    RuboCop reports on the false-branch expression for ternaries, and on the
///    `else` keyword for else-branch duplicates.
///
/// 5. **else branch in case/when** - `check_case_branches` didn't include the
///    `else_clause` body, so `case x; when a; foo; else; foo; end` was missed.
///    Fixed by including the else clause in the branch set.
///
/// 6. **Config options** - `IgnoreLiteralBranches`, `IgnoreConstantBranches`, and
///    `IgnoreDuplicateElseBranch` were read but never applied. All three are now wired up.
///
/// 7. **Offense location** - RuboCop reports on the `else` keyword for else-branch
///    duplicates, and on the parent clause node (elsif/when/rescue/in) for others.
///    The ternary case reports on the false-branch expression itself. Fixed to match.
///
/// ## Follow-up (2026-03-10)
///
/// Remaining FNs include branches whose source differs but whose literal values
/// are equivalent. One real corpus case used two string literals with different
/// escape spellings (`"\u2028"` vs `"\342\200\250"`) that both decode to the
/// same Ruby string, which RuboCop treats as duplicate branch bodies.
///
/// Fix: keep the existing source-based comparison for general branches, but
/// canonicalize single string-literal branches by their unescaped bytes so those
/// escape-equivalent forms compare equal. Also treat `KeywordHashNode` as a
/// literal branch for the `IgnoreLiteralBranches` config path to satisfy the
/// Prism hash/keyword-hash split.
///
/// ## Follow-up (2026-03-18)
///
/// Remaining 15 FN were caused by whitespace-only differences between branches.
/// RuboCop compares branches by AST structure (which ignores whitespace and
/// comments), but nitrocop was comparing raw source bytes.
///
/// Examples: `{|child| check(child)}` vs `{|child| check(child) }` (trailing
/// space before `}`), or rescue branches with extra blank lines between
/// statements.
///
/// Fix: normalize the source comparison key by collapsing all whitespace runs
/// to a single space. Comments are already excluded since Prism's StatementsNode
/// source range doesn't include them. String literals continue to use unescaped
/// content for escape-equivalence.
pub struct DuplicateBranch;

impl Cop for DuplicateBranch {
    fn name(&self) -> &'static str {
        "Lint/DuplicateBranch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, CASE_NODE, CASE_MATCH_NODE, BEGIN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ignore_literal = config.get_bool("IgnoreLiteralBranches", false);
        let ignore_constant = config.get_bool("IgnoreConstantBranches", false);
        let ignore_dup_else = config.get_bool("IgnoreDuplicateElseBranch", false);

        if let Some(if_node) = node.as_if_node() {
            check_if_branches(
                self,
                source,
                &if_node,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
                diagnostics,
            );
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            check_unless_branches(
                self,
                source,
                &unless_node,
                ignore_literal,
                ignore_constant,
                diagnostics,
            );
            return;
        }

        if let Some(case_node) = node.as_case_node() {
            check_case_branches(
                self,
                source,
                &case_node,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
                diagnostics,
            );
            return;
        }

        if let Some(case_match_node) = node.as_case_match_node() {
            check_case_match_branches(
                self,
                source,
                &case_match_node,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
                diagnostics,
            );
            return;
        }

        if let Some(begin_node) = node.as_begin_node() {
            if begin_node.rescue_clause().is_some() {
                check_rescue_branches(
                    self,
                    source,
                    &begin_node,
                    ignore_literal,
                    ignore_constant,
                    ignore_dup_else,
                    diagnostics,
                );
            }
        }
    }
}

/// Extract a comparison key for branch body.
///
/// RuboCop compares branches by AST structure, not raw source text. Two branches
/// with different whitespace or string escape spellings are duplicates if they
/// produce the same AST.
///
/// We approximate this by normalizing the source text:
/// 1. For single string-literal branches, use unescaped byte content so
///    equivalent escape spellings compare equal.
/// 2. For heredocs, extend the source range to include the closing delimiter.
/// 3. Collapse all whitespace runs to a single space, so formatting differences
///    (extra spaces, blank lines, indentation) don't prevent duplicate detection.
fn stmts_source(source: &SourceFile, stmts: &Option<ruby_prism::StatementsNode<'_>>) -> Vec<u8> {
    match stmts {
        Some(s) => {
            let body = s.body();
            if body.len() == 1 {
                if let Some(node) = body.iter().next() {
                    if let Some(string) = node.as_string_node() {
                        let mut fingerprint = b"string:".to_vec();
                        fingerprint.extend_from_slice(string.unescaped());
                        return fingerprint;
                    }
                }
            }

            let loc = s.location();
            let start = loc.start_offset();
            let mut end = loc.end_offset();

            let mut finder = MaxExtentFinder { max_end: end };
            finder.visit(&s.as_node());
            end = finder.max_end;

            let bytes = source.as_bytes();
            let raw = if end <= bytes.len() {
                &bytes[start..end]
            } else {
                loc.as_slice()
            };

            normalize_whitespace(raw)
        }
        None => Vec::new(),
    }
}

/// Strip all ASCII whitespace, inserting a single space only between two
/// adjacent word characters (alphanumeric or underscore) to prevent identifier
/// merging. This matches RuboCop's AST-based comparison which ignores all
/// formatting whitespace.
fn normalize_whitespace(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(src.len());
    let mut pending_ws = false;
    for &b in src {
        if b.is_ascii_whitespace() {
            pending_ws = true;
        } else {
            // Insert a space only if both the previous and current characters
            // are word characters (to avoid merging `return []` into `return[]`)
            if pending_ws && !result.is_empty() {
                let prev = *result.last().unwrap();
                if is_word_byte(prev) && is_word_byte(b) {
                    result.push(b' ');
                }
            }
            result.push(b);
            pending_ws = false;
        }
    }
    result
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

struct MaxExtentFinder {
    max_end: usize,
}

impl<'pr> Visit<'pr> for MaxExtentFinder {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_string_node(self, node);
    }
}

/// Returns true if a branch body is a literal that should be ignored when
/// `IgnoreLiteralBranches` is true.
fn is_literal_branch(
    stmts: &Option<ruby_prism::StatementsNode<'_>>,
    ignore_constant: bool,
) -> bool {
    let stmts = match stmts {
        Some(s) => s,
        None => return false,
    };
    let body = stmts.body();
    if body.len() != 1 {
        return false;
    }
    let node = match body.iter().next() {
        Some(n) => n,
        None => return false,
    };
    is_literal_node(&node, ignore_constant)
}

fn is_literal_node(node: &ruby_prism::Node<'_>, ignore_constant: bool) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
    {
        return true;
    }

    if ignore_constant
        && (node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some())
    {
        return true;
    }

    if node.as_regular_expression_node().is_some() {
        return true;
    }

    if let Some(range) = node.as_range_node() {
        let left_ok = range
            .left()
            .is_none_or(|l| is_literal_node(&l, ignore_constant));
        let right_ok = range
            .right()
            .is_none_or(|r| is_literal_node(&r, ignore_constant));
        return left_ok && right_ok;
    }

    if let Some(arr) = node.as_array_node() {
        return arr
            .elements()
            .iter()
            .all(|e| is_literal_node(&e, ignore_constant));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal_node(&assoc.key(), ignore_constant)
                    && is_literal_node(&assoc.value(), ignore_constant)
            } else {
                false
            }
        });
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal_node(&assoc.key(), ignore_constant)
                    && is_literal_node(&assoc.value(), ignore_constant)
            } else {
                false
            }
        });
    }

    false
}

fn is_constant_branch(stmts: &Option<ruby_prism::StatementsNode<'_>>) -> bool {
    let stmts = match stmts {
        Some(s) => s,
        None => return false,
    };
    let body = stmts.body();
    if body.len() != 1 {
        return false;
    }
    let node = match body.iter().next() {
        Some(n) => n,
        None => return false,
    };
    node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some()
}

/// Check if a branch should be considered for duplicate detection based on config.
#[allow(clippy::too_many_arguments)]
fn should_consider(
    stmts: &Option<ruby_prism::StatementsNode<'_>>,
    body: &[u8],
    is_else: bool,
    is_last: bool,
    total_branches: usize,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
) -> bool {
    if body.is_empty() {
        return false;
    }
    if ignore_literal && is_literal_branch(stmts, ignore_constant) {
        return false;
    }
    if ignore_constant && is_constant_branch(stmts) {
        return false;
    }
    if ignore_dup_else && is_else && is_last && total_branches > 2 {
        return false;
    }
    true
}

fn emit(
    cop: &DuplicateBranch,
    source: &SourceFile,
    offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, column) = source.offset_to_line_col(offset);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Duplicate branch body detected.".to_string(),
    ));
}

/// A collected branch: body bytes, reporting offset, else flag, last flag, statements.
struct BranchInfo<'pr> {
    body: Vec<u8>,
    report_offset: usize,
    is_else: bool,
    is_last: bool,
    stmts: Option<ruby_prism::StatementsNode<'pr>>,
}

fn check_if_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    if_node: &ruby_prism::IfNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip elsif nodes - only process the outermost if.
    // In Prism, elsif is a nested IfNode whose if_keyword_loc() says "elsif".
    if let Some(kw_loc) = if_node.if_keyword_loc() {
        if kw_loc.as_slice() == b"elsif" {
            return;
        }
    }

    let is_ternary = if_node.if_keyword_loc().is_none();

    // Count total branches
    let mut total = 1usize;
    let mut sub = if_node.subsequent();
    while let Some(s) = sub {
        total += 1;
        if let Some(elsif) = s.as_if_node() {
            sub = elsif.subsequent();
        } else {
            break;
        }
    }

    let mut branches: Vec<BranchInfo<'_>> = Vec::new();

    // The if/ternary true branch
    let if_stmts = if_node.statements();
    let if_body = stmts_source(source, &if_stmts);
    branches.push(BranchInfo {
        body: if_body,
        report_offset: if_node.location().start_offset(),
        is_else: false,
        is_last: false,
        stmts: if_stmts,
    });

    // Walk elsif/else chain
    let mut idx = 1usize;
    let mut subsequent = if_node.subsequent();
    while let Some(sub) = subsequent {
        idx += 1;
        let is_last = idx == total;
        if let Some(elsif) = sub.as_if_node() {
            let stmts = elsif.statements();
            let body = stmts_source(source, &stmts);
            branches.push(BranchInfo {
                body,
                report_offset: elsif.location().start_offset(),
                is_else: false,
                is_last,
                stmts,
            });
            subsequent = elsif.subsequent();
        } else if let Some(else_node) = sub.as_else_node() {
            let stmts = else_node.statements();
            let body = stmts_source(source, &stmts);
            let report_offset = if is_ternary {
                // For ternary, report on the false-branch expression itself
                if let Some(ref s) = stmts {
                    let s_body = s.body();
                    if let Some(first) = s_body.first() {
                        first.location().start_offset()
                    } else {
                        else_node.else_keyword_loc().start_offset()
                    }
                } else {
                    else_node.else_keyword_loc().start_offset()
                }
            } else {
                else_node.else_keyword_loc().start_offset()
            };
            branches.push(BranchInfo {
                body,
                report_offset,
                is_else: true,
                is_last: true,
                stmts,
            });
            break;
        } else {
            break;
        }
    }

    if branches.len() < 2 {
        return;
    }

    let total_branches = branches.len();
    let mut seen = HashSet::new();

    for bi in &branches {
        if !should_consider(
            &bi.stmts,
            &bi.body,
            bi.is_else,
            bi.is_last,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) {
            continue;
        }
        if !seen.insert(bi.body.clone()) {
            emit(cop, source, bi.report_offset, diagnostics);
        }
    }
}

fn check_unless_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    unless_node: &ruby_prism::UnlessNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // unless only has 2 branches (body and else), so IgnoreDuplicateElseBranch
    // doesn't apply (requires > 2 branches).
    let else_clause = match unless_node.else_clause() {
        Some(e) => e,
        None => return,
    };

    let body_stmts = unless_node.statements();
    let body_src = stmts_source(source, &body_stmts);

    let else_stmts = else_clause.statements();
    let else_src = stmts_source(source, &else_stmts);

    if body_src.is_empty() || else_src.is_empty() {
        return;
    }

    if ignore_literal
        && is_literal_branch(&body_stmts, ignore_constant)
        && is_literal_branch(&else_stmts, ignore_constant)
    {
        return;
    }

    if body_src == else_src {
        emit(
            cop,
            source,
            else_clause.else_keyword_loc().start_offset(),
            diagnostics,
        );
    }
}

fn check_case_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    case_node: &ruby_prism::CaseNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let conditions = case_node.conditions();
    let has_else = case_node.else_clause().is_some();
    let total_branches = conditions.len() + if has_else { 1 } else { 0 };

    let mut seen = HashSet::new();

    for when_ref in conditions.iter() {
        if let Some(when_node) = when_ref.as_when_node() {
            let stmts = when_node.statements();
            let body = stmts_source(source, &stmts);
            if !should_consider(
                &stmts,
                &body,
                false,
                false,
                total_branches,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
            ) {
                continue;
            }
            if !seen.insert(body) {
                emit(
                    cop,
                    source,
                    when_node.keyword_loc().start_offset(),
                    diagnostics,
                );
            }
        }
    }

    if let Some(else_clause) = case_node.else_clause() {
        let stmts = else_clause.statements();
        let body = stmts_source(source, &stmts);
        if should_consider(
            &stmts,
            &body,
            true,
            true,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) && !seen.insert(body)
        {
            emit(
                cop,
                source,
                else_clause.else_keyword_loc().start_offset(),
                diagnostics,
            );
        }
    }
}

fn check_case_match_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    case_match_node: &ruby_prism::CaseMatchNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let conditions = case_match_node.conditions();
    let has_else = case_match_node.else_clause().is_some();
    let total_branches = conditions.len() + if has_else { 1 } else { 0 };

    let mut seen = HashSet::new();

    for in_ref in conditions.iter() {
        if let Some(in_node) = in_ref.as_in_node() {
            let stmts = in_node.statements();
            let body = stmts_source(source, &stmts);
            if !should_consider(
                &stmts,
                &body,
                false,
                false,
                total_branches,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
            ) {
                continue;
            }
            if !seen.insert(body) {
                emit(cop, source, in_node.in_loc().start_offset(), diagnostics);
            }
        }
    }

    if let Some(else_clause) = case_match_node.else_clause() {
        let stmts = else_clause.statements();
        let body = stmts_source(source, &stmts);
        if should_consider(
            &stmts,
            &body,
            true,
            true,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) && !seen.insert(body)
        {
            emit(
                cop,
                source,
                else_clause.else_keyword_loc().start_offset(),
                diagnostics,
            );
        }
    }
}

fn check_rescue_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    begin_node: &ruby_prism::BeginNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut rescue_data: Vec<(Vec<u8>, usize, Option<ruby_prism::StatementsNode<'_>>)> = Vec::new();

    let mut rescue_opt = begin_node.rescue_clause();
    while let Some(rescue_node) = rescue_opt {
        let stmts = rescue_node.statements();
        let body = stmts_source(source, &stmts);
        let offset = rescue_node.keyword_loc().start_offset();
        rescue_data.push((body, offset, stmts));
        rescue_opt = rescue_node.subsequent();
    }

    let has_else = begin_node.else_clause().is_some();
    let total_branches = rescue_data.len() + if has_else { 1 } else { 0 };

    let mut seen = HashSet::new();

    for (body, offset, stmts) in &rescue_data {
        if !should_consider(
            stmts,
            body,
            false,
            false,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) {
            continue;
        }
        if !seen.insert(body.clone()) {
            emit(cop, source, *offset, diagnostics);
        }
    }

    if let Some(else_clause) = begin_node.else_clause() {
        let stmts = else_clause.statements();
        let body = stmts_source(source, &stmts);
        if should_consider(
            &stmts,
            &body,
            true,
            true,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) && !seen.insert(body)
        {
            emit(
                cop,
                source,
                else_clause.else_keyword_loc().start_offset(),
                diagnostics,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateBranch, "cops/lint/duplicate_branch");

    #[test]
    fn keyword_hash_counts_as_literal_branch() {
        let result = ruby_prism::parse(b"call(foo: 1)\n");
        let root = result.node();
        let program = root.as_program_node().unwrap();
        let stmts = program.statements();
        let call = stmts.body().iter().next().unwrap().as_call_node().unwrap();
        let arg = call.arguments().unwrap().arguments().iter().next().unwrap();

        assert!(arg.as_keyword_hash_node().is_some());
        assert!(is_literal_node(&arg, false));
    }

    #[test]
    fn whitespace_normalization() {
        // Whitespace is stripped, with spaces preserved only between word chars
        assert_eq!(
            normalize_whitespace(b"foo(x)  .bar  {|y| baz(y)}"),
            b"foo(x).bar{|y|baz(y)}"
        );
        // Both should match after normalization when the only difference is
        // trailing space before closing brace
        assert_eq!(
            normalize_whitespace(b"each_child(node).all? {|child| check(child)}"),
            normalize_whitespace(b"each_child(node).all? {|child| check(child) }"),
        );
        // Blank lines between statements collapse
        assert_eq!(
            normalize_whitespace(b"report(e)\n  false"),
            normalize_whitespace(b"report(e)\n\n  false"),
        );
        // Space only preserved between two word chars
        assert_eq!(normalize_whitespace(b"return []"), b"return[]");
        // Space preserved between identifiers
        assert_eq!(normalize_whitespace(b"foo bar baz"), b"foo bar baz");
    }
}
