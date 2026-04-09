use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/Caller — flags `caller.first`, `caller[n]`, `caller_locations.first`,
/// `caller_locations[n]` and suggests `caller(n..n).first` instead.
///
/// ## Investigation (2026-03-20)
/// FP=5 in extended corpus from two patterns:
/// 1. `caller_locations&.first` — safe navigation `&.` not matched by RuboCop (uses `send` not `csend`)
/// 2. `caller.first(n)` / `caller_locations.first(n)` — `first` with arguments returns an array,
///    not a single element. RuboCop's pattern `(send #slow_caller? :first)` only matches
///    zero-argument `first`.
///
/// Fixed by checking `call_operator_loc()` for `&.` and rejecting `first` with arguments.
pub struct Caller;

fn parse_int_literal(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<i64> {
    let int_node = node.as_integer_node()?;
    let loc = int_node.location();
    let mut text = source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .replace('_', "");

    // Conservative parser for fixture/real-world decimal ints; skip uncommon bases.
    if text.starts_with("0x") || text.starts_with("0X") || text.starts_with("0o") {
        return None;
    }

    if let Some(stripped) = text.strip_prefix('+') {
        text = stripped.to_string();
    }

    text.parse::<i64>().ok()
}

impl Cop for Caller {
    fn name(&self) -> &'static str {
        "Performance/Caller"
    }

    fn uses_node_check(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Pattern: caller.first, caller[n], caller_locations.first, caller_locations[n]
        // Also: caller(n).first, caller(n)[n]
        if let Some(chain) = as_method_chain(node) {
            let is_caller = chain.inner_method == b"caller";
            let is_caller_locations = chain.inner_method == b"caller_locations";
            if (is_caller || is_caller_locations) && chain.inner_call.receiver().is_none() {
                // inner call must have 0 or 1 integer arguments:
                //   caller.first / caller[n]       — 0 args, flagged
                //   caller(1).first / caller(2)[1] — 1 integer arg, flagged
                //   caller(1, 1).first             — 2 args, skip
                //   caller(1..1).first             — 1 range arg (already correct form), skip
                let inner_args = chain.inner_call.arguments();
                let inner_arg_count = inner_args.as_ref().map_or(0, |a| a.arguments().len());
                if inner_arg_count > 1 {
                    return;
                }

                let mut n: i64 = 1;
                if inner_arg_count == 1 {
                    let arg = inner_args.unwrap().arguments().iter().next().unwrap();
                    if arg.as_integer_node().is_none() {
                        return;
                    }
                    n = match parse_int_literal(source, &arg) {
                        Some(v) => v,
                        None => return,
                    };
                }

                let outer_call = node.as_call_node().unwrap();

                // Skip safe navigation: caller_locations&.first is not flagged by RuboCop
                if outer_call
                    .call_operator_loc()
                    .is_some_and(|loc| loc.as_slice() == b"&.")
                {
                    return;
                }

                let is_first = chain.outer_method == b"first";
                let is_bracket = chain.outer_method == b"[]";

                if is_first {
                    // caller.first — flagged, but caller.first(n) returns an array — skip
                    let has_args = outer_call
                        .arguments()
                        .is_some_and(|args| !args.arguments().is_empty());
                    if has_args {
                        return;
                    }
                } else if is_bracket {
                    // caller[n] — only flag when the argument is a single integer
                    // caller[0..10], caller[2..-1], caller[2, 10] should NOT be flagged
                    let args = match outer_call.arguments() {
                        Some(a) => a,
                        None => return,
                    };
                    if args.arguments().len() != 1 {
                        return;
                    }
                    let idx = args.arguments().iter().next().unwrap();
                    if idx.as_integer_node().is_none() {
                        return;
                    }
                    let m = match parse_int_literal(source, &idx) {
                        Some(v) => v,
                        None => return,
                    };
                    n += m;
                } else {
                    return;
                }

                let method_name = if is_caller {
                    "caller"
                } else {
                    "caller_locations"
                };
                let preferred_method = format!("{method_name}({n}..{n}).first");

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use `{method_name}(n..n).first` instead of `{method_name}.first` or `{method_name}[n]`."
                    ),
                );

                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset(),
                        end: loc.end_offset(),
                        replacement: preferred_method,
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Caller, "cops/performance/caller");
    crate::cop_autocorrect_fixture_tests!(Caller, "cops/performance/caller");

    #[test]
    fn supports_autocorrect() {
        assert!(Caller.supports_autocorrect());
    }
}
