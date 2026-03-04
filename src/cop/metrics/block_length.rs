use crate::cop::node_type::{
    BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, LAMBDA_NODE,
};
use crate::cop::util::{collect_foldable_ranges, collect_heredoc_ranges, count_body_lines_ex};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=105, FN=5.
///
/// A high-volume FP pattern was blocks whose body is only a heredoc expression:
/// `render do; <<~RUBY ... RUBY; end`.
///
/// In RuboCop (Parser AST), that body is a `str`/`dstr` node whose source range
/// is just the heredoc opening line (`<<~RUBY`), so it counts as one body line.
/// Our Prism implementation counted the full physical heredoc content range,
/// producing false positives on large documentation/example blocks.
///
/// Fix: detect "single heredoc expression body" and count it as one line.
pub struct BlockLength;

impl Cop for BlockLength {
    fn name(&self) -> &'static str {
        "Metrics/BlockLength"
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            LAMBDA_NODE,
        ]
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
        // Handle lambda nodes: ->(x) do...end / ->(x) {...}
        if let Some(lambda_node) = node.as_lambda_node() {
            self.check_lambda(source, &lambda_node, config, diagnostics);
            return;
        }

        // We check CallNode (not BlockNode) so we can read the method name
        // for AllowedMethods/AllowedPatterns filtering.
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block_node = match call_node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // RuboCop skips class constructor blocks (Struct.new, Class.new, etc.)
        if is_class_constructor(&call_node) {
            return;
        }

        let max = config.get_usize("Max", 25);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        // AllowedMethods / AllowedPatterns: skip blocks on matching method calls
        let method_name = std::str::from_utf8(call_node.name().as_slice()).unwrap_or("");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            for pat in patterns {
                if let Ok(re) = regex::Regex::new(pat) {
                    if re.is_match(method_name) {
                        return;
                    }
                }
            }
        }

        let end_offset = block_node.closing_loc().start_offset();
        let count = count_block_lines(
            source,
            block_node.opening_loc().start_offset(),
            end_offset,
            block_node.body(),
            count_comments,
            &count_as_one,
        );

        if count > max {
            // Use call_node location (not block opening) to match RuboCop's
            // offense position which spans the full expression in Parser AST.
            let (line, column) = source.offset_to_line_col(call_node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Block has too many lines. [{count}/{max}]"),
            ));
        }
    }
}

impl BlockLength {
    fn check_lambda(
        &self,
        source: &SourceFile,
        lambda_node: &ruby_prism::LambdaNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let max = config.get_usize("Max", 25);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        let end_offset = lambda_node.closing_loc().start_offset();
        let count = count_block_lines(
            source,
            lambda_node.opening_loc().start_offset(),
            end_offset,
            lambda_node.body(),
            count_comments,
            &count_as_one,
        );

        if count > max {
            let (line, column) = source.offset_to_line_col(lambda_node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Block has too many lines. [{count}/{max}]"),
            ));
        }
    }
}

/// Count body lines for a block, folding heredocs and CountAsOne constructs.
/// Uses the body node's start offset (not opening_loc) to avoid counting
/// heredoc content lines that physically appear before the body starts.
fn count_block_lines(
    source: &SourceFile,
    opening_offset: usize,
    end_offset: usize,
    body: Option<ruby_prism::Node<'_>>,
    count_comments: bool,
    count_as_one: &Option<Vec<String>>,
) -> usize {
    let body = match body {
        Some(b) => b,
        None => return 0,
    };

    // Parser/RuboCop behavior: when a block body is a single heredoc expression,
    // code length is based on the heredoc opener node source, not heredoc content.
    // This makes the body count as one line.
    if is_single_heredoc_expression(source, &body) {
        return 1;
    }

    // Use body start offset to skip heredoc content that appears before body.
    // Same approach as method_length.rs.
    let (body_start_line, _) = source.offset_to_line_col(body.location().start_offset());
    let effective_start_offset = if body_start_line > 1 {
        source
            .line_col_to_offset(body_start_line - 1, 0)
            .unwrap_or(opening_offset)
    } else {
        opening_offset
    };

    // Collect foldable ranges from CountAsOne config. Heredocs are only
    // folded when "heredoc" is explicitly in CountAsOne (default: []).
    // For non-bare-heredoc bodies, RuboCop's CodeLengthCalculator includes
    // heredoc content lines by default. We replicate that here.
    let mut all_foldable: Vec<(usize, usize)> = Vec::new();
    if let Some(cao) = count_as_one {
        if !cao.is_empty() {
            all_foldable.extend(collect_foldable_ranges(source, &body, cao));
            if cao.iter().any(|s| s == "heredoc") {
                all_foldable.extend(collect_heredoc_ranges(source, &body));
            }
        }
    }
    all_foldable.sort();
    all_foldable.dedup();

    count_body_lines_ex(
        source,
        effective_start_offset,
        end_offset,
        count_comments,
        &all_foldable,
    )
}

fn is_single_heredoc_expression(source: &SourceFile, body: &ruby_prism::Node<'_>) -> bool {
    if is_heredoc_node(source, body) {
        return true;
    }

    if let Some(stmts) = body.as_statements_node() {
        let mut iter = stmts.body().iter();
        if let Some(first) = iter.next() {
            return iter.next().is_none() && is_heredoc_node(source, &first);
        }
    }

    false
}

fn is_heredoc_node(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        return s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
    }

    if let Some(s) = node.as_interpolated_string_node() {
        return s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
    }

    false
}

/// Check if a call is a class constructor like `Struct.new`, `Class.new`, `Module.new`, etc.
/// RuboCop's Metrics/BlockLength does not count these blocks.
fn is_class_constructor(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"new" {
        return false;
    }
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    // Check for simple constant receiver (Struct, Class, Module, etc.)
    if let Some(cr) = recv.as_constant_read_node() {
        let name = cr.name().as_slice();
        return matches!(name, b"Struct" | b"Class" | b"Module");
    }
    // Check for constant path (e.g., ::Struct.new)
    if let Some(cp) = recv.as_constant_path_node() {
        if let Some(name_node) = cp.name() {
            let name = name_node.as_slice();
            return matches!(name, b"Struct" | b"Class" | b"Module");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockLength, "cops/metrics/block_length");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line block");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_array() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountAsOne".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("array".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, [\n1,\n2\n] = 2 + 1 folded = 3 lines
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  arr = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire when array is folded (3/3)"
        );
    }

    #[test]
    fn allowed_methods_refine() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "AllowedMethods".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("refine".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // refine block with 4 lines should NOT fire because refine is allowed
        let source =
            b"refine String do\n  def a; end\n  def b; end\n  def c; end\n  def d; end\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire on allowed method 'refine'"
        );
    }
}
