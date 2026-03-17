use std::collections::{HashMap, HashSet};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for block parameters or block-local variables that shadow outer local variables.
///
/// ## Root causes of historical FP/FN (corpus conformance ~57%):
///
/// 1. **FP: Variable added to scope before RHS visited.** `visit_local_variable_write_node`
///    called `add_local` before visiting the value child. This caused `foo = bar { |foo| ... }`
///    to incorrectly flag `foo` as shadowing, because the LHS `foo` was already in scope when
///    the block was processed. RuboCop's VariableForce processes the RHS before declaring the
///    variable, so `foo` isn't in scope yet. Fix: visit the value first, then add to scope.
///
/// 2. **FN: Overly aggressive conditional suppression.** The `is_different_conditional_branch`
///    function had a `(None, Some(_)) => true` case that suppressed ALL shadowing when the
///    block was inside any conditional but the outer var was not. Per RuboCop, suppression
///    only applies when BOTH the outer var and the block are in different branches of the
///    SAME conditional node. Fix: remove the incorrect `(None, Some(_))` case.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=19, FN=51.
///
/// FP:
/// - Method parameters were predeclared before any default expression ran, so later
///   parameters leaked into earlier lambda defaults like
///   `outer: ->(cursor) { ... }, cursor: nil`.
/// - Class/module/singleton class bodies only pushed a nested scope, so top-level locals
///   leaked into class-body procs and lambdas.
///
/// FN:
/// - `params.posts()` and `params.keyword_rest()` were not checked or collected, so shadowing
///   was missed for post-splat params and `**kwargs`.
/// - Lambda/block body scopes also omitted some parameter kinds, so nested blocks could miss
///   outer block params.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=10, FN=42.
///
/// FP fixes applied:
/// - **Ractor.new block detection**: `is_ractor_new_call` detects `Ractor.new(...)` calls
///   and handles their blocks with an isolated scope (no shadowing check). RuboCop
///   explicitly skips Ractor blocks because Ractors cannot access outer scope, so
///   shadowing is intentional. Previously `is_ractor_new_block` was stubbed out.
///   Implementation uses `visit_call_node` override since Prism's BlockNode lacks
///   parent pointers.
/// - **When-condition assignment suppression**: Variables assigned in `when` conditions
///   (e.g., `when decl = env.fetch(...)`) are now marked with `when_condition_of_case`.
///   Blocks in the same `when` body that reuse the variable name are suppressed.
///   This matches RuboCop's `same_conditions_node_different_branch?` logic where both
///   the block and the outer variable resolve to the same conditional (case) node.
///
/// ## Corpus investigation (2026-03-17)
///
/// Corpus oracle reported FP=4, FN=26. After fixes: FP=0, FN=18.
///
/// FP fixes applied:
/// - **Else clause suppression**: Added `in_else_of_if` tracking. When both the outer
///   variable and block are in the same else clause, suppresses shadowing. Matches
///   RuboCop's `variable_node == outer_local_variable_node.else_branch` check.
///   Only set for actual else clauses, not elsif chains.
/// - **If-predicate suppression**: Added `in_predicate_of_if` tracking. When the outer
///   variable was assigned in an if/unless predicate (`if item = expr`) and the block
///   is in the body or else of the same conditional, suppresses shadowing. Matches
///   RuboCop's `variable_node == outer_local_variable_node` check.
/// - **Adjacent elsif predicate refinement**: Skip adjacent elsif suppression when BOTH
///   the outer variable and block are in predicates of adjacent elsifs. In Parser AST,
///   the block inside `elsif x = find { |x| }` has a send parent (not the elsif node),
///   so RuboCop's else_branch check doesn't match.
/// - **Variable reassignment context preservation**: `add_local` now preserves the first
///   assignment's conditional context on reassignment. Fixes FP where `node = x;
///   node = y while cond` changed the variable's context from case/when to while.
///
/// FN fixes applied:
/// - **Non-adjacent elsif branches**: Variables from 2+ elsifs apart now correctly
///   shadow (e.g., `e` in elsif #2 shadowed by `|e|` in elsif #4).
/// - **While-loop variable in else**: `part` assigned in `while` inside if-body correctly
///   shadows `|part|` in the else clause of the same if.
/// - **Block body scope inherits else context**: `build_block_body_scope` now receives
///   `in_else_of_if` so block params get the correct else context for nested blocks.
///
/// Remaining FN (18): Various patterns including lambda scoping, complex control flow,
/// and edge cases in VariableForce's variable_table lookup order.
pub struct ShadowingOuterLocalVariable;

