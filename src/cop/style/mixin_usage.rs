use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation: 14+ FPs from `include T('default/layout/html')` in YARD templates.
/// Root cause: we checked `node.arguments().is_some()` which matches any argument including
/// method calls. RuboCop's node pattern requires arguments to be `const` nodes. Fixed by
/// verifying all arguments are ConstantReadNode or ConstantPathNode before flagging.
pub struct MixinUsage;

const MIXIN_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for MixinUsage {
    fn name(&self) -> &'static str {
        "Style/MixinUsage"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = MixinUsageVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_class_or_module: false,
            in_block: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MixinUsageVisitor<'a> {
    cop: &'a MixinUsage,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_class_or_module: bool,
    in_block: bool,
}

impl<'pr> Visit<'pr> for MixinUsageVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_bytes = node.name().as_slice();

        if MIXIN_METHODS.contains(&method_bytes)
            && node.receiver().is_none()
            && !self.in_class_or_module
            && !self.in_block
        {
            // RuboCop's node pattern requires `const` args — only flag when all
            // arguments are constants (ConstantReadNode or ConstantPathNode).
            // Method call arguments like `include T('...')` are not flagged.
            let is_const_mixin = node.arguments().is_some_and(|args| {
                args.arguments().iter().all(|arg| {
                    arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some()
                })
            });

            if is_const_mixin {
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("include");
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "`{method_str}` is used at the top level. Use inside `class` or `module`."
                    ),
                ));
            }
        }

        // Visit children
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let prev = self.in_class_or_module;
        self.in_class_or_module = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_class_or_module = prev;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let prev = self.in_class_or_module;
        self.in_class_or_module = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_class_or_module = prev;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let prev = self.in_class_or_module;
        self.in_class_or_module = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_class_or_module = prev;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let prev = self.in_block;
        self.in_block = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixinUsage, "cops/style/mixin_usage");
}
