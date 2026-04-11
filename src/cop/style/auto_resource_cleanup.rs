use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/AutoResourceCleanup
///
/// Investigation: 19 FPs from qualified constant paths like `Zip::File.open(...)`.
/// The `ConstantPathNode` branch extracted the last component name (e.g. "File" from
/// `Zip::File`) which falsely matched the stdlib `File` check. Fix: only match
/// `ConstantPathNode` when `parent()` is `None` (root-scoped `::File`/`::Tempfile`).
pub struct AutoResourceCleanup;

impl Cop for AutoResourceCleanup {
    fn name(&self) -> &'static str {
        "Style/AutoResourceCleanup"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_enabled(&self) -> bool {
        false
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
        let mut visitor = AutoResourceCleanupVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct AutoResourceCleanupVisitor<'a, 'corr> {
    cop: &'a AutoResourceCleanup,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'corr mut Vec<crate::correction::Correction>>,
}

/// Check if a call node is `File.open(...)` or `Tempfile.open(...)` without a block.
fn is_resource_open_without_block(call: &ruby_prism::CallNode<'_>) -> Option<String> {
    let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
    if method_name != "open" {
        return None;
    }

    let receiver = call.receiver()?;

    let recv_name = if let Some(read) = receiver.as_constant_read_node() {
        std::str::from_utf8(read.name().as_slice()).unwrap_or("")
    } else if let Some(path) = receiver.as_constant_path_node() {
        // Only match root-scoped ::File or ::Tempfile (parent is None).
        // Skip qualified paths like Zip::File where parent is Some.
        if path.parent().is_some() {
            return None;
        }
        std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("")
    } else {
        return None;
    };

    if !matches!(recv_name, "File" | "Tempfile") {
        return None;
    }

    // Skip if it has a block
    if call.block().is_some() {
        return None;
    }

    // Skip if it has a block argument (&:read etc)
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if arg.as_block_argument_node().is_some() {
                return None;
            }
        }
    }

    let recv_str = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("File");
    Some(recv_str.to_string())
}

impl<'pr> Visit<'pr> for AutoResourceCleanupVisitor<'_, '_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // Only flag File.open/Tempfile.open when assigned to a local variable
        if let Some(call) = node.value().as_call_node() {
            if let Some(recv_str) = is_resource_open_without_block(&call) {
                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Use the block version of `{}.open`.", recv_str),
                );
                if let Some(corrections) = self.corrections.as_deref_mut() {
                    corrections.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: "nil".to_string(),
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
                self.diagnostics.push(diagnostic);
            }
        }

        // Recurse into children
        ruby_prism::visit_local_variable_write_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AutoResourceCleanup, "cops/style/auto_resource_cleanup");

    #[test]
    fn autocorrect_replaces_file_open_without_block_assignment_with_nil() {
        crate::testutil::assert_cop_autocorrect(
            &AutoResourceCleanup,
            b"file = File.open(path)\n",
            b"file = nil\n",
        );
    }
}