impl Cop for ShadowingOuterLocalVariable {
    fn name(&self) -> &'static str {
        "Lint/ShadowingOuterLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    /// This cop is disabled by default in RuboCop (Enabled: false).
    fn default_enabled(&self) -> bool {
        false
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
        let mut visitor = ShadowVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            conditional_branch_stack: Vec::new(),
            when_condition_case_offset: None,
            in_when_body_of_case: None,
            in_else_of_if: None,
            in_predicate_of_if: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Info about where a variable was declared.
#[derive(Clone, Debug)]
struct VarInfo {
    /// If the variable was declared inside a `when`/`if`/`else` branch,
    /// this is the (cond_node_offset, branch_offset) pair for the NEAREST
    /// conditional. Used to skip shadowing when block and outer var are in
    /// different branches of the same conditional.
    conditional_branch: Option<(usize, usize)>,
    /// The offset of the nearest conditional's subsequent/else clause.
    /// Used to check adjacent elsif suppression: if a block's nearest
    /// conditional starts at this offset, the block is in the immediate
    /// else/elsif of the variable's conditional → suppress.
    cond_subsequent_offset: Option<usize>,
    /// If the variable was assigned inside a `when` condition (not body),
    /// this is the case node offset. Used to suppress shadowing when a
    /// block in the same `when` body reuses the variable name — matching
    /// RuboCop's VariableForce behavior where both resolve to the same
    /// conditional (case) node.
    when_condition_of_case: Option<usize>,
    /// If the variable is inside an else/elsif clause of an if/unless,
    /// this is the offset of the if/unless node that owns the else clause.
    /// RuboCop suppresses shadowing when `variable_node == olv.else_branch`,
    /// which fires when the block's parent IS the else clause body.
    in_else_of_if: Option<usize>,
    /// If the variable was assigned in the predicate of an if/unless
    /// (e.g., `if item = expr`), this is the offset of that if/unless node.
    /// RuboCop suppresses when `variable_node == outer_local_variable_node`,
    /// which fires when the block's parent IS the same conditional.
    in_predicate_of_if: Option<usize>,
}

struct ShadowVisitor<'a, 'src> {
    cop: &'a ShadowingOuterLocalVariable,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Stack of maps of local variable names -> declaration info.
    scopes: Vec<HashMap<String, VarInfo>>,
    /// Stack of (cond_node_offset, branch_offset, subsequent_offset) for current
    /// when/if/else branch. The subsequent_offset is the start of the conditional's
    /// else/elsif clause (if any), used for adjacent-branch suppression.
    conditional_branch_stack: Vec<(usize, usize, Option<usize>)>,
    /// When visiting a `when` condition, the case node offset.
    /// Variables assigned while this is Some are marked as when-condition vars.
    when_condition_case_offset: Option<usize>,
    /// When inside a `when` body, the case node offset for suppression checks.
    in_when_body_of_case: Option<usize>,
    /// When inside an else/elsif clause, the if/unless node offset.
    /// Variables assigned here get `in_else_of_if` set.
    in_else_of_if: Option<usize>,
    /// When visiting an if/unless predicate, the if/unless node offset.
    /// Variables assigned here get `in_predicate_of_if` set.
    in_predicate_of_if: Option<usize>,
}

impl ShadowVisitor<'_, '_> {
    fn current_locals(&self) -> HashMap<String, VarInfo> {
        let mut all = HashMap::new();
        for scope in &self.scopes {
            for (name, info) in scope {
                all.insert(name.clone(), info.clone());
            }
        }
        all
    }

    fn add_local(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            // If the variable already exists in this scope, preserve its
            // original conditional context. RuboCop's VariableForce uses
            // the FIRST declaration's conditional ancestor, not the latest
            // reassignment. Without this, `node = x; node = y while cond`
            // would change node's conditional from case/when to while.
            if scope.contains_key(name) {
                return;
            }
            let last = self.conditional_branch_stack.last();
            let info = VarInfo {
                conditional_branch: last.map(|&(c, b, _)| (c, b)),
                cond_subsequent_offset: last.and_then(|&(_, _, s)| s),
                when_condition_of_case: self.when_condition_case_offset,
                in_else_of_if: self.in_else_of_if,
                in_predicate_of_if: self.in_predicate_of_if,
            };
            scope.insert(name.to_string(), info);
        }
    }

    fn current_conditional_branch(&self) -> Option<(usize, usize)> {
        self.conditional_branch_stack
            .last()
            .map(|&(c, b, _)| (c, b))
    }

    /// Visit an if/elsif/else node. Each IfNode uses its own offset as the
    /// conditional identity. The then-body and else/subsequent share this offset
    /// but with different branch_offsets. This matches RuboCop's Parser-gem
    /// behavior where each elsif is a nested if node.
    ///
    /// Adjacent elsif suppression is handled via `cond_subsequent_offset`:
    /// variables in the then-body record the subsequent's offset so that
    /// blocks in the immediate next elsif can be recognized as "adjacent."
    fn visit_if_node_impl(&mut self, node: &ruby_prism::IfNode<'_>) {
        let if_offset = node.location().start_offset();
        let subsequent_offset = node.subsequent().map(|s| s.location().start_offset());

        // Compute then-body branch offset for predicate context.
        // Variables assigned in `if x = expr` are treated as belonging to
        // the then-body branch. This ensures:
        //   - block in same if-body → same branch → offense reported
        //   - block in else/elsif  → different branch → suppressed
        let then_branch_offset = node
            .statements()
            .map(|s| s.location().start_offset())
            .unwrap_or(if_offset);

        // Visit predicate with the then-body's conditional context.
        // Variables assigned in the predicate (e.g., `if item = expr`) get
        // in_predicate_of_if set, enabling suppression when a block in the
        // same if body reuses the variable name.
        self.conditional_branch_stack
            .push((if_offset, then_branch_offset, subsequent_offset));
        let saved_pred = self.in_predicate_of_if;
        self.in_predicate_of_if = Some(if_offset);
        self.visit(&node.predicate());
        self.in_predicate_of_if = saved_pred;
        self.conditional_branch_stack.pop();

        // Visit then-body with the same branch tracking
        if let Some(stmts) = node.statements() {
            self.conditional_branch_stack
                .push((if_offset, then_branch_offset, subsequent_offset));
            self.visit_statements_node(&stmts);
            self.conditional_branch_stack.pop();
        }

        // Visit else/elsif — set in_else_of_if only for actual else clauses,
        // NOT for elsif chains. For elsif, the nested `visit_if_node_impl`
        // will set in_else_of_if for its own else clause if present.
        if let Some(subsequent) = node.subsequent() {
            if let Some(elsif_node) = subsequent.as_if_node() {
                // elsif — do NOT set in_else_of_if here. The elsif is a
                // separate conditional, not the else body.
                let branch_offset = subsequent.location().start_offset();
                self.conditional_branch_stack
                    .push((if_offset, branch_offset, None));
                self.visit_if_node_impl(&elsif_node);
                self.conditional_branch_stack.pop();
            } else {
                // else clause — set in_else_of_if for this if
                let saved_else = self.in_else_of_if;
                self.in_else_of_if = Some(if_offset);
                let branch_offset = subsequent.location().start_offset();
                self.conditional_branch_stack
                    .push((if_offset, branch_offset, None));
                self.visit(&subsequent);
                self.conditional_branch_stack.pop();
                self.in_else_of_if = saved_else;
            }
        }
    }

    /// Visit a when node, tracking when-condition vs when-body context.
    /// Variables assigned in when conditions are marked with `when_condition_of_case`
    /// so that blocks in the same when body don't report false-positive shadowing.
    fn visit_when_node_with_case_offset(
        &mut self,
        node: &ruby_prism::WhenNode<'_>,
        case_offset: usize,
    ) {
        // Visit when conditions with when_condition_case_offset set
        let saved = self.when_condition_case_offset;
        self.when_condition_case_offset = Some(case_offset);
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        self.when_condition_case_offset = saved;

        // Visit when body with in_when_body_of_case set
        if let Some(stmts) = node.statements() {
            let saved_body = self.in_when_body_of_case;
            self.in_when_body_of_case = Some(case_offset);
            self.visit_statements_node(&stmts);
            self.in_when_body_of_case = saved_body;
        }
    }

    fn visit_def_parameters_in_order(&mut self, params: &ruby_prism::ParametersNode<'_>) {
        for param in params.requireds().iter() {
            self.declare_parameter_node(&param);
        }

        for param in params.optionals().iter() {
            if let Some(optional) = param.as_optional_parameter_node() {
                self.visit(&optional.value());
                if let Ok(name) = std::str::from_utf8(optional.name().as_slice()) {
                    self.add_local(name);
                }
            }
        }

        if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                if let Some(name) = rest_param.name() {
                    if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                        self.add_local(name);
                    }
                }
            }
        }

        for param in params.posts().iter() {
            self.declare_parameter_node(&param);
        }

        for param in params.keywords().iter() {
            if let Some(keyword) = param.as_required_keyword_parameter_node() {
                if let Ok(name) = std::str::from_utf8(keyword.name().as_slice()) {
                    self.add_local(name.trim_end_matches(':'));
                }
            } else if let Some(keyword) = param.as_optional_keyword_parameter_node() {
                self.visit(&keyword.value());
                if let Ok(name) = std::str::from_utf8(keyword.name().as_slice()) {
                    self.add_local(name.trim_end_matches(':'));
                }
            }
        }

        if let Some(keyword_rest) = params.keyword_rest() {
            if let Some(keyword_rest) = keyword_rest.as_keyword_rest_parameter_node() {
                if let Some(name) = keyword_rest.name() {
                    if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                        self.add_local(name);
                    }
                }
            }
        }

        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                    self.add_local(name);
                }
            }
        }
    }

    fn declare_parameter_node(&mut self, node: &ruby_prism::Node<'_>) {
        if let Some(required) = node.as_required_parameter_node() {
            if let Ok(name) = std::str::from_utf8(required.name().as_slice()) {
                self.add_local(name);
            }
            return;
        }

        if let Some(multi_target) = node.as_multi_target_node() {
            let mut names = HashSet::new();
            collect_multi_target_names(&multi_target, &mut names);
            for name in names {
                self.add_local(&name);
            }
            return;
        }

        if let Some(keyword_rest) = node.as_keyword_rest_parameter_node() {
            if let Some(name) = keyword_rest.name() {
                if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                    self.add_local(name);
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for ShadowVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }

        // def creates a completely new scope — save and replace the entire scope
        // stack. RuboCop's VariableForce treats method definitions as scope
        // barriers: class/module-level variables are NOT visible inside methods.
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_cond = std::mem::take(&mut self.conditional_branch_stack);
        self.scopes.push(HashMap::new());
        if let Some(params) = node.parameters() {
            self.visit_def_parameters_in_order(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
        self.conditional_branch_stack = saved_cond;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.visit(&node.constant_path());
        if let Some(superclass) = node.superclass() {
            self.visit(&superclass);
        }
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(HashMap::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.visit(&node.constant_path());
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(HashMap::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // Visit the value (RHS) BEFORE adding the variable to scope.
        // This matches RuboCop's VariableForce which processes the RHS before
        // declaring the LHS variable. Without this ordering, patterns like
        // `foo = bar { |foo| baz(foo) }` would incorrectly flag `foo` as
        // shadowing because the LHS `foo` would already be in scope.
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        // Visit the value (RHS) first before adding targets to scope
        self.visit(&node.value());
        // Then add all target locals to scope
        for target in node.lefts().iter() {
            if let Some(local) = target.as_local_variable_target_node() {
                let name = std::str::from_utf8(local.name().as_slice())
                    .unwrap_or("")
                    .to_string();
                self.add_local(&name);
            }
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(local) = expr.as_local_variable_target_node() {
                        let name = std::str::from_utf8(local.name().as_slice())
                            .unwrap_or("")
                            .to_string();
                        self.add_local(&name);
                    }
                }
            }
        }
        for target in node.rights().iter() {
            if let Some(local) = target.as_local_variable_target_node() {
                let name = std::str::from_utf8(local.name().as_slice())
                    .unwrap_or("")
                    .to_string();
                self.add_local(&name);
            }
        }
    }

    // Singleton class (class << self) creates a new scope
    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.visit(&node.expression());
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(HashMap::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // For Ractor.new calls, handle the block with an isolated scope
        // (no shadowing check) since Ractor intentionally shadows outer vars.
        if is_ractor_new_call(node) {
            // Visit receiver and arguments normally
            if let Some(receiver) = node.receiver() {
                self.visit(&receiver);
            }
            if let Some(arguments) = node.arguments() {
                self.visit_arguments_node(&arguments);
            }
            // Visit block with isolated scope (no shadowing check)
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    self.scopes.push(HashMap::new());
                    ruby_prism::visit_block_node(self, &block_node);
                    self.scopes.pop();
                }
            }
            return;
        }
        // Default call node visiting
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let outer_locals = self.current_locals();
        let block_cond_branch = self.current_conditional_branch();
        let when_body_case = self.in_when_body_of_case;
        let block_in_else_of_if = self.in_else_of_if;
        let block_in_predicate = self.in_predicate_of_if.is_some();

        // Check block parameters against outer locals
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                check_block_parameters_shadow(
                    self.cop,
                    self.source,
                    &block_params,
                    &outer_locals,
                    block_cond_branch,
                    when_body_case,
                    block_in_else_of_if,
                    block_in_predicate,
                    &mut self.diagnostics,
                );
            }
        }

        // Push a new scope for the block body that includes the block parameters.
        // This ensures inner blocks can see outer block params for shadowing detection.
        // Do NOT merge back into the outer scope — block-internal variables are
        // local to the block, not visible to sibling blocks.
        let mut body_scope = HashMap::new();
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                body_scope = build_block_body_scope(&block_params, block_in_else_of_if);
            }
        }
        self.scopes.push(body_scope);
        ruby_prism::visit_block_node(self, node);
        self.scopes.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas behave like blocks for shadowing purposes
        let outer_locals = self.current_locals();
        let block_cond_branch = self.current_conditional_branch();
        let when_body_case = self.in_when_body_of_case;
        let block_in_else_of_if = self.in_else_of_if;
        let block_in_predicate = self.in_predicate_of_if.is_some();

        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                check_block_parameters_shadow(
                    self.cop,
                    self.source,
                    &block_params,
                    &outer_locals,
                    block_cond_branch,
                    when_body_case,
                    block_in_else_of_if,
                    block_in_predicate,
                    &mut self.diagnostics,
                );
            }
        }

        // Lambda creates an isolated scope — params do NOT persist to the
        // enclosing scope (unlike blocks). Lambda parameters are method-like.
        let mut body_scope = HashMap::new();
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                body_scope = build_block_body_scope(&block_params, block_in_else_of_if);
            }
        }
        self.scopes.push(body_scope);
        ruby_prism::visit_lambda_node(self, node);
        self.scopes.pop();
    }

    // Handle top-level assignments (outside any method)
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.scopes.push(HashMap::new());
        ruby_prism::visit_program_node(self, node);
        self.scopes.pop();
    }

    // Track unless/else branches for the same_conditions_node_different_branch check.
    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let unless_offset = node.location().start_offset();
        let else_offset = node.else_clause().map(|e| e.location().start_offset());

        // Visit predicate with in_predicate_of_if set
        let saved_pred = self.in_predicate_of_if;
        self.in_predicate_of_if = Some(unless_offset);
        self.visit(&node.predicate());
        self.in_predicate_of_if = saved_pred;

        // Visit body (the unless-true branch) with branch tracking
        if let Some(stmts) = node.statements() {
            let branch_offset = stmts.location().start_offset();
            self.conditional_branch_stack
                .push((unless_offset, branch_offset, else_offset));
            self.visit_statements_node(&stmts);
            self.conditional_branch_stack.pop();
        }

        // Visit else clause with branch tracking and in_else_of_if set.
        // In Parser gem, `unless A; B; else; C; end` → `if A; C(then); B(else)`.
        // So the unless-else body is treated as the then-branch. But RuboCop's
        // suppression still fires because the block parent matches the else_branch
        // check. We set in_else_of_if for consistency.
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            let saved_else = self.in_else_of_if;
            self.in_else_of_if = Some(unless_offset);
            self.conditional_branch_stack
                .push((unless_offset, branch_offset, None));
            self.visit_else_node(&else_clause);
            self.conditional_branch_stack.pop();
            self.in_else_of_if = saved_else;
        }
    }

    // Handle for loops and while/until which share scope
    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        ruby_prism::visit_for_node(self, node);
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        // While loops are conditionals in RuboCop's model. Variables assigned
        // inside a while condition (e.g., `while part = io.read(...)`) have the
        // while as their nearest conditional ancestor. This prevents false
        // different-branch suppression with outer if/else.
        let while_offset = node.location().start_offset();
        let pred_offset = node.predicate().location().start_offset();
        self.conditional_branch_stack
            .push((while_offset, pred_offset, None));
        ruby_prism::visit_while_node(self, node);
        self.conditional_branch_stack.pop();
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let until_offset = node.location().start_offset();
        let pred_offset = node.predicate().location().start_offset();
        self.conditional_branch_stack
            .push((until_offset, pred_offset, None));
        ruby_prism::visit_until_node(self, node);
        self.conditional_branch_stack.pop();
    }

    // Track case/when branches for the same_conditions_node_different_branch check.
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let case_offset = node.location().start_offset();

        // Visit the predicate (expression after `case`) with its own branch context.
        // Variables assigned in `case val = expr` get a unique branch_offset
        // (the predicate's own offset) that differs from all when/else branches,
        // so blocks in when/else bodies are suppressed as "different branch."
        if let Some(pred) = node.predicate() {
            let pred_offset = pred.location().start_offset();
            self.conditional_branch_stack
                .push((case_offset, pred_offset, None));
            self.visit(&pred);
            self.conditional_branch_stack.pop();
        }

        // Visit each when clause with branch tracking
        for condition in node.conditions().iter() {
            let branch_offset = condition.location().start_offset();
            self.conditional_branch_stack
                .push((case_offset, branch_offset, None));
            // Visit the when node — our visit_when_node handles
            // condition vs body tracking for when-condition assignments.
            if let Some(when_node) = condition.as_when_node() {
                self.visit_when_node_with_case_offset(&when_node, case_offset);
            } else {
                self.visit(&condition);
            }
            self.conditional_branch_stack.pop();
        }

        // Visit the else clause (consequent) with its own branch
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            self.conditional_branch_stack
                .push((case_offset, branch_offset, None));
            self.visit_else_node(&else_clause);
            self.conditional_branch_stack.pop();
        }
    }

    // Track if/unless branches for the same_conditions_node_different_branch check.
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.visit_if_node_impl(node);
    }
}

