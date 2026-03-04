use ruby_prism::Visit;

use crate::cop::node_type::{CLASS_NODE, MODULE_NODE, STATEMENTS_NODE};
use crate::cop::util::{
    collect_foldable_ranges, count_body_lines_ex, count_body_lines_full, inner_classlike_ranges,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Artifact data for this cop remains FN-heavy with low FP. The dominant FN
/// examples are top-level `class << self` blocks that RuboCop checks in
/// `on_sclass` (except when nested under a real `class` ancestor).
///
/// Previous broad rewrite attempts regressed badly, so this implementation is
/// intentionally incremental and validated against per-repo reruns:
/// - `origin/main` implementation rerun: actual 14,382 vs expected 14,177.
/// - This change rerun: actual 14,494 vs expected 14,177.
/// - Delta vs `origin/main`: +112 offenses, decomposed into:
///   - ~109 recovered missing offenses (mostly singleton-class FNs)
///   - ~3 additional non-noise offenses (fastlane, sentry-ruby, mongoid)
///   - large `jruby` file-drop noise dominates aggregate counts in both runs.
///
/// Remaining known gap:
/// - Struct/Class.new assignment forms (`Foo = Class.new do ... end`) are still
///   not handled here and should be fixed in a follow-up change.
pub struct ClassLength;

fn check_classlike_length(
    cop: &ClassLength,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    max: usize,
    count_comments: bool,
    count_as_one: Option<&Vec<String>>,
    start_offset: usize,
    end_offset: usize,
    body: Option<ruby_prism::Node<'_>>,
) {
    let mut foldable_ranges = Vec::new();
    if let Some(cao) = count_as_one {
        if !cao.is_empty() {
            if let Some(body_node) = &body {
                foldable_ranges.extend(collect_foldable_ranges(source, body_node, cao));
            }
        }
    }

    let inner_ranges = body
        .as_ref()
        .map(|b| inner_classlike_ranges(source, b))
        .unwrap_or_default();

    let count = count_body_lines_full(
        source,
        start_offset,
        end_offset,
        count_comments,
        &foldable_ranges,
        &inner_ranges,
    );

    if count > max {
        let (line, column) = source.offset_to_line_col(start_offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Class has too many lines. [{count}/{max}]"),
        ));
    }
}

fn check_singleton_class_length(
    cop: &ClassLength,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    max: usize,
    count_comments: bool,
    count_as_one: Option<&Vec<String>>,
    start_offset: usize,
    end_offset: usize,
    body: Option<ruby_prism::Node<'_>>,
) {
    let mut foldable_ranges = Vec::new();
    if let Some(cao) = count_as_one {
        if !cao.is_empty() {
            if let Some(body_node) = &body {
                foldable_ranges.extend(collect_foldable_ranges(source, body_node, cao));
            }
        }
    }

    // RuboCop handles `sclass` via generic code-length calculation (not the
    // class/module classlike path), so use non-classlike counting here.
    let count = count_body_lines_ex(
        source,
        start_offset,
        end_offset,
        count_comments,
        &foldable_ranges,
    );

    if count > max {
        let (line, column) = source.offset_to_line_col(start_offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Class has too many lines. [{count}/{max}]"),
        ));
    }
}

struct SingletonClassLengthVisitor<'a> {
    cop: &'a ClassLength,
    source: &'a SourceFile,
    max: usize,
    count_comments: bool,
    count_as_one: Option<Vec<String>>,
    diagnostics: &'a mut Vec<Diagnostic>,
    class_depth: usize,
}

impl<'pr> Visit<'pr> for SingletonClassLengthVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.class_depth += 1;
        ruby_prism::visit_class_node(self, node);
        self.class_depth -= 1;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // Match RuboCop's on_sclass: skip singleton classes nested under class.
        if self.class_depth == 0 {
            check_singleton_class_length(
                self.cop,
                self.source,
                self.diagnostics,
                self.max,
                self.count_comments,
                self.count_as_one.as_ref(),
                node.class_keyword_loc().start_offset(),
                node.end_keyword_loc().start_offset(),
                node.body(),
            );
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }
}

impl Cop for ClassLength {
    fn name(&self) -> &'static str {
        "Metrics/ClassLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, MODULE_NODE, STATEMENTS_NODE]
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
        let max = config.get_usize("Max", 100);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        let class_node = if let Some(class_node) = node.as_class_node() {
            class_node
        } else {
            return;
        };

        check_classlike_length(
            self,
            source,
            diagnostics,
            max,
            count_comments,
            count_as_one.as_ref(),
            class_node.class_keyword_loc().start_offset(),
            class_node.end_keyword_loc().start_offset(),
            class_node.body(),
        );
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
        let mut visitor = SingletonClassLengthVisitor {
            cop: self,
            source,
            max: config.get_usize("Max", 100),
            count_comments: config.get_bool("CountComments", false),
            count_as_one: config.get_string_array("CountAsOne"),
            diagnostics,
            class_depth: 0,
        };
        visitor.visit(&parse_result.node());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassLength, "cops/metrics/class_length");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"class Foo\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line class");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_hash() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // With CountAsOne: ["hash"], a multiline hash counts as 1 line
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountAsOne".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("hash".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, { k: v, \n k2: v2 \n } = 2 + 1 folded = 3 lines
        let source = b"class Foo\n  a = 1\n  b = 2\n  HASH = {\n    k: 1,\n    k2: 2\n  }\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire when hash is folded (3/3)"
        );
    }

    #[test]
    fn singleton_class_nested_under_class_is_skipped() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };

        let source = b"class Outer\n  class << self\n    a = 1\n    b = 2\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);

        assert_eq!(diags.len(), 1, "Nested singleton class should be skipped");
        assert_eq!(diags[0].location.line, 1);
    }
}
