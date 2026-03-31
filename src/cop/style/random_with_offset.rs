use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTEGER_NODE, RANGE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RandomWithOffset;

impl Cop for RandomWithOffset {
    fn name(&self) -> &'static str {
        "Style/RandomWithOffset"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTEGER_NODE,
            RANGE_NODE,
        ]
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

        let method_bytes = call.name().as_slice();
        let replacement = if method_bytes == b"+" || method_bytes == b"-" {
            self.arithmetic_replacement(source, &call)
        } else if method_bytes == b"succ" || method_bytes == b"next" || method_bytes == b"pred" {
            self.succ_pred_replacement(source, &call)
        } else {
            None
        };

        let Some(replacement) = replacement else {
            return;
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Prefer ranges when generating random numbers instead of integers with offsets."
                .to_string(),
        );

        if let Some(corr) = corrections.as_mut() {
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

impl RandomWithOffset {
    /// Returns (prefix, left_bound, right_bound) for rand-like calls.
    /// Prefix is `rand`, `Random.rand`, `::Random.rand`, `Kernel.rand`, etc.
    fn rand_call_info(
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Option<(String, i64, i64)> {
        let call = node.as_call_node()?;
        if call.name().as_slice() != b"rand" {
            return None;
        }

        let prefix = if let Some(recv) = call.receiver() {
            let is_random_or_kernel = recv.as_constant_read_node().is_some_and(|c| {
                let name = c.name().as_slice();
                name == b"Random" || name == b"Kernel"
            }) || recv.as_constant_path_node().is_some_and(|cp| {
                let src = cp.location().as_slice();
                src == b"Random" || src == b"Kernel" || src == b"::Random" || src == b"::Kernel"
            });
            if !is_random_or_kernel {
                return None;
            }
            let loc = recv.location();
            format!(
                "{}.rand",
                source.byte_slice(loc.start_offset(), loc.end_offset(), "")
            )
        } else {
            "rand".to_string()
        };

        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }

        if let Some(value) = parse_int(&arg_list[0]) {
            return Some((prefix, 0, value - 1));
        }

        if let Some(range) = arg_list[0].as_range_node() {
            let left = parse_int(&range.left()?)?;
            let mut right = parse_int(&range.right()?)?;
            if range.is_exclude_end() {
                right -= 1;
            }
            return Some((prefix, left, right));
        }

        None
    }

    fn arithmetic_replacement(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
    ) -> Option<String> {
        let receiver = call.receiver()?;
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }

        let op = call.name().as_slice();

        if let (Some((prefix, left, right)), Some(offset)) = (
            Self::rand_call_info(source, &receiver),
            parse_int(&arg_list[0]),
        ) {
            let (new_left, new_right) = if op == b"+" {
                (left + offset, right + offset)
            } else {
                (left - offset, right - offset)
            };
            return Some(format!("{prefix}({new_left}..{new_right})"));
        }

        if let (Some(offset), Some((prefix, left, right))) = (
            parse_int(&receiver),
            Self::rand_call_info(source, &arg_list[0]),
        ) {
            let (new_left, new_right) = if op == b"+" {
                (offset + left, offset + right)
            } else {
                (offset - right, offset - left)
            };
            return Some(format!("{prefix}({new_left}..{new_right})"));
        }

        None
    }

    fn succ_pred_replacement(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
    ) -> Option<String> {
        let receiver = call.receiver()?;
        let (prefix, left, right) = Self::rand_call_info(source, &receiver)?;

        let (new_left, new_right) = match call.name().as_slice() {
            b"succ" | b"next" => (left + 1, right + 1),
            b"pred" => (left - 1, right - 1),
            _ => return None,
        };

        Some(format!("{prefix}({new_left}..{new_right})"))
    }
}

fn parse_int(node: &ruby_prism::Node<'_>) -> Option<i64> {
    let int_node = node.as_integer_node()?;
    let src = std::str::from_utf8(int_node.location().as_slice()).ok()?;
    src.replace('_', "").parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RandomWithOffset, "cops/style/random_with_offset");
    crate::cop_autocorrect_fixture_tests!(RandomWithOffset, "cops/style/random_with_offset");
}
