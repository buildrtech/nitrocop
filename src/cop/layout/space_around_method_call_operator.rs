use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceAroundMethodCallOperator;

const MESSAGE: &str = "Avoid using spaces around a method call operator.";

impl Cop for SpaceAroundMethodCallOperator {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundMethodCallOperator"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        // Handle CallNode (method calls with . or &.)
        if let Some(call) = node.as_call_node() {
            if let Some(dot_loc) = call.call_operator_loc() {
                let dot_slice = dot_loc.as_slice();
                // Only check . and &. operators
                if dot_slice == b"." || dot_slice == b"&." {
                    // Check space before dot (between receiver end and dot start)
                    if let Some(receiver) = call.receiver() {
                        let recv_end = receiver.location().end_offset();
                        let dot_start = dot_loc.start_offset();
                        if dot_start > recv_end {
                            let bytes = &source.as_bytes()[recv_end..dot_start];
                            if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                                // Space before dot on the same line
                                let (recv_end_line, _) = source.offset_to_line_col(recv_end);
                                let (dot_start_line, _) = source.offset_to_line_col(dot_start);
                                if recv_end_line == dot_start_line {
                                    push_whitespace_gap_offense(
                                        self,
                                        source,
                                        recv_end,
                                        dot_start,
                                        diagnostics,
                                        &mut corrections,
                                    );
                                }
                            }
                        }
                    }

                    // Check space after dot (between dot end and method start)
                    if let Some(msg_loc) = call.message_loc() {
                        let dot_end = dot_loc.end_offset();
                        let msg_start = msg_loc.start_offset();
                        if msg_start > dot_end {
                            let bytes = &source.as_bytes()[dot_end..msg_start];
                            if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                                let (dot_end_line, _) = source.offset_to_line_col(dot_end);
                                let (msg_start_line, _) = source.offset_to_line_col(msg_start);
                                if dot_end_line == msg_start_line {
                                    push_whitespace_gap_offense(
                                        self,
                                        source,
                                        dot_end,
                                        msg_start,
                                        diagnostics,
                                        &mut corrections,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Handle ConstantPathNode (:: operator)
        if let Some(cp) = node.as_constant_path_node() {
            // Only check when there's a name (e.g., `Foo::Bar`, not bare `::`)
            if cp.name().is_some() {
                let delim_loc = cp.delimiter_loc();
                let delim_end = delim_loc.end_offset();
                let name_loc = cp.name_loc();
                let name_start = name_loc.start_offset();
                if name_start > delim_end {
                    let bytes = &source.as_bytes()[delim_end..name_start];
                    if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                        let (delim_line, _) = source.offset_to_line_col(delim_end);
                        let (name_line, _) = source.offset_to_line_col(name_start);
                        if delim_line == name_line {
                            push_whitespace_gap_offense(
                                self,
                                source,
                                delim_end,
                                name_start,
                                diagnostics,
                                &mut corrections,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn push_whitespace_gap_offense(
    cop: &dyn Cop,
    source: &SourceFile,
    gap_start: usize,
    gap_end: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(gap_start);
    let mut diagnostic = cop.diagnostic(source, line, col, MESSAGE.to_string());
    if let Some(corrections) = corrections.as_mut() {
        corrections.push(Correction {
            start: gap_start,
            end: gap_end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diagnostic.corrected = true;
    }
    diagnostics.push(diagnostic);
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceAroundMethodCallOperator,
        "cops/layout/space_around_method_call_operator"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceAroundMethodCallOperator,
        "cops/layout/space_around_method_call_operator"
    );

    #[test]
    fn autocorrect_constant_path_spacing() {
        let source = b"RuboCop:: Cop\n";
        let (_diagnostics, corrections) =
            crate::testutil::run_cop_autocorrect(&SpaceAroundMethodCallOperator, source);
        let corrected = crate::correction::CorrectionSet::from_vec(corrections).apply(source);
        assert_eq!(corrected, b"RuboCop::Cop\n");
    }
}
