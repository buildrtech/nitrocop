use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/YodaExpression: Forbids Yoda expressions where a constant/numeric
/// value appears on the LHS of a commutative operator.
///
/// Corpus investigation: FP=4 FN=8.
///
/// FP root cause: Missing `offended_ancestor?` check. When a Yoda expression
/// like `3.0 * method_call(x)` is nested inside another Yoda expression like
/// `5.0 + (3.0 * method_call(x))`, RuboCop only flags the outermost one.
/// nitrocop was flagging both.
///
/// FN root causes:
/// 1. Multiple arguments rejected (7 FNs): Calls like `Sequel.|([:visible], name: locs)`
///    or `Sequel.&(*predicates, cond)` have multiple arguments. RuboCop checks
///    only `first_argument` for `constant_portion?`, ignoring extra args. nitrocop
///    was rejecting any call with `arg_list.len() != 1`.
/// 2. Unary minus on numeric (1 FN): `- 1 + offset` — Prism represents `- 1`
///    (with space) as `CallNode(name: "-@", receiver: IntegerNode)`, not as a
///    negative integer literal. RuboCop's Parser gem folds this into a numeric
///    node. nitrocop's `is_constant_portion` didn't recognize unary minus/plus
///    on numeric literals as constant.
///
/// Fix: (1) Switch from `check_node` to `check_source` with a custom visitor
/// that tracks offended node byte ranges, suppressing nested Yoda expressions.
/// (2) Check only the first argument instead of requiring exactly one argument.
/// (3) Recognize unary `-@`/`+@` on numeric literals as constant portions.
pub struct YodaExpression;

impl Cop for YodaExpression {
    fn name(&self) -> &'static str {
        "Style/YodaExpression"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let supported_operators = config.get_string_array("SupportedOperators");
        let mut visitor = YodaVisitor {
            cop: self,
            source,
            supported_operators,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            autocorrect: corrections.is_some(),
            offended_ranges: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
        if let Some(corr) = corrections.as_mut() {
            corr.extend(visitor.corrections);
        }
    }
}

struct YodaVisitor<'a> {
    cop: &'a YodaExpression,
    source: &'a SourceFile,
    supported_operators: Option<Vec<String>>,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    autocorrect: bool,
    /// Byte ranges (start..end) of nodes already flagged as Yoda expressions.
    /// Used to suppress nested Yoda expressions (offended_ancestor? equivalent).
    offended_ranges: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for YodaVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        let name_str = match std::str::from_utf8(name) {
            Ok(s) => s,
            Err(_) => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        // Check if operator is in supported list (default: *, +, &, |, ^)
        let is_supported = if let Some(ref ops) = self.supported_operators {
            ops.iter().any(|op| op == name_str)
        } else {
            matches!(name, b"*" | b"+" | b"&" | b"|" | b"^")
        };

        if is_supported {
            if let (Some(receiver), Some(args)) = (node.receiver(), node.arguments()) {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    let lhs_constant = is_constant_portion(&receiver);
                    let rhs_constant = is_constant_portion(&arg_list[0]);

                    if lhs_constant && !rhs_constant {
                        let loc = node.location();
                        let start = loc.start_offset();
                        let end = loc.end_offset();

                        // Check if any ancestor was already flagged (offended_ancestor?)
                        let has_offended_ancestor = self
                            .offended_ranges
                            .iter()
                            .any(|&(a_start, a_end)| a_start <= start && end <= a_end);

                        if !has_offended_ancestor {
                            let (line, column) = self.source.offset_to_line_col(start);
                            let mut diag = self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Prefer placing the expression on the left side of the operator."
                                    .to_string(),
                            );

                            if self.autocorrect
                                && arg_list.len() == 1
                                && node.block().is_none()
                                && arg_list[0].as_splat_node().is_none()
                                && arg_list[0].as_block_argument_node().is_none()
                            {
                                let lhs_loc = receiver.location();
                                let rhs_loc = arg_list[0].location();
                                let lhs = String::from_utf8_lossy(
                                    &self.source.as_bytes()[lhs_loc.start_offset()..lhs_loc.end_offset()],
                                );
                                let rhs = String::from_utf8_lossy(
                                    &self.source.as_bytes()[rhs_loc.start_offset()..rhs_loc.end_offset()],
                                );
                                self.corrections.push(crate::correction::Correction {
                                    start,
                                    end,
                                    replacement: format!("{rhs} {name_str} {lhs}"),
                                    cop_name: self.cop.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }

                            self.diagnostics.push(diag);
                            self.offended_ranges.push((start, end));
                        }
                    }
                }
            }
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

fn is_constant_portion(node: &ruby_prism::Node<'_>) -> bool {
    // Match RuboCop's constant_portion? which checks :numeric and :const
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
    {
        return true;
    }

    // Handle unary -@ / +@ on numeric literals (e.g., `- 1` with space)
    // Prism represents this as CallNode(name: "-@", receiver: IntegerNode)
    // while Parser gem folds it into a single numeric node.
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"-@" || name == b"+@") && call.arguments().is_none() {
            if let Some(receiver) = call.receiver() {
                return receiver.as_integer_node().is_some()
                    || receiver.as_float_node().is_some()
                    || receiver.as_rational_node().is_some()
                    || receiver.as_imaginary_node().is_some();
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(YodaExpression, "cops/style/yoda_expression");
    crate::cop_autocorrect_fixture_tests!(YodaExpression, "cops/style/yoda_expression");
}
