use std::sync::LazyLock;

use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/WhereNot - detects manually constructed negated SQL in `where` calls.
///
/// ## Corpus investigation findings
///
/// FN root cause (35 FN): RuboCop's `where_method_call?` pattern matches two forms:
///
/// - `(call _ :where $str_type? $_ ?)` -- bare string arg: `where('name != ?', val)`
/// - `(call _ :where (array $str_type? $_ ?))` -- array-wrapped: `where(['name != ?', val])`
///
/// Nitrocop originally only handled the bare string form; the array-wrapped form was missed.
///
/// FP root cause (27 FP): RuboCop's `offense_range` starts at `node.loc.selector`
/// (the `where` method name), not the full node including receiver. Nitrocop used
/// `node.location()` which includes the receiver (e.g., `User.where(...)` vs `where(...)`).
/// On multiline chains this causes line-number mismatches: nitrocop reports on the receiver
/// line while RuboCop reports on the `where` line, creating paired FP+FN on adjacent lines.
///
/// Fix applied: Added array-unwrapping for first arg, and changed offense location
/// to start at `call.message_loc()` (the `where` keyword) instead of `node.location()`.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1: `builder.where("id NOT IN (:selected_tag_ids)")` — named parameter
/// form without a hash second argument. RuboCop's `extract_column_and_value`
/// requires a hash argument for named patterns (`:name`) and a positional
/// argument for anonymous patterns (`?`). Fixed by classifying negation
/// patterns into Anonymous/Named/IsNotNull and validating that the required
/// value argument exists.
///
/// ## Corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=2, FN=0.
///
/// FP=1: `where(["state not in (?) ", ...])` — trailing space in SQL template.
/// RuboCop uses `\A...\z` anchored regex without trimming. nitrocop must not
/// trim before matching.
///
/// FP=2: `where("repositories.private <> ?", true, user.repository_ids)` — 3
/// call arguments. RuboCop's pattern `(call _ :where $str_type? $_ ?)` matches
/// at most 2 args. Added early return when bare string form has >2 args.
pub struct WhereNot;

static NOT_EQ_ANON_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^([\w.]+)\s+(?:!=|<>)\s+\?$").unwrap());
static NOT_IN_ANON_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^([\w.]+)\s+NOT\s+IN\s+\(\?\)$").unwrap());
static NOT_EQ_NAMED_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^([\w.]+)\s+(?:!=|<>)\s+:(\w+)$").unwrap());
static NOT_IN_NAMED_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^([\w.]+)\s+NOT\s+IN\s+\(:(\w+)\)$").unwrap());
static IS_NOT_NULL_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^([\w.]+)\s+IS\s+NOT\s+NULL$").unwrap());

enum NegSqlPattern {
    NotEqAnon { column: String },
    NotInAnon { column: String },
    NotEqNamed { column: String, key: String },
    NotInNamed { column: String, key: String },
    IsNotNull { column: String },
}

impl Cop for WhereNot {
    fn name(&self) -> &'static str {
        "Rails/WhereNot"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"where" {
            return;
        }

        let Some((template, value_nodes)) = extract_template_and_values(&call) else {
            return;
        };

        let Some(pattern) = parse_negation_pattern(&template) else {
            return;
        };

        let column = match &pattern {
            NegSqlPattern::NotEqAnon { column }
            | NegSqlPattern::NotInAnon { column }
            | NegSqlPattern::NotEqNamed { column, .. }
            | NegSqlPattern::NotInNamed { column, .. }
            | NegSqlPattern::IsNotNull { column } => column,
        };

        // Reject db-qualified columns (`db.table.column`) to match RuboCop behavior.
        if column.chars().filter(|&c| c == '.').count() > 1 {
            return;
        }

        let Some(value_source) = extract_value_source(source, &pattern, &value_nodes) else {
            return;
        };

        let good_method = build_good_method(source, &call, column, &value_source);

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column_no) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column_no,
            "Use `where.not(...)` instead of manually constructing negated SQL.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: call.location().end_offset(),
                replacement: good_method,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

