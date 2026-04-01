use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-13)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP=2: both false positives came from `class << self` bodies that contain
/// multi-argument mixin macros such as `include Foo, Bar` in puppetlabs/puppet.
///
/// Root cause: RuboCop only defines `on_class` and `on_module` (aliased) for
/// this cop — there is no `on_sclass`. So `class << self` bodies are never
/// checked by RuboCop. nitrocop was incorrectly visiting `SingletonClassNode`.
///
/// Fix: removed `visit_singleton_class_node` entirely. The default Visit impl
/// still recurses into singleton class children, so any nested class/module
/// nodes inside `class << self` are still checked by `visit_class_node` /
/// `visit_module_node`.
///
/// Previous attempts that also landed at actual=181 likely had a different bug
/// (e.g., breaking the recursive visit into singleton class children).
pub struct MixinGrouping;

const MIXIN_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for MixinGrouping {
    fn name(&self) -> &'static str {
        "Style/MixinGrouping"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "separated").to_string();
        let mut visitor = MixinGroupingVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
            style,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MixinGroupingVisitor<'a> {
    cop: &'a MixinGrouping,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
    style: String,
}

impl MixinGroupingVisitor<'_> {
    fn check_body_statements(&mut self, stmts: &ruby_prism::StatementsNode<'_>) {
        for stmt in stmts.body().iter() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            let method_bytes = call.name().as_slice();

            if !MIXIN_METHODS.contains(&method_bytes) {
                continue;
            }

            // Must not have a receiver (bare include/extend/prepend)
            if call.receiver().is_some() {
                continue;
            }

            let args = match call.arguments() {
                Some(a) => a,
                None => continue,
            };

            let arg_list: Vec<_> = args.arguments().iter().collect();

            if self.style == "separated" && arg_list.len() > 1 {
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("include");
                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Put `{method_str}` mixins in separate statements."),
                ));

                if let Some(corrections) = self.corrections.as_deref_mut() {
                    if line_contains_only_call(self.source, loc.start_offset(), loc.end_offset()) {
                        let replacement = build_split_mixin_replacement(
                            self.source,
                            method_str,
                            &arg_list,
                            loc.start_offset(),
                        );
                        corrections.push(crate::correction::Correction {
                            start: loc.start_offset(),
                            end: loc.end_offset(),
                            replacement,
                            cop_name: self.cop.name(),
                            cop_index: 0,
                        });
                    }
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for MixinGroupingVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.check_body_statements(&stmts);
            }
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.check_body_statements(&stmts);
            }
        }
        ruby_prism::visit_module_node(self, node);
    }

    // Note: RuboCop's on_class/on_module do NOT handle sclass (class << self).
    // We intentionally skip visit_singleton_class_node. The default Visit impl
    // still recurses into singleton class children, so nested class/module nodes
    // inside class << self are still checked.
}

fn line_start_offset(source: &SourceFile, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    line_start
}

fn line_contains_only_call(source: &SourceFile, call_start: usize, call_end: usize) -> bool {
    let bytes = source.as_bytes();
    let line_start = line_start_offset(source, call_start);

    let mut line_end = call_end;
    while line_end < bytes.len() && bytes[line_end] != b'\n' {
        line_end += 1;
    }

    bytes[line_start..call_start]
        .iter()
        .all(|b| b.is_ascii_whitespace())
        && bytes[call_end..line_end]
            .iter()
            .all(|b| b.is_ascii_whitespace())
}

fn build_split_mixin_replacement(
    source: &SourceFile,
    method: &str,
    args: &[ruby_prism::Node<'_>],
    call_start: usize,
) -> String {
    let line_start = line_start_offset(source, call_start);
    let indent = &source.as_bytes()[line_start..call_start];
    let indent = std::str::from_utf8(indent).unwrap_or("");

    let mut lines = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");
        if idx == 0 {
            lines.push(format!("{method} {arg_src}"));
        } else {
            lines.push(format!("{indent}{method} {arg_src}"));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixinGrouping, "cops/style/mixin_grouping");
    crate::cop_autocorrect_fixture_tests!(MixinGrouping, "cops/style/mixin_grouping");
}
