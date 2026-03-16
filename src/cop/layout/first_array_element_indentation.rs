use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Byte offset of the first non-whitespace character on a line.
/// Equivalent to RuboCop's `source_line =~ /\S/`. Handles both spaces and tabs.
/// Returns the byte count of leading whitespace (tabs count as 1 byte each).
fn first_non_whitespace_column(line: &[u8]) -> usize {
    line.iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

/// Layout/FirstArrayElementIndentation cop.
///
/// ## Investigation findings (2026-03-15)
///
/// **FP root cause (208 FPs, now fixed):** `find_left_paren_on_line` was finding
/// the `(` of method calls like `gc.draw('fmt' % [...])` where the `%` operator
/// sits between `(` and `[`. The array is the RHS of `%`, not a direct argument
/// of the method call. Fix: `is_preceded_by_percent_operator()` checks if `[` is
/// immediately preceded by `%` and falls back to line-relative indent.
///
/// **FP root cause #2 (subset):** `find_left_paren_on_line` stopped at unmatched
/// `{` (hash literal) before `[`, returning `None` and using line-relative. This
/// caused missed offenses for patterns like `method({ key: [...] })` where RuboCop
/// uses paren-relative. Fix: continue scanning past unmatched `{` to find `(`.
/// `is_direct_argument` now also scans past `}` after `]` to detect hash chaining
/// (`{ key: [...] }.to_json`), correctly falling back to line-relative in that case.
///
/// **FP root cause #3 (2026-03-16, 138 FPs):** Two sub-causes:
/// a) Tab indentation: `indentation_of()` only counted spaces, returning 0 for
///    tab-indented lines. But `offset_to_line_col` counted tabs as 1 character.
///    So closing bracket `\t]` had close_col=1 but indent_base=0. Fix: use
///    `first_non_whitespace_column()` (byte offset of first non-whitespace char,
///    matching RuboCop's `source_line =~ /\S/`) for both sides of the comparison.
///    This fixed WhatWeb (~54 FPs) and phlex (~23 FPs).
/// b) Array inside hash that is chained: `method({ key: [...], k2: v }.to_json)`
///    — `is_direct_argument` checked after `]`, saw `,` (hash entry separator),
///    and returned true. But the enclosing hash is chained (`.to_json`), so
///    RuboCop uses line-relative indent. Fix: `find_left_paren_on_line` now
///    tracks unmatched `{` between `(` and `[`; when present, `is_direct_argument`
///    scans forward past hash entries to find `}`, then checks after `}`.
///    This fixed restforce (~24 FPs) and similar patterns.
///
/// **FP root cause #4 (2026-03-16):** Three additional sub-causes:
/// a) Array chained inside hash: `{ "c" => [...].compact }` — when `inside_hash`
///    is true, `is_direct_argument` scanned for `}` without first checking if `]`
///    is directly followed by `.compact`. Fix: check array chain before hash scan.
/// b) Binary operator between `(` and `[`: `(CONST + [...]).freeze` — the `(`
///    is a grouping paren, not a method call paren. Fix: `find_left_paren_on_line`
///    now detects binary operators at depth 0 between `(` and `[`.
/// c) Hash-key-relative indentation: `{ ruby: [...], js: [...] }` — elements
///    indented relative to hash key, not line start. Fix: detect hash key before
///    `[` and accept `key_col + width` / `key_col` as valid indentation.
pub struct FirstArrayElementIndentation;

/// Describes what the expected indentation is relative to.
#[derive(Clone, Copy)]
enum IndentBaseType {
    /// `align_brackets` style: relative to the opening bracket `[`
    LeftBracket,
    /// `special_inside_parentheses`: relative to the first position after `(`
    FirstColumnAfterLeftParenthesis,
    /// Default: relative to the start of the line where `[` appears
    StartOfLine,
}

/// Result of scanning backwards from `[` to find an enclosing `(`.
struct ParenScanResult {
    /// Column of the unmatched `(`, if found.
    paren_col: Option<usize>,
    /// Whether there is an unmatched `{` between the `(` and `[`,
    /// indicating the array is nested inside a hash literal.
    has_unmatched_brace: bool,
    /// Whether there is a binary operator (`+`, `-`, `*`, `/`, `|`, `&`, `^`)
    /// at depth 0 between `(` and `[`, indicating the `(` is a grouping
    /// paren and the array is part of an expression (e.g., `(CONST + [...])`).
    has_binary_operator_at_depth_zero: bool,
}

/// Scan backwards from `bracket_col` on `line_bytes` to find an unmatched `(`
/// that contains this array. Also tracks whether there's an unmatched `{`
/// between `(` and `[`, indicating hash nesting, and whether there's a binary
/// operator at depth 0 (indicating grouping parens).
///
/// This tracks balanced parens, brackets, and braces. Unmatched `{` or `[`
/// are allowed — the array may be nested inside a hash literal or another
/// array that is itself inside method call parens (e.g.,
/// `method({ key: [...] })`). Only an unmatched `(` is returned.
fn find_left_paren_on_line(line_bytes: &[u8], bracket_col: usize) -> ParenScanResult {
    let end = bracket_col.min(line_bytes.len());
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut has_unmatched_brace = false;
    let mut has_binary_op = false;
    for i in (0..end).rev() {
        match line_bytes[i] {
            b')' => paren_depth += 1,
            b'(' => {
                if paren_depth == 0 {
                    return ParenScanResult {
                        paren_col: Some(i),
                        has_unmatched_brace,
                        has_binary_operator_at_depth_zero: has_binary_op,
                    };
                }
                paren_depth -= 1;
            }
            b']' => bracket_depth += 1,
            b'[' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            b'}' => brace_depth += 1,
            b'{' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                } else {
                    has_unmatched_brace = true;
                }
            }
            // Detect binary operators at depth 0 (not inside nested parens/brackets/braces).
            // These indicate the `(` is a grouping paren, e.g., `(CONST + [...])`.
            b'+' | b'-' | b'*' | b'/' | b'|' | b'&' | b'^'
                if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 =>
            {
                has_binary_op = true;
            }
            _ => {}
        }
    }
    ParenScanResult {
        paren_col: None,
        has_unmatched_brace,
        has_binary_operator_at_depth_zero: has_binary_op,
    }
}

