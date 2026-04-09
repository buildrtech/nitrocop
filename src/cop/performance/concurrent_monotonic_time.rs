// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::Concurrent)
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ConcurrentMonotonicTime;

impl Cop for ConcurrentMonotonicTime {
    fn name(&self) -> &'static str {
        "Performance/ConcurrentMonotonicTime"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        if call.name().as_slice() != b"monotonic_time" {
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

        if recv_name != b"Concurrent" {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `Process.clock_gettime(Process::CLOCK_MONOTONIC)` instead of `Concurrent.monotonic_time`."
                .to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let replacement = if let Some(first_arg) = call
                .arguments()
                .and_then(|args| args.arguments().iter().next())
            {
                let arg_loc = first_arg.location();
                let arg_source =
                    source.byte_slice(arg_loc.start_offset(), arg_loc.end_offset(), "");
                format!("Process.clock_gettime(Process::CLOCK_MONOTONIC, {arg_source})")
            } else {
                "Process.clock_gettime(Process::CLOCK_MONOTONIC)".to_string()
            };

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
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
    crate::cop_fixture_tests!(
        ConcurrentMonotonicTime,
        "cops/performance/concurrent_monotonic_time"
    );
    crate::cop_autocorrect_fixture_tests!(
        ConcurrentMonotonicTime,
        "cops/performance/concurrent_monotonic_time"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(ConcurrentMonotonicTime.supports_autocorrect());
    }
}
