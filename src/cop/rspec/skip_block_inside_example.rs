use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SkipBlockInsideExample;

/// Flags `skip 'reason' do ... end` inside an example.
/// `skip` should not be passed a block.
impl Cop for SkipBlockInsideExample {
    fn name(&self) -> &'static str {
        "RSpec/SkipBlockInsideExample"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() || !is_rspec_example(call.name().as_slice()) {
            return;
        }

        let block_node = match call.block().and_then(|b| b.as_block_node()) {
            Some(b) => b,
            None => return,
        };

        let Some(body) = block_node.body() else {
            return;
        };

        let mut hits = Vec::new();
        find_skip_with_block_recursive(&body, &mut hits);

        for hit in hits {
            let (line, column) = source.offset_to_line_col(hit.start);
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Don't pass a block to `skip` inside examples.".to_string(),
            );

            if let Some(corrections) = corrections.as_deref_mut() {
                corrections.push(crate::correction::Correction {
                    start: hit.start,
                    end: hit.end,
                    replacement: "skip('TODO: reason')".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

struct SkipBlockHit {
    start: usize,
    end: usize,
}

/// Recursively search inside a node for `skip` calls with a block.
fn find_skip_with_block_recursive(node: &ruby_prism::Node<'_>, hits: &mut Vec<SkipBlockHit>) {
    if let Some(call) = node.as_call_node()
        && call.name().as_slice() == b"skip"
        && call.receiver().is_none()
        && call.block().is_some()
    {
        let loc = call.location();
        hits.push(SkipBlockHit {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        return; // Don't recurse into the skip block itself
    }

    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_skip_with_block_recursive(&child, hits);
        }
        return;
    }
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            find_skip_with_block_recursive(&recv, hits);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_skip_with_block_recursive(&arg, hits);
            }
        }
        if let Some(block) = call.block() {
            find_skip_with_block_recursive(&block, hits);
        }
        return;
    }
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            find_skip_with_block_recursive(&body, hits);
        }
        return;
    }
    if let Some(begin) = node.as_begin_node() {
        if let Some(stmts) = begin.statements() {
            find_skip_with_block_recursive(&stmts.as_node(), hits);
        }
        return;
    }
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            find_skip_with_block_recursive(&stmts.as_node(), hits);
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_skip_with_block_recursive(&subsequent, hits);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            find_skip_with_block_recursive(&stmts.as_node(), hits);
        }
        if let Some(else_clause) = unless_node.else_clause()
            && let Some(stmts) = else_clause.statements()
        {
            find_skip_with_block_recursive(&stmts.as_node(), hits);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SkipBlockInsideExample,
        "cops/rspec/skip_block_inside_example"
    );
    crate::cop_autocorrect_fixture_tests!(
        SkipBlockInsideExample,
        "cops/rspec/skip_block_inside_example"
    );
}
