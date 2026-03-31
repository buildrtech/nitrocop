use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/InfiniteLoop
///
/// ## Investigation findings
///
/// FP=317 root cause: RuboCop uses VariableForce to track local variable
/// assignments and references. It skips the offense when a local variable is
/// first assigned inside the `while true`/`until false` loop body and then
/// referenced after the loop ends — because converting to `loop do` would
/// create a block scope that hides the variable. nitrocop was not implementing
/// this variable scoping exemption at all.
///
/// FN=19 root cause: nitrocop only matched `true`/`false` literals as
/// conditions. RuboCop's `truthy_literal?` also matches integer, float, array,
/// and hash literals; `falsey_literal?` also matches `nil`.
///
/// Fix: switched from `check_node` to `check_source` with a visitor that
/// collects local variable writes inside loop bodies and reads after the loop,
/// implementing the variable scoping exemption. Also added truthy/falsey
/// literal detection for integers, floats, arrays, hashes, and nil.
pub struct InfiniteLoop;

impl Cop for InfiniteLoop {
    fn name(&self) -> &'static str {
        "Style/InfiniteLoop"
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = InfiniteLoopVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(ref mut corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

struct InfiniteLoopVisitor<'a> {
    cop: &'a InfiniteLoop,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
}

/// Returns true if the node is a truthy literal (true, integer, float, array, hash).
fn is_truthy_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_true_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
}

/// Returns true if the node is a falsey literal (false, nil).
fn is_falsey_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_false_node().is_some() || node.as_nil_node().is_some()
}

/// Visitor to collect local variable write names from a node tree.
struct LvarWriteCollector {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LvarWriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = node.name().as_slice().to_vec();
        if !self.names.contains(&name) {
            self.names.push(name);
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = node.name().as_slice().to_vec();
        if !self.names.contains(&name) {
            self.names.push(name);
        }
    }
}

/// Visitor to check if any of the given variable names are read.
struct LvarReadChecker<'a> {
    names: &'a [Vec<u8>],
    found: bool,
}

impl<'pr> Visit<'pr> for LvarReadChecker<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if self.names.contains(&node.name().as_slice().to_vec()) {
            self.found = true;
        }
    }
}

/// Visitor to check if a specific variable name is written.
struct LvarWriteChecker<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for LvarWriteChecker<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }
}

fn collect_lvar_writes(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarWriteCollector { names: Vec::new() };
    collector.visit(node);
    collector.names
}

fn has_lvar_read(node: &ruby_prism::Node<'_>, names: &[Vec<u8>]) -> bool {
    let mut checker = LvarReadChecker {
        names,
        found: false,
    };
    checker.visit(node);
    checker.found
}

fn has_lvar_write(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    let mut checker = LvarWriteChecker { name, found: false };
    checker.visit(node);
    checker.found
}