/// Find the column of a hash key that precedes `[` on the same line.
/// Detects patterns like `key: [`, `key => [`, and `"key" => [`.
/// Returns the column of the hash key's first character, or `None` if
/// no hash key is found.
fn find_hash_key_column(line_bytes: &[u8], bracket_col: usize) -> Option<usize> {
    let end = bracket_col.min(line_bytes.len());
    if end == 0 {
        return None;
    }
    let mut i = end;
    loop {
        if i == 0 {
            return None;
        }
        i -= 1;
        if line_bytes[i] != b' ' && line_bytes[i] != b'\t' {
            break;
        }
    }
    if line_bytes[i] == b'>' && i > 0 && line_bytes[i - 1] == b'=' {
        // `=> [` — scan back past `=>` and whitespace to find key start
        i -= 1;
        while i > 0 && (line_bytes[i - 1] == b' ' || line_bytes[i - 1] == b'\t') {
            i -= 1;
        }
        if i == 0 {
            return None;
        }
        let key_end = i;
        if line_bytes[key_end - 1] == b'"' || line_bytes[key_end - 1] == b'\'' {
            let quote = line_bytes[key_end - 1];
            if key_end < 2 {
                return None;
            }
            let mut j = key_end - 2;
            while j > 0 && line_bytes[j] != quote {
                j -= 1;
            }
            return Some(j);
        }
        let mut j = key_end - 1;
        while j > 0
            && (line_bytes[j - 1].is_ascii_alphanumeric()
                || line_bytes[j - 1] == b'_'
                || line_bytes[j - 1] == b':')
        {
            j -= 1;
        }
        return Some(j);
    }
    // Ruby 1.9 hash syntax: `key: [`
    if line_bytes[i] != b':' {
        return None;
    }
    if i == 0 || !(line_bytes[i - 1].is_ascii_alphanumeric() || line_bytes[i - 1] == b'_') {
        return None;
    }
    if i >= 2 && line_bytes[i - 1] == b':' {
        return None;
    }
    let mut j = i - 1;
    while j > 0
        && (line_bytes[j - 1].is_ascii_alphanumeric()
            || line_bytes[j - 1] == b'_'
            || line_bytes[j - 1] == b'?'
            || line_bytes[j - 1] == b'!')
    {
        j -= 1;
    }
    Some(j)
}

