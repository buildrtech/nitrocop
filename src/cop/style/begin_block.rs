use crate::cop::node_type::PRE_EXECUTION_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BeginBlock;

impl Cop for BeginBlock {
    fn name(&self) -> &'static str {
        "Style/BeginBlock"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PRE_EXECUTION_NODE]
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
        let mut corrections = corrections;
        let pre_exe = match node.as_pre_execution_node() {
            Some(n) => n,
            None => return,
        };

        let kw_loc = pre_exe.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of `BEGIN` blocks.".to_string(),
        );
        if let Some(corrections) = corrections.as_deref_mut() {
            let loc = pre_exe.location();
            corrections.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement: String::new(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeginBlock, "cops/style/begin_block");

    #[test]
    fn autocorrect_removes_begin_block() {
        crate::testutil::assert_cop_autocorrect(
            &BeginBlock,
            b"BEGIN { puts 'boot' }\nputs 'run'\n",
            b"\nputs 'run'\n",
        );
    }
}
