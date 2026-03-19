use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for blocks that are known to need more positional arguments than given.
/// By default checks methods like `reduce`, `inject`, `sort`, `each_with_index`,
/// `each_with_object`, `chunk_while`, `slice_when`, `max`, `min`, `minmax`
/// which all expect 2 positional block arguments.
///
/// Root cause of 93 FN (corpus 0.0% match): The `Methods` config from
/// vendor/rubocop/config/default.yml has integer values (e.g., `inject: 2`),
/// which serde_yml loads as `Value::Number`. The `get_string_hash` helper
/// was using `v.as_str()` which returns `None` for numbers, silently producing
/// an empty methods list and causing the cop to report zero offenses. Fixed by
/// extending `get_string_hash` to convert numeric YAML values to strings.
pub struct UnexpectedBlockArity;

impl Cop for UnexpectedBlockArity {
    fn name(&self) -> &'static str {
        "Lint/UnexpectedBlockArity"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Read configured methods
        let methods = get_methods(config);

        let mut visitor = ArityVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            methods,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

fn get_methods(config: &CopConfig) -> Vec<(String, usize)> {
    if let Some(hash) = config.get_string_hash("Methods") {
        return hash
            .iter()
            .filter_map(|(k, v)| {
                let arity: usize = v.parse().ok()?;
                Some((k.clone(), arity))
            })
            .collect();
    }
    // Defaults from vendor/rubocop/config/default.yml
    vec![
        ("chunk_while".to_string(), 2),
        ("each_with_index".to_string(), 2),
        ("each_with_object".to_string(), 2),
        ("inject".to_string(), 2),
        ("max".to_string(), 2),
        ("min".to_string(), 2),
        ("minmax".to_string(), 2),
        ("reduce".to_string(), 2),
        ("slice_when".to_string(), 2),
        ("sort".to_string(), 2),
    ]
}

struct ArityVisitor<'a, 'src> {
    cop: &'a UnexpectedBlockArity,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    methods: Vec<(String, usize)>,
}

impl<'pr> Visit<'pr> for ArityVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call has a block and is one of the configured methods
        if let Some(block) = node.block() {
            if node.receiver().is_some() {
                let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
                if let Some(expected) = self.expected_arity(method_name) {
                    if let Some(block_node) = block.as_block_node() {
                        let actual = count_block_args(&block_node);
                        if actual < expected {
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!(
                                    "`{}` expects at least {} positional arguments, got {}.",
                                    method_name, expected, actual
                                ),
                            ));
                        }
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

impl ArityVisitor<'_, '_> {
    fn expected_arity(&self, method_name: &str) -> Option<usize> {
        for (name, arity) in &self.methods {
            if name == method_name {
                return Some(*arity);
            }
        }
        None
    }
}

fn count_block_args(block: &ruby_prism::BlockNode<'_>) -> usize {
    let params = match block.parameters() {
        Some(p) => p,
        None => return 0,
    };

    // NumberedParametersNode (Ruby 2.7+): `values.reduce { _1 }` — maximum() gives the highest _N used
    if let Some(numbered) = params.as_numbered_parameters_node() {
        return numbered.maximum() as usize;
    }

    // ItParametersNode (Ruby 3.4+): `values.reduce { it }` — always counts as 1 arg
    if params.as_it_parameters_node().is_some() {
        return 1;
    }

    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return 0,
    };

    let parameters = match block_params.parameters() {
        Some(p) => p,
        None => return 0,
    };

    // Check for rest args (splat) - if present, the block accepts unlimited args
    if parameters.rest().is_some() {
        return usize::MAX;
    }

    // Count positional args (required + optional)
    // Destructured parameters like |(a, b)| count as 1 positional arg (they appear in requireds())
    // Keyword-only parameters (a:, b:, **kwargs) are NOT positional args
    parameters.requireds().len() + parameters.optionals().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnexpectedBlockArity, "cops/lint/unexpected_block_arity");

    /// Verify the cop works when `Methods` config has integer values (as loaded
    /// from vendor/rubocop/config/default.yml via serde_yml). Integer YAML values
    /// are `Value::Number`, not `Value::String`, so `get_string_hash` must
    /// convert them. Without this, the cop silently gets an empty methods list
    /// and reports zero offenses.
    #[test]
    fn methods_config_with_integer_values() {
        use serde_yml::Value;
        use std::collections::HashMap;

        let mut methods = serde_yml::Mapping::new();
        methods.insert(Value::String("reduce".into()), Value::Number(2.into()));
        methods.insert(
            Value::String("each_with_index".into()),
            Value::Number(2.into()),
        );
        methods.insert(Value::String("sort".into()), Value::Number(2.into()));

        let config = CopConfig {
            options: HashMap::from([("Methods".into(), Value::Mapping(methods))]),
            ..CopConfig::default()
        };

        let methods = get_methods(&config);
        assert_eq!(methods.len(), 3);
        assert!(methods.iter().any(|(n, a)| n == "reduce" && *a == 2));
        assert!(
            methods
                .iter()
                .any(|(n, a)| n == "each_with_index" && *a == 2)
        );
        assert!(methods.iter().any(|(n, a)| n == "sort" && *a == 2));
    }
}
