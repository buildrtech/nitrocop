use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus FP fix: regular method calls like `Statuses.describe(:foo)` have an
/// explicit receiver and are not RSpec describe blocks. Fixed by requiring the
/// call to be receiverless or have `RSpec` as the receiver, matching the
/// pattern in ExcessiveDocstringSpacing.
pub struct DescribeSymbol;

impl Cop for DescribeSymbol {
    fn name(&self) -> &'static str {
        "RSpec/DescribeSymbol"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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

        let method = call.name().as_slice();
        if method != b"describe" {
            return;
        }

        // Must be receiverless or RSpec.describe / ::RSpec.describe
        // Regular method calls like `obj.describe(:sym)` are not RSpec describe blocks.
        if let Some(recv) = call.receiver() {
            if crate::cop::util::constant_name(&recv).is_none_or(|n| n != b"RSpec") {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument is a symbol
        if arg_list[0].as_symbol_node().is_none() {
            return;
        }

        let symbol = arg_list[0]
            .as_symbol_node()
            .expect("checked symbol argument above");
        let loc = symbol.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Avoid describing symbols.".to_string(),
        );

        if let Some(corrections) = corrections.as_deref_mut() {
            let symbol_name = std::str::from_utf8(symbol.unescaped()).unwrap_or("");
            let escaped = symbol_name.replace('\\', "\\\\").replace('"', "\\\"");
            corrections.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: format!("\"#{escaped}\""),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DescribeSymbol, "cops/rspec/describe_symbol");

    #[test]
    fn supports_autocorrect() {
        assert!(DescribeSymbol.supports_autocorrect());
    }

    #[test]
    fn autocorrect_rewrites_symbol_description_to_string() {
        crate::testutil::assert_cop_autocorrect(
            &DescribeSymbol,
            b"describe(:to_s) { }\n",
            b"describe(\"#to_s\") { }\n",
        );
    }
}
