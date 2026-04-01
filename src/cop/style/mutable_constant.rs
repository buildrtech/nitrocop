use crate::cop::node_type::{
    CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE, CONSTANT_PATH_WRITE_NODE,
    CONSTANT_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MutableConstant: freeze mutable objects assigned to constants.
///
/// ## Root cause analysis (extended corpus FP=336, FN=119, 98.7% match)
///
/// **FN root causes (resolved):**
/// - Missing `CONST ||= value` handling (`ConstantOrWriteNode` / `ConstantPathOrWriteNode`)
/// - `frozen_string_literal: true` incorrectly suppressed interpolated strings;
///   Ruby 3.0+ only freezes non-interpolated string literals with the magic comment
/// - `EnforcedStyle: strict` not implemented — strict mode flags all non-frozen
///   non-immutable RHS values (including method calls like `Something.new`)
/// - Missing `shareable_constant_value` magic comment handling (caused FPs)
/// - Missing XStringNode (backtick literal) and InterpolatedXStringNode handling;
///   these are mutable literals per RuboCop's MUTABLE_LITERALS = %i[str dstr xstr ...]
///
/// **FP root causes (resolved):**
/// - `shareable_constant_value: literal` / `experimental_everything` / `experimental_copy`
///   magic comments suppress offenses in Ruby 3.0+, but were not implemented
pub struct MutableConstant;

impl MutableConstant {
    /// Check if a node is a mutable literal (array, hash, string, xstring).
    /// In `literals` mode, only literal values are flagged.
    /// Matches RuboCop's MUTABLE_LITERALS = %i[str dstr xstr array hash regexp irange erange]
    /// (regexp/range are excluded via frozen_regexp_or_range_literals? for Ruby 3.0+).
    fn is_mutable_literal(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
        node.as_array_node().is_some()
            || node.as_hash_node().is_some()
            || node.as_keyword_hash_node().is_some()
            || node.as_string_node().is_some()
            || Self::is_interpolated_string(source, node)
            // XStringNode = backtick literal (`command`), always mutable
            || node.as_x_string_node().is_some()
            // InterpolatedXStringNode = backtick with interpolation (`cmd #{x}`)
            || node.as_interpolated_x_string_node().is_some()
    }

    /// Check if node is a non-interpolated string literal (StringNode only, no heredocs
    /// with interpolation). `frozen_string_literal: true` only freezes these.
    fn is_plain_string(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(s) = node.as_string_node() {
            // Heredocs are mutable even with frozen_string_literal: true in Ruby 3.0+
            // ... actually no: plain (non-interpolated) heredocs ARE frozen with the magic comment.
            // Only interpolated heredocs are not frozen.
            // StringNode = non-interpolated, so always plain.
            let _ = s;
            // But we need to check: is it a heredoc? Plain heredocs are still frozen
            // with the magic comment. StringNode heredocs are non-interpolated, so they're fine.
            return true;
        }
        // InterpolatedStringNode that has NO actual interpolation parts:
        // In Ruby, `"hello"` can parse as InterpolatedStringNode in some contexts,
        // but for frozen_string_literal purposes, only non-interpolated strings are frozen.
        // Multiline string concatenation with `\` produces InterpolatedStringNode.
        // We need to check if it actually has interpolation.
        if let Some(isn) = node.as_interpolated_string_node() {
            // Check if it has no embedded expressions — just string parts
            let has_interpolation = isn.parts().iter().any(|part| {
                part.as_embedded_statements_node().is_some()
                    || part.as_embedded_variable_node().is_some()
            });
            if !has_interpolation {
                // Also check: is it a heredoc?
                // Non-interpolated heredocs are frozen with the magic comment.
                // Non-interpolated multiline strings are frozen too.
                let _ = source;
                return true;
            }
        }
        false
    }

    /// Check if node is an InterpolatedStringNode (which includes heredocs with interpolation).
    fn is_interpolated_string(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(isn) = node.as_interpolated_string_node() {
            // Check if it's a heredoc
            if let Some(opening) = isn.opening_loc() {
                let bytes = &source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    return true;
                }
            }
            // Regular interpolated string
            return true;
        }
        false
    }

    fn is_heredoc_literal(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(s) = node.as_string_node() {
            if let Some(opening) = s.opening_loc() {
                let bytes = &source.as_bytes()[opening.start_offset()..opening.end_offset()];
                return bytes.starts_with(b"<<");
            }
        }

        if let Some(isn) = node.as_interpolated_string_node() {
            if let Some(opening) = isn.opening_loc() {
                let bytes = &source.as_bytes()[opening.start_offset()..opening.end_offset()];
                return bytes.starts_with(b"<<");
            }
        }

        false
    }

    /// Check if the value is a `.freeze` call (meaning the value is already frozen).
    fn is_frozen_value(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"freeze" {
                return true;
            }
        }
        false
    }

    /// For `strict` mode: check if the value is an immutable literal
    /// (numbers, symbols, booleans, nil, regexps, ranges in Ruby 3.0+).
    fn is_immutable_literal(node: &ruby_prism::Node<'_>) -> bool {
        node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_symbol_node().is_some()
            || node.as_true_node().is_some()
            || node.as_false_node().is_some()
            || node.as_nil_node().is_some()
            || node.as_rational_node().is_some()
            || node.as_imaginary_node().is_some()
            // Regexp and Range are frozen since Ruby 3.0
            || node.as_regular_expression_node().is_some()
            || node.as_interpolated_regular_expression_node().is_some()
            || node.as_range_node().is_some()
            // Parenthesized range: (1..10) is a BeginNode wrapping a RangeNode
            || Self::is_parenthesized_range(node)
    }

    fn is_parenthesized_range(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(paren) = node.as_parentheses_node() {
            if let Some(body) = paren.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let parts: Vec<_> = stmts.body().iter().collect();
                    if parts.len() == 1 {
                        return parts[0].as_range_node().is_some();
                    }
                }
            }
        }
        false
    }

    /// For `strict` mode: check if operation produces an immutable object.
    /// Matches RuboCop's `operation_produces_immutable_object?` NodePattern.
    fn operation_produces_immutable_object(node: &ruby_prism::Node<'_>) -> bool {
        // Constants (OTHER_CONST, Namespace::CONST) are immutable references
        if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
            return true;
        }

        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();

            // .freeze calls
            if name_bytes == b"freeze" {
                return true;
            }

            // Struct.new / ::Struct.new
            if name_bytes == b"new" {
                if let Some(recv) = call.receiver() {
                    if Self::is_struct_constant(&recv) {
                        return true;
                    }
                }
            }

            // ENV['foo'] / ::ENV['foo']
            if name_bytes == b"[]" {
                if let Some(recv) = call.receiver() {
                    if Self::is_env_constant(&recv) {
                        return true;
                    }
                }
            }

            // Comparison operators: ==, ===, !=, <=, >=, <, >
            if matches!(
                name_bytes,
                b"==" | b"===" | b"!=" | b"<=" | b">=" | b"<" | b">"
            ) {
                return true;
            }

            // count/length/size methods
            if matches!(name_bytes, b"count" | b"length" | b"size") {
                return true;
            }

            // Arithmetic with int/float operands: int/float op anything, or anything op int/float
            if matches!(name_bytes, b"+" | b"-" | b"*" | b"**" | b"/" | b"%" | b"<<") {
                if let Some(recv) = call.receiver() {
                    if recv.as_integer_node().is_some() || recv.as_float_node().is_some() {
                        return true;
                    }
                }
                let args = call.arguments();
                if let Some(args) = args {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if let Some(arg) = arg_list.first() {
                        if arg.as_integer_node().is_some() || arg.as_float_node().is_some() {
                            return true;
                        }
                    }
                }
            }
        }

        // Block with Struct.new: `Struct.new(:a) do ... end`
        if let Some(block) = node.as_call_node() {
            // Already handled above via call_node
            let _ = block;
        }

        // ENV['foo'] || 'fallback'
        if let Some(or_node) = node.as_or_node() {
            let left = or_node.left();
            if let Some(call) = left.as_call_node() {
                if call.name().as_slice() == b"[]" {
                    if let Some(recv) = call.receiver() {
                        if Self::is_env_constant(&recv) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn is_struct_constant(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(cr) = node.as_constant_read_node() {
            return cr.name().as_slice() == b"Struct";
        }
        if let Some(cp) = node.as_constant_path_node() {
            // ::Struct
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"Struct";
                }
            }
        }
        false
    }

    fn is_env_constant(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(cr) = node.as_constant_read_node() {
            return cr.name().as_slice() == b"ENV";
        }
        if let Some(cp) = node.as_constant_path_node() {
            // ::ENV
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"ENV";
                }
            }
        }
        false
    }

    /// Check if the source file has `# frozen_string_literal: true` in the
    /// first few lines (before any code). This magic comment makes plain string
    /// literals frozen (but NOT interpolated strings in Ruby 3.0+).
    ///
    /// Supports both simple format (`# frozen_string_literal: true`) and
    /// Emacs format (`# -*- frozen_string_literal: true -*-`).
    fn has_frozen_string_literal_true(source: &SourceFile) -> bool {
        let lines = source.lines();
        for (i, line) in lines.enumerate() {
            if i >= 3 {
                break;
            }
            let s = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            if let Some(rest) = s.strip_prefix('#') {
                let rest = rest.trim_start();
                // Simple format: # frozen_string_literal: true
                if let Some(value) = rest.strip_prefix("frozen_string_literal:") {
                    return value.trim() == "true";
                }
                // Emacs format: # -*- frozen_string_literal: true -*-
                if rest.starts_with("-*-") && rest.ends_with("-*-") {
                    // Extract token content between -*- markers
                    let inner = &rest[3..rest.len() - 3].trim();
                    // Split by ';' for multiple directives
                    for directive in inner.split(';') {
                        let directive = directive.trim();
                        if let Some(value) = directive.strip_prefix("frozen_string_literal:") {
                            return value.trim() == "true";
                        }
                    }
                }
            }
        }
        false
    }

    /// Find the most recent `shareable_constant_value` magic comment that applies
    /// to the given byte offset. Returns true if it enables sharing (literal,
    /// experimental_everything, experimental_copy), false otherwise.
    fn has_shareable_constant_value(source: &SourceFile, node_offset: usize) -> bool {
        let (node_line, _) = source.offset_to_line_col(node_offset);
        let mut result = false;

        let lines = source.lines();
        for (i, line) in lines.enumerate() {
            let line_num = i + 1;
            if line_num > node_line {
                break;
            }
            let s = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let s = s.trim();
            if let Some(rest) = s.strip_prefix('#') {
                let rest = rest.trim_start();
                // Simple format: # shareable_constant_value: literal
                if let Some(value) = rest.strip_prefix("shareable_constant_value:") {
                    let value = value.trim();
                    result = matches!(
                        value,
                        "literal" | "experimental_everything" | "experimental_copy"
                    );
                }
                // Emacs format: # -*- shareable_constant_value: literal -*-
                if rest.starts_with("-*-") && rest.ends_with("-*-") {
                    let inner = &rest[3..rest.len() - 3].trim();
                    for directive in inner.split(';') {
                        let directive = directive.trim();
                        if let Some(value) = directive.strip_prefix("shareable_constant_value:") {
                            let value = value.trim();
                            result = matches!(
                                value,
                                "literal" | "experimental_everything" | "experimental_copy"
                            );
                        }
                    }
                }
            }
        }
        result
    }

    /// Check if a `CallNode` wraps a Struct.new with a block (strict mode immutable).
    fn is_struct_new_block(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"new" && call.block().is_some() {
                if let Some(recv) = call.receiver() {
                    return Self::is_struct_constant(&recv);
                }
            }
        }
        false
    }

    fn check_value(
        &self,
        source: &SourceFile,
        value: &ruby_prism::Node<'_>,
        frozen_strings: bool,
        enforced_style: &str,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) -> Vec<Diagnostic> {
        // Already frozen via .freeze call
        if Self::is_frozen_value(value) {
            return Vec::new();
        }

        // Check shareable_constant_value magic comment
        if Self::has_shareable_constant_value(source, value.location().start_offset()) {
            return Vec::new();
        }

        if enforced_style == "strict" {
            // Strict mode: flag everything that isn't immutable
            if Self::is_immutable_literal(value) {
                return Vec::new();
            }
            if Self::operation_produces_immutable_object(value) {
                return Vec::new();
            }
            if Self::is_struct_new_block(value) {
                return Vec::new();
            }
            // In strict mode, frozen_string_literal: true makes plain strings immutable
            if frozen_strings && Self::is_plain_string(source, value) {
                return Vec::new();
            }
        } else {
            // Literals mode: only flag mutable literals
            if !Self::is_mutable_literal(source, value) {
                return Vec::new();
            }
            // When frozen_string_literal: true is set, plain (non-interpolated) string
            // constants are already frozen — don't flag them.
            // But interpolated strings are NOT frozen in Ruby 3.0+.
            if frozen_strings && Self::is_plain_string(source, value) {
                return Vec::new();
            }
        }

        // Point at the mutable value (RHS), matching RuboCop behavior
        let (line, column) = source.offset_to_line_col(value.location().start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Freeze mutable objects assigned to constants.".to_string(),
        );

        // Conservative autocorrect: append `.freeze` to the RHS value source.
        // Skip heredocs because appending after the heredoc terminator can
        // produce invalid output.
        if let Some(ref mut corr) = corrections {
            if !Self::is_heredoc_literal(source, value) {
                let value_loc = value.location();
                corr.push(crate::correction::Correction {
                    start: value_loc.end_offset(),
                    end: value_loc.end_offset(),
                    replacement: ".freeze".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }

        vec![diag]
    }
}

impl Cop for MutableConstant {
    fn name(&self) -> &'static str {
        "Style/MutableConstant"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "literals");
        let frozen_strings = Self::has_frozen_string_literal_true(source);

        // Check ConstantWriteNode (CONST = value)
        if let Some(cw) = node.as_constant_write_node() {
            let value = cw.value();
            diagnostics.extend(self.check_value(
                source,
                &value,
                frozen_strings,
                enforced_style,
                corrections.as_deref_mut(),
            ));
            return;
        }

        // Check ConstantPathWriteNode (Module::CONST = value)
        if let Some(cpw) = node.as_constant_path_write_node() {
            let value = cpw.value();
            diagnostics.extend(self.check_value(
                source,
                &value,
                frozen_strings,
                enforced_style,
                corrections.as_deref_mut(),
            ));
            return;
        }

        // Check ConstantOrWriteNode (CONST ||= value)
        if let Some(cow) = node.as_constant_or_write_node() {
            let value = cow.value();
            diagnostics.extend(self.check_value(
                source,
                &value,
                frozen_strings,
                enforced_style,
                corrections.as_deref_mut(),
            ));
            return;
        }

        // Check ConstantPathOrWriteNode (Module::CONST ||= value)
        if let Some(cpow) = node.as_constant_path_or_write_node() {
            let value = cpow.value();
            diagnostics.extend(self.check_value(
                source,
                &value,
                frozen_strings,
                enforced_style,
                corrections.as_deref_mut(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MutableConstant, "cops/style/mutable_constant");
    crate::cop_autocorrect_fixture_tests!(MutableConstant, "cops/style/mutable_constant");

    /// XStringNode (backtick) should be flagged even with frozen_string_literal: true.
    /// The magic comment only freezes str/dstr, not xstr.
    #[test]
    fn xstring_flagged_with_frozen_string_literal() {
        let cop = MutableConstant;
        let source = b"# frozen_string_literal: true\n\nCONST = `uname`\n";
        let diags =
            crate::testutil::run_cop_full_internal(&cop, source, CopConfig::default(), "test.rb");
        assert_eq!(
            diags.len(),
            1,
            "xstring should be flagged even with frozen_string_literal: true, got {:?}",
            diags
        );
    }

    /// Plain strings should NOT be flagged with frozen_string_literal: true.
    #[test]
    fn plain_string_not_flagged_with_frozen_string_literal() {
        let cop = MutableConstant;
        let source = b"# frozen_string_literal: true\n\nCONST = \"hello\"\n";
        let diags =
            crate::testutil::run_cop_full_internal(&cop, source, CopConfig::default(), "test.rb");
        assert_eq!(
            diags.len(),
            0,
            "plain string should not be flagged with frozen_string_literal: true, got {:?}",
            diags
        );
    }

    /// Emacs-style frozen_string_literal comment should also suppress plain strings.
    #[test]
    fn emacs_style_frozen_string_literal() {
        let cop = MutableConstant;
        let source = b"# -*- frozen_string_literal: true -*-\n\nCONST = \"hello\"\n";
        let diags =
            crate::testutil::run_cop_full_internal(&cop, source, CopConfig::default(), "test.rb");
        assert_eq!(
            diags.len(),
            0,
            "Emacs-style frozen_string_literal should suppress plain string offense, got {:?}",
            diags
        );
    }

    /// Emacs-style frozen_string_literal combined with encoding.
    #[test]
    fn emacs_style_combined_magic_comment() {
        let cop = MutableConstant;
        let source = b"# -*- coding: utf-8; frozen_string_literal: true -*-\n\nCONST = \"hello\"\n";
        let diags =
            crate::testutil::run_cop_full_internal(&cop, source, CopConfig::default(), "test.rb");
        assert_eq!(
            diags.len(),
            0,
            "Emacs-style combined magic comment should suppress plain string offense, got {:?}",
            diags
        );
    }
}
