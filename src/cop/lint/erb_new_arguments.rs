use crate::cop::node_type::{CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for deprecated `ERB.new` with positional arguments beyond the first.
/// Since Ruby 2.6, non-keyword arguments other than the first one are deprecated.
pub struct ErbNewArguments;

fn build_arguments_correction(
    source: &SourceFile,
    args: &[ruby_prism::Node<'_>],
) -> Option<(usize, usize, String)> {
    if args.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let first = &args[0];
    let first_loc = first.location();
    parts.push(
        source
            .byte_slice(first_loc.start_offset(), first_loc.end_offset(), "")
            .to_string(),
    );

    // second arg (safe_level) is removed entirely
    if args.len() > 2 {
        let arg = &args[2];
        if arg.as_keyword_hash_node().is_none() && arg.as_hash_node().is_none() {
            let loc = arg.location();
            let src = source.byte_slice(loc.start_offset(), loc.end_offset(), "");
            parts.push(format!("trim_mode: {src}"));
        }
    }

    if args.len() > 3 {
        let arg = &args[3];
        if arg.as_keyword_hash_node().is_none() && arg.as_hash_node().is_none() {
            let loc = arg.location();
            let src = source.byte_slice(loc.start_offset(), loc.end_offset(), "");
            parts.push(format!("eoutvar: {src}"));
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some((
        first_loc.start_offset(),
        args.last()?.location().end_offset(),
        parts.join(", "),
    ))
}

impl Cop for ErbNewArguments {
    fn name(&self) -> &'static str {
        "Lint/ErbNewArguments"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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

        if call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let name = match constant_name(&receiver) {
            Some(n) => n,
            None => return,
        };

        if name != b"ERB" {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = arguments.arguments().iter().collect();

        // ERB.new(str) or ERB.new(str, key: val) are fine
        if args.len() <= 1 {
            return;
        }
        if args.len() == 2 && args[1].as_keyword_hash_node().is_some() {
            return;
        }

        let correction = build_arguments_correction(source, &args);

        // Check args at positions 1, 2, 3 (safe_level, trim_mode, eoutvar)
        for (i, arg) in args.iter().enumerate().skip(1).take(3) {
            // Skip if it's a hash (keyword args)
            if arg.as_keyword_hash_node().is_some() || arg.as_hash_node().is_some() {
                continue;
            }

            let msg = match i {
                1 => "Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.".to_string(),
                2 => {
                    let arg_src = source.byte_slice(arg.location().start_offset(), arg.location().end_offset(), "...");
                    format!("Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: {})` instead.", arg_src)
                }
                3 => {
                    let arg_src = source.byte_slice(arg.location().start_offset(), arg.location().end_offset(), "...");
                    format!("Passing eoutvar with the 4th argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, eoutvar: {})` instead.", arg_src)
                }
                _ => continue,
            };

            let loc = arg.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diag = self.diagnostic(source, line, column, msg);

            if let (Some(corr), Some((start, end, replacement))) =
                (corrections.as_mut(), correction.as_ref())
            {
                corr.push(crate::correction::Correction {
                    start: *start,
                    end: *end,
                    replacement: replacement.clone(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }

            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ErbNewArguments, "cops/lint/erb_new_arguments");
    crate::cop_autocorrect_fixture_tests!(ErbNewArguments, "cops/lint/erb_new_arguments");
}
