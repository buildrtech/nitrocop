use crate::cop::node_type::{
    ARRAY_NODE, ASSOC_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, HASH_NODE, IMAGINARY_NODE,
    INTEGER_NODE, KEYWORD_HASH_NODE, NIL_NODE, RANGE_NODE, RATIONAL_NODE, REGULAR_EXPRESSION_NODE,
    STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=20, FN=1.
///
/// FP=20 root cause: matcher-shape checks were too permissive. We flagged runner
/// calls where `.to`/`.not_to` had multiple args (matcher + failure message) and
/// chained matcher receivers (for example `have_matcher.with(...)`), while
/// RuboCop only matches a single matcher arg in the form `send nil? ...` or
/// `be == expected`.
///
/// FN=1 root cause: `__FILE__` was not treated as a literal expect actual value.
///
/// Fixes applied:
/// - Require exactly one runner argument.
/// - Match only RuboCop-compatible matcher forms.
/// - Treat `__FILE__` (`SourceFileNode`) as a literal.
pub struct ExpectActual;

impl Cop for ExpectActual {
    fn name(&self) -> &'static str {
        "RSpec/ExpectActual"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            ASSOC_NODE,
            CALL_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            HASH_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            KEYWORD_HASH_NODE,
            NIL_NODE,
            RANGE_NODE,
            RATIONAL_NODE,
            REGULAR_EXPRESSION_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
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
        // Look for expect(literal).to/to_not/not_to matcher(args) chains
        // RuboCop only flags when the full chain has a matcher with arguments.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        // Must be a runner method (.to, .to_not, .not_to)
        if method_name != b"to" && method_name != b"to_not" && method_name != b"not_to" {
            return;
        }

        // Receiver must be expect(literal)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let expect_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if expect_call.name().as_slice() != b"expect" || expect_call.receiver().is_some() {
            return;
        }

        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let expect_arg_list: Vec<ruby_prism::Node<'_>> = expect_args.arguments().iter().collect();
        if expect_arg_list.len() != 1 {
            return;
        }

        let literal_arg = &expect_arg_list[0];
        if !is_literal_value(source, literal_arg) {
            return;
        }

        // Check that the matcher has arguments (RuboCop requires this).
        // `expect(5).to eq(price)` → matcher `eq` has arg `price` → flagged
        // `expect(".foo").to be_present` → `be_present` has no args → NOT flagged
        let matcher_args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let matcher_list: Vec<ruby_prism::Node<'_>> = matcher_args.arguments().iter().collect();
        if matcher_list.len() != 1 {
            return;
        }

        // RuboCop only matches:
        // - (send nil? matcher expected ...)
        // - (send (send nil? :be) :== expected)
        let matcher = &matcher_list[0];
        let Some(matcher_name) = expect_actual_matcher_name(matcher) else {
            return;
        };
        // Skip route_to and be_routable matchers
        if matcher_name == b"route_to" || matcher_name == b"be_routable" {
            return;
        }

        let loc = literal_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Provide the actual value you are testing to `expect(...)`.".to_string(),
        );

        // Baseline RuboCop-aligned autocorrect for swappable matchers.
        if let Some(ref mut corr) = corrections
            && (matcher_name == b"eq"
                || matcher_name == b"eql"
                || matcher_name == b"equal"
                || matcher_name == b"be")
            && let Some(expected_node) = matcher_expected_node(matcher)
            && !is_literal_value(source, &expected_node)
        {
            let expected_loc = expected_node.location();
            let expected_src = std::str::from_utf8(
                &source.as_bytes()[expected_loc.start_offset()..expected_loc.end_offset()],
            )
            .unwrap_or("")
            .to_string();
            let actual_src = std::str::from_utf8(
                &source.as_bytes()[loc.start_offset()..loc.end_offset()],
            )
            .unwrap_or("")
            .to_string();

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: expected_src,
                cop_name: self.name(),
                cop_index: 0,
            });
            corr.push(crate::correction::Correction {
                start: expected_loc.start_offset(),
                end: expected_loc.end_offset(),
                replacement: actual_src,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

fn is_literal_value(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_regular_expression_node().is_some()
    {
        return true;
    }

    // String without interpolation
    if let Some(s) = node.as_string_node() {
        // RuboCop's Parser AST treats multiline string literals as dynamic string
        // (`dstr`), so they are not considered simple literals for this cop.
        let loc = s.location();
        let bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        if bytes.contains(&b'\n') {
            return false;
        }
        return true;
    }

    // Symbol without interpolation
    if node.as_symbol_node().is_some() {
        return true;
    }

    // Array with all literal elements
    if let Some(arr) = node.as_array_node() {
        let elements: Vec<ruby_prism::Node<'_>> = arr.elements().iter().collect();
        if elements.iter().all(|e| is_literal_value(source, e)) {
            return true;
        }
    }

    // Hash with all literal keys and values
    if let Some(hash) = node.as_hash_node() {
        let pairs: Vec<ruby_prism::Node<'_>> = hash.elements().iter().collect();
        if pairs.iter().all(|p| {
            if let Some(assoc) = p.as_assoc_node() {
                is_literal_value(source, &assoc.key()) && is_literal_value(source, &assoc.value())
            } else {
                false
            }
        }) {
            return true;
        }
    }

    // Range with literal endpoints
    if let Some(range) = node.as_range_node() {
        let left_ok =
            range.left().is_none() || range.left().is_some_and(|l| is_literal_value(source, &l));
        let right_ok =
            range.right().is_none() || range.right().is_some_and(|r| is_literal_value(source, &r));
        if left_ok && right_ok {
            return true;
        }
    }

    // Keyword hash (bare key-value pairs used in method args)
    if let Some(kh) = node.as_keyword_hash_node() {
        let elems: Vec<ruby_prism::Node<'_>> = kh.elements().iter().collect();
        if elems.iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal_value(source, &assoc.key()) && is_literal_value(source, &assoc.value())
            } else {
                false
            }
        }) {
            return true;
        }
    }

    false
}

