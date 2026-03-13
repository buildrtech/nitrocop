use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks that access modifiers are declared in the correct style (group or inline).
///
/// ## Investigation (2026-03-13)
///
/// Root cause of 543 FPs: nitrocop was flagging `private def method_name` inside
/// block bodies (e.g., `class_methods do`, `included do`, `concern do`). RuboCop's
/// `allowed?` method checks `node.parent&.type?(:pair, :any_block)` — access modifiers
/// whose parent is a block node are always skipped. This is because DSL blocks like
/// `class_methods do...end` are not class/module bodies, so the group/inline style
/// enforcement doesn't apply there.
///
/// Fix: Switched from `check_node` to `check_source` with a visitor that tracks whether
/// the current scope is a class/module body vs a block body. Access modifiers are only
/// checked when directly inside a class/module/sclass body, not inside block bodies.
pub struct AccessModifierDeclarations;

const ACCESS_MODIFIERS: &[&str] = &["private", "protected", "public", "module_function"];

struct AccessModifierVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a AccessModifierDeclarations,
    enforced_style: &'a str,
    allow_modifiers_on_symbols: bool,
    allow_modifiers_on_attrs: bool,
    allow_modifiers_on_alias_method: bool,
    diagnostics: Vec<Diagnostic>,
    /// true when the current scope is a class/module/sclass body (not a block)
    in_class_body: bool,
}

impl AccessModifierVisitor<'_> {
    fn check_access_modifier(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !ACCESS_MODIFIERS.contains(&method_name) {
            return;
        }

        // Skip if has receiver (must be bare access modifier call)
        if call.receiver().is_some() {
            return;
        }

        // Skip if not in a class/module body (i.e., inside a block body)
        if !self.in_class_body {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return, // Group-style modifier with no args is fine
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the argument is a symbol
        let first_arg = &arg_list[0];
        let is_symbol_arg = first_arg.as_symbol_node().is_some();

        if is_symbol_arg && self.allow_modifiers_on_symbols {
            return;
        }

        // Check for attr_* calls
        if self.allow_modifiers_on_attrs {
            if let Some(inner_call) = first_arg.as_call_node() {
                let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                if matches!(
                    inner_name,
                    "attr_reader" | "attr_writer" | "attr_accessor" | "attr"
                ) {
                    return;
                }
            }
        }

        // Check for alias_method
        if self.allow_modifiers_on_alias_method {
            if let Some(inner_call) = first_arg.as_call_node() {
                let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                if inner_name == "alias_method" {
                    return;
                }
            }
        }

        // Distinguish between inline modifier declarations and visibility-change calls
        let is_inline_modifier =
            first_arg.as_def_node().is_some() || first_arg.as_symbol_node().is_some();

        match self.enforced_style {
            "inline" => {
                // Inline style with args = OK
            }
            "group" => {
                if !is_inline_modifier {
                    return;
                }

                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "`{}` should not be inlined in method definitions.",
                        method_name
                    ),
                ));
            }
            _ => {}
        }
    }
}

impl<'pr> Visit<'pr> for AccessModifierVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let saved = self.in_class_body;
        self.in_class_body = true;
        ruby_prism::visit_class_node(self, node);
        self.in_class_body = saved;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let saved = self.in_class_body;
        self.in_class_body = true;
        ruby_prism::visit_module_node(self, node);
        self.in_class_body = saved;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let saved = self.in_class_body;
        self.in_class_body = true;
        ruby_prism::visit_singleton_class_node(self, node);
        self.in_class_body = saved;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let saved = self.in_class_body;
        self.in_class_body = false;
        ruby_prism::visit_block_node(self, node);
        self.in_class_body = saved;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let saved = self.in_class_body;
        self.in_class_body = false;
        ruby_prism::visit_lambda_node(self, node);
        self.in_class_body = saved;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_access_modifier(node);
        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for AccessModifierDeclarations {
    fn name(&self) -> &'static str {
        "Style/AccessModifierDeclarations"
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
        let enforced_style = config.get_str("EnforcedStyle", "group");
        let allow_modifiers_on_symbols = config.get_bool("AllowModifiersOnSymbols", true);
        let allow_modifiers_on_attrs = config.get_bool("AllowModifiersOnAttrs", true);
        let allow_modifiers_on_alias_method = config.get_bool("AllowModifiersOnAliasMethod", true);

        let mut visitor = AccessModifierVisitor {
            source,
            cop: self,
            enforced_style,
            allow_modifiers_on_symbols,
            allow_modifiers_on_attrs,
            allow_modifiers_on_alias_method,
            diagnostics: Vec::new(),
            in_class_body: true,
        };

        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AccessModifierDeclarations,
        "cops/style/access_modifier_declarations"
    );
}
