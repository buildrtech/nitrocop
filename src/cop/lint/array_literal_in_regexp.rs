use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for array literals interpolated inside regexps.
/// When interpolating an array literal, it is converted to a string,
/// which is likely not the intended behavior inside a regexp.
pub struct ArrayLiteralInRegexp;

impl Cop for ArrayLiteralInRegexp {
    fn name(&self) -> &'static str {
        "Lint/ArrayLiteralInRegexp"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RegexpArrayVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

struct RegexpArrayVisitor<'a, 'src> {
    cop: &'a ArrayLiteralInRegexp,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
}

impl RegexpArrayVisitor<'_, '_> {
    fn literal_value(&self, node: &ruby_prism::Node<'_>) -> Option<String> {
        if let Some(s) = node.as_string_node() {
            return Some(String::from_utf8_lossy(s.unescaped()).to_string());
        }
        if let Some(sym) = node.as_symbol_node() {
            return Some(String::from_utf8_lossy(sym.unescaped()).to_string());
        }
        if node.as_true_node().is_some() {
            return Some("true".to_string());
        }
        if node.as_false_node().is_some() {
            return Some("false".to_string());
        }
        if node.as_nil_node().is_some() {
            return Some(String::new());
        }

        let loc = node.location();
        if node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_local_variable_read_node().is_some()
            || node.as_instance_variable_read_node().is_some()
            || node.as_class_variable_read_node().is_some()
            || node.as_global_variable_read_node().is_some()
            || node.as_constant_read_node().is_some()
            || node.as_constant_path_node().is_some()
        {
            return self
                .source
                .try_byte_slice(loc.start_offset(), loc.end_offset())
                .map(str::to_string);
        }

        if let Some(call) = node.as_call_node()
            && call.receiver().is_none()
            && call.arguments().is_none()
            && call.block().is_none()
        {
            return self
                .source
                .try_byte_slice(loc.start_offset(), loc.end_offset())
                .map(str::to_string);
        }

        None
    }

    fn escape_regex_fragment(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            if matches!(
                ch,
                '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\'
            ) {
                out.push('\\');
            }
            out.push(ch);
        }
        out
    }

    fn replacement_for_array(&self, array: &ruby_prism::ArrayNode<'_>) -> Option<String> {
        let values: Vec<String> = array
            .elements()
            .iter()
            .map(|elem| self.literal_value(&elem))
            .collect::<Option<Vec<_>>>()?;

        if values.iter().all(|v| v.chars().count() == 1) {
            let body = values
                .iter()
                .map(|v| Self::escape_regex_fragment(v))
                .collect::<Vec<_>>()
                .join("");
            return Some(format!("[{body}]"));
        }

        let body = values
            .iter()
            .map(|v| Self::escape_regex_fragment(v))
            .collect::<Vec<_>>()
            .join("|");
        Some(format!("(?:{body})"))
    }
}

impl<'pr> Visit<'pr> for RegexpArrayVisitor<'_, '_> {
    fn visit_interpolated_regular_expression_node(
        &mut self,
        node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>,
    ) {
        // Check if any parts contain array literals
        for part in node.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if let Some(last) = body.last() {
                        if let Some(array) = last.as_array_node() {
                            // Report at the regex location, not the embedded node
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            let mut diag = self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Use alternation or a character class instead of interpolating an array in a regexp."
                                    .to_string(),
                            );

                            if let Some(replacement) = self.replacement_for_array(&array) {
                                let embed_loc = embedded.location();
                                self.corrections.push(crate::correction::Correction {
                                    start: embed_loc.start_offset(),
                                    end: embed_loc.end_offset(),
                                    replacement,
                                    cop_name: self.cop.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }

                            self.diagnostics.push(diag);
                        }
                    }
                }
            }
        }

        ruby_prism::visit_interpolated_regular_expression_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayLiteralInRegexp, "cops/lint/array_literal_in_regexp");
    crate::cop_autocorrect_fixture_tests!(
        ArrayLiteralInRegexp,
        "cops/lint/array_literal_in_regexp"
    );

    #[test]
    fn autocorrects_identifier_array_in_regexp_to_alternation() {
        crate::testutil::assert_cop_autocorrect(
            &ArrayLiteralInRegexp,
            b"/#{[foo, bar]}/\n",
            b"/(?:foo|bar)/\n",
        );
    }
}
