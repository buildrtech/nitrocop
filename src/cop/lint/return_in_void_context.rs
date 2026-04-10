use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for the use of `return` with a value in a context where the value
/// will be ignored (`initialize` and setter methods like `foo=`).
///
/// ## Investigation findings
///
/// FP root cause: the cop did not track lambda/define_method/define_singleton_method
/// scope boundaries. A `return` inside a lambda returns from the lambda, not from
/// the enclosing method, so it should not be flagged. Same for `define_method` and
/// `define_singleton_method` blocks.
///
/// FN root cause: the cop only checked `initialize` methods, but RuboCop also flags
/// `return value` inside setter methods (names ending in `=`, e.g. `foo=`).
/// RuboCop skips class-level `initialize` (`def self.initialize`) but still flags
/// class-level setters like `def self.foo=(...)` and `def self.[]=(...)`.
///
/// RuboCop's `SCOPE_CHANGING_METHODS` are: `lambda`, `define_method`,
/// `define_singleton_method`. In Prism, `lambda { }` and `lambda do...end` both
/// produce `LambdaNode`, so we stop recursion there. For `define_method` and
/// `define_singleton_method`, we intercept `visit_call_node` and skip the block body.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=15, FN=11.
///
/// FP=15: `===` methods were being treated as setters because the old heuristic
/// matched any method name ending in `=` that was not in a short operator allowlist.
/// Ruby's comparison operators (`==`, `===`, `!=`, `<=`, `>=`, `<=>`, `=~`) are
/// not void-context methods and must be excluded.
///
/// FN=11: `[]=` methods were incorrectly excluded from void-context detection,
/// and all class methods were skipped wholesale. RuboCop treats indexed
/// assignment methods as void-context just like regular setters, and it only
/// exempts class-level `initialize`, not class-level setters.
pub struct ReturnInVoidContext;

impl Cop for ReturnInVoidContext {
    fn name(&self) -> &'static str {
        "Lint/ReturnInVoidContext"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        let mut visitor = VoidContextVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Returns true if `name` is a void context method: `initialize`, a setter
/// (`foo=`), or indexed assignment (`[]=`). Excludes comparison/match operators
/// that happen to end in `=`.
fn is_void_method(name: &[u8]) -> bool {
    if name == b"initialize" {
        return true;
    }
    if name == b"[]=" {
        return true;
    }

    name.ends_with(b"=")
        && !matches!(
            name,
            b"==" | b"===" | b"!=" | b"<=" | b">=" | b"<=>" | b"=~"
        )
}

/// Format the method name for the diagnostic message.
fn format_method_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name).to_string()
}

struct VoidContextVisitor<'a, 'src> {
    cop: &'a ReturnInVoidContext,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
}

impl<'pr> Visit<'pr> for VoidContextVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();

        // RuboCop only exempts class-level `initialize`. Class-level setters like
        // `def self.foo=(...)` and `def self.[]=(...)` are still void-context methods.
        if node.receiver().is_some() && name == b"initialize" {
            ruby_prism::visit_def_node(self, node);
            return;
        }

        if !is_void_method(name) {
            // Still recurse into nested defs to find void context methods inside
            ruby_prism::visit_def_node(self, node);
            return;
        }

        let method_name = format_method_name(name);

        // Found void context method, look for return nodes with values
        let mut finder = ReturnWithValueFinder {
            returns: Vec::new(),
        };
        if let Some(body) = node.body() {
            finder.visit(&body);
        }

        for ret in finder.returns {
            let (line, column) = self.source.offset_to_line_col(ret.start_offset);
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Do not return a value in `{method_name}`."),
            );

            if let Some(corrections) = self.corrections.as_mut() {
                let keyword_end = ret.start_offset.saturating_add(6); // "return"
                corrections.push(crate::correction::Correction {
                    start: keyword_end,
                    end: ret.args_end_offset,
                    replacement: String::new(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            self.diagnostics.push(diagnostic);
        }
    }
}

struct ReturnWithValueFinder {
    returns: Vec<ReturnWithValueLoc>,
}

struct ReturnWithValueLoc {
    start_offset: usize,
    args_end_offset: usize,
}

/// Scope-changing method names where `return` exits the block, not the enclosing method.
const SCOPE_CHANGING_METHODS: &[&[u8]] = &[b"lambda", b"define_method", b"define_singleton_method"];

impl<'pr> Visit<'pr> for ReturnWithValueFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // ReturnNode with arguments means `return value`
        if let Some(args) = node.arguments() {
            self.returns.push(ReturnWithValueLoc {
                start_offset: node.location().start_offset(),
                args_end_offset: args.location().end_offset(),
            });
        }
        ruby_prism::visit_return_node(self, node);
    }

    // Don't recurse into nested def/class/module — they create new scopes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}

    // Don't recurse into lambda — return inside lambda returns from the lambda
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}

    // Don't recurse into define_method/define_singleton_method blocks —
    // return inside these exits the block, not the enclosing method
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        if SCOPE_CHANGING_METHODS.contains(&name) && node.block().is_some() {
            // Skip the entire call (including its block body)
            return;
        }
        // For other calls, recurse normally (including into block bodies)
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReturnInVoidContext, "cops/lint/return_in_void_context");
    crate::cop_autocorrect_fixture_tests!(ReturnInVoidContext, "cops/lint/return_in_void_context");
}
