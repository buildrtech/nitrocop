use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereRange;

enum SqlPattern {
    GteAnon { column: String },
    LteAnon { column: String, op: String },
    RangeAnon { column: String, op: String },
    GteNamed { column: String, key: String },
    LteNamed { column: String, op: String, key: String },
    RangeNamed {
        column: String,
        low_key: String,
        op: String,
        high_key: String,
    },
}

impl Cop for WhereRange {
    fn name(&self) -> &'static str {
        "Rails/WhereRange"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !config.rails_version_at_least(6.0) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        if method != b"where" && method != b"not" {
            return;
        }

        if method == b"not" {
            let Some(recv) = call.receiver() else {
                return;
            };
            let Some(recv_call) = recv.as_call_node() else {
                return;
            };
            if recv_call.name().as_slice() != b"where" {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let mut arg_iter = args.arguments().iter();
        let Some(first_arg) = arg_iter.next() else {
            return;
        };

        let (template, value_nodes): (String, Vec<ruby_prism::Node<'_>>) =
            if let Some(str_node) = first_arg.as_string_node() {
                (
                    std::str::from_utf8(str_node.unescaped())
                        .unwrap_or("")
                        .to_owned(),
                    arg_iter.collect(),
                )
            } else if let Some(array_node) = first_arg.as_array_node() {
                let mut elems = array_node.elements().iter();
                let Some(first_elem) = elems.next() else {
                    return;
                };
                let Some(str_node) = first_elem.as_string_node() else {
                    return;
                };
                (
                    std::str::from_utf8(str_node.unescaped())
                        .unwrap_or("")
                        .to_owned(),
                    elems.collect(),
                )
            } else {
                return;
            };

        let Some(pattern) = parse_sql_pattern(&template) else {
            return;
        };

        let column_name = match &pattern {
            SqlPattern::GteAnon { column }
            | SqlPattern::LteAnon { column, .. }
            | SqlPattern::RangeAnon { column, .. }
            | SqlPattern::GteNamed { column, .. }
            | SqlPattern::LteNamed { column, .. }
            | SqlPattern::RangeNamed { column, .. } => column,
        };

        if !is_column_identifier(column_name) {
            return;
        }

        let Some(range_value) = build_range_value(source, &pattern, &value_nodes) else {
            return;
        };

        let method_name = std::str::from_utf8(method).unwrap_or("where");
        let good_method = build_good_method(method_name, column_name, &range_value);

        let offense_loc = call.location();
        let (line, column) = source.offset_to_line_col(offense_loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use a range in `where` instead of manually constructing SQL conditions.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let replace_start = call
                .message_loc()
                .map(|l| l.start_offset())
                .unwrap_or(offense_loc.start_offset());
            corr.push(crate::correction::Correction {
                start: replace_start,
                end: offense_loc.end_offset(),
                replacement: good_method,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

fn parse_sql_pattern(template: &str) -> Option<SqlPattern> {
    let s = template.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();

    if parts.len() == 3 {
        let col = parts[0];
        let op = parts[1];
        let val = parts[2];

        if !is_column_identifier(col) {
            return None;
        }

        if val == "?" {
            return match op {
                ">=" => Some(SqlPattern::GteAnon {
                    column: col.to_string(),
                }),
                "<" | "<=" => Some(SqlPattern::LteAnon {
                    column: col.to_string(),
                    op: op.to_string(),
                }),
                _ => None,
            };
        }

        if let Some(key) = val.strip_prefix(':') {
            if !is_placeholder_key(key) {
                return None;
            }

            return match op {
                ">=" => Some(SqlPattern::GteNamed {
                    column: col.to_string(),
                    key: key.to_string(),
                }),
                "<" | "<=" => Some(SqlPattern::LteNamed {
                    column: col.to_string(),
                    op: op.to_string(),
                    key: key.to_string(),
                }),
                _ => None,
            };
        }

        return None;
    }

    if parts.len() == 7 {
        let col1 = parts[0];
        let op1 = parts[1];
        let v1 = parts[2];
        let and_kw = parts[3];
        let col2 = parts[4];
        let op2 = parts[5];
        let v2 = parts[6];

        if !is_column_identifier(col1)
            || !is_column_identifier(col2)
            || col1 != col2
            || op1 != ">="
            || !and_kw.eq_ignore_ascii_case("AND")
            || !(op2 == "<" || op2 == "<=")
        {
            return None;
        }

        if v1 == "?" && v2 == "?" {
            return Some(SqlPattern::RangeAnon {
                column: col1.to_string(),
                op: op2.to_string(),
            });
        }

        let low_key = v1.strip_prefix(':')?;
        let high_key = v2.strip_prefix(':')?;
        if !is_placeholder_key(low_key) || !is_placeholder_key(high_key) {
            return None;
        }

        return Some(SqlPattern::RangeNamed {
            column: col1.to_string(),
            low_key: low_key.to_string(),
            op: op2.to_string(),
            high_key: high_key.to_string(),
        });
    }

    None
}

fn is_placeholder_key(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_column_identifier(s: &str) -> bool {
    if s.is_empty() || s.contains('(') || s.contains(')') {
        return false;
    }

    let dot_count = s.chars().filter(|&c| c == '.').count();
    if dot_count > 1 {
        return false;
    }

    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn range_operator(op: &str) -> &'static str {
    if op == "<" {
        "..."
    } else {
        ".."
    }
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

fn build_range_value(
    source: &SourceFile,
    pattern: &SqlPattern,
    value_nodes: &[ruby_prism::Node<'_>],
) -> Option<String> {
    match pattern {
        SqlPattern::GteAnon { .. } => {
            let lhs = value_nodes.first()?;
            Some(format!("{}..", node_source(source, lhs)))
        }
        SqlPattern::LteAnon { op, .. } => {
            let rhs = value_nodes.first()?;
            Some(format!("{}{}", range_operator(op), node_source(source, rhs)))
        }
        SqlPattern::RangeAnon { op, .. } => {
            if value_nodes.len() < 2 {
                return None;
            }
            Some(format!(
                "{}{}{}",
                node_source(source, &value_nodes[0]),
                range_operator(op),
                node_source(source, &value_nodes[1])
            ))
        }
        SqlPattern::GteNamed { key, .. } => {
            let hash_like = value_nodes.first()?;
            let lhs = find_named_value(source, hash_like, key)?;
            Some(format!("{lhs}.."))
        }
        SqlPattern::LteNamed { op, key, .. } => {
            let hash_like = value_nodes.first()?;
            let rhs = find_named_value(source, hash_like, key)?;
            Some(format!("{}{rhs}", range_operator(op)))
        }
        SqlPattern::RangeNamed {
            low_key,
            op,
            high_key,
            ..
        } => {
            let hash_like = value_nodes.first()?;
            let low = find_named_value(source, hash_like, low_key)?;
            let high = find_named_value(source, hash_like, high_key)?;
            Some(format!("{low}{}{high}", range_operator(op)))
        }
    }
}

fn find_named_value(source: &SourceFile, node: &ruby_prism::Node<'_>, key: &str) -> Option<String> {
    let elements = if let Some(hash) = node.as_hash_node() {
        hash.elements()
    } else if let Some(kw_hash) = node.as_keyword_hash_node() {
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
        return Some(node_source(source, &assoc.value()));
    }

    None
}

fn build_good_method(method_name: &str, column: &str, value: &str) -> String {
    if let Some((table, col)) = column.split_once('.') {
        format!("{method_name}({table}: {{ {col}: {value} }})")
    } else {
        format!("{method_name}({column}: {value})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(WhereRange, "cops/rails/where_range", 6.0);

    #[test]
    fn autocorrect_fixture() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &WhereRange,
            include_bytes!("../../../tests/fixtures/cops/rails/where_range/offense.rb"),
            include_bytes!("../../../tests/fixtures/cops/rails/where_range/corrected.rb"),
            rails_config(),
        );
    }

    #[test]
    fn supports_autocorrect() {
        assert!(WhereRange.supports_autocorrect());
    }

    #[test]
    fn does_not_flag_complex_sql() {
        let config = rails_config();
        let diags = crate::testutil::run_cop_full_with_config(
            &WhereRange,
            b"User.where('COALESCE(status_stats.reblogs_count, 0) < ?', min_reblogs)\n",
            config,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn does_not_flag_non_comparison() {
        let config = rails_config();
        let diags = crate::testutil::run_cop_full_with_config(
            &WhereRange,
            b"User.where('name = ?', name)\n",
            config,
        );
        assert!(diags.is_empty());
    }
}
