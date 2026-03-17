use ruby_prism::Visit;

use crate::cop::util::is_blank_or_whitespace_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/EmptyLinesAfterModuleInclusion
///
/// Checks for an empty line after a module inclusion method (`extend`,
/// `include` and `prepend`), or a group of them.
///
/// ## Investigation findings (2026-03-11, 2026-03-14)
///
/// Corpus work on this cop found two separate logic gaps:
///
/// - RuboCop skips include/extend/prepend when the direct parent is an
///   `if`/`unless` body, so nitrocop now tracks `in_if_body` and resets that
///   state when entering nested class/module/singleton-class bodies.
/// - RuboCop still checks bare method-local include/extend/prepend calls, so
///   nitrocop must not treat method bodies as a blanket skip context.
///
/// The 2026-03-14 corpus pass also showed several line-structure mismatches:
///
/// - Whitespace-only lines after an inclusion should count as blank.
/// - Inline/same-line continuations like `class A; include M; end` should not
///   require a synthetic blank line.
/// - Structural/clause followers such as `})`, `else`, `rescue`, `ensure`,
///   `elsif`, and `when` should be treated as allowed siblings.
///
/// The cop now follows RuboCop's sibling-oriented behavior instead of trying
/// to infer a narrower "real module body" context from surrounding nodes.
///
/// ## Investigation findings (2026-03-17)
///
/// 88 FNs across 52 repos from two root causes:
///
/// 1. **if/unless body over-suppression**: The `in_if_body` flag was set for
///    ALL descendants of if/unless nodes. RuboCop only skips when
///    `node.parent.if_type?` — true only when the include is the sole
///    statement in a branch (single child of if node). Multi-statement
///    branches use a `begin` parent, so `if_type?` is false and the cop
///    fires. Fix: only set `in_if_body` when `StatementsNode.body().len() <= 1`.
///
/// 2. **Rescue modifier false skip**: `include Foo rescue Bar` wraps the call
///    in a `RescueModifierNode`. The `line_has_trailing_code` check saw
///    `rescue Bar` as trailing code and bailed out. Fix: handle
///    `visit_rescue_modifier_node` to pass the modifier's end_offset,
///    bypassing the trailing-code check.
pub struct EmptyLinesAfterModuleInclusion;

const MODULE_INCLUSION_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

/// Check if a trimmed line starts with a module inclusion method call.
fn is_module_inclusion_line(trimmed: &[u8]) -> bool {
    for &method in MODULE_INCLUSION_METHODS {
        if trimmed.starts_with(method) {
            let after = trimmed.get(method.len());
            if after.is_none() || matches!(after, Some(b' ') | Some(b'(')) {
                return true;
            }
        }
    }
    false
}

fn trim_leading(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line.len());
    &line[start..]
}

fn is_enable_directive_comment(trimmed: &[u8]) -> bool {
    trimmed.starts_with(b"# rubocop:enable")
        || trimmed.starts_with(b"#rubocop:enable")
        || trimmed.starts_with(b"# rubocop: enable")
        || trimmed.starts_with(b"#rubocop: enable")
}