/// Check if the outer variable and block are in different branches of the same
/// conditional, meaning they can never both be in scope. Returns true if
/// shadowing should be suppressed.
///
/// This matches RuboCop's `same_conditions_node_different_branch?` logic:
/// 1. Same conditional node, different branches → suppress (handles if/else, case/when)
/// 2. Adjacent elsif: the block's nearest conditional IS the outer variable's
///    conditional's direct subsequent → suppress (handles if A / elsif B)
fn is_different_conditional_branch(
    outer_info: &VarInfo,
    block_branch: Option<(usize, usize)>,
    block_in_predicate: bool,
) -> bool {
    let Some(block_branch) = block_branch else {
        return false;
    };
    // Check 1: same conditional node, different branch
    if let Some((outer_cond, outer_branch)) = outer_info.conditional_branch {
        if outer_cond == block_branch.0 && outer_branch != block_branch.1 {
            return true;
        }
    }
    // Check 2: adjacent elsif suppression.
    // If the outer variable's conditional has a subsequent (else/elsif) and
    // the block's nearest conditional starts AT that subsequent, the block
    // is in the immediate next elsif/else → suppress.
    // EXCEPTION: if BOTH the outer variable and the block are in predicates
    // of adjacent elsifs (e.g., `elsif a = find { |a| }; elsif a = find { |a| }`),
    // do NOT suppress. RuboCop's check compares block.parent with if.else_branch;
    // a block nested inside a predicate's call chain has a send/call parent, not
    // the elsif node. But if only the block is in a predicate (block IS the
    // predicate expression), its parent IS the elsif node → suppress correctly.
    let both_in_predicates = block_in_predicate && outer_info.in_predicate_of_if.is_some();
    if !both_in_predicates {
        if let Some(subsequent_offset) = outer_info.cond_subsequent_offset {
            if block_branch.0 == subsequent_offset {
                return true;
            }
        }
    }
    false
}

