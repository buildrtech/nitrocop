use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for useless `rescue` blocks that only re-raise the exception.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN fix:
/// - Prism represents inline `... rescue ...` as `RescueModifierNode`, not a
///   regular `RescueNode`. The remaining corpus miss was
///   `raise "TEST_ME" rescue raise rescue nil`, where the inner modifier
///   rescue only re-raises and should be flagged.
pub struct UselessRescue;

impl Cop for UselessRescue {
    fn name(&self) -> &'static str {
        "Lint/UselessRescue"
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
        let mut visitor = RescueVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            ensure_var_names: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(c) = corrections {
            c.extend(visitor.corrections);
        }
    }
}

struct RescueVisitor<'a, 'src> {
    cop: &'a UselessRescue,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    /// Local variable names referenced in the current ensure context.
    /// When visiting inside a begin/def node with an ensure clause,
    /// this contains variable names used in the ensure body.
    ensure_var_names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for RescueVisitor<'_, '_> {
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // If this begin has an ensure clause, collect local variable names used in it
        let prev_len = self.ensure_var_names.len();
        if let Some(ensure_clause) = node.ensure_clause() {
            collect_ensure_lvar_names(ensure_clause, &mut self.ensure_var_names);
        }

        ruby_prism::visit_begin_node(self, node);

        self.ensure_var_names.truncate(prev_len);
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // Check if this is the last rescue clause (no subsequent rescue)
        // RescueNode has a `subsequent` which is the next rescue clause
        if node.subsequent().is_none()
            && only_reraising(node, self.source)
            && !self.exception_var_used_in_ensure(node)
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Useless `rescue` detected.".to_string(),
            );

            let mut start = loc.start_offset();
            if start > 0 && self.source.as_bytes()[start - 1] == b'\n' {
                start -= 1;
            }
            self.corrections.push(crate::correction::Correction {
                start,
                end: loc.end_offset(),
                replacement: String::new(),
                cop_name: self.cop.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
            self.diagnostics.push(diagnostic);
        }

        // Continue visiting children
        ruby_prism::visit_rescue_node(self, node);
    }

    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        if modifier_only_reraising(node, self.source) {
            let rescue_offset = rescue_modifier_keyword_offset(node, self.source)
                .unwrap_or_else(|| node.location().start_offset());
            let (line, column) = self.source.offset_to_line_col(rescue_offset);
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Useless `rescue` detected.".to_string(),
            );

            let bytes = self.source.as_bytes();
            let mut start = rescue_offset;
            if start > 0 && bytes[start - 1].is_ascii_whitespace() && bytes[start - 1] != b'\n' {
                start -= 1;
            }
            self.corrections.push(crate::correction::Correction {
                start,
                end: node.location().end_offset(),
                replacement: String::new(),
                cop_name: self.cop.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
            self.diagnostics.push(diagnostic);
        }

        ruby_prism::visit_rescue_modifier_node(self, node);
    }
}

impl RescueVisitor<'_, '_> {
    fn exception_var_used_in_ensure(&self, rescue_node: &ruby_prism::RescueNode<'_>) -> bool {
        if self.ensure_var_names.is_empty() {
            return false;
        }

        if let Some(reference) = rescue_node.reference() {
            if let Some(local_var) = reference.as_local_variable_target_node() {
                let var_name = local_var.name().as_slice();
                return self.ensure_var_names.iter().any(|n| n == var_name);
            }
        }

        false
    }
}

/// Collect all local variable read names from an ensure clause's body.
fn collect_ensure_lvar_names(ensure_clause: ruby_prism::EnsureNode<'_>, names: &mut Vec<Vec<u8>>) {
    struct LvarCollector<'a> {
        names: &'a mut Vec<Vec<u8>>,
    }
    impl<'pr> Visit<'pr> for LvarCollector<'_> {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            self.names.push(node.name().as_slice().to_vec());
        }
    }
    let mut collector = LvarCollector { names };
    if let Some(statements) = ensure_clause.statements() {
        for stmt in statements.body().iter() {
            collector.visit(&stmt);
        }
    }
}

fn only_reraising(rescue_node: &ruby_prism::RescueNode<'_>, source: &SourceFile) -> bool {
    let statements = match rescue_node.statements() {
        Some(s) => s,
        None => return false,
    };

    let body: Vec<_> = statements.body().iter().collect();
    if body.len() != 1 {
        return false;
    }

    let stmt = &body[0];

    // Check if it's a `raise` call
    let call = match stmt.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != b"raise" {
        return false;
    }

    // Must not have a receiver
    if call.receiver().is_some() {
        return false;
    }

    // `raise` with no args => re-raises current exception
    let args = match call.arguments() {
        Some(a) => a,
        None => return true, // bare `raise`
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return false;
    }

    let first_arg = &arg_list[0];
    let arg_src = source.byte_slice(
        first_arg.location().start_offset(),
        first_arg.location().end_offset(),
        "",
    );

    // Check if it's re-raising the same exception variable
    if arg_src == "$!" || arg_src == "$ERROR_INFO" {
        return true;
    }

    // Check if it matches the rescue variable name
    if let Some(reference) = rescue_node.reference() {
        if let Some(local_var) = reference.as_local_variable_target_node() {
            let var_name = std::str::from_utf8(local_var.name().as_slice()).unwrap_or("");
            if !var_name.is_empty() && arg_src == var_name {
                return true;
            }
        }
    }

    false
}

fn modifier_only_reraising(
    rescue_mod: &ruby_prism::RescueModifierNode<'_>,
    source: &SourceFile,
) -> bool {
    is_reraise_expression(&rescue_mod.rescue_expression(), source, None)
}

fn is_reraise_expression(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    rescue_var_name: Option<&[u8]>,
) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != b"raise" || call.receiver().is_some() {
        return false;
    }

    let args = match call.arguments() {
        Some(a) => a,
        None => return true,
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return false;
    }

    let first_arg = &arg_list[0];
    let arg_src = source.byte_slice(
        first_arg.location().start_offset(),
        first_arg.location().end_offset(),
        "",
    );

    if arg_src == "$!" || arg_src == "$ERROR_INFO" {
        return true;
    }

    if let Some(var_name) = rescue_var_name {
        if !var_name.is_empty() && arg_src.as_bytes() == var_name {
            return true;
        }
    }

    false
}

fn rescue_modifier_keyword_offset(
    rescue_mod: &ruby_prism::RescueModifierNode<'_>,
    source: &SourceFile,
) -> Option<usize> {
    let expr_end = rescue_mod.expression().location().end_offset();
    let mod_end = rescue_mod.location().end_offset();
    let slice = source.try_byte_slice(expr_end, mod_end)?;
    let rel = slice.as_bytes().windows(6).position(|w| w == b"rescue")?;
    Some(expr_end + rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessRescue, "cops/lint/useless_rescue");

    #[test]
    fn supports_autocorrect() {
        assert!(UselessRescue.supports_autocorrect());
    }

    #[test]
    fn autocorrect_removes_useless_rescue_clause() {
        crate::testutil::assert_cop_autocorrect(
            &UselessRescue,
            b"def foo\n  do_something\nrescue\n  raise\nend\n",
            b"def foo\n  do_something\nend\n",
        );
    }

    #[test]
    fn autocorrect_removes_modifier_rescue_reraise_segment() {
        crate::testutil::assert_cop_autocorrect(
            &UselessRescue,
            b"raise 'TEST_ME' rescue raise rescue nil\n",
            b"raise 'TEST_ME' rescue nil\n",
        );
    }
}