fn starts_with_keyword(trimmed: &[u8], keyword: &[u8]) -> bool {
    trimmed.starts_with(keyword)
        && (trimmed.len() == keyword.len()
            || !matches!(trimmed[keyword.len()], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}

/// Check if a line contains a call to include/extend/prepend, even with a receiver.
/// This matches RuboCop's `allowed_method?` which checks `node.method_name` regardless
/// of receiver (e.g., `singleton_class.extend(ClassMethods)` has method_name `:extend`).
fn line_has_inclusion_method_call(trimmed: &[u8]) -> bool {
    for &method in MODULE_INCLUSION_METHODS {
        // Check for `.method(` or `.method ` patterns (receiver-qualified calls)
        let dot_method = {
            let mut v = Vec::with_capacity(1 + method.len());
            v.push(b'.');
            v.extend_from_slice(method);
            v
        };
        if let Some(pos) = trimmed
            .windows(dot_method.len())
            .position(|w| w == dot_method.as_slice())
        {
            let after_pos = pos + dot_method.len();
            let after = trimmed.get(after_pos);
            if after.is_none() || matches!(after, Some(b' ') | Some(b'(')) {
                return true;
            }
        }
    }
    false
}

fn is_allowed_following_line(trimmed: &[u8]) -> bool {
    is_module_inclusion_line(trimmed)
        || line_has_inclusion_method_call(trimmed)
        || starts_with_keyword(trimmed, b"end")
        || starts_with_keyword(trimmed, b"else")
        || starts_with_keyword(trimmed, b"elsif")
        || starts_with_keyword(trimmed, b"rescue")
        || starts_with_keyword(trimmed, b"ensure")
        || starts_with_keyword(trimmed, b"when")
        || trimmed.starts_with(b"}")
        || trimmed.starts_with(b")")
        || trimmed.starts_with(b"]")
}

fn line_has_trailing_code(source: &SourceFile, loc: &ruby_prism::Location<'_>) -> bool {
    let bytes = source.as_bytes();
    let end = loc.end_offset().min(bytes.len());
    let line_end = bytes[end..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|offset| end + offset)
        .unwrap_or(bytes.len());
    let trailing = trim_leading(&bytes[end..line_end]);
    !trailing.is_empty() && !trailing.starts_with(b"#")
}

impl Cop for EmptyLinesAfterModuleInclusion {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAfterModuleInclusion"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = InclusionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            in_block_or_send: false,
            in_if_body: false,
        };
        visitor.visit(&parse_result.node());
        if let Some(ref mut corr) = corrections {
            for d in &mut visitor.diagnostics {
                d.corrected = true;
            }
            corr.extend(visitor.corrections);
        }
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InclusionVisitor<'a> {
    cop: &'a EmptyLinesAfterModuleInclusion,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    /// True when inside a block, lambda, or array — contexts where
    /// include/extend/prepend are NOT module inclusions
    in_block_or_send: bool,
    /// True when inside an if/unless body — RuboCop's `next_line_node` returns
    /// nil when `node.parent.if_type?`, so the cop never fires in these contexts.
    in_if_body: bool,
}

impl InclusionVisitor<'_> {
    /// Check whether a call node is an include/extend/prepend at module level.
    /// `end_offset_override` allows rescue modifiers to pass their own end offset,
    /// since `include Foo rescue Bar` has trailing code after the call but before
    /// the rescue modifier's end.
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>, end_offset_override: Option<usize>) {
        // Must be a bare call (no receiver)
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !MODULE_INCLUSION_METHODS.contains(&method_name) {
            return;
        }

        // Must have arguments (e.g., `include Foo`)
        if call.arguments().is_none() {
            return;
        }

        // Skip if inside a block, array, or used as argument to another call
        // (matches RuboCop: `return if node.parent&.type?(:send, :any_block, :array)`)
        if self.in_block_or_send {
            return;
        }

        // Skip if inside an if/unless body
        // (matches RuboCop: `return if node.parent&.if_type?` in `next_line_node`)
        if self.in_if_body {
            return;
        }

        let loc = call.location();
        let effective_end = end_offset_override.unwrap_or_else(|| loc.end_offset());
        let (last_line, _) = self
            .source
            .offset_to_line_col(effective_end.saturating_sub(1));
        let lines: Vec<&[u8]> = self.source.lines().collect();

        // Only check for trailing code when no end_offset_override is provided
        // (rescue modifiers provide their own end offset so trailing code is expected)
        if end_offset_override.is_none() && line_has_trailing_code(self.source, &loc) {
            return;
        }

        // Check if the next line exists
        if last_line >= lines.len() {
            return; // End of file
        }

        let next_line = lines[last_line]; // next line (0-indexed)

        // If next line is blank, no offense
        if is_blank_or_whitespace_line(next_line) {
            return;
        }

        let next_trimmed = trim_leading(next_line);
        if is_allowed_following_line(next_trimmed) {
            return;
        }

        // If next line is a rubocop:enable directive comment, check the line after
        if is_enable_directive_comment(next_trimmed) {
            // Check the line after the enable directive
            if last_line + 1 < lines.len() {
                let line_after = lines[last_line + 1];
                if is_blank_or_whitespace_line(line_after) {
                    return;
                }
            } else {
                return; // enable directive at end of file
            }
        }

        if next_trimmed.starts_with(b"#") {
            let mut idx = last_line + 1;
            while idx < lines.len() {
                let line_trimmed = trim_leading(lines[idx]);
                if line_trimmed.starts_with(b"#") || is_blank_or_whitespace_line(lines[idx]) {
                    idx += 1;
                    continue;
                }
                if is_allowed_following_line(line_trimmed) {
                    return;
                }
                break;
            }
        }

        let (line, col) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            col,
            "Add an empty line after module inclusion.".to_string(),
        ));
        // Insert a blank line after the inclusion line
        if let Some(offset) = self.source.line_col_to_offset(last_line + 1, 0) {
            self.corrections.push(crate::correction::Correction {
                start: offset,
                end: offset,
                replacement: "\n".to_string(),
                cop_name: self.cop.name(),
                cop_index: 0,
            });
        }
    }
}

