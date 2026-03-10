use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/HashConversion checks for uses of `Hash[...]` which can be replaced
/// with literal hash syntax or `.to_h`.
///
/// ## Investigation findings (2026-03-10)
///
/// Root cause of 8 FPs: RuboCop uses `ignore_node`/`part_of_ignored_node?` to
/// suppress nested `Hash[]` calls. After processing an outer `Hash[]`, RuboCop
/// marks it as ignored, so any inner `Hash[]` that is a descendant of the outer
/// one is skipped. nitrocop was flagging all `Hash[]` calls independently,
/// producing FPs on inner nested calls.
///
/// All 8 FPs were nested `Hash[]` patterns like:
///   `Hash[items.map { |k, v| [k, Hash[v.map { ... }]] }]`
///
/// Fix: converted from `check_node` to `check_source` with a visitor that
/// tracks `hash_bracket_depth`. When depth > 0, `Hash[]` calls are skipped.
pub struct HashConversion;

impl Cop for HashConversion {
    fn name(&self) -> &'static str {
        "Style/HashConversion"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_splat = config.get_bool("AllowSplatArgument", true);

        let mut visitor = HashConversionVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            allow_splat,
            hash_bracket_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HashConversionVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a HashConversion,
    diagnostics: Vec<Diagnostic>,
    allow_splat: bool,
    /// How many `Hash[...]` calls we are currently nested inside.
    /// When > 0, new `Hash[]` calls are suppressed (RuboCop's ignore_node behavior).
    hash_bracket_depth: usize,
}

fn is_hash_bracket_call(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"[]" {
        return false;
    }
    let Some(receiver) = call.receiver() else {
        return false;
    };
    receiver
        .as_constant_read_node()
        .is_some_and(|c| c.name().as_slice() == b"Hash")
        || receiver.as_constant_path_node().is_some_and(|cp| {
            cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Hash")
        })
}

impl<'a> HashConversionVisitor<'a> {
    fn check_hash_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());

        let args = call.arguments();

        if let Some(args) = args {
            let arg_list: Vec<_> = args.arguments().iter().collect();

            // Check for splat argument
            if self.allow_splat && arg_list.iter().any(|a| a.as_splat_node().is_some()) {
                return;
            }

            // Check for keyword hash argument
            if arg_list.len() == 1 && arg_list[0].as_keyword_hash_node().is_some() {
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Prefer literal hash to `Hash[key: value, ...]`.".to_string(),
                ));
                return;
            }

            if arg_list.len() == 1 {
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Prefer `ary.to_h` to `Hash[ary]`.".to_string(),
                ));
                return;
            }

            // Multi-argument
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer literal hash to `Hash[arg1, arg2, ...]`.".to_string(),
            ));
            return;
        }

        // No arguments: Hash[]
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Prefer literal hash to `Hash[arg1, arg2, ...]`.".to_string(),
        ));
    }
}

impl<'a> Visit<'_> for HashConversionVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        if is_hash_bracket_call(node) {
            if self.hash_bracket_depth == 0 {
                // Outermost Hash[] — flag it
                self.check_hash_call(node);
            }
            // Recurse into children with incremented depth to suppress nested Hash[] calls
            self.hash_bracket_depth += 1;
            ruby_prism::visit_call_node(self, node);
            self.hash_bracket_depth -= 1;
        } else {
            ruby_prism::visit_call_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashConversion, "cops/style/hash_conversion");
}
