// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::URI)
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UriRegexp;

impl Cop for UriRegexp {
    fn name(&self) -> &'static str {
        "Lint/UriRegexp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"regexp" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_name = match constant_name(&receiver) {
            Some(n) => n,
            None => return,
        };

        if recv_name != b"URI" {
            return;
        }

        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "`URI.regexp` is obsolete and should not be used.".to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let receiver_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("URI");
            let first_arg = call
                .arguments()
                .and_then(|a| a.arguments().iter().next())
                .and_then(|arg| std::str::from_utf8(arg.location().as_slice()).ok());
            let replacement = if let Some(arg) = first_arg {
                format!("{receiver_src}::DEFAULT_PARSER.make_regexp({arg})")
            } else {
                format!("{receiver_src}::DEFAULT_PARSER.make_regexp")
            };
            let loc = call.location();
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UriRegexp, "cops/lint/uri_regexp");
    crate::cop_autocorrect_fixture_tests!(UriRegexp, "cops/lint/uri_regexp");
}
