use crate::cop::node_type::{DEF_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SingleLineMethods;

impl Cop for SingleLineMethods {
    fn name(&self) -> &'static str {
        "Style/SingleLineMethods"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_empty = config.get_bool("AllowIfMethodIsEmpty", true);
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Check if the method has a body
        let has_body = match def_node.body() {
            None => false,
            Some(body) => {
                if let Some(stmts) = body.as_statements_node() {
                    !stmts.body().is_empty()
                } else {
                    true
                }
            }
        };

        // AllowIfMethodIsEmpty: skip empty methods when enabled (default true)
        if !has_body && allow_empty {
            return;
        }

        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_kw_loc.start_offset());

        if def_line == end_line {
            let (line, column) = source.offset_to_line_col(def_loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Avoid single-line method definitions.".to_string(),
            );

            if let Some(replacement) =
                multiline_replacement(source, &def_node, end_kw_loc.start_offset())
            {
                if let Some(corrections) = corrections {
                    corrections.push(Correction {
                        start: def_node.location().start_offset(),
                        end: def_node.location().end_offset(),
                        replacement,
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

fn multiline_replacement(
    source: &SourceFile,
    def_node: &ruby_prism::DefNode<'_>,
    end_start: usize,
) -> Option<String> {
    let def_start = def_node.location().start_offset();
    let indent = leading_indent(source, def_start);

    let header_end = def_node
        .body()
        .map(|body| body.location().start_offset())
        .unwrap_or(end_start);

    let mut header = String::from_utf8_lossy(&source.as_bytes()[def_start..header_end]).to_string();
    header = header
        .trim_end()
        .trim_end_matches(';')
        .trim_end()
        .to_string();

    let mut replacement = String::new();
    replacement.push_str(&header);

    if let Some(body) = def_node.body() {
        let body_src = String::from_utf8_lossy(body.location().as_slice())
            .trim()
            .to_string();
        if !body_src.is_empty() {
            replacement.push('\n');
            replacement.push_str(&indent);
            replacement.push_str("  ");
            replacement.push_str(&body_src);
        }
    }

    replacement.push('\n');
    replacement.push_str(&indent);
    replacement.push_str("end");

    Some(replacement)
}

fn leading_indent(source: &SourceFile, offset: usize) -> String {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }

    let mut i = line_start;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }

    String::from_utf8_lossy(&bytes[line_start..i]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(SingleLineMethods, "cops/style/single_line_methods");
    crate::cop_autocorrect_fixture_tests!(SingleLineMethods, "cops/style/single_line_methods");

    #[test]
    fn empty_single_line_method_is_ok() {
        let source = b"def foo; end\n";
        let diags = run_cop_full(&SingleLineMethods, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn endless_method_is_ok() {
        let source = b"def foo = 42\n";
        let diags = run_cop_full(&SingleLineMethods, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn disallow_empty_single_line_methods() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowIfMethodIsEmpty".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        // Empty single-line `def foo; end` should be flagged when AllowIfMethodIsEmpty is false
        let source = b"def foo; end\n";
        let diags = run_cop_full_with_config(&SingleLineMethods, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag empty single-line method when AllowIfMethodIsEmpty is false"
        );
    }
}
