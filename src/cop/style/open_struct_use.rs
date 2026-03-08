use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation: 15 FPs from `class OpenStruct` (reopening/defining the class).
/// Root cause: cop flagged ANY reference to `OpenStruct`, including class/module definitions.
/// RuboCop's `custom_class_or_module_definition?` skips when the constant is the name of a
/// class or module node (first child, i.e. left_siblings.empty?). Fixed by switching to a
/// visitor that tracks whether we're visiting the name position of a class/module definition.
pub struct OpenStructUse;

impl Cop for OpenStructUse {
    fn name(&self) -> &'static str {
        "Style/OpenStructUse"
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
        let mut visitor = OpenStructUseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OpenStructUseVisitor<'a> {
    cop: &'a OpenStructUse,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl OpenStructUseVisitor<'_> {
    fn check_open_struct(&mut self, name: &[u8], start_offset: usize) {
        if name == b"OpenStruct" {
            let (line, column) = self.source.offset_to_line_col(start_offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead."
                    .to_string(),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for OpenStructUseVisitor<'_> {
    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        self.check_open_struct(node.name().as_slice(), node.location().start_offset());
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        // Only flag root-scoped ::OpenStruct (parent is None),
        // not namespaced like YARD::OpenStruct or Foo::Bar::OpenStruct
        if node.parent().is_none() {
            if let Some(name) = node.name() {
                self.check_open_struct(name.as_slice(), node.location().start_offset());
            }
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Skip the constant_path (class name) — don't flag `class OpenStruct`
        // Only visit superclass and body
        if let Some(superclass) = node.superclass() {
            self.visit(&superclass);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        // Skip the constant_path (module name) — don't flag `module OpenStruct`
        // Only visit body
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OpenStructUse, "cops/style/open_struct_use");
}
