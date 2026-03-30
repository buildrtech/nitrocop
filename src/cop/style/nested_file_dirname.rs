use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct NestedFileDirname;

impl Cop for NestedFileDirname {
    fn name(&self) -> &'static str {
        "Style/NestedFileDirname"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // minimum_target_ruby_version 3.1
        let ruby_ver = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.4);
        if ruby_ver < 3.1 {
            return;
        }

        let mut visitor = DirnameVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            emit_corrections: corrections.is_some(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(ref mut corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

struct DirnameVisitor<'a, 'src> {
    cop: &'a NestedFileDirname,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    emit_corrections: bool,
}

impl<'pr> Visit<'pr> for DirnameVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"dirname" {
            if let Some(recv) = node.receiver() {
                if is_file_const(&recv) {
                    if let Some(args) = node.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() && is_file_dirname_call(&arg_list[0]) {
                            // Outermost nested File.dirname — report it.
                            let level = count_dirname_nesting(&arg_list[0], 1) + 1;
                            let inner_path_src =
                                get_innermost_path_source(&arg_list[0], self.source);
                            let msg_loc = node.message_loc().unwrap_or_else(|| node.location());
                            let (line, column) =
                                self.source.offset_to_line_col(msg_loc.start_offset());
                            let replacement = format!("dirname({}, {})", inner_path_src, level);
                            let mut diag = self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!("Use `{}` instead.", replacement),
                            );

                            if self.emit_corrections {
                                self.corrections.push(crate::correction::Correction {
                                    start: msg_loc.start_offset(),
                                    end: node.location().end_offset(),
                                    replacement,
                                    cop_name: self.cop.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }

                            self.diagnostics.push(diag);
                            // Skip visiting children — inner File.dirname calls
                            // are already counted; don't produce inner reports.
                            return;
                        }
                    }
                }
            }
        }

        // Default: visit all children
        ruby_prism::visit_call_node(self, node);
    }
}

fn is_file_dirname_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"dirname" {
            if let Some(recv) = call.receiver() {
                return is_file_const(&recv);
            }
        }
    }
    false
}

fn count_dirname_nesting(node: &ruby_prism::Node<'_>, level: usize) -> usize {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"dirname" {
            if let Some(recv) = call.receiver() {
                if is_file_const(&recv) {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() && is_file_dirname_call(&arg_list[0]) {
                            return count_dirname_nesting(&arg_list[0], level + 1);
                        }
                    }
                }
            }
        }
    }
    level
}

fn get_innermost_path_source(node: &ruby_prism::Node<'_>, source: &SourceFile) -> String {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"dirname" {
            if let Some(recv) = call.receiver() {
                if is_file_const(&recv) {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() {
                            return get_innermost_path_source(&arg_list[0], source);
                        }
                    }
                }
            }
        }
    }
    let loc = node.location();
    std::str::from_utf8(&source.content[loc.start_offset()..loc.end_offset()])
        .unwrap_or("path")
        .to_string()
}

fn is_file_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == b"File";
    }
    if let Some(cp) = node.as_constant_path_node() {
        return cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"File");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedFileDirname, "cops/style/nested_file_dirname");
    crate::cop_autocorrect_fixture_tests!(NestedFileDirname, "cops/style/nested_file_dirname");
}
