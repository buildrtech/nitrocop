use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

// Corpus investigation: FP=2, FN=250. Root cause: last_match_used_in_scope checked
// if ANY MatchData ref in the same scope had offset >= if_node_offset, without
// limiting the search to the range before the next match node. This caused all
// matches in a scope to be suppressed if any one of them had a MatchData ref
// after it. Fixed by collecting all match condition positions upfront and computing
// the range [if_node_offset, next_match_offset) for each match, matching RuboCop's
// range_to_search_for_last_matches / next_match_pos logic.

pub struct RegexpMatch;

impl Cop for RegexpMatch {
    fn name(&self) -> &'static str {
        "Performance/RegexpMatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Pass 1: Collect all MatchData reference positions with their scope info
        let mut ref_collector = MatchDataRefCollector {
            refs: Vec::new(),
            current_scope: None,
        };
        ref_collector.visit(&parse_result.node());

        // Pass 2: Collect all match condition positions (=~, !~, .match, ===)
        // so we can compute the "next match" boundary for each match.
        let mut match_collector = MatchConditionCollector {
            positions: Vec::new(),
            current_scope: None,
        };
        match_collector.visit(&parse_result.node());

        // Pass 3: Visit conditions and check for matches
        let mut visitor = ConditionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            match_data_refs: ref_collector.refs,
            match_positions: match_collector.positions,
            current_scope: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// A scope boundary (def, class, module) identified by byte offset range.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ScopeId {
    start: usize,
    end: usize,
}

/// A reference to MatchData ($~, $1, $&, etc.) with its scope info.
struct MatchDataRef {
    offset: usize,
    scope: Option<ScopeId>,
}

/// A match condition position (=~, !~, .match, ===) in an if/unless/case condition,
/// with the if_node_offset (start of enclosing if/unless/case for modifier form handling).
struct MatchConditionPos {
    /// Start of the if/unless/case node (for modifier form: includes the body before `if`)
    if_node_offset: usize,
    scope: Option<ScopeId>,
}

/// Pass 1: Collect all MatchData references ($~, $1, $&, $', $`, $+,
/// $MATCH, $PREMATCH, $POSTMATCH, $LAST_PAREN_MATCH, Regexp.last_match).
struct MatchDataRefCollector {
    refs: Vec<MatchDataRef>,
    current_scope: Option<ScopeId>,
}

