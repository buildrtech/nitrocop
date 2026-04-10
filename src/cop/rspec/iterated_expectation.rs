use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03):
/// FP1: `.each(as: :array)` — RuboCop's `(send _ :each)` pattern only matches `.each` without
/// arguments. Fixed by checking `call.arguments().is_none()`.
/// FP2: `.to receive(:x) do |msg| ... end` — when `.to` has a block, the AST is
/// `(block (send ...) ...)` not `(send ...)`, so RuboCop's pattern doesn't match.
/// Fixed by checking `call.block().is_none()` in `is_expectation_with_param`.
pub struct IteratedExpectation;

impl Cop for IteratedExpectation {
    fn name(&self) -> &'static str {
        "RSpec/IteratedExpectation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
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
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Flag `.each { |x| expect(x)... }` — suggest using `all` matcher
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"each" {
            return;
        }

        // Must have a receiver (the array/collection)
        if call.receiver().is_none() {
            return;
        }

        // RuboCop pattern is `(send _ :each)` — no arguments on .each
        if call.arguments().is_some() {
            return;
        }

        // Must have a block with a parameter
        let block_raw = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have block parameters
        let params = match block.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(p) => p,
            None => return,
        };

        let inner_params = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        let requireds: Vec<_> = inner_params.requireds().iter().collect();
        // RuboCop pattern requires exactly one block parameter: (args (arg $_))
        if requireds.len() != 1 {
            return;
        }

        // Check if the parameter starts with _ (unused)
        if let Some(first_param) = requireds.first() {
            if let Some(req) = first_param.as_required_parameter_node() {
                if req.name().as_slice().starts_with(b"_") {
                    return;
                }
            }
        }

        // Check if the block body is expect(block_param).to ...
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        // Get block parameter name
        let param_name = if let Some(first_param) = requireds.first() {
            if let Some(req) = first_param.as_required_parameter_node() {
                req.name().as_slice().to_vec()
            } else {
                return;
            }
        } else {
            return;
        };

        // RuboCop requires ALL statements in the block body to be
        // expect(block_param).to ... where block_param is a bare lvar.
        let single_to_call = if is_expectation_with_param(&body, &param_name) {
            body.as_call_node()
        } else if let Some(stmts) = body.as_statements_node() {
            let children: Vec<_> = stmts.body().iter().collect();
            if children.len() == 1 && is_expectation_with_param(&children[0], &param_name) {
                children[0].as_call_node()
            } else {
                None
            }
        } else {
            None
        };

        if single_to_call.is_some() || is_all_expectations_with_param(&body, &param_name) {
            let recv = call.receiver().unwrap();
            let loc = recv.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Prefer using the `all` matcher instead of iterating over an array.".to_string(),
            );

            if let (Some(to_call), Some(corrections), Some(block_raw)) =
                (single_to_call, corrections, call.block())
                && let Some(matcher) = to_call
                    .arguments()
                    .and_then(|a| a.arguments().iter().next())
                && !matcher_uses_param(&matcher, &param_name)
                && let Some(collection_text) = source
                    .try_byte_slice(recv.location().start_offset(), recv.location().end_offset())
                && let Some(matcher_text) = source.try_byte_slice(
                    matcher.location().start_offset(),
                    matcher.location().end_offset(),
                )
                && let Some(_block_node) = block_raw.as_block_node()
            {
                let call_loc = call.location();
                corrections.push(crate::correction::Correction {
                    start: call_loc.start_offset(),
                    end: call_loc.end_offset(),
                    replacement: format!("expect({collection_text}).to all({matcher_text})"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

/// Check if a node is `expect(param).to ...` where param is a bare local variable.
fn is_expectation_with_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // RuboCop's pattern only matches `.to`, NOT `.not_to` or `.to_not`:
    //   (send (send nil? :expect (lvar %)) :to ...)
    let method = call.name().as_slice();
    if method != b"to" {
        return false;
    }

    // A `.to` call with a block (e.g. `expect(x).to receive(:y) do |msg| ... end`)
    // changes the AST shape from `(send ...)` to `(block (send ...) ...)`, so
    // RuboCop's `(send ...)` pattern doesn't match it.
    if call.block().is_some() {
        return false;
    }

    // The receiver should be `expect(param)`
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    let expect_call = match recv.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if expect_call.receiver().is_some() || expect_call.name().as_slice() != b"expect" {
        return false;
    }

    // The argument to expect should be a bare local variable matching the block param
    let args = match expect_call.arguments() {
        Some(a) => a,
        None => return false,
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return false;
    }

    if let Some(lvar) = arg_list[0].as_local_variable_read_node() {
        lvar.name().as_slice() == param_name
    } else {
        false
    }
}

fn matcher_uses_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    use ruby_prism::Visit;

    struct ParamUseVisitor<'a> {
        target: &'a [u8],
        found: bool,
    }

    impl<'pr> Visit<'pr> for ParamUseVisitor<'_> {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            if node.name().as_slice() == self.target {
                self.found = true;
            }
        }
    }

    let mut visitor = ParamUseVisitor {
        target: param_name,
        found: false,
    };
    visitor.visit(node);
    visitor.found
}

/// Check if all statements in a begin/statements node are expectations with the param.
fn is_all_expectations_with_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    if let Some(stmts) = node.as_statements_node() {
        let children: Vec<_> = stmts.body().iter().collect();
        if children.is_empty() {
            return false;
        }
        children
            .iter()
            .all(|child| is_expectation_with_param(child, param_name))
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IteratedExpectation, "cops/rspec/iterated_expectation");

    #[test]
    fn supports_autocorrect() {
        assert!(IteratedExpectation.supports_autocorrect());
    }

    #[test]
    fn autocorrects_simple_each_expectation_to_all_matcher() {
        crate::testutil::assert_cop_autocorrect(
            &IteratedExpectation,
            b"users.each { |user| expect(user).to be_valid }\n",
            b"expect(users).to all(be_valid)\n",
        );
    }

    #[test]
    fn does_not_autocorrect_when_matcher_uses_block_param() {
        let input = b"users.each { |user| expect(user).to have_attributes(id: user.id) }\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&IteratedExpectation, input);
        assert_eq!(diags.len(), 1);
        assert!(!diags[0].corrected);
        assert!(corrections.is_empty());
    }
}
