use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for ambiguous operators in the first argument of a method invocation
/// without parentheses. For example, `do_something *some_array` where `*` could
/// be interpreted as either a splat or multiplication.
///
/// ## Implementation
///
/// Uses Prism parser warnings to detect ambiguous operators, matching RuboCop's
/// approach of relying on parser diagnostics (`:ambiguous_prefix` reason).
///
/// Prism emits these verbose-level warnings:
/// - `PM_WARN_AMBIGUOUS_PREFIX_STAR`: `*` splat vs multiplication
/// - `PM_WARN_AMBIGUOUS_PREFIX_STAR_STAR`: `**` keyword splat vs exponent
/// - `PM_WARN_AMBIGUOUS_PREFIX_AMPERSAND`: `&` block vs binary AND
/// - `PM_WARN_AMBIGUOUS_FIRST_ARGUMENT_PLUS`: `+` positive number vs addition
/// - `PM_WARN_AMBIGUOUS_FIRST_ARGUMENT_MINUS`: `-` negative number vs subtraction
///
/// ## Root cause of historical FNs (473 FNs, 50.7% match rate)
///
/// The original implementation only handled `*` (splat) via AST node inspection
/// of `CallNode`/`SplatNode`, missing `+`, `-`, `&`, and `**`. Switching to
/// Prism parser warnings covers all 5 operators in a single pass.
pub struct AmbiguousOperator;

/// Describes an ambiguous operator type.
struct AmbiguityInfo {
    actual: &'static str,
    operator: &'static str,
    possible: &'static str,
}

/// Try to classify a Prism warning message as an ambiguous operator.
fn classify_warning(message: &str) -> Option<AmbiguityInfo> {
    if message.contains("ambiguous `*`") && !message.contains("`**`") {
        Some(AmbiguityInfo {
            actual: "splat",
            operator: "*",
            possible: "a multiplication",
        })
    } else if message.contains("ambiguous `**`") {
        Some(AmbiguityInfo {
            actual: "keyword splat",
            operator: "**",
            possible: "an exponent",
        })
    } else if message.contains("ambiguous `&`") {
        Some(AmbiguityInfo {
            actual: "block",
            operator: "&",
            possible: "a binary AND",
        })
    } else if message.contains("after `+` operator") {
        Some(AmbiguityInfo {
            actual: "positive number",
            operator: "+",
            possible: "an addition",
        })
    } else if message.contains("after `-` operator") {
        Some(AmbiguityInfo {
            actual: "negative number",
            operator: "-",
            possible: "a subtraction",
        })
    } else {
        None
    }
}

impl Cop for AmbiguousOperator {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousOperator"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let call_ranges = collect_ambiguous_argument_ranges(parse_result);

        for warning in parse_result.warnings() {
            let message = warning.message();
            let info = match classify_warning(message) {
                Some(i) => i,
                None => continue,
            };

            let loc = warning.location();
            let start = loc.start_offset();
            let (line, column) = source.offset_to_line_col(start);

            let msg = format!(
                "Ambiguous {} operator. Parenthesize the method arguments \
                 if it's surely a {} operator, or add a whitespace to the \
                 right of the `{}` if it should be {}.",
                info.actual, info.actual, info.operator, info.possible
            );

            let mut diag = self.diagnostic(source, line, column, msg);
            if let Some(corr) = corrections.as_mut() {
                if let Some(range) = call_ranges.iter().find(|r| r.warning_offset == start) {
                    corr.push(crate::correction::Correction {
                        start: range.open_replace_start,
                        end: range.open_replace_end,
                        replacement: "(".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    corr.push(crate::correction::Correction {
                        start: range.args_end,
                        end: range.args_end,
                        replacement: ")".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
            }

            diagnostics.push(diag);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AmbiguousArgumentRange {
    warning_offset: usize,
    open_replace_start: usize,
    open_replace_end: usize,
    args_end: usize,
}

struct AmbiguousArgumentRangeVisitor {
    ranges: Vec<AmbiguousArgumentRange>,
}

impl AmbiguousArgumentRangeVisitor {
    fn new() -> Self {
        Self { ranges: Vec::new() }
    }
}

impl<'pr> Visit<'pr> for AmbiguousArgumentRangeVisitor {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.opening_loc().is_none() {
            let mut warning_offset = None;
            let mut args_end = None;

            if let Some(args) = node.arguments() {
                let mut iter = args.arguments().iter();
                if let Some(first) = iter.next() {
                    warning_offset = Some(first.location().start_offset());
                    let mut end = first.location().end_offset();
                    for arg in iter {
                        end = arg.location().end_offset();
                    }
                    args_end = Some(end);
                }
            }

            if warning_offset.is_none() {
                if let Some(block_arg) = node.block().and_then(|b| b.as_block_argument_node()) {
                    warning_offset = Some(block_arg.location().start_offset());
                    args_end = Some(block_arg.location().end_offset());
                }
            }

            if let (Some(warning_offset), Some(args_end), Some(message_loc)) =
                (warning_offset, args_end, node.message_loc())
            {
                self.ranges.push(AmbiguousArgumentRange {
                    warning_offset,
                    open_replace_start: message_loc.end_offset(),
                    open_replace_end: warning_offset,
                    args_end,
                });
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

fn collect_ambiguous_argument_ranges(
    parse_result: &ruby_prism::ParseResult<'_>,
) -> Vec<AmbiguousArgumentRange> {
    let mut visitor = AmbiguousArgumentRangeVisitor::new();
    visitor.visit(&parse_result.node());
    visitor.ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousOperator, "cops/lint/ambiguous_operator");
    crate::cop_autocorrect_fixture_tests!(AmbiguousOperator, "cops/lint/ambiguous_operator");
}
