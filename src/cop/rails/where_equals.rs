use std::sync::LazyLock;

use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereEquals;

static EQ_ANON_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^([\w.]+)\s+=\s+\?$").unwrap());
static IN_ANON_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^([\w.]+)\s+IN\s+\(\?\)$").unwrap());
static IS_NULL_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^([\w.]+)\s+IS\s+NULL$").unwrap());
static EQ_NAMED_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^([\w.]+)\s+=\s+:(\w+)$").unwrap());
static IN_NAMED_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^([\w.]+)\s+IN\s+\(:(\w+)\)$").unwrap());

enum SqlPattern {
    EqAnon { column: String },
    InAnon { column: String },
    IsNull { column: String },
    EqNamed { column: String, key: String },
    InNamed { column: String, key: String },
}

impl Cop for WhereEquals {
    fn name(&self) -> &'static str {
        "Rails/WhereEquals"
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

        let name = call.name().as_slice();
        if name != b"where" && name != b"not" {
            return;
        }

        // If `not`, check that receiver is a `where` call
        if name == b"not" {
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    if recv_call.name().as_slice() != b"where" {
                        return;
                    }
                } else {
                    return;
                }
            } else {
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

        let column = match &pattern {
            SqlPattern::EqAnon { column }
            | SqlPattern::InAnon { column }
            | SqlPattern::IsNull { column }
            | SqlPattern::EqNamed { column, .. }
            | SqlPattern::InNamed { column, .. } => column,
        };

        // Reject database-qualified columns: allow `col` or `table.col`, reject `db.table.col`.
        if column.chars().filter(|&c| c == '.').count() > 1 {
            return;
        }

        let Some(value_source) = extract_value_source(source, &pattern, &value_nodes) else {
            return;
        };

        let method = std::str::from_utf8(name).unwrap_or("where");
        let good_method = build_good_method(method, column, &value_source);

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column_no) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column_no,
            format!("Use `{good_method}` instead of manually constructing SQL."),
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

fn parse_sql_pattern(template: &str) -> Option<SqlPattern> {
    if let Some(caps) = EQ_ANON_RE.captures(template) {
        return Some(SqlPattern::EqAnon {
            column: caps.get(1)?.as_str().to_string(),
        });
    }
    if let Some(caps) = IN_ANON_RE.captures(template) {
        return Some(SqlPattern::InAnon {
            column: caps.get(1)?.as_str().to_string(),
        });
    }
    if let Some(caps) = IS_NULL_RE.captures(template) {
        return Some(SqlPattern::IsNull {
            column: caps.get(1)?.as_str().to_string(),
        });
    }
    if let Some(caps) = EQ_NAMED_RE.captures(template) {
        return Some(SqlPattern::EqNamed {
            column: caps.get(1)?.as_str().to_string(),
            key: caps.get(2)?.as_str().to_string(),
        });
    }
    if let Some(caps) = IN_NAMED_RE.captures(template) {
        return Some(SqlPattern::InNamed {
            column: caps.get(1)?.as_str().to_string(),
            key: caps.get(2)?.as_str().to_string(),
        });
    }
    None
}

fn extract_value_source(
    source: &SourceFile,
    pattern: &SqlPattern,
    value_nodes: &[ruby_prism::Node<'_>],
) -> Option<String> {
    match pattern {
        SqlPattern::IsNull { .. } => Some("nil".to_string()),
        SqlPattern::EqAnon { .. } | SqlPattern::InAnon { .. } => {
            let value = value_nodes.first()?;
            let loc = value.location();
            Some(
                source
                    .byte_slice(loc.start_offset(), loc.end_offset(), "")
                    .to_string(),
            )
        }
        SqlPattern::EqNamed { key, .. } | SqlPattern::InNamed { key, .. } => {
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
    crate::cop_fixture_tests!(WhereEquals, "cops/rails/where_equals");
    crate::cop_autocorrect_fixture_tests!(WhereEquals, "cops/rails/where_equals");

    #[test]
    fn supports_autocorrect() {
        assert!(WhereEquals.supports_autocorrect());
    }

    #[test]
    fn test_array_argument_form() {
        let cop = WhereEquals;
        let source = b"User.where(['name = ?', 'Gabe'])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array argument form");
    }

    #[test]
    fn test_array_is_null_form() {
        let cop = WhereEquals;
        let source = b"User.where(['name IS NULL'])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array IS NULL form");
    }

    #[test]
    fn test_array_named_placeholder() {
        let cop = WhereEquals;
        let source = b"User.where(['name = :name', { name: 'Gabe' }])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array named placeholder form");
    }

    #[test]
    fn test_array_in_form() {
        let cop = WhereEquals;
        let source = b"User.where([\"name IN (?)\", ['john', 'jane']])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array IN form");
    }

    #[test]
    fn test_array_namespaced_column() {
        let cop = WhereEquals;
        let source = b"Course.where(['enrollments.student_id = ?', student.id])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array namespaced column form");
    }

    #[test]
    fn test_where_not_regular_form() {
        let cop = WhereEquals;
        let source = b"User.where.not('name = ?', 'Gabe')\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect where.not form");
    }

    #[test]
    fn test_scope_where() {
        let cop = WhereEquals;
        let source = b"scope :active, -> { where('active = ?', true) }\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect where inside scope lambda");
    }

    #[test]
    fn test_chained_where() {
        let cop = WhereEquals;
        let source = b"User.active.where('name = ?', 'Gabe')\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect chained where");
    }
}
