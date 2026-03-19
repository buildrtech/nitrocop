use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for usages of `each` with `<<`, `push`, or `append` which
/// can be replaced by `map`.
///
/// ## Investigation Notes (2026-03-18)
///
/// **FP root causes fixed:**
/// - Safe navigation `&.each` (e.g., `opts[:key]&.each { |x| arr << x }`) was not
///   excluded. RuboCop's NodePattern uses `(send ...)` which only matches regular
///   method calls, not `csend` (safe navigation). Fixed by checking
///   `call.call_operator()` for `&.`.
/// - `LocalVariableOperatorWriteNode` (e.g., `arr += other`) between array init and
///   each loop was not detected by `references_variable`, causing the cop to miss
///   that the array was modified. Same issue with `LocalVariableOrWriteNode` and
///   `LocalVariableAndWriteNode`. Fixed by adding these node types to both the
///   backwards assignment search and the `references_variable` helper.
///
/// **FN root causes fixed:**
/// - `Array.new` and `Array[]` as empty array initializers were not recognized.
///   RuboCop accepts `Array.new`, `Array.new([])`, `Array[]`, and `Array([])`.
///   Added detection for `Array.new` (no args or empty array arg) and `Array[]`
///   (no args) as `CallNode` patterns.
///
/// ## Investigation Notes (2026-03-19)
///
/// **FP root cause (1 FP):**
/// - `binding` inside the each block body implicitly captures all local variables
///   in scope (including the destination array variable). RuboCop's `VariableForce`
///   counts `binding` calls as implicit references, so `dest_var.references.one?`
///   returns false and the cop doesn't flag it. Fixed by checking for `binding`
///   calls inside the each block body and skipping if found.
///
/// **FN root causes (7 FN):**
/// - The `[].tap { |dest| src.each { |e| dest << expr } }` pattern was not
///   handled at all. RuboCop supports this via `empty_array_tap` node matcher.
///   Fixed by adding `visit_block_node` to detect `[].tap` blocks where the
///   only body statement is an `each` with push into the tap block parameter.
///   The tap block must contain only the each call (no other statements).
pub struct MapIntoArray;

impl Cop for MapIntoArray {
    fn name(&self) -> &'static str {
        "Style/MapIntoArray"
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
        let mut visitor = MapIntoArrayVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MapIntoArrayVisitor<'a> {
    cop: &'a MapIntoArray,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Check if a node is an empty array expression: `[]`, `Array.new`, `Array.new([])`,
/// `Array[]`, or `Array([])`.
fn is_empty_array_value(value: &ruby_prism::Node<'_>) -> bool {
    // Literal empty array: `[]`
    if let Some(arr) = value.as_array_node() {
        return arr.elements().iter().count() == 0;
    }
    // Call-based patterns: `Array.new`, `Array.new([])`, `Array[]`, `Array([])`
    if let Some(call) = value.as_call_node() {
        let method = call.name().as_slice();
        if let Some(receiver) = call.receiver() {
            // `Array.new` or `Array.new([])`  or  `Array[]`
            let is_array_const = receiver
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Array")
                || receiver
                    .as_constant_path_node()
                    .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"Array"));
            if is_array_const {
                if method == b"new" {
                    // Array.new or Array.new([])
                    if call.arguments().is_none() {
                        return true;
                    }
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            if let Some(arr) = arg_list[0].as_array_node() {
                                return arr.elements().iter().count() == 0;
                            }
                        }
                    }
                } else if method == b"[]" {
                    // Array[]
                    return call.arguments().is_none();
                }
            }
        } else {
            // `Array([])` — this is a Kernel method call with no receiver
            if method == b"Array" {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        if let Some(arr) = arg_list[0].as_array_node() {
                            return arr.elements().iter().count() == 0;
                        }
                    }
                }
            }
        }
    }
    false
}