impl<'pr> Visit<'pr> for InclusionVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call is an include/extend/prepend at the right level
        self.check_call(node, None);

        // When descending into arguments of a call node, mark that we're
        // inside a "send" context. This prevents include/extend/prepend
        // used as arguments (e.g., `.and include(Foo)`) from being flagged.
        if let Some(args) = node.arguments() {
            let was = self.in_block_or_send;
            self.in_block_or_send = true;
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
            self.in_block_or_send = was;
        }

        // Visit receiver in "send" context — include/extend/prepend used as a
        // receiver (e.g., `include(x).and(y)`) should not be flagged. This matches
        // RuboCop's `node.parent&.type?(:send)` check.
        if let Some(recv) = node.receiver() {
            let was = self.in_block_or_send;
            self.in_block_or_send = true;
            self.visit(&recv);
            self.in_block_or_send = was;
        }

        // Visit block argument if any (this IS a block context)
        if let Some(block) = node.block() {
            let was = self.in_block_or_send;
            self.in_block_or_send = true;
            self.visit(&block);
            self.in_block_or_send = was;
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let was = self.in_block_or_send;
        // In RuboCop, `return if node.parent&.type?(:send, :any_block, :array)`.
        // This only skips when the include's *direct parent* is a block.
        // In Prism, when a block body has multiple statements, they are children
        // of a StatementsNode, so the include's parent is NOT the block.
        // Only set in_block_or_send for single-statement block bodies.
        // For multi-statement bodies, RESET to false so nested includes are checked.
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                if stmts.body().len() <= 1 {
                    // Single statement: include's parent would be the block in RuboCop
                    self.in_block_or_send = true;
                } else {
                    // Multiple statements: include's parent is begin/StatementsNode
                    // Reset flag so nested includes at this level are checked
                    self.in_block_or_send = false;
                }
            }
            self.visit(&body);
        }
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        self.in_block_or_send = was;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let was = self.in_block_or_send;
        // Same logic as block_node: only set in_block_or_send for single-statement bodies
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.in_block_or_send = stmts.body().len() <= 1;
            }
            self.visit(&body);
        }
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        self.in_block_or_send = was;
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let was = self.in_block_or_send;
        self.in_block_or_send = true;
        for elem in node.elements().iter() {
            self.visit(&elem);
        }
        self.in_block_or_send = was;
    }

    // Class and module bodies reset the block context — include/extend/prepend
    // at the class/module body level SHOULD be flagged.
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let was_block = self.in_block_or_send;
        let was_if = self.in_if_body;
        self.in_block_or_send = false;
        self.in_if_body = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was_block;
        self.in_if_body = was_if;
        // Visit superclass expression
        if let Some(sup) = node.superclass() {
            self.visit(&sup);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let was_block = self.in_block_or_send;
        let was_if = self.in_if_body;
        self.in_block_or_send = false;
        self.in_if_body = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was_block;
        self.in_if_body = was_if;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let was_block = self.in_block_or_send;
        let was_if = self.in_if_body;
        self.in_block_or_send = false;
        self.in_if_body = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was_block;
        self.in_if_body = was_if;
    }

    // If/unless bodies: RuboCop's `next_line_node` returns nil when
    // `node.parent.if_type?`, meaning the include is the *direct* child
    // of the if node (sole statement in a branch). In Prism, branches always
    // use StatementsNode, so we check statement count: only skip when the
    // branch has exactly 1 statement (matching RuboCop's single-child parent).
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let was = self.in_if_body;

        // Visit predicate normally
        self.visit(&node.predicate());

        // Visit then-branch: only set in_if_body for single-statement branches
        if let Some(stmts) = node.statements() {
            self.in_if_body = stmts.body().len() <= 1;
            self.visit(&stmts.as_node());
        }

        // Visit else/elsif branch
        if let Some(subsequent) = node.subsequent() {
            // elsif is another IfNode — it will handle its own branches
            // else branch: check if it's an ElseNode with single statement
            if let Some(else_node) = subsequent.as_else_node() {
                if let Some(else_stmts) = else_node.statements() {
                    self.in_if_body = else_stmts.body().len() <= 1;
                    self.visit(&else_stmts.as_node());
                }
            } else {
                // elsif — recurse via visit_if_node which handles its own logic
                self.in_if_body = was;
                self.visit(&subsequent);
            }
        }

        self.in_if_body = was;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let was = self.in_if_body;

        // Visit predicate normally
        self.visit(&node.predicate());

        // Visit then-branch: only set in_if_body for single-statement branches
        if let Some(stmts) = node.statements() {
            self.in_if_body = stmts.body().len() <= 1;
            self.visit(&stmts.as_node());
        }

        // Visit else branch
        if let Some(else_clause) = node.else_clause() {
            if let Some(else_stmts) = else_clause.statements() {
                self.in_if_body = else_stmts.body().len() <= 1;
                self.visit(&else_stmts.as_node());
            }
        }

        self.in_if_body = was;
    }

    // Rescue modifier: `include Foo rescue Bar` wraps the include call
    // in a RescueModifierNode. The call's end_offset is at `Foo`, but the
    // line continues with `rescue Bar`. Without special handling,
    // line_has_trailing_code would bail out. Pass the rescue modifier's
    // end_offset so the trailing-code check is skipped and the line-end
    // position is correct.
    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        let expr = node.expression();
        if let Some(call) = expr.as_call_node() {
            self.check_call(&call, Some(node.location().end_offset()));
        } else {
            self.visit(&expr);
        }
        self.visit(&node.rescue_expression());
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAfterModuleInclusion,
        "cops/layout/empty_lines_after_module_inclusion"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAfterModuleInclusion,
        "cops/layout/empty_lines_after_module_inclusion"
    );
}