/// Check if a CallNode is `Ractor.new(...)` or `::Ractor.new(...)`.
fn is_ractor_new_call(node: &ruby_prism::CallNode<'_>) -> bool {
    let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
    if name != "new" {
        return false;
    }
    if let Some(receiver) = node.receiver() {
        if let Some(constant) = receiver.as_constant_read_node() {
            let const_name = std::str::from_utf8(constant.name().as_slice()).unwrap_or("");
            return const_name == "Ractor";
        }
        if let Some(path) = receiver.as_constant_path_node() {
            if let Some(child) = path.name() {
                let const_name = std::str::from_utf8(child.as_slice()).unwrap_or("");
                return const_name == "Ractor";
            }
        }
    }
    false
}

/// Check multi-target (destructured) block params for shadowing.
/// E.g., `|(theme_id, upload_id, sprite)|`
#[allow(clippy::too_many_arguments)]
fn check_multi_target_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    mt: &ruby_prism::MultiTargetNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    in_when_body_of_case: Option<usize>,
    block_in_else_of_if: Option<usize>,
    block_in_predicate: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for target in mt.lefts().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        } else if let Some(inner) = target.as_multi_target_node() {
            check_multi_target_shadow(
                cop,
                source,
                &inner,
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }
    for target in mt.rights().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_block_parameters_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    block_params: &ruby_prism::BlockParametersNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    in_when_body_of_case: Option<usize>,
    block_in_else_of_if: Option<usize>,
    block_in_predicate: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(inner_params) = block_params.parameters() {
        check_block_params_shadow(
            cop,
            source,
            &inner_params,
            outer_locals,
            block_cond_branch,
            in_when_body_of_case,
            block_in_else_of_if,
            block_in_predicate,
            diagnostics,
        );

        for param in inner_params.requireds().iter() {
            if let Some(multi_target) = param.as_multi_target_node() {
                check_multi_target_shadow(
                    cop,
                    source,
                    &multi_target,
                    outer_locals,
                    block_cond_branch,
                    in_when_body_of_case,
                    block_in_else_of_if,
                    block_in_predicate,
                    diagnostics,
                );
            }
        }

        for param in inner_params.posts().iter() {
            if let Some(multi_target) = param.as_multi_target_node() {
                check_multi_target_shadow(
                    cop,
                    source,
                    &multi_target,
                    outer_locals,
                    block_cond_branch,
                    in_when_body_of_case,
                    block_in_else_of_if,
                    block_in_predicate,
                    diagnostics,
                );
            }
        }
    }

    for local in block_params.locals().iter() {
        let name = std::str::from_utf8(
            local
                .as_block_local_variable_node()
                .map_or(&[][..], |node| node.name().as_slice()),
        )
        .unwrap_or("")
        .to_string();
        check_shadow(
            cop,
            source,
            &name,
            local.location(),
            outer_locals,
            block_cond_branch,
            in_when_body_of_case,
            block_in_else_of_if,
            block_in_predicate,
            diagnostics,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_block_params_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    params: &ruby_prism::ParametersNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    in_when_body_of_case: Option<usize>,
    block_in_else_of_if: Option<usize>,
    block_in_predicate: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Required params
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }

    // Optional params
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                opt.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }

    // Post params
    for p in params.posts().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }

    // Keyword params
    for p in params.keywords().iter() {
        if let Some(keyword) = p.as_required_keyword_parameter_node() {
            let name = std::str::from_utf8(keyword.name().as_slice())
                .unwrap_or("")
                .trim_end_matches(':')
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                keyword.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        } else if let Some(keyword) = p.as_optional_keyword_parameter_node() {
            let name = std::str::from_utf8(keyword.name().as_slice())
                .unwrap_or("")
                .trim_end_matches(':')
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                keyword.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }

    // Rest param
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice())
                    .unwrap_or("")
                    .to_string();
                check_shadow(
                    cop,
                    source,
                    &name,
                    rest_param.location(),
                    outer_locals,
                    block_cond_branch,
                    in_when_body_of_case,
                    block_in_else_of_if,
                    block_in_predicate,
                    diagnostics,
                );
            }
        }
    }

    // Keyword rest param (**kwargs)
    if let Some(keyword_rest) = params.keyword_rest() {
        if let Some(keyword_rest) = keyword_rest.as_keyword_rest_parameter_node() {
            if let Some(name) = keyword_rest.name() {
                let name = std::str::from_utf8(name.as_slice())
                    .unwrap_or("")
                    .to_string();
                check_shadow(
                    cop,
                    source,
                    &name,
                    keyword_rest.location(),
                    outer_locals,
                    block_cond_branch,
                    in_when_body_of_case,
                    block_in_else_of_if,
                    block_in_predicate,
                    diagnostics,
                );
            }
        }
    }

    // Block param (&block)
    if let Some(block) = params.block() {
        if let Some(name_const) = block.name() {
            let name = std::str::from_utf8(name_const.as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                block.location(),
                outer_locals,
                block_cond_branch,
                in_when_body_of_case,
                block_in_else_of_if,
                block_in_predicate,
                diagnostics,
            );
        }
    }
}