impl<'pr> Visit<'pr> for MatchDataRefCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_def_node(self, node);
        self.current_scope = old;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_class_node(self, node);
        self.current_scope = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_module_node(self, node);
        self.current_scope = old;
    }

    fn visit_back_reference_read_node(&mut self, node: &ruby_prism::BackReferenceReadNode<'pr>) {
        // $&, $`, $', $+, $~
        self.refs.push(MatchDataRef {
            offset: node.location().start_offset(),
            scope: self.current_scope,
        });
    }

    fn visit_numbered_reference_read_node(
        &mut self,
        node: &ruby_prism::NumberedReferenceReadNode<'pr>,
    ) {
        // $1, $2, ..., $100, etc.
        self.refs.push(MatchDataRef {
            offset: node.location().start_offset(),
            scope: self.current_scope,
        });
    }

    fn visit_global_variable_read_node(&mut self, node: &ruby_prism::GlobalVariableReadNode<'pr>) {
        // $~, $MATCH, $PREMATCH, $POSTMATCH, $LAST_PAREN_MATCH, $LAST_MATCH_INFO
        let name = node.name().as_slice();
        if name == b"$~"
            || name == b"$MATCH"
            || name == b"$PREMATCH"
            || name == b"$POSTMATCH"
            || name == b"$LAST_PAREN_MATCH"
            || name == b"$LAST_MATCH_INFO"
        {
            self.refs.push(MatchDataRef {
                offset: node.location().start_offset(),
                scope: self.current_scope,
            });
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Regexp.last_match or ::Regexp.last_match
        if node.name().as_slice() == b"last_match" {
            if let Some(recv) = node.receiver() {
                let is_regexp_const = recv
                    .as_constant_read_node()
                    .is_some_and(|c| c.name().as_slice() == b"Regexp")
                    || recv.as_constant_path_node().is_some_and(|cp| {
                        cp.name().is_some_and(|n| n.as_slice() == b"Regexp")
                            && cp.parent().is_none()
                    });
                if is_regexp_const {
                    self.refs.push(MatchDataRef {
                        offset: node.location().start_offset(),
                        scope: self.current_scope,
                    });
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

/// Pass 2: Collect positions of all match conditions (=~, !~, .match, ===) in
/// if/unless/case conditions. Used to compute the "next match" boundary.
struct MatchConditionCollector {
    positions: Vec<MatchConditionPos>,
    current_scope: Option<ScopeId>,
}

impl MatchConditionCollector {
    fn record_condition(&mut self, cond: &ruby_prism::Node<'_>, if_node_offset: usize) {
        if let Some(call) = cond.as_call_node() {
            let method = call.name().as_slice();
            let is_match_cond = if method == b"=~" || method == b"!~" {
                call.receiver().is_some()
            } else if method == b"match" {
                // Same filtering as check_match_method
                if let (Some(recv), Some(args)) = (call.receiver(), call.arguments()) {
                    let first_arg = args.arguments().iter().next();
                    let recv_lit = is_match_literal(&recv);
                    let arg_lit = first_arg.as_ref().is_some_and(is_match_literal);
                    (recv_lit || arg_lit) && call.block().is_none()
                } else {
                    false
                }
            } else if method == b"===" {
                call.receiver()
                    .is_some_and(|r| r.as_regular_expression_node().is_some())
                    && call.arguments().is_some()
            } else {
                false
            };
            if is_match_cond {
                self.positions.push(MatchConditionPos {
                    if_node_offset,
                    scope: self.current_scope,
                });
            }
        }
        // Also check for MatchWriteNode (named captures) — these count as "matches"
        // that reset MatchData, so they serve as boundaries.
        if cond.as_match_write_node().is_some() {
            self.positions.push(MatchConditionPos {
                if_node_offset,
                scope: self.current_scope,
            });
        }
    }
}

impl<'pr> Visit<'pr> for MatchConditionCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_def_node(self, node);
        self.current_scope = old;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_class_node(self, node);
        self.current_scope = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_module_node(self, node);
        self.current_scope = old;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let if_start = node.location().start_offset();
        self.record_condition(&node.predicate(), if_start);
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let unless_start = node.location().start_offset();
        self.record_condition(&node.predicate(), unless_start);
        ruby_prism::visit_unless_node(self, node);
    }

    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        if node.predicate().is_none() {
            let case_start = node.location().start_offset();
            for condition in node.conditions().iter() {
                if let Some(when_node) = condition.as_when_node() {
                    for when_cond in when_node.conditions().iter() {
                        self.record_condition(&when_cond, case_start);
                    }
                }
            }
        }
        ruby_prism::visit_case_node(self, node);
    }
}

struct ConditionVisitor<'a, 'src> {
    cop: &'a RegexpMatch,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    match_data_refs: Vec<MatchDataRef>,
    match_positions: Vec<MatchConditionPos>,
    current_scope: Option<ScopeId>,
}

impl<'pr> Visit<'pr> for ConditionVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_def_node(self, node);
        self.current_scope = old;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_class_node(self, node);
        self.current_scope = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_module_node(self, node);
        self.current_scope = old;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let if_start = node.location().start_offset();
        check_condition(
            self.cop,
            self.source,
            &node.predicate(),
            if_start,
            &self.match_data_refs,
            &self.match_positions,
            self.current_scope,
            &mut self.diagnostics,
        );
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let unless_start = node.location().start_offset();
        check_condition(
            self.cop,
            self.source,
            &node.predicate(),
            unless_start,
            &self.match_data_refs,
            &self.match_positions,
            self.current_scope,
            &mut self.diagnostics,
        );
        ruby_prism::visit_unless_node(self, node);
    }

    // RuboCop only checks on_if (covers if/unless/elsif/ternary) and on_case.
    // It does NOT check while/until conditions.

    // In pattern matching `case/in`, the guard `if`/`unless` is embedded as an
    // IfNode/UnlessNode inside InNode.pattern(). The default visitor would descend
    // into these and treat the guard condition as a regular if-condition. RuboCop's
    // `on_if` does NOT fire for pattern matching guards, so we skip the pattern
    // and only visit the body (statements).
    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // RuboCop only checks case-less when (i.e., `case\n when cond\n ...`)
        if node.predicate().is_none() {
            let case_start = node.location().start_offset();
            for condition in node.conditions().iter() {
                if let Some(when_node) = condition.as_when_node() {
                    for when_cond in when_node.conditions().iter() {
                        check_condition(
                            self.cop,
                            self.source,
                            &when_cond,
                            case_start,
                            &self.match_data_refs,
                            &self.match_positions,
                            self.current_scope,
                            &mut self.diagnostics,
                        );
                    }
                }
            }
        }
        ruby_prism::visit_case_node(self, node);
    }
}