impl MapIntoArrayVisitor<'_> {
    /// Check if a statements node contains:
    ///   dest = []
    ///   ...each { |x| dest << expr }
    /// Pattern: look at pairs of siblings in a statements block.
    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            // Check if this is a `collection.each { |x| var << expr }` pattern
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.name().as_slice() != b"each" {
                continue;
            }
            if call.receiver().is_none() {
                continue;
            }
            // Skip safe navigation `&.each` — RuboCop only matches `send`, not `csend`
            if call
                .call_operator_loc()
                .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
            {
                continue;
            }
            // each must have no arguments
            if call.arguments().is_some() {
                continue;
            }

            let block = match call.block() {
                Some(b) => b,
                None => continue,
            };
            let block_node = match block.as_block_node() {
                Some(b) => b,
                None => continue,
            };
            let body = match block_node.body() {
                Some(b) => b,
                None => continue,
            };
            let body_stmts = match body.as_statements_node() {
                Some(s) => s,
                None => continue,
            };
            let body_nodes: Vec<_> = body_stmts.body().iter().collect();
            if body_nodes.len() != 1 {
                continue;
            }

            // Check for var << expr or var.push(expr) or var.append(expr)
            let push_call = match body_nodes[0].as_call_node() {
                Some(c) => c,
                None => continue,
            };
            let push_method = push_call.name().as_slice();
            if push_method != b"<<" && push_method != b"push" && push_method != b"append" {
                continue;
            }

            // Receiver must be a local variable
            let push_receiver = match push_call.receiver() {
                Some(r) => r,
                None => continue,
            };
            let lvar = match push_receiver.as_local_variable_read_node() {
                Some(l) => l,
                None => continue,
            };

            let var_name = lvar.name();

            // Check that the push argument is suitable (not a splat, etc.)
            if let Some(args) = push_call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    continue;
                }
                // Skip if argument is a splat
                if arg_list[0].as_splat_node().is_some() {
                    continue;
                }
            } else {
                continue;
            }

            // Now check: is there a preceding `var = []` (or Array.new etc.) in the same scope?
            let mut found_empty_array_init = false;
            let mut init_idx = 0;
            for j in (0..i).rev() {
                // Check plain assignment: `var = expr`
                if let Some(asgn) = stmts[j].as_local_variable_write_node() {
                    if asgn.name().as_slice() == var_name.as_slice() {
                        // Check if the value is an empty array
                        if is_empty_array_value(&asgn.value()) {
                            found_empty_array_init = true;
                            init_idx = j;
                        }
                        break; // found the most recent assignment, stop
                    }
                }
                // Check operator assignments (+=, ||=, &&=) — these mean the var
                // was modified, so any earlier `var = []` is stale.
                if is_local_var_operator_write(&stmts[j], var_name.as_slice())
                    || is_local_var_or_write(&stmts[j], var_name.as_slice())
                    || is_local_var_and_write(&stmts[j], var_name.as_slice())
                {
                    break; // var was modified by operator assignment, stop
                }
            }

            if !found_empty_array_init {
                continue;
            }

            // Check that var is not referenced between the init and the each call.
            // If there are other uses of the variable (like `var << something`),
            // we can't guarantee it's still an empty array.
            let var_name_slice = var_name.as_slice();
            let mut has_intermediate_ref = false;
            for stmt in &stmts[(init_idx + 1)..i] {
                if references_variable(stmt, var_name_slice) {
                    has_intermediate_ref = true;
                    break;
                }
            }
            if has_intermediate_ref {
                continue;
            }

            // Receiver of `each` must not be `self`
            if let Some(each_receiver) = call.receiver() {
                if each_receiver.as_self_node().is_some() {
                    continue;
                }
            }

            // Skip if the block body contains a `binding` call, which implicitly
            // captures all local variables (including the dest array). RuboCop's
            // VariableForce counts these as implicit references.
            if let Some(ref block_body) = block_node.body() {
                if contains_binding_call(block_body) {
                    continue;
                }
            }

            let loc = call.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `map` instead of `each` to map elements into an array.".to_string(),
            ));
        }
    }
}

