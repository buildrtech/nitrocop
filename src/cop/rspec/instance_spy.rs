use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

pub struct InstanceSpy;

impl Cop for InstanceSpy {
    fn name(&self) -> &'static str {
        "RSpec/InstanceSpy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = InstanceSpyVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InstanceSpyVisitor<'a> {
    cop: &'a InstanceSpy,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Option<&'a mut Vec<crate::correction::Correction>>,
}

impl<'pr> Visit<'pr> for InstanceSpyVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // RuboCop runs this cop inside RSpec examples only.
        if node.receiver().is_none() && is_rspec_example(node.name().as_slice()) {
            if let Some(block) = node.block().and_then(|b| b.as_block_node()) {
                self.check_example(block);
                return;
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

impl InstanceSpyVisitor<'_> {
    fn check_example(&mut self, block: ruby_prism::BlockNode<'_>) {
        let mut collector = ExampleCollector::default();
        if let Some(body) = block.body() {
            collector.visit(&body);
        }

        for assignment in collector.assignments {
            if !collector.have_received_vars.contains(&assignment.var_name) {
                continue;
            }

            let (line, column) = self
                .source
                .offset_to_line_col(assignment.instance_double_selector_start);
            let mut diagnostic = self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `instance_spy` when you check your double with `have_received`.".to_string(),
            );

            if let Some(ref mut corr) = self.corrections {
                corr.push(crate::correction::Correction {
                    start: assignment.instance_double_selector_start,
                    end: assignment.instance_double_selector_end,
                    replacement: "instance_spy".to_string(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                corr.push(crate::correction::Correction {
                    start: assignment.as_null_object_dot_start,
                    end: assignment.as_null_object_selector_end,
                    replacement: String::new(),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            self.diagnostics.push(diagnostic);
        }
    }
}

#[derive(Default)]
struct ExampleCollector {
    assignments: Vec<NullDoubleAssignment>,
    have_received_vars: HashSet<Vec<u8>>,
}

struct NullDoubleAssignment {
    var_name: Vec<u8>,
    instance_double_selector_start: usize,
    instance_double_selector_end: usize,
    as_null_object_dot_start: usize,
    as_null_object_selector_end: usize,
}

impl<'pr> Visit<'pr> for ExampleCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if let Some(calls) = instance_double_call_from_null_double_value(&node.value())
            && let Some(instance_double_selector) = calls.instance_double_call.message_loc()
            && let Some(as_null_object_dot) = calls.as_null_object_call.call_operator_loc()
            && let Some(as_null_object_selector) = calls.as_null_object_call.message_loc()
        {
            self.assignments.push(NullDoubleAssignment {
                var_name: node.name().as_slice().to_vec(),
                instance_double_selector_start: instance_double_selector.start_offset(),
                instance_double_selector_end: instance_double_selector.end_offset(),
                as_null_object_dot_start: as_null_object_dot.start_offset(),
                as_null_object_selector_end: as_null_object_selector.end_offset(),
            });
        }

        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(var_name) = have_received_expected_var(node) {
            self.have_received_vars.insert(var_name);
        }

        ruby_prism::visit_call_node(self, node);
    }
}

struct NullDoubleCalls<'pr> {
    instance_double_call: ruby_prism::CallNode<'pr>,
    as_null_object_call: ruby_prism::CallNode<'pr>,
}

fn instance_double_call_from_null_double_value<'pr>(
    value: &ruby_prism::Node<'pr>,
) -> Option<NullDoubleCalls<'pr>> {
    let as_null_object = value.as_call_node()?;
    if as_null_object.name().as_slice() != b"as_null_object" {
        return None;
    }

    let recv_call = as_null_object.receiver()?.as_call_node()?;
    if recv_call.receiver().is_some() || recv_call.name().as_slice() != b"instance_double" {
        return None;
    }

    Some(NullDoubleCalls {
        instance_double_call: recv_call,
        as_null_object_call: as_null_object,
    })
}

fn have_received_expected_var(node: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    // (send (send nil? :expect (lvar $_)) :to (send nil? :have_received ...))
    if node.name().as_slice() != b"to" {
        return None;
    }

    let expect_call = node.receiver()?.as_call_node()?;
    if expect_call.receiver().is_some() || expect_call.name().as_slice() != b"expect" {
        return None;
    }

    let expect_args = expect_call.arguments()?;
    let expected_var = expect_args
        .arguments()
        .iter()
        .next()?
        .as_local_variable_read_node()?
        .name()
        .as_slice()
        .to_vec();

    let matcher = node
        .arguments()?
        .arguments()
        .iter()
        .next()?
        .as_call_node()?;
    if matcher.receiver().is_some() || matcher.name().as_slice() != b"have_received" {
        return None;
    }

    Some(expected_var)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InstanceSpy, "cops/rspec/instance_spy");

    #[test]
    fn supports_autocorrect() {
        assert!(InstanceSpy.supports_autocorrect());
    }

    #[test]
    fn autocorrects_instance_double_null_object_to_instance_spy() {
        crate::testutil::assert_cop_autocorrect(
            &InstanceSpy,
            b"it do\n  foo = instance_double(Foo).as_null_object\n  expect(foo).to have_received(:bar)\nend\n",
            b"it do\n  foo = instance_spy(Foo)\n  expect(foo).to have_received(:bar)\nend\n",
        );
    }
}