fn matcher_expected_node<'a>(node: &'a ruby_prism::Node<'_>) -> Option<ruby_prism::Node<'a>> {
    let matcher = node.as_call_node()?;

    // Regular matcher call: eq(expected), include(expected), etc.
    if matcher.receiver().is_none() {
        if let Some(args) = matcher.arguments() {
            let mut it = args.arguments().iter();
            if let Some(first) = it.next() {
                return Some(first);
            }
        }
    }

    // Special RuboCop pattern: be == expected
    if matcher.name().as_slice() == b"==" && matcher.arguments().is_some() {
        let be_call = matcher.receiver().and_then(|r| r.as_call_node())?;
        if be_call.receiver().is_none() && be_call.name().as_slice() == b"be" {
            let args = matcher.arguments()?;
            let mut it = args.arguments().iter();
            if let Some(first) = it.next() {
                if it.next().is_none() {
                    return Some(first);
                }
            }
        }
    }

    None
}

fn expect_actual_matcher_name<'a>(node: &'a ruby_prism::Node<'_>) -> Option<&'a [u8]> {
    let matcher = node.as_call_node()?;

    // Regular matcher call: eq(expected), include(expected), etc.
    if matcher.receiver().is_none() && matcher.arguments().is_some() {
        return Some(matcher.name().as_slice());
    }

    // Special RuboCop pattern: be == expected
    if matcher.name().as_slice() == b"==" && matcher.arguments().is_some() {
        let be_call = matcher.receiver().and_then(|r| r.as_call_node())?;
        if be_call.receiver().is_none() && be_call.name().as_slice() == b"be" {
            return Some(b"be");
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectActual, "cops/rspec/expect_actual");
    crate::cop_autocorrect_fixture_tests!(ExpectActual, "cops/rspec/expect_actual");
}
