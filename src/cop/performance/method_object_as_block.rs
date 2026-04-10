use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// NOTE: Known conformance difference (~4 excess offenses vs RuboCop).
/// 1 was a genuine FP from safe navigation (fixed). The remaining ~3 are pre-existing
/// config differences — repos where nitrocop enables this cop but RuboCop doesn't
/// (e.g., different rubocop-performance gem versions or config inheritance).
pub struct MethodObjectAsBlock;

impl Cop for MethodObjectAsBlock {
    fn name(&self) -> &'static str {
        "Performance/MethodObjectAsBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = MethodObjectVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

struct MethodObjectVisitor<'a, 'src> {
    cop: &'a MethodObjectAsBlock,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
}

/// Check if a block argument node contains a call to `method(:symbol)`.
/// RuboCop only flags `&method(:sym)` where the argument is a symbol literal,
/// not `&method(variable)` or `&method("string")`.
fn is_method_object_block_arg(block_arg: &ruby_prism::BlockArgumentNode<'_>) -> bool {
    let expr = match block_arg.expression() {
        Some(e) => e,
        None => return false,
    };
    let call = match expr.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if call.name().as_slice() != b"method" {
        return false;
    }
    // Require exactly one argument that is a symbol literal
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };
    let arg_list = args.arguments();
    arg_list.len() == 1 && arg_list.iter().next().unwrap().as_symbol_node().is_some()
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    source
        .byte_slice(
            node.location().start_offset(),
            node.location().end_offset(),
            "",
        )
        .to_string()
}

fn replacement_for_call(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    block_arg: &ruby_prism::BlockArgumentNode<'_>,
) -> Option<String> {
    let method_call = block_arg.expression()?.as_call_node()?;
    let args = method_call.arguments()?;
    let sym = args.arguments().iter().next()?.as_symbol_node()?;
    let sym_name = String::from_utf8_lossy(sym.unescaped()).to_string();

    let target = if let Some(recv) = method_call.receiver() {
        format!("{}.{}", node_source(source, &recv), sym_name)
    } else {
        sym_name
    };

    let call_name = String::from_utf8_lossy(call.name().as_slice()).to_string();
    let call_prefix = if let Some(recv) = call.receiver() {
        let op = call
            .call_operator_loc()
            .map(|l| l.as_slice())
            .unwrap_or(b".");
        format!(
            "{}{}{}",
            node_source(source, &recv),
            String::from_utf8_lossy(op),
            call_name
        )
    } else {
        call_name
    };

    let mut arg_sources = Vec::new();
    if let Some(call_args) = call.arguments() {
        for arg in call_args.arguments().iter() {
            if arg.as_block_argument_node().is_some() {
                continue;
            }
            arg_sources.push(node_source(source, &arg));
        }
    }

    if arg_sources.is_empty() {
        Some(format!("{call_prefix} {{ |arg| {target}(arg) }}"))
    } else {
        Some(format!(
            "{call_prefix}({}) {{ |arg| {target}(arg) }}",
            arg_sources.join(", ")
        ))
    }
}

impl MethodObjectVisitor<'_, '_> {
    fn emit_for_block_arg(
        &mut self,
        call: &ruby_prism::CallNode<'_>,
        block_arg: &ruby_prism::BlockArgumentNode<'_>,
    ) {
        let loc = block_arg.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());

        if self
            .diagnostics
            .iter()
            .any(|d| d.location.line == line && d.location.column == column)
        {
            return;
        }

        let mut diagnostic = self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use a block instead of `&method(...)` for better performance.".to_string(),
        );

        if let Some(replacement) = replacement_for_call(self.source, call, block_arg) {
            self.corrections.push(crate::correction::Correction {
                start: call.location().start_offset(),
                end: call.location().end_offset(),
                replacement,
                cop_name: self.cop.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        self.diagnostics.push(diagnostic);
    }
}

impl<'pr> Visit<'pr> for MethodObjectVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // RuboCop's pattern uses ^send which excludes csend (safe navigation &.),
        // so skip when the parent call uses safe navigation.
        let is_safe_nav = if let Some(op) = node.call_operator_loc() {
            op.as_slice() == b"&."
        } else {
            false
        };
        if !is_safe_nav {
            // Check if this call node has a block argument that's &method(...)
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(block_arg) = arg.as_block_argument_node()
                        && is_method_object_block_arg(&block_arg)
                    {
                        self.emit_for_block_arg(node, &block_arg);
                    }
                }
            }
            // Also check the block argument slot (outside of arguments list)
            if let Some(block) = node.block() {
                if let Some(block_arg) = block.as_block_argument_node()
                    && is_method_object_block_arg(&block_arg)
                {
                    self.emit_for_block_arg(node, &block_arg);
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    // Intentionally do NOT visit super nodes — RuboCop's pattern requires ^send parent
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MethodObjectAsBlock,
        "cops/performance/method_object_as_block"
    );
    crate::cop_autocorrect_fixture_tests!(
        MethodObjectAsBlock,
        "cops/performance/method_object_as_block"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(MethodObjectAsBlock.supports_autocorrect());
    }
}
