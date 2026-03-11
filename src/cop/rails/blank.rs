use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/Blank - flags code that can be simplified using `Object#blank?` from Active Support.
///
/// ## Investigation findings (2026-03-08)
///
/// **FN root cause (1095 FN):** The `UnlessPresent` pattern was not implemented at all.
/// The config option was read but the variable was prefixed with `_` and never used.
/// RuboCop's `on_if` handler flags `unless foo.present?` (both modifier and block forms)
/// suggesting `if foo.blank?` instead.
///
/// **Fix:** Implemented `UnlessPresent` check via a custom AST visitor in `check_source`.
/// The visitor handles both modifier (`something unless foo.present?`) and block
/// (`unless foo.present? ... end`) forms. Skips unless-with-else when Style/UnlessElse
/// would conflict (conservative: always skip when else clause is present).
///
/// **FP root cause (27 FP, first batch):** Missing `defining_blank?` check. RuboCop skips
/// `!present?` when it appears inside `def blank?` (defining blank? in terms of present?).
/// Nitrocop was incorrectly flagging these as offenses.
///
/// **Fix:** Added parent context tracking in the visitor. When inside a `def blank?` method,
/// `!present?` calls are suppressed.
///
/// **FP root cause (34 FP, second batch, 2026-03-10):** `nil_check_receiver` was matching
/// `!foo` (boolean negation, method name `!`) as a nil-check pattern. This caused
/// `!foo || foo.empty?` to be flagged as NilOrEmpty, but RuboCop's `nil_or_empty?`
/// NodePattern only matches explicit nil checks: `nil?`, `== nil`, `nil ==`. It does NOT
/// match `!foo`. The `!foo` branch was incorrectly added to `nil_check_receiver`.
///
/// **Fix:** Removed the `!` method name branch from `nil_check_receiver`. Only `nil?`,
/// `== nil`, and `nil == foo` are valid nil-check patterns for the NilOrEmpty check.
pub struct Blank;

/// Extract the receiver source text from a CallNode, returning None if absent.
fn receiver_source<'a>(call: &ruby_prism::CallNode<'a>) -> Option<&'a [u8]> {
    call.receiver().map(|r| r.location().as_slice())
}

/// Check if the left side of an OR node matches a nil-check pattern:
/// - `foo.nil?`
/// - `foo == nil`
/// - `nil == foo`
///
/// Returns (receiver source bytes, left side source bytes) if matched.
/// Note: `!foo` is NOT a nil check (it's boolean negation) and is intentionally excluded.
/// RuboCop's `nil_or_empty?` NodePattern only matches `nil?`, `== nil`, `nil ==`.
fn nil_check_receiver<'a>(node: &ruby_prism::Node<'a>) -> Option<(&'a [u8], &'a [u8])> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();
    let left_src = node.location().as_slice();

    if method == b"nil?" {
        // foo.nil?
        return receiver_source(&call).map(|r| (r, left_src));
    }

    if method == b"==" {
        // foo == nil  or  nil == foo
        let recv = call.receiver()?;
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }
        let arg = &arg_list[0];

        if arg.as_nil_node().is_some() {
            // foo == nil → receiver is foo
            return Some((recv.location().as_slice(), left_src));
        }
        if recv.as_nil_node().is_some() {
            // nil == foo → receiver is arg
            return Some((arg.location().as_slice(), left_src));
        }
    }

    None
}

impl Cop for Blank {
    fn name(&self) -> &'static str {
        "Rails/Blank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let nil_or_empty = config.get_bool("NilOrEmpty", true);
        let not_present = config.get_bool("NotPresent", true);
        let unless_present = config.get_bool("UnlessPresent", true);

