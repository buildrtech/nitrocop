use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TopLevelMethodDefinition;

impl Cop for TopLevelMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/TopLevelMethodDefinition"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut corrections = corrections;
        let root = parse_result.node();
        if let Some(program) = root.as_program_node() {
            let stmts = program.statements();
            for stmt in stmts.body().iter() {
                if stmt.as_def_node().is_some() {
                    let loc = stmt.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let mut diagnostic = self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not define methods at the top level.".to_string(),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TopLevelMethodDefinition,
        "cops/style/top_level_method_definition"
    );

    #[test]
    fn autocorrect_removes_top_level_method_definitions() {
        crate::testutil::assert_cop_autocorrect(
            &TopLevelMethodDefinition,
            b"def top\n  1\nend\nputs 'x'\n",
            b"\nputs 'x'\n",
        );
    }
}