/// Check if the `[` is immediately preceded by a `%` operator (string formatting).
/// In patterns like `gc.draw('format' % [...])`, the array is the RHS of the `%`
/// operator, not a direct argument of the method call's parentheses. Scans
/// backwards from the `[` position, skipping whitespace.
fn is_preceded_by_percent_operator(line_bytes: &[u8], bracket_col: usize) -> bool {
    let end = bracket_col.min(line_bytes.len());
    for i in (0..end).rev() {
        match line_bytes[i] {
            b' ' | b'\t' => continue,
            b'%' => return true,
            _ => return false,
        }
    }
    false
}

/// Check what follows a given position in source bytes, skipping whitespace
/// (but not newlines). Returns `true` if the expression is "chained" or
/// combined with an operator (`.`, `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`).
fn is_chained_after(source_bytes: &[u8], start: usize) -> bool {
    let len = source_bytes.len();
    let mut i = start;
    while i < len && (source_bytes[i] == b' ' || source_bytes[i] == b'\t') {
        i += 1;
    }
    if i >= len {
        return false;
    }
    matches!(
        source_bytes[i],
        b'.' | b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^'
    )
}

/// Check if the array is used as a direct argument (not as a receiver of
/// a method chain or part of a binary expression). Checks the source bytes
/// immediately after the array's closing bracket `]`.
///
/// Returns `true` if the array is a standalone argument (next non-whitespace
/// after `]` is `)`, `,`, end of line, or nothing relevant).
/// Returns `false` if `]` is followed by `.`, `+`, `-`, `*`, etc. indicating
/// the array is part of a larger expression.
///
/// When `inside_hash` is true (the array is inside a hash literal within
/// method parens), this scans forward from `]` to find the matching `}`
/// of the enclosing hash, then checks what follows that `}`.
fn is_direct_argument(source_bytes: &[u8], closing_end_offset: usize, inside_hash: bool) -> bool {
    let mut i = closing_end_offset;
    let len = source_bytes.len();

    // First, always check if the array itself is chained (e.g. `].compact`,
    // `].join`). This takes priority even when inside a hash.
    if is_chained_after(source_bytes, i) {
        return false;
    }

    if inside_hash {
        // The array is inside a hash literal. We need to find the enclosing
        // hash's closing `}` and check what follows it. Scan forward from
        // after `]`, tracking brace/bracket/paren depth to find the matching `}`.
        let mut brace_depth: i32 = 1; // we're inside one unmatched `{`
        let mut bracket_depth: i32 = 0;
        let mut paren_depth: i32 = 0;
        while i < len && brace_depth > 0 {
            match source_bytes[i] {
                b'{' => brace_depth += 1,
                b'}' => brace_depth -= 1,
                b'[' => bracket_depth += 1,
                b']' => bracket_depth -= 1,
                b'(' => paren_depth += 1,
                b')' => paren_depth -= 1,
                b'#' => {
                    // Skip to end of line (comment)
                    while i < len && source_bytes[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                b'\'' | b'"' => {
                    // Skip past string literals (simple — no interpolation tracking)
                    let quote = source_bytes[i];
                    i += 1;
                    while i < len && source_bytes[i] != quote {
                        if source_bytes[i] == b'\\' {
                            i += 1; // skip escaped char
                        }
                        i += 1;
                    }
                    // i now points at closing quote
                }
                _ => {}
            }
            i += 1;
        }
        let _ = (bracket_depth, paren_depth); // suppress unused warnings
        // i is now past the `}`. Check what follows.
        return !is_chained_after(source_bytes, i);
    }

    // Skip whitespace (but not newlines)
    while i < len && (source_bytes[i] == b' ' || source_bytes[i] == b'\t') {
        i += 1;
    }
    if i >= len {
        return true;
    }
    match source_bytes[i] {
        // Array is followed by a method call or operator => part of expression
        b'.' | b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' => false,
        // If `]` is followed by `}`, the array is inside a hash. Skip past
        // the `}` and any closing parens/whitespace to check if the HASH
        // is chained with a method call.
        b'}' => {
            i += 1;
            !is_chained_after(source_bytes, i)
        }
        // Everything else (closing paren, comma, newline, etc.) => direct argument
        _ => true,
    }
}

impl Cop for FirstArrayElementIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstArrayElementIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
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
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let opening_loc = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let elements: Vec<_> = array_node.elements().iter().collect();
        if elements.is_empty() {
            return;
        }

        let first_element = &elements[0];

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let first_loc = first_element.location();
        let (elem_line, elem_col) = source.offset_to_line_col(first_loc.start_offset());

        // Skip if first element is on same line as opening bracket
        if elem_line == open_line {
            return;
        }

        let style = config.get_str("EnforcedStyle", "special_inside_parentheses");
        let width = config.get_usize("IndentationWidth", 2);

        // Get the indentation of the line where `[` appears
        let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = first_non_whitespace_column(open_line_bytes);
        let (_, open_col) = source.offset_to_line_col(opening_loc.start_offset());

        // Check if `[` is preceded by a hash key on the same line.
        let hash_key_col = find_hash_key_column(open_line_bytes, open_col);

        // Compute the indent base column (before adding width) and its type.
        // The first element should be at `indent_base + width`.
        // The closing bracket should be at `indent_base`.
        let (indent_base, base_type) = match style {
            "consistent" => (open_line_indent, IndentBaseType::StartOfLine),
            "align_brackets" => (open_col, IndentBaseType::LeftBracket),
            _ => {
                // "special_inside_parentheses" (default):
                let closing_end = array_node
                    .closing_loc()
                    .map(|loc| loc.end_offset())
                    .unwrap_or(0);

                let paren_scan = find_left_paren_on_line(open_line_bytes, open_col);
                if let Some(paren_col) = paren_scan.paren_col {
                    // If the `[` is on the same line as a method call's `(`,
                    // indent relative to the position after `(`, unless:
                    // - The `[` is preceded by a `%` operator
                    // - There's a binary operator between `(` and `[` at depth 0
                    //   (indicating grouping parens, not method call parens)
                    // - The array (or enclosing hash) is part of a chain or expression
                    let use_paren_relative =
                        !is_preceded_by_percent_operator(open_line_bytes, open_col)
                            && !paren_scan.has_binary_operator_at_depth_zero
                            && is_direct_argument(
                                source.as_bytes(),
                                closing_end,
                                paren_scan.has_unmatched_brace,
                            );
                    if use_paren_relative {
                        (
                            paren_col + 1,
                            IndentBaseType::FirstColumnAfterLeftParenthesis,
                        )
                    } else {
                        (open_line_indent, IndentBaseType::StartOfLine)
                    }
                } else {
                    (open_line_indent, IndentBaseType::StartOfLine)
                }
            }
        };

        let expected_elem = indent_base + width;

        if elem_col != expected_elem {
            // Check if indentation matches hash-key-relative style.
            // RuboCop accepts elements indented relative to the parent
            // hash key when the array is a hash value.
            let matches_hash_key = hash_key_col.is_some_and(|key_col| elem_col == key_col + width);
            if !matches_hash_key {
                let base_description = match base_type {
                    IndentBaseType::LeftBracket => "the position of the opening bracket",
                    IndentBaseType::FirstColumnAfterLeftParenthesis => {
                        "the first position after the preceding left parenthesis"
                    }
                    IndentBaseType::StartOfLine => {
                        "the start of the line where the left square bracket is"
                    }
                };
                diagnostics.push(self.diagnostic(
                    source,
                    elem_line,
                    elem_col,
                    format!(
                        "Use {} spaces for indentation in an array, relative to {}.",
                        width, base_description
                    ),
                ));
            }
        }

        // Check closing bracket indentation
        if let Some(closing_loc) = array_node.closing_loc() {
            let (close_line, close_col) = source.offset_to_line_col(closing_loc.start_offset());

            // Only check if the closing bracket is on its own line
            // (no non-whitespace characters before it on that line)
            let close_line_bytes = source.lines().nth(close_line - 1).unwrap_or(b"");
            let only_whitespace_before = close_line_bytes[..close_col.min(close_line_bytes.len())]
                .iter()
                .all(|&b| b == b' ' || b == b'\t');

            // For StartOfLine, compare using first_non_whitespace_column instead
            // of character column — this matches RuboCop's `source_line =~ /\S/`
            // and handles tab-indented files correctly (tabs count as 1 byte).
            let effective_close_col = match base_type {
                IndentBaseType::StartOfLine => first_non_whitespace_column(close_line_bytes),
                _ => close_col,
            };

            if only_whitespace_before && effective_close_col != indent_base {
                // Check if closing bracket matches hash-key-relative style.
                let matches_hash_key =
                    hash_key_col.is_some_and(|key_col| effective_close_col == key_col);
                if !matches_hash_key {
                    let msg = match base_type {
                        IndentBaseType::LeftBracket => {
                            "Indent the right bracket the same as the left bracket.".to_string()
                        }
                        IndentBaseType::FirstColumnAfterLeftParenthesis => {
                            "Indent the right bracket the same as the first position \
                             after the preceding left parenthesis."
                                .to_string()
                        }
                        IndentBaseType::StartOfLine => {
                            "Indent the right bracket the same as the start of the line \
                             where the left bracket is."
                                .to_string()
                        }
                    };
                    diagnostics.push(self.diagnostic(source, close_line, close_col, msg));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        FirstArrayElementIndentation,
        "cops/layout/first_array_element_indentation"
    );

    #[test]
    fn same_line_elements_ignored() {
        let source = b"x = [1, 2, 3]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn align_brackets_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("align_brackets".into()),
            )]),
            ..CopConfig::default()
        };
        // Element at bracket_col + width (4 + 2 = 6), bracket at bracket_col (4) => good
        let src = b"x = [\n      1\n    ]\n";
        let diags = run_cop_full_with_config(&FirstArrayElementIndentation, src, config.clone());
        assert!(
            diags.is_empty(),
            "align_brackets should accept element at bracket_col + width: {:?}",
            diags
        );

        // Element indented normally (2 from line start) should be flagged
        let src2 = b"x = [\n  1\n]\n";
        let diags2 = run_cop_full_with_config(&FirstArrayElementIndentation, src2, config.clone());
        assert!(
            diags2.len() >= 1,
            "align_brackets should flag element not at bracket_col + width: {:?}",
            diags2
        );

        // Bracket not aligned with opening bracket should be flagged
        let src3 = b"x = [\n      1\n]\n";
        let diags3 = run_cop_full_with_config(&FirstArrayElementIndentation, src3, config);
        assert_eq!(
            diags3.len(),
            1,
            "align_brackets should flag bracket not at opening bracket column: {:?}",
            diags3
        );
    }

    #[test]
    fn special_inside_parentheses_method_call() {
        let src = b"foo([\n      :bar,\n      :baz\n    ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array arg with [ on same line as ( should not be flagged"
        );
    }

    #[test]
    fn special_inside_parentheses_nested_call() {
        let src =
            b"expect(cli.run([\n                 :a,\n                 :b\n               ]))\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "nested call array arg should use innermost paren"
        );
    }

    #[test]
    fn array_with_method_chain_uses_line_indent() {
        let src = b"expect(x).to eq([\n  'hello',\n  'world'\n].join(\"\\n\"))\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array with .join chain should use line-relative indent"
        );
    }

    #[test]
    fn array_in_grouping_paren_uses_line_indent() {
        let src = b"X = (%i[\n  a\n  b\n] + other).freeze\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array in grouping paren with + operator should use line-relative indent"
        );
    }

    #[test]
    fn percent_i_array_inside_method_call_paren() {
        let src = b"eq(%i[\n     :a,\n     :b\n   ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "%i[ inside method call paren should use paren-relative indent: {:?}",
            diags
        );
    }

    #[test]
    fn percent_i_array_inside_method_call_paren_wrong_indent() {
        let src = b"eq(%i[\n  :a,\n  :b\n])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert_eq!(
            diags.len(),
            2,
            "%i[ inside method call paren with wrong indent should flag element and bracket: {:?}",
            diags
        );
    }

    #[test]
    fn closing_bracket_wrong_indent_in_method_call() {
        let src =
            b"    expect(validation_attributes).to eq(%i[\n      client_id\n      client\n    ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert_eq!(
            diags.len(),
            2,
            "should flag both element and bracket in method call: {:?}",
            diags
        );
        let bracket_diag = diags
            .iter()
            .find(|d| d.message.contains("right bracket"))
            .unwrap();
        assert!(
            bracket_diag
                .message
                .contains("first position after the preceding left parenthesis"),
            "bracket message should reference left parenthesis: {}",
            bracket_diag.message
        );
    }

    #[test]
    fn closing_bracket_on_same_line_as_last_element_not_flagged() {
        let src = b"x = [\n  1,\n  2]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "bracket on same line as last element should not be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn closing_bracket_correct_indent_no_parens() {
        let src = b"x = [\n  1,\n  2\n]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "bracket at line indent should not be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn percent_operator_array_not_paren_relative() {
        let src = b"gc.draw('text %d,%d' % [\n  left.round,\n  header_height\n])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array after % operator should use line-relative indent, not paren-relative: {:?}",
            diags
        );
    }

    #[test]
    fn percent_operator_array_indented_in_method() {
        let src = b"    gc.draw('rect %d,%d %d,%d' % [\n      0, 0, width, height\n    ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "indented % operator array should use line-relative indent: {:?}",
            diags
        );
    }

    #[test]
    fn array_inside_hash_arg_inside_parens_flags_paren_relative() {
        let src = b"build_type(\"test\", { \"associations\" => [\n  {\n    \"key\" => \"docs\",\n  },\n] })\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            !diags.is_empty(),
            "array inside hash arg inside parens should flag with paren-relative indent: {:?}",
            diags
        );
        assert!(
            diags[0]
                .message
                .contains("first position after the preceding left parenthesis"),
            "should use paren-relative message: {}",
            diags[0].message
        );
    }

    #[test]
    fn hash_with_to_json_chain_uses_line_indent() {
        let src = b"foo(status: 200, body: { \"responses\" => [\n  \"code\" => 200, \"body\" => \"OK\"\n] }.to_json)\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array in hash chained with .to_json should use line-relative: {:?}",
            diags
        );
    }

    #[test]
    fn tab_indented_closing_bracket_not_flagged() {
        let src = b"\tauthors [\n\t\t\"name\",\n\t]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        let bracket_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("right bracket"))
            .collect();
        assert!(
            bracket_diags.is_empty(),
            "tab-indented closing bracket at same level should not be flagged: {:?}",
            bracket_diags
        );
    }

    #[test]
    fn tab_indented_nested_closing_bracket_not_flagged() {
        let src = b"\t\tmatches [\n\t\t\t{ :text => \"test\" },\n\t\t]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        let bracket_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("right bracket"))
            .collect();
        assert!(
            bracket_diags.is_empty(),
            "nested tab-indented closing bracket should not be flagged: {:?}",
            bracket_diags
        );
    }

    #[test]
    fn array_inside_chained_hash_in_method_call() {
        let src = b"  client.\n    with(endpoint, { requests: [\n      { method: 'POST' }\n    ], flag: true }.to_json).\n    and_return(response)\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array inside chained hash should use line-relative indent: {:?}",
            diags
        );
    }

    #[test]
    fn closing_bracket_wrong_indent_no_parens() {
        let src = b"x = [\n  1,\n  2\n  ]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert_eq!(
            diags.len(),
            1,
            "bracket at wrong indent should be flagged: {:?}",
            diags
        );
        assert!(
            diags[0].message.contains("right bracket"),
            "should be a bracket message: {}",
            diags[0].message
        );
    }
}