fn build_block_body_scope(
    block_params: &ruby_prism::BlockParametersNode<'_>,
    in_else_of_if: Option<usize>,
) -> HashMap<String, VarInfo> {
    let mut scope = HashMap::new();

    if let Some(params) = block_params.parameters() {
        let mut param_names = HashSet::new();
        collect_param_names_into(&params, &mut param_names);
        for name in param_names {
            scope.insert(
                name,
                VarInfo {
                    conditional_branch: None,
                    cond_subsequent_offset: None,
                    when_condition_of_case: None,
                    in_else_of_if,
                    in_predicate_of_if: None,
                },
            );
        }
        collect_multi_target_names_from_params(&params, &mut scope, in_else_of_if);
    }

    for local in block_params.locals().iter() {
        let Some(local) = local.as_block_local_variable_node() else {
            continue;
        };
        if let Ok(name) = std::str::from_utf8(local.name().as_slice()) {
            scope.insert(
                name.to_string(),
                VarInfo {
                    conditional_branch: None,
                    cond_subsequent_offset: None,
                    when_condition_of_case: None,
                    in_else_of_if,
                    in_predicate_of_if: None,
                },
            );
        }
    }

    scope
}

#[allow(clippy::too_many_arguments)]
fn check_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    name: &str,
    loc: ruby_prism::Location<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    in_when_body_of_case: Option<usize>,
    block_in_else_of_if: Option<usize>,
    block_in_predicate: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if name.is_empty() || name.starts_with('_') {
        return;
    }
    if let Some(info) = outer_locals.get(name) {
        // Skip if the outer variable and block are in different branches
        // of the same conditional (case/when, if/else).
        if is_different_conditional_branch(info, block_cond_branch, block_in_predicate) {
            return;
        }
        // Skip if the outer variable was assigned in a `when` condition and
        // the block is inside a `when` body of the same case node. RuboCop's
        // VariableForce resolves both to the same conditional (case) node
        // and suppresses the shadowing warning.
        if let (Some(var_case), Some(block_case)) =
            (info.when_condition_of_case, in_when_body_of_case)
        {
            if var_case == block_case {
                return;
            }
        }
        // Suppress when both the outer variable and the block are inside
        // the same else/elsif clause of the same if/unless. In RuboCop,
        // `variable_node == outer_local_variable_node.else_branch` fires
        // because the block's parent in Parser AST IS the else clause body.
        if let (Some(var_else), Some(block_else)) = (info.in_else_of_if, block_in_else_of_if) {
            if var_else == block_else {
                return;
            }
        }
        // Suppress when the outer variable was assigned in an if/unless
        // predicate (e.g., `if item = expr`) and the block is inside
        // the same conditional. In RuboCop, `variable_node ==
        // outer_local_variable_node` fires because the block's parent
        // IS the if node itself.
        if let Some(pred_if) = info.in_predicate_of_if {
            // Block is in the body of the same if (same cond offset)
            if let Some((block_cond, _)) = block_cond_branch {
                if block_cond == pred_if {
                    return;
                }
            }
            // Block is in the else clause of the same if
            if block_in_else_of_if == Some(pred_if) {
                return;
            }
        }
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Shadowing outer local variable - `{}`.", name),
        ));
    }
}