fn extract_template_and_values<'a>(
    call: &ruby_prism::CallNode<'a>,
) -> Option<(String, Vec<ruby_prism::Node<'a>>)> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    let first_arg = arg_list.first()?;

    if let Some(str_node) = first_arg.as_string_node() {
        // RuboCop matcher `(call _ :where $str_type? $_ ?)` supports at most one value arg.
        if arg_list.len() > 2 {
            return None;
        }

        return Some((
            std::str::from_utf8(str_node.unescaped())
                .unwrap_or("")
                .to_owned(),
            arg_list.into_iter().skip(1).collect(),
        ));
    }

    if let Some(array_node) = first_arg.as_array_node() {
        let elements: Vec<_> = array_node.elements().iter().collect();
        let first_elem = elements.first()?;
        let str_node = first_elem.as_string_node()?;

        // RuboCop matcher `(array $str_type? $_ ?)` supports at most one value element.
        if elements.len() > 2 {
            return None;
        }

        return Some((
            std::str::from_utf8(str_node.unescaped())
                .unwrap_or("")
                .to_owned(),
            elements.into_iter().skip(1).collect(),
        ));
    }

    None
}

fn parse_negation_pattern(sql: &str) -> Option<NegSqlPattern> {
    if let Some(caps) = NOT_EQ_ANON_RE.captures(sql) {
        return Some(NegSqlPattern::NotEqAnon {
            column: caps.get(1)?.as_str().to_string(),
        });
    }

    if let Some(caps) = NOT_IN_ANON_RE.captures(sql) {
        return Some(NegSqlPattern::NotInAnon {
            column: caps.get(1)?.as_str().to_string(),
        });
    }

    if let Some(caps) = NOT_EQ_NAMED_RE.captures(sql) {
        return Some(NegSqlPattern::NotEqNamed {
            column: caps.get(1)?.as_str().to_string(),
            key: caps.get(2)?.as_str().to_string(),
        });
    }

    if let Some(caps) = NOT_IN_NAMED_RE.captures(sql) {
        return Some(NegSqlPattern::NotInNamed {
            column: caps.get(1)?.as_str().to_string(),
            key: caps.get(2)?.as_str().to_string(),
        });
    }

    if let Some(caps) = IS_NOT_NULL_RE.captures(sql) {
        return Some(NegSqlPattern::IsNotNull {
            column: caps.get(1)?.as_str().to_string(),
        });
    }

    None
}

fn extract_value_source(
    source: &SourceFile,
    pattern: &NegSqlPattern,
    value_nodes: &[ruby_prism::Node<'_>],
) -> Option<String> {
    match pattern {
        NegSqlPattern::IsNotNull { .. } => Some("nil".to_string()),
        NegSqlPattern::NotEqAnon { .. } | NegSqlPattern::NotInAnon { .. } => {
            let value = value_nodes.first()?;
            let loc = value.location();
            Some(
                source
                    .byte_slice(loc.start_offset(), loc.end_offset(), "")
                    .to_string(),
            )
        }
        NegSqlPattern::NotEqNamed { key, .. } | NegSqlPattern::NotInNamed { key, .. } => {
            let hash_like = value_nodes.first()?;
            let elements = if let Some(hash) = hash_like.as_hash_node() {
                hash.elements()
            } else if let Some(kw_hash) = hash_like.as_keyword_hash_node() {
                kw_hash.elements()
            } else {
                return None;
            };

            for elem in elements.iter() {
                let assoc = elem.as_assoc_node()?;
                let assoc_key = assoc.key();
                let key_matches = assoc_key
                    .as_symbol_node()
                    .is_some_and(|sym| sym.unescaped() == key.as_bytes())
                    || assoc_key
                        .as_string_node()
                        .is_some_and(|s| s.unescaped() == key.as_bytes());
                if !key_matches {
                    continue;
                }

                let vloc = assoc.value().location();
                return Some(
                    source
                        .byte_slice(vloc.start_offset(), vloc.end_offset(), "")
                        .to_string(),
                );
            }

            None
        }
    }
}

fn build_good_method(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    column: &str,
    value: &str,
) -> String {
    let dot = call
        .call_operator_loc()
        .map(|op| source.byte_slice(op.start_offset(), op.end_offset(), "."))
        .unwrap_or(".");

    if let Some((table, col)) = column.split_once('.') {
        format!("where{dot}not({table}: {{ {col}: {value} }})")
    } else {
        format!("where{dot}not({column}: {value})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereNot, "cops/rails/where_not");
    crate::cop_autocorrect_fixture_tests!(WhereNot, "cops/rails/where_not");

    #[test]
    fn supports_autocorrect() {
        assert!(WhereNot.supports_autocorrect());
    }
}
