use crate::cop::node_type::FLOAT_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FloatOutOfRange;

impl Cop for FloatOutOfRange {
    fn name(&self) -> &'static str {
        "Lint/FloatOutOfRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[FLOAT_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let float_node = match node.as_float_node() {
            Some(n) => n,
            None => return,
        };

        let loc = float_node.location();
        let src = loc.as_slice();

        // Remove underscores and parse as f64
        let cleaned: Vec<u8> = src.iter().copied().filter(|&b| b != b'_').collect();
        let text = match std::str::from_utf8(&cleaned) {
            Ok(t) => t,
            Err(_) => return,
        };

        match text.parse::<f64>() {
            Ok(val) if val.is_infinite() => {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic =
                    self.diagnostic(source, line, column, "Float out of range.".to_string());
                if let Some(corrections) = corrections {
                    let replacement = if val.is_sign_negative() {
                        "(-Float::INFINITY)"
                    } else {
                        "Float::INFINITY"
                    };
                    corrections.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: replacement.to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
                diagnostics.push(diagnostic);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FloatOutOfRange, "cops/lint/float_out_of_range");

    #[test]
    fn supports_autocorrect() {
        assert!(FloatOutOfRange.supports_autocorrect());
    }

    #[test]
    fn autocorrect_positive_overflow() {
        crate::testutil::assert_cop_autocorrect(
            &FloatOutOfRange,
            b"x = 1.0e309\n",
            b"x = Float::INFINITY\n",
        );
    }
}