/// Collect names from MultiTargetNode entries in block parameters (destructuring).
/// E.g., `|(a, b, c)|` creates a MultiTargetNode with lefts [a, b, c].
fn collect_multi_target_names(node: &ruby_prism::MultiTargetNode<'_>, names: &mut HashSet<String>) {
    for target in node.lefts().iter() {
        if let Some(local) = target.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                names.insert(s.to_string());
            }
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
    if let Some(rest) = node.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(local) = expr.as_required_parameter_node() {
                    if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                        names.insert(s.to_string());
                    }
                }
            }
        }
    }
    for target in node.rights().iter() {
        if let Some(local) = target.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                names.insert(s.to_string());
            }
        }
    }
}

/// Extract names from multi-target (destructured) params and add to scope.
fn collect_multi_target_names_from_params(
    params: &ruby_prism::ParametersNode<'_>,
    scope: &mut HashMap<String, VarInfo>,
    in_else_of_if: Option<usize>,
) {
    for p in params.requireds().iter() {
        if let Some(mt) = p.as_multi_target_node() {
            let mut names = HashSet::new();
            collect_multi_target_names(&mt, &mut names);
            for name in names {
                scope.insert(
                    name,
                    VarInfo {
                        conditional_branch: None,
                        when_condition_of_case: None,
                        cond_subsequent_offset: None,
                        in_else_of_if,
                        in_predicate_of_if: None,
                    },
                );
            }
        }
    }

    for p in params.posts().iter() {
        if let Some(mt) = p.as_multi_target_node() {
            let mut names = HashSet::new();
            collect_multi_target_names(&mt, &mut names);
            for name in names {
                scope.insert(
                    name,
                    VarInfo {
                        conditional_branch: None,
                        when_condition_of_case: None,
                        cond_subsequent_offset: None,
                        in_else_of_if,
                        in_predicate_of_if: None,
                    },
                );
            }
        }
    }
}

fn collect_param_names_into(params: &ruby_prism::ParametersNode<'_>, scope: &mut HashSet<String>) {
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            if let Ok(s) = std::str::from_utf8(opt.name().as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name) = rest_param.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    for p in params.posts().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                scope.insert(s.to_string());
            }
        } else if let Some(kw_rest) = p.as_keyword_rest_parameter_node() {
            if let Some(name) = kw_rest.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                scope.insert(s.trim_end_matches(':').to_string());
            }
        } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                scope.insert(s.trim_end_matches(':').to_string());
            }
        }
    }
    if let Some(keyword_rest) = params.keyword_rest() {
        if let Some(keyword_rest) = keyword_rest.as_keyword_rest_parameter_node() {
            if let Some(name) = keyword_rest.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    if let Some(block) = params.block() {
        if let Some(name) = block.name() {
            if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ShadowingOuterLocalVariable,
        "cops/lint/shadowing_outer_local_variable"
    );
}