impl MapIntoArrayVisitor<'_> {
    /// Check for tap pattern on a call node: `[].tap { |dest| src.each { |e| dest << expr } }`
    fn check_tap_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must be `.tap` with an empty array receiver
        if call.name().as_slice() != b"tap" {
            return;
        }
        if call.arguments().is_some() {
            return;
        }
        // Receiver must be an empty array literal `[]`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if let Some(arr) = receiver.as_array_node() {
            if arr.elements().iter().count() != 0 {
                return;
            }
        } else {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Block must have exactly one parameter
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return,
        };
        let requireds: Vec<_> = param_list.requireds().iter().collect();
        if requireds.len() != 1 {
            return;
        }
        let param_node = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        // Get the block parameter name
        let block_param_name = param_node.name();

        // Block body must have exactly one statement: the each call
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let body_stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };
        let body_nodes: Vec<_> = body_stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // The single statement must be an `each` call
        let each_call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        if each_call.name().as_slice() != b"each" {
            return;
        }
        if each_call.receiver().is_none() {
            return;
        }
        // Skip safe navigation
        if each_call
            .call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
        {
            return;
        }
        if each_call.arguments().is_some() {
            return;
        }

        // each must have a block
        let each_block = match each_call.block() {
            Some(b) => b,
            None => return,
        };
        let each_block_node = match each_block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let each_body = match each_block_node.body() {
            Some(b) => b,
            None => return,
        };
        let each_body_stmts = match each_body.as_statements_node() {
            Some(s) => s,
            None => return,
        };
        let each_body_nodes: Vec<_> = each_body_stmts.body().iter().collect();
        if each_body_nodes.len() != 1 {
            return;
        }

        // Check for dest << expr or dest.push(expr) or dest.append(expr)
        let push_call = match each_body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        let push_method = push_call.name().as_slice();
        if push_method != b"<<" && push_method != b"push" && push_method != b"append" {
            return;
        }

        // Push receiver must be the tap block parameter
        let push_receiver = match push_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let lvar = match push_receiver.as_local_variable_read_node() {
            Some(l) => l,
            None => return,
        };
        if lvar.name().as_slice() != block_param_name.as_slice() {
            return;
        }

        // Check push has exactly one non-splat argument
        if let Some(args) = push_call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
            if arg_list[0].as_splat_node().is_some() {
                return;
            }
        } else {
            return;
        }

        // Receiver of `each` must not be `self`
        if let Some(each_receiver) = each_call.receiver() {
            if each_receiver.as_self_node().is_some() {
                return;
            }
        }

        // Skip if the block body contains a `binding` call
        if contains_binding_call(&each_body) {
            return;
        }

        // Report offense on the each call
        let loc = each_call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `map` instead of `each` to map elements into an array.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for MapIntoArrayVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let stmts: Vec<_> = node.body().iter().collect();
        self.check_statements(&stmts);
        ruby_prism::visit_statements_node(self, node);
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(body) = node.statements() {
            let stmts: Vec<_> = body.body().iter().collect();
            self.check_statements(&stmts);
        }
        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_tap_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

/// Check if a node (recursively) contains a call to `binding` (no receiver, no args).
/// `binding` implicitly captures all local variables in scope, so the destination
/// variable gets additional implicit references that prevent the map transformation.
fn contains_binding_call(node: &ruby_prism::Node<'_>) -> bool {
    struct BindingFinder {
        found: bool,
    }
    impl<'pr> ruby_prism::Visit<'pr> for BindingFinder {
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            if node.receiver().is_none()
                && node.name().as_slice() == b"binding"
                && node.arguments().is_none()
            {
                self.found = true;
            }
            ruby_prism::visit_call_node(self, node);
        }
    }
    let mut finder = BindingFinder { found: false };
    ruby_prism::Visit::visit(&mut finder, node);
    finder.found
}

/// Check if a node is a `LocalVariableOperatorWriteNode` (e.g., `x += y`) for the given var name.
fn is_local_var_operator_write(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    node.as_local_variable_operator_write_node()
        .is_some_and(|n| n.name().as_slice() == var_name)
}

/// Check if a node is a `LocalVariableOrWriteNode` (e.g., `x ||= y`) for the given var name.
fn is_local_var_or_write(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    node.as_local_variable_or_write_node()
        .is_some_and(|n| n.name().as_slice() == var_name)
}

/// Check if a node is a `LocalVariableAndWriteNode` (e.g., `x &&= y`) for the given var name.
fn is_local_var_and_write(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    node.as_local_variable_and_write_node()
        .is_some_and(|n| n.name().as_slice() == var_name)
}

/// Check if a node (recursively) references a local variable with the given name.
fn references_variable(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
    }
    if let Some(lv) = node.as_local_variable_write_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
    }
    // Check operator write nodes (+=, ||=, &&=)
    if is_local_var_operator_write(node, var_name)
        || is_local_var_or_write(node, var_name)
        || is_local_var_and_write(node, var_name)
    {
        return true;
    }
    // Check children recursively
    struct VarRefFinder<'a> {
        var_name: &'a [u8],
        found: bool,
    }
    impl<'pr> ruby_prism::Visit<'pr> for VarRefFinder<'_> {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
        }
        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
            // Must recurse into the value of the write node, otherwise
            // we miss references inside the RHS (e.g., `entries = src.map { order << x }`)
            ruby_prism::visit_local_variable_write_node(self, node);
        }
        fn visit_local_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
            ruby_prism::visit_local_variable_operator_write_node(self, node);
        }
        fn visit_local_variable_or_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
            ruby_prism::visit_local_variable_or_write_node(self, node);
        }
        fn visit_local_variable_and_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
            ruby_prism::visit_local_variable_and_write_node(self, node);
        }
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            // Check receiver and arguments
            ruby_prism::visit_call_node(self, node);
        }
    }
    let mut finder = VarRefFinder {
        var_name,
        found: false,
    };
    ruby_prism::Visit::visit(&mut finder, node);
    finder.found
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapIntoArray, "cops/style/map_into_array");
}