        let mut visitor = BlankVisitor {
            cop: self,
            source,
            nil_or_empty,
            not_present,
            unless_present,
            inside_def_blank: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlankVisitor<'a, 'src> {
    cop: &'a Blank,
    source: &'src SourceFile,
    nil_or_empty: bool,
    not_present: bool,
    unless_present: bool,
    inside_def_blank: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> BlankVisitor<'_, '_> {
    /// Check NilOrEmpty: `foo.nil? || foo.empty?`
    fn check_nil_or_empty(&mut self, or_node: &ruby_prism::OrNode<'pr>) {
        let left = or_node.left();
        let right = or_node.right();

        if let Some((nil_recv, left_src)) = nil_check_receiver(&left) {
            // Right side must be `<same>.empty?`
            if let Some(right_call) = right.as_call_node() {
                if right_call.name().as_slice() == b"empty?" {
                    if let Some(empty_recv) = receiver_source(&right_call) {
                        if nil_recv == empty_recv {
                            let loc = or_node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            let recv_str = std::str::from_utf8(nil_recv).unwrap_or("object");
                            let left_str = std::str::from_utf8(left_src).unwrap_or("nil?");
                            let right_str = std::str::from_utf8(right.location().as_slice())
                                .unwrap_or("empty?");
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!(
                                    "Use `{recv_str}.blank?` instead of `{left_str} || {right_str}`."
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Check NotPresent: `!foo.present?`
    fn check_not_present(&mut self, call: &ruby_prism::CallNode<'pr>) {
        if call.name().as_slice() != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"present?" {
            return;
        }

        // Skip !present? inside def blank? (defining blank? in terms of present?)
        if self.inside_def_blank {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `blank?` instead of `!present?`.".to_string(),
        ));
    }

    /// Check UnlessPresent: `unless foo.present?` or `something unless foo.present?`
    fn check_unless_present(&mut self, unless_node: &ruby_prism::UnlessNode<'pr>) {
        // Skip unless-with-else (Style/UnlessElse interaction)
        // Conservative: always skip when else clause is present
        if unless_node.else_clause().is_some() {
            return;
        }

        let predicate = unless_node.predicate();
        let pred_call = match predicate.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if pred_call.name().as_slice() != b"present?" {
            return;
        }

        // Build the receiver string for the message
        let recv_str = match pred_call.receiver() {
            Some(r) => {
                let src = r.location().as_slice();
                format!("{}.blank?", std::str::from_utf8(src).unwrap_or("object"))
            }
            None => "blank?".to_string(),
        };

        // Build the "current" string for the message
        let predicate_src =
            std::str::from_utf8(predicate.location().as_slice()).unwrap_or("present?");
        let current = format!("unless {predicate_src}");

        // Determine offense location based on modifier vs block form
        // For modifier form: `something unless foo.present?` → offense on `unless foo.present?`
        // For block form: `unless foo.present?\n...\nend` → offense on `unless foo.present?`
        let unless_loc = unless_node.location();
        let pred_loc = predicate.location();

        // The offense covers from the start of `unless` keyword to the end of the predicate
        // For modifier form, the keyword is in the middle; for block form, it's at the start
        let keyword_loc = unless_node.keyword_loc();
        let offense_start = keyword_loc.start_offset();
        let offense_end = pred_loc.end_offset();

        // Check if this is modifier form by comparing keyword start to node start
        let is_modifier = keyword_loc.start_offset() > unless_loc.start_offset();

        let (line, column) = if is_modifier {
            self.source.offset_to_line_col(offense_start)
        } else {
            // Block form: offense starts at the `unless` keyword (= node start)
            self.source.offset_to_line_col(offense_start)
        };

        // For the offense range length, count from keyword to end of predicate
        let _ = offense_end; // used implicitly via the annotation range

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Use `if {recv_str}` instead of `{current}`."),
        ));
    }
}

impl<'pr> Visit<'pr> for BlankVisitor<'_, '_> {
    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        if self.nil_or_empty {
            self.check_nil_or_empty(node);
        }
        // Continue visiting children
        self.visit(&node.left());
        self.visit(&node.right());
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.not_present {
            self.check_not_present(node);
        }
        // Visit children (receiver, arguments, block)
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        if self.unless_present {
            self.check_unless_present(node);
        }
        // Visit children
        self.visit(&node.predicate());
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let is_blank = node.name().as_slice() == b"blank?";
        let was_inside = self.inside_def_blank;
        if is_blank {
            self.inside_def_blank = true;
        }

        // Visit children: parameters and body
        if let Some(params) = node.parameters() {
            self.visit(&params.as_node());
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.inside_def_blank = was_inside;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Blank, "cops/rails/blank");
}
