/// Performance/IoReadlines
///
/// Identifies places where inefficient `readlines` method can be replaced by
/// `each_line` to avoid fully loading file content into memory.
///
/// ## Investigation (2026-03-04)
///
/// Root cause of 0% match rate (208 FN): the original implementation listened
/// on CONSTANT_PATH_NODE/CONSTANT_READ_NODE and tried to look outward via
/// `as_method_chain`. This meant:
/// 1. Instance calls (`file.readlines.each`) were never matched (no constant receiver)
/// 2. Only `each` and `map` were checked, missing all other Enumerable methods
/// 3. Message format didn't match RuboCop ("IO.foreach" vs "each_line")
/// 4. Offense location covered the whole expression instead of readlines..method range
///
/// Fix: Rewrote to listen on CALL_NODE, check if method is an Enumerable method,
/// look at receiver for a `readlines` call, and match RuboCop's message format
/// and offense range exactly.
use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IoReadlines;

/// Enumerable instance methods that trigger the cop when chained after `readlines`.
const ENUMERABLE_METHODS: &[&[u8]] = &[
    b"all?",
    b"any?",
    b"chain",
    b"chunk",
    b"chunk_while",
    b"collect",
    b"collect_concat",
    b"compact",
    b"count",
    b"cycle",
    b"detect",
    b"drop",
    b"drop_while",
    b"each",
    b"each_cons",
    b"each_entry",
    b"each_slice",
    b"each_with_index",
    b"each_with_object",
    b"entries",
    b"filter",
    b"filter_map",
    b"find",
    b"find_all",
    b"find_index",
    b"first",
    b"flat_map",
    b"grep",
    b"grep_v",
    b"group_by",
    b"include?",
    b"inject",
    b"lazy",
    b"map",
    b"max",
    b"max_by",
    b"member?",
    b"min",
    b"min_by",
    b"minmax",
    b"minmax_by",
    b"none?",
    b"one?",
    b"partition",
    b"reduce",
    b"reject",
    b"reverse_each",
    b"select",
    b"slice_after",
    b"slice_before",
    b"slice_when",
    b"sort",
    b"sort_by",
    b"sum",
    b"take",
    b"take_while",
    b"tally",
    b"to_a",
    b"to_h",
    b"to_set",
    b"uniq",
    b"zip",
];

fn is_enumerable_method(name: &[u8]) -> bool {
    ENUMERABLE_METHODS.contains(&name)
}

/// Check if a node is an IO or File constant (handles both simple and qualified paths).
fn is_io_or_file_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        let name = cr.name().as_slice();
        return name == b"IO" || name == b"File";
    }
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(child) = cp.name() {
            let name = child.as_slice();
            return name == b"IO" || name == b"File";
        }
    }
    false
}

impl Cop for IoReadlines {
    fn name(&self) -> &'static str {
        "Performance/IoReadlines"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();
        if !is_enumerable_method(outer_method) {
            return;
        }

        // The receiver of the outer call must be a `readlines` call
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let readlines_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if readlines_call.name().as_slice() != b"readlines" {
            return;
        }

        // RuboCop matches two patterns:
        // 1. Class call: (IO|File).readlines(...).method — receiver is IO/File constant
        // 2. Instance call: expr.readlines(...).method — receiver is non-constant or nil
        // We accept both. If receiver is a constant but NOT IO/File, skip.
        if let Some(recv) = readlines_call.receiver() {
            if (recv.as_constant_read_node().is_some() || recv.as_constant_path_node().is_some())
                && !is_io_or_file_const(&recv)
            {
                return;
            }
        }
        // nil receiver (bare `readlines`) is allowed for instance pattern

        // Build message matching RuboCop format
        let outer_name = std::str::from_utf8(outer_method).unwrap_or("?");
        let message = if outer_method == b"each" {
            "Use `each_line` instead of `readlines.each`.".to_string()
        } else {
            format!("Use `each_line.{outer_name}` instead of `readlines.{outer_name}`.")
        };

        // Offense location starts at `readlines` method name
        let readlines_loc = readlines_call
            .message_loc()
            .unwrap_or(readlines_call.location());
        let (line, column) = source.offset_to_line_col(readlines_loc.start_offset());

        diagnostics.push(self.diagnostic(source, line, column, message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IoReadlines, "cops/performance/io_readlines");
}