/// Check if converting a while/until loop to `loop do` would break variable scoping.
/// Returns true if the offense should be suppressed.
fn would_break_scoping(
    siblings: &[ruby_prism::Node<'_>],
    loop_index: usize,
    loop_stmts: Option<ruby_prism::StatementsNode<'_>>,
) -> bool {
    let stmts_node = match loop_stmts {
        Some(ref s) => s.as_node(),
        None => return false,
    };

    let vars_written_inside = collect_lvar_writes(&stmts_node);
    if vars_written_inside.is_empty() {
        return false;
    }

    for var_name in &vars_written_inside {
        // Check if variable is assigned before the loop
        let assigned_before = siblings[..loop_index]
            .iter()
            .any(|s| has_lvar_write(s, var_name));
        if assigned_before {
            continue;
        }

        // Check if variable is referenced after the loop
        let referenced_after = siblings[loop_index + 1..]
            .iter()
            .any(|s| has_lvar_read(s, std::slice::from_ref(var_name)));
        if referenced_after {
            return true;
        }
    }

    false
}

impl InfiniteLoopVisitor<'_> {
    fn build_replacement(
        &self,
        keyword_start: usize,
        predicate_end: usize,
        end_start: usize,
    ) -> String {
        let indent = " ".repeat(self.source.offset_to_line_col(keyword_start).1);
        let bytes = self.source.as_bytes();
        if predicate_end < end_start {
            let mut body = String::from_utf8_lossy(&bytes[predicate_end..end_start]).to_string();
            if body.starts_with(" do") {
                body = body[3..].to_string();
            }
            if !body.starts_with('\n') {
                body = format!("\n{}", body.trim_start_matches([' ', '\t']));
            }
            body = body.trim_end_matches([' ', '\t', '\n']).to_string();
            body.push('\n');
            format!("loop do{}{indent}end", body)
        } else {
            format!("loop do\n{indent}end")
        }
    }

    fn find_end_keyword_start(&self, loc: ruby_prism::Location<'_>) -> Option<usize> {
        let bytes = self.source.as_bytes();
        let start = loc.start_offset();
        let end = loc.end_offset();
        if end <= start + 3 || end > bytes.len() {
            return None;
        }
        let slice = &bytes[start..end];
        for i in (0..=slice.len().saturating_sub(3)).rev() {
            if &slice[i..i + 3] == b"end" {
                let abs = start + i;
                let prev_ok = abs == 0
                    || bytes[abs - 1] == b'\n'
                    || bytes[abs - 1] == b' '
                    || bytes[abs - 1] == b'\t';
                let next_ok = abs + 3 >= bytes.len()
                    || matches!(bytes[abs + 3], b'\n' | b' ' | b'\t' | b';');
                if prev_ok && next_ok {
                    return Some(abs);
                }
            }
        }
        None
    }

    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let Some(while_node) = stmt.as_while_node() {
                if is_truthy_literal(&while_node.predicate())
                    && !would_break_scoping(stmts, i, while_node.statements())
                {
                    let kw_loc = while_node.keyword_loc();
                    let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
                    let mut diag = self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `Kernel#loop` for infinite loops.".to_string(),
                    );

                    if let Some(end_start) = self.find_end_keyword_start(stmt.location()) {
                        let replacement = self.build_replacement(
                            kw_loc.start_offset(),
                            while_node.predicate().location().end_offset(),
                            end_start,
                        );
                        self.corrections.push(crate::correction::Correction {
                            start: stmt.location().start_offset(),
                            end: stmt.location().end_offset(),
                            replacement,
                            cop_name: self.cop.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    self.diagnostics.push(diag);
                }
            } else if let Some(until_node) = stmt.as_until_node()
                && is_falsey_literal(&until_node.predicate())
                && !would_break_scoping(stmts, i, until_node.statements())
            {
                let kw_loc = until_node.keyword_loc();
                let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
                let mut diag = self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use `Kernel#loop` for infinite loops.".to_string(),
                );

                if let Some(end_start) = self.find_end_keyword_start(stmt.location()) {
                    let replacement = self.build_replacement(
                        kw_loc.start_offset(),
                        until_node.predicate().location().end_offset(),
                        end_start,
                    );
                    self.corrections.push(crate::correction::Correction {
                        start: stmt.location().start_offset(),
                        end: stmt.location().end_offset(),
                        replacement,
                        cop_name: self.cop.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                self.diagnostics.push(diag);
            }
        }
    }
}

impl<'pr> Visit<'pr> for InfiniteLoopVisitor<'_> {
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        let stmts: Vec<_> = node.statements().body().iter().collect();
        self.check_statements(&stmts);
        ruby_prism::visit_program_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                self.check_statements(&children);
            }
        }
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                self.check_statements(&children);
            }
        }
        ruby_prism::visit_block_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                self.check_statements(&children);
            }
        }
        ruby_prism::visit_lambda_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                self.check_statements(&children);
            }
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                self.check_statements(&children);
            }
        }
        ruby_prism::visit_module_node(self, node);
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                self.check_statements(&children);
            }
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(stmts) = node.statements() {
            let children: Vec<_> = stmts.body().iter().collect();
            self.check_statements(&children);
        }
        ruby_prism::visit_begin_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InfiniteLoop, "cops/style/infinite_loop");
    crate::cop_autocorrect_fixture_tests!(InfiniteLoop, "cops/style/infinite_loop");
}
