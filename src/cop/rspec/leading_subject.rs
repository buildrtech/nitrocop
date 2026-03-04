use crate::cop::node_type::PROGRAM_NODE;
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook,
    is_rspec_let, is_rspec_subject,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/LeadingSubject checks that `subject` is declared before `let`, hooks,
/// examples, and other declarations within an example group.
///
/// RuboCop uses `InsideExampleGroup` to determine whether a `subject` node
/// should be checked. This check walks up to the file's root-level node and
/// verifies it is a spec group (describe/context/shared_examples block). When
/// the describe block is wrapped in a `module` or `class` declaration, the
/// root-level node is the module/class — NOT a spec group — so RuboCop skips
/// the cop entirely. This is a documented side-effect of `InsideExampleGroup`.
///
/// We replicate this by only checking subjects inside spec groups that are
/// at the file's top level (direct children of the program node, or within a
/// top-level `begin`). Spec groups inside module/class wrappers are skipped.
pub struct LeadingSubject;

impl Cop for LeadingSubject {
    fn name(&self) -> &'static str {
        "RSpec/LeadingSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        // Walk top-level statements looking for spec groups.
        // Only spec groups at the file root (not inside module/class) are checked,
        // matching RuboCop's InsideExampleGroup behavior.
        for stmt in program.statements().body().iter() {
            if is_spec_group_call(&stmt) {
                self.check_example_group_recursive(source, &stmt, diagnostics);
            }
            // Skip modules, classes, requires, and anything else at the top level.
        }
    }
}

impl LeadingSubject {
    /// Recursively check an example group and all nested example groups
    /// for subject ordering violations.
    fn check_example_group_recursive(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Check subject ordering within this example group
        let mut first_relevant_name: Option<&[u8]> = None;

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();
                if c.receiver().is_some() {
                    // Recurse into RSpec.describe nested calls
                    if util::constant_name(&c.receiver().unwrap()).is_some_and(|n| n == b"RSpec")
                        && name == b"describe"
                    {
                        self.check_example_group_recursive(source, &stmt, diagnostics);
                    }
                    continue;
                }

                if is_rspec_subject(name) {
                    // Subject found -- check if something relevant came before it
                    if let Some(prev_name) = first_relevant_name {
                        let prev_str = std::str::from_utf8(prev_name).unwrap_or("let");
                        let loc = stmt.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Declare `subject` above any other `{prev_str}` declarations."),
                        ));
                    }
                } else if is_rspec_example_group(name) {
                    // Recurse into nested context/describe blocks
                    self.check_example_group_recursive(source, &stmt, diagnostics);
                    if first_relevant_name.is_none() {
                        first_relevant_name = Some(name);
                    }
                } else if (is_rspec_let(name)
                    || is_rspec_hook(name)
                    || is_rspec_example(name)
                    || is_example_include(name))
                    && first_relevant_name.is_none()
                {
                    first_relevant_name = Some(name);
                }
            }
        }
    }
}

fn is_spec_group_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && name == b"describe"
    } else {
        is_rspec_example_group(name)
    }
}

fn is_example_include(name: &[u8]) -> bool {
    name == b"include_examples"
        || name == b"it_behaves_like"
        || name == b"it_should_behave_like"
        || name == b"include_context"
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeadingSubject, "cops/rspec/leading_subject");
}