/// Check a condition expression for =~, !~, .match(), or === usage.
/// `if_node_offset` is the start of the enclosing if/unless/case node,
/// used for modifier-form MatchData detection.
#[allow(clippy::too_many_arguments)]
fn check_condition(
    cop: &RegexpMatch,
    source: &SourceFile,
    cond: &ruby_prism::Node<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchConditionPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(call) = cond.as_call_node() {
        let method = call.name().as_slice();

        if method == b"=~" || method == b"!~" {
            check_match_operator(
                cop,
                source,
                &call,
                method,
                if_node_offset,
                match_data_refs,
                match_positions,
                current_scope,
                diagnostics,
            );
        } else if method == b"match" {
            check_match_method(
                cop,
                source,
                &call,
                if_node_offset,
                match_data_refs,
                match_positions,
                current_scope,
                diagnostics,
            );
        } else if method == b"===" {
            check_threequals(
                cop,
                source,
                &call,
                if_node_offset,
                match_data_refs,
                match_positions,
                current_scope,
                diagnostics,
            );
        }
    }
    // MatchWriteNode (/(?<name>...)/ =~ expr) is handled by NOT matching it here —
    // named captures create local vars, so they should not be flagged.
    // NOTE: RuboCop only checks the top-level condition expression, not
    // sub-expressions within && or || chains. We match that behavior.
}

/// Check if MatchData is used in the same scope as a match at the given offset.
///
/// RuboCop's logic: search for MatchData refs in the range from the match position
/// (or if_branch start for modifier forms) to the NEXT match position in the same
/// scope. This ensures that a MatchData ref after a later match doesn't suppress
/// an earlier match that has nothing to do with it.
///
/// `if_node_offset` is the start of the enclosing if/unless node (for modifier forms,
/// this includes the body before `if`, e.g., `return $1 if x =~ /re/`).
fn last_match_used_in_scope(
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchConditionPos],
    current_scope: Option<ScopeId>,
) -> bool {
    // Find the next match position in the same scope after this one.
    // "After" means a match whose if_node_offset > this match's if_node_offset.
    let next_match_offset = match_positions
        .iter()
        .filter(|m| m.scope == current_scope && m.if_node_offset > if_node_offset)
        .map(|m| m.if_node_offset)
        .min()
        .unwrap_or(usize::MAX);

    // Check if any MatchData ref in the same scope falls within
    // [if_node_offset, next_match_offset).
    for r in match_data_refs {
        if r.scope == current_scope && r.offset >= if_node_offset && r.offset < next_match_offset {
            return true;
        }
    }
    false
}

/// Check =~ or !~ operator usage.
#[allow(clippy::too_many_arguments)]
fn check_match_operator(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    method: &[u8],
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchConditionPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip if either side is nil (shouldn't happen for =~/!~ but be safe)
    if call.receiver().is_none() {
        return;
    }

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        if_node_offset,
        match_data_refs,
        match_positions,
        current_scope,
    ) {
        return;
    }

    let op_str = if method == b"!~" { "!~" } else { "=~" };
    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!(
            "Use `match?` instead of `{}` when `MatchData` is not used.",
            op_str
        ),
    ));
}

/// Check .match() method call usage.
#[allow(clippy::too_many_arguments)]
fn check_match_method(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchConditionPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must have a receiver (x.match)
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return,
    };

    // Must have arguments (x.match(y) or x.match(y, pos))
    let arguments = match call.arguments() {
        Some(a) => a,
        None => return,
    };

    let first_arg = match arguments.arguments().iter().next() {
        Some(a) => a,
        None => return,
    };

    // RuboCop requires at least one side to be a regexp, string, or symbol literal
    let recv_is_literal = is_match_literal(&receiver);
    let arg_is_literal = is_match_literal(&first_arg);

    if !recv_is_literal && !arg_is_literal {
        return;
    }

    // Don't flag if the call has a block
    if call.block().is_some() {
        return;
    }

    // Skip safe navigation (&.match)
    if let Some(op) = call.call_operator_loc() {
        let bytes = &source.as_bytes()[op.start_offset()..op.end_offset()];
        if bytes == b"&." {
            return;
        }
    }

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        if_node_offset,
        match_data_refs,
        match_positions,
        current_scope,
    ) {
        return;
    }

    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Use `match?` instead of `match` when `MatchData` is not used.".to_string(),
    ));
}

/// Check === with regexp literal on LHS.
#[allow(clippy::too_many_arguments)]
fn check_threequals(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchConditionPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // RuboCop only flags /re/ === foo (regexp literal on LHS)
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return,
    };

    // Must have an argument
    if call.arguments().is_none() {
        return;
    }

    // Check receiver is a regexp literal (simple or with flags, not interpolated)
    if receiver.as_regular_expression_node().is_none() {
        return;
    }

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        if_node_offset,
        match_data_refs,
        match_positions,
        current_scope,
    ) {
        return;
    }

    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Use `match?` instead of `===` when `MatchData` is not used.".to_string(),
    ));
}

/// Check if a node is a regexp, string, or symbol literal.
fn is_match_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_symbol_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RegexpMatch, "cops/performance/regexp_match");
}
