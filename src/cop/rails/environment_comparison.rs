use crate::cop::node_type::CALL_NODE;
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnvironmentComparison;

/// Check if a node is `Rails.env` (CallNode `env` on ConstantReadNode/ConstantPathNode `Rails`).
fn is_rails_env(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if call.name().as_slice() != b"env" {
        return false;
    }
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
    util::constant_name(&recv) == Some(b"Rails")
}

/// Check if a node is a string or symbol literal.
fn is_string_or_symbol_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some() || node.as_symbol_node().is_some()
}

fn env_name_from_literal(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(sym) = node.as_symbol_node() {
        let value_loc = sym.value_loc()?;
        let value = std::str::from_utf8(value_loc.as_slice()).ok()?;
        return Some(value.to_string());
    }

    if let Some(str_node) = node.as_string_node() {
        let value = std::str::from_utf8(str_node.unescaped()).ok()?;
        return Some(value.to_string());
    }

    None
}

impl Cop for EnvironmentComparison {
    fn name(&self) -> &'static str {
        "Rails/EnvironmentComparison"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        let method = call.name().as_slice();
        if method != b"==" && method != b"!=" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Check if either side is Rails.env and the other side is a string/symbol literal.
        // RuboCop only flags comparisons where one side is Rails.env and the other
        // is a string or symbol literal (e.g., `Rails.env == "production"`), not
        // comparisons like `variable == Rails.env` where the other side is arbitrary.
        let recv_node: ruby_prism::Node<'_> = recv;
        let arg_node = &arg_list[0];

        let is_comparison = (is_rails_env(&recv_node) && is_string_or_symbol_literal(arg_node))
            || (is_rails_env(arg_node) && is_string_or_symbol_literal(&recv_node));

        if !is_comparison {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.env.production?` instead of comparing `Rails.env`.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let (rails_env_node, literal_node) = if is_rails_env(&recv_node) {
                (&recv_node, arg_node)
            } else {
                (arg_node, &recv_node)
            };

            if let Some(env_name) = env_name_from_literal(literal_node) {
                let bang = if method == b"!=" { "!" } else { "" };
                let rails_src = source.byte_slice(
                    rails_env_node.location().start_offset(),
                    rails_env_node.location().end_offset(),
                    "Rails.env",
                );
                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: format!("{bang}{rails_src}.{env_name}?"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvironmentComparison, "cops/rails/environment_comparison");
    crate::cop_autocorrect_fixture_tests!(
        EnvironmentComparison,
        "cops/rails/environment_comparison"
    );
}
