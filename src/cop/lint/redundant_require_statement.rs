use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for unnecessary `require` statements.
///
/// Detects `require` calls for features already loaded by default in the target
/// Ruby version. For `require 'pp'` (Ruby 2.5+), the require is only redundant
/// if the file does not use `PP` constant or any `pretty_print*` / `pretty_inspect`
/// methods, matching RuboCop's `need_to_require_pp?` heuristic.
///
/// Root cause of 1,223 FNs (37.4% match rate): the original implementation was
/// missing `pp` support entirely. Since `require 'pp'` is extremely common in
/// Ruby codebases and the corpus uses TargetRubyVersion: 4.0 (well above 2.5),
/// nearly all `require 'pp'` calls should be flagged.
pub struct RedundantRequireStatement;

/// Features that are always redundant (Ruby 2.0+, well below any supported version).
const ALWAYS_REDUNDANT: &[&[u8]] = &[b"enumerator"];

/// Features redundant since Ruby 2.1+.
const RUBY_21_REDUNDANT: &[&[u8]] = &[b"thread"];

/// Features redundant since Ruby 2.2+.
const RUBY_22_REDUNDANT: &[&[u8]] = &[b"rational", b"complex"];

/// Features redundant since Ruby 2.7+.
const RUBY_27_REDUNDANT: &[&[u8]] = &[b"ruby2_keywords"];

/// Features redundant since Ruby 3.1+.
const RUBY_31_REDUNDANT: &[&[u8]] = &[b"fiber"];

/// Features redundant since Ruby 3.2+.
const RUBY_32_REDUNDANT: &[&[u8]] = &[b"set"];

/// Pretty-print method names that indicate `require 'pp'` is needed.
const PRETTY_PRINT_METHODS: &[&[u8]] = &[
    b"pretty_inspect",
    b"pretty_print",
    b"pretty_print_cycle",
    b"pretty_print_inspect",
    b"pretty_print_instance_variables",
];

/// Get the target Ruby version from cop config, defaulting to 2.7
/// (matching RuboCop's default when no version is specified).
fn target_ruby_version(config: &CopConfig) -> f64 {
    config
        .options
        .get("TargetRubyVersion")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
        .unwrap_or(2.7)
}

/// Check if a feature is redundant given the target Ruby version.
/// For `pp`, returns true only based on version; the caller must separately
/// check for PP usage in the file.
fn is_redundant_feature(feature: &[u8], ruby_version: f64) -> bool {
    if ALWAYS_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 2.1 && RUBY_21_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 2.2 && RUBY_22_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 2.5 && feature == b"pp" {
        return true;
    }
    if ruby_version >= 2.7 && RUBY_27_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 3.1 && RUBY_31_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 3.2 && RUBY_32_REDUNDANT.contains(&feature) {
        return true;
    }
    false
}

/// Visitor that checks if the file uses PP constant or pretty_print methods.
struct PpUsageVisitor {
    found: bool,
}

impl<'pr> Visit<'pr> for PpUsageVisitor {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }

        let method_name = node.name().as_slice();

        // Check for pretty_print methods
        if PRETTY_PRINT_METHODS.contains(&method_name) {
            self.found = true;
            return;
        }

        // Check for PP.method_name or ::PP.method_name
        if let Some(recv) = node.receiver() {
            if is_pp_const(&recv) {
                self.found = true;
                return;
            }
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

/// Check if a node is the `PP` constant (bare or with `::` prefix).
fn is_pp_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == b"PP";
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::PP — parent is None (cbase), child is PP
        if cp.parent().is_none() {
            if let Some(name) = cp.name() {
                return name.as_slice() == b"PP";
            }
        }
    }
    false
}

/// Check if the AST contains any usage of PP constant or pretty_print methods.
fn needs_pp_require(parse_result: &ruby_prism::ParseResult<'_>) -> bool {
    let mut visitor = PpUsageVisitor { found: false };
    visitor.visit(&parse_result.node());
    visitor.found
}

/// Visitor that finds redundant require statements and collects diagnostics.
struct RequireVisitor<'a, 'src, 'pr> {
    cop: &'a RedundantRequireStatement,
    source: &'src SourceFile,
    ruby_version: f64,
    needs_pp: bool,
    diagnostics: Vec<Diagnostic>,
    _phantom: std::marker::PhantomData<&'pr ()>,
}

impl<'a, 'src, 'pr> Visit<'pr> for RequireVisitor<'a, 'src, 'pr> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"require" && node.receiver().is_none() {
            if let Some(arguments) = node.arguments() {
                let args = arguments.arguments();
                if args.len() == 1 {
                    if let Some(first_arg) = args.iter().next() {
                        if let Some(string_node) = first_arg.as_string_node() {
                            let feature = string_node.unescaped();
                            if is_redundant_feature(feature, self.ruby_version) {
                                // For 'pp', skip if file uses PP/pretty_print
                                if feature == b"pp" && self.needs_pp {
                                    // Not redundant — file uses PP
                                } else {
                                    let loc = node.location();
                                    let (line, column) =
                                        self.source.offset_to_line_col(loc.start_offset());
                                    self.diagnostics.push(self.cop.diagnostic(
                                        self.source,
                                        line,
                                        column,
                                        "Remove unnecessary `require` statement.".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Continue visiting children (require could be nested in conditionals)
        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for RedundantRequireStatement {
    fn name(&self) -> &'static str {
        "Lint/RedundantRequireStatement"
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
        let ruby_ver = target_ruby_version(config);

        // Pre-compute whether the file needs require 'pp'
        let needs_pp = if ruby_ver >= 2.5 {
            needs_pp_require(parse_result)
        } else {
            false
        };

        let mut visitor = RequireVisitor {
            cop: self,
            source,
            ruby_version: ruby_ver,
            needs_pp,
            diagnostics: Vec::new(),
            _phantom: std::marker::PhantomData,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantRequireStatement,
        "cops/lint/redundant_require_statement"
    );
}
