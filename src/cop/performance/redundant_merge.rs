use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=2, FN=5. Both fixed.
///
/// FP=2 fix: Both FPs were `merge!` as the last statement in a class body. RuboCop's
/// `value_used?` considers class/module body return values as "used", so these should be
/// skipped. Fixed by adding `visit_class_node`, `visit_module_node`, and
/// `visit_singleton_class_node` that set `value_used = true`.
///
/// FN=5 fix: All were `merge!` on the accumulator inside `each_with_object` blocks.
/// RuboCop's `EachWithObjectInspector` detects this pattern and allows flagging even when
/// `value_used?` is true. Implemented by tracking the accumulator name (second block param)
/// when entering an `each_with_object` block. The accumulator is cleared inside nested
/// scopes (class/module/def/block bodies, conditionals, loops, case) to match RuboCop's
/// inspector which only matches `merge!` as a direct statement of the block body.
///
/// A previous attempt (commit 80417659, reverted a639bfc4) failed because the accumulator
/// leaked into nested scopes, causing 6 new FPs. The key insight is that RuboCop's
/// inspector navigates UP from the merge! node and only matches if the parent (or
/// begin-wrapped parent) is the block node itself.
///
/// FP=1 fix (2026-03-07): The accumulator was still leaking into nested blocks (e.g.,
/// `.each do` inside `each_with_object`). Fixed by saving/clearing the accumulator in
/// `visit_call_node`'s else branch for non-each_with_object blocks.
pub struct RedundantMerge;

impl Cop for RedundantMerge {
    fn name(&self) -> &'static str {
        "Performance/RedundantMerge"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max_kv_pairs = config.get_usize("MaxKeyValuePairs", 2);
        let mut visitor = RedundantMergeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            max_kv_pairs,
            value_used: false,
            each_with_object_accumulator: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantMergeVisitor<'a, 'src> {
    cop: &'a RedundantMerge,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    max_kv_pairs: usize,
    /// Whether the current expression's value is used by a parent.
    value_used: bool,
    /// Name of the each_with_object accumulator variable, if currently inside one.
    each_with_object_accumulator: Option<String>,
}

impl<'a, 'src> RedundantMergeVisitor<'a, 'src> {
    /// If `call` is `something.each_with_object(arg) { |elem, acc| ... }`,
    /// return the name of the accumulator parameter (second block param).
    fn extract_each_with_object_accumulator(
        &self,
        call: &ruby_prism::CallNode<'_>,
        block: &ruby_prism::Node<'_>,
    ) -> Option<String> {
        if call.name().as_slice() != b"each_with_object" {
            return None;
        }
        let block_node = block.as_block_node()?;
        let params = block_node.parameters()?;
        let block_params = params.as_block_parameters_node()?;
        let param_list = block_params.parameters()?;
        let requireds = param_list.requireds();
        if requireds.len() < 2 {
            return None;
        }
        let second = requireds.iter().nth(1)?;
        let req = second.as_required_parameter_node()?;
        Some(String::from_utf8_lossy(req.name().as_slice()).into_owned())
    }

    fn check_merge_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        if call.name().as_slice() != b"merge!" {
            return;
        }

        // Must have a receiver (hash.merge!)
        if call.receiver().is_none() {
            return;
        }

        // merge! with a conflict resolution block cannot be replaced with []=
        if call.block().is_some() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();

        // Count key-value pairs in the merge! argument
        let kv_count = if args.len() == 1 {
            let first = args.iter().next().unwrap();
            // Don't flag if argument contains a splat (**hash)
            if let Some(kw) = first.as_keyword_hash_node() {
                if kw
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
                kw.elements().len()
            } else if let Some(hash) = first.as_hash_node() {
                if hash
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
                hash.elements().len()
            } else {
                0
            }
        } else {
            0
        };

        if kv_count == 0 || kv_count > self.max_kv_pairs {
            return;
        }

        // RuboCop: when pairs > 1, only flag if receiver is "pure" (a simple
        // local variable). Method calls, indexing etc. could have side effects.
        let receiver = call.receiver().unwrap();
        if kv_count > 1 {
            let is_pure = receiver.as_local_variable_read_node().is_some()
                || receiver.as_instance_variable_read_node().is_some()
                || receiver.as_class_variable_read_node().is_some()
                || receiver.as_constant_read_node().is_some()
                || receiver.as_constant_path_node().is_some()
                || receiver.as_self_node().is_some();
            if !is_pure {
                return;
            }
        }

        // Don't flag if the return value of merge! is used. merge! returns
        // the hash, while []= returns the assigned value — they're not
        // interchangeable when the result is consumed.
        // Exception: inside each_with_object, the accumulator is passed by
        // reference, so merge! on the accumulator is redundant even though
        // the block's return value appears "used". This matches RuboCop's
        // EachWithObjectInspector.
        if self.value_used {
            // Check if the receiver is the each_with_object accumulator
            let is_accumulator = if let Some(ref acc_name) = self.each_with_object_accumulator {
                receiver
                    .as_local_variable_read_node()
                    .is_some_and(|lvar| lvar.name().as_slice() == acc_name.as_bytes())
            } else {
                false
            };
            if !is_accumulator {
                return;
            }
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        let msg = if kv_count == 1 {
            "Use `[]=` instead of `merge!` with a single key-value pair.".to_string()
        } else {
            format!("Use `[]=` instead of `merge!` with {kv_count} key-value pairs.")
        };
        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, msg));
    }
}

impl<'pr> Visit<'pr> for RedundantMergeVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check this call for merge! offense
        self.check_merge_call(node);

        // Visit receiver — its value is used (as the receiver)
        if let Some(recv) = node.receiver() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&recv);
            self.value_used = prev;
        }

        // Visit arguments — their values are used
        if let Some(args) = node.arguments() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit_arguments_node(&args);
            self.value_used = prev;
        }

        // Visit block — detect each_with_object accumulator
        if let Some(block) = node.block() {
            // Check if this is an each_with_object call with a block
            let acc_name = self.extract_each_with_object_accumulator(node, &block);
            if let Some(name) = acc_name {
                let prev_acc = self.each_with_object_accumulator.take();
                self.each_with_object_accumulator = Some(name);
                self.visit(&block);
                self.each_with_object_accumulator = prev_acc;
            } else {
                let prev_acc = self.each_with_object_accumulator.take();
                self.visit(&block);
                self.each_with_object_accumulator = prev_acc;
            }
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // RHS of assignment — value is used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_instance_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_class_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_global_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_constant_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_constant_path_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_operator_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_or_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_and_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        // Value part of a hash pair — value is used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_assoc_node(self, node);
        self.value_used = prev;
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_assoc_splat_node(self, node);
        self.value_used = prev;
    }

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_return_node(self, node);
        self.value_used = prev;
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        // Parenthesized expression passes through value_used context
        ruby_prism::visit_parentheses_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // The condition's value is used; the branches inherit parent's value_used
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        // Clear accumulator inside conditionals — RuboCop's EachWithObjectInspector
        // only matches merge! as a direct statement of the block body, not inside if/unless.
        let prev_acc = self.each_with_object_accumulator.take();
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        if let Some(subsequent) = node.subsequent() {
            self.visit(&subsequent);
        }
        self.each_with_object_accumulator = prev_acc;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        let prev_acc = self.each_with_object_accumulator.take();
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
        self.each_with_object_accumulator = prev_acc;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        let prev_acc = self.each_with_object_accumulator.take();
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        self.each_with_object_accumulator = prev_acc;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        let prev_acc = self.each_with_object_accumulator.take();
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        self.each_with_object_accumulator = prev_acc;
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        // In a statements list, only the last statement's value is potentially
        // used (as the implicit return). All other statements are side-effect only.
        let stmts: Vec<_> = node.body().iter().collect();
        let last_idx = stmts.len().saturating_sub(1);
        for (i, stmt) in stmts.iter().enumerate() {
            if i == last_idx {
                // Last statement inherits parent's value_used
                self.visit(stmt);
            } else {
                let prev = self.value_used;
                self.value_used = false;
                self.visit(stmt);
                self.value_used = prev;
            }
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Class body's last expression is the class's return value.
        // RuboCop's value_used? considers this as used, so merge! in
        // the last position of a class body should not be flagged.
        let prev = self.value_used;
        self.value_used = true;
        // Clear accumulator inside class bodies — class bodies are not
        // part of an each_with_object iteration.
        let prev_acc = self.each_with_object_accumulator.take();
        ruby_prism::visit_class_node(self, node);
        self.each_with_object_accumulator = prev_acc;
        self.value_used = prev;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        let prev_acc = self.each_with_object_accumulator.take();
        ruby_prism::visit_module_node(self, node);
        self.each_with_object_accumulator = prev_acc;
        self.value_used = prev;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        let prev_acc = self.each_with_object_accumulator.take();
        ruby_prism::visit_singleton_class_node(self, node);
        self.each_with_object_accumulator = prev_acc;
        self.value_used = prev;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Method body's last expression is the implicit return value —
        // treat it as value_used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_def_node(self, node);
        self.value_used = prev;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Block body's last expression becomes the block's return value —
        // conservatively treat as value_used.
        // Note: each_with_object accumulator is set in visit_call_node and
        // intentionally NOT cleared here — it needs to flow into the block body.
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_block_node(self, node);
        self.value_used = prev;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_lambda_node(self, node);
        self.value_used = prev;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // begin..end passes through value_used to its statements
        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // rescue clauses inherit value_used from the begin block
        ruby_prism::visit_rescue_node(self, node);
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        ruby_prism::visit_ensure_node(self, node);
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // The predicate value is used; when branches inherit value_used
        if let Some(pred) = node.predicate() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&pred);
            self.value_used = prev;
        }
        let prev_acc = self.each_with_object_accumulator.take();
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
        self.each_with_object_accumulator = prev_acc;
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        // Interpolated parts have their value used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_interpolated_string_node(self, node);
        self.value_used = prev;
    }

    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_embedded_statements_node(self, node);
        self.value_used = prev;
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Array elements have their value used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_array_node(self, node);
        self.value_used = prev;
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_hash_node(self, node);
        self.value_used = prev;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_keyword_hash_node(self, node);
        self.value_used = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMerge, "cops/performance/redundant_merge");

    #[test]
    fn config_max_kv_pairs_flags_two() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // Default MaxKeyValuePairs:2 should flag merge! with 2 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag merge! with 2 pairs when MaxKeyValuePairs:2"
        );
    }

    #[test]
    fn config_max_kv_pairs_allows_three() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // MaxKeyValuePairs:2 should NOT flag merge! with 3 KV pairs
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag merge! with 3 pairs when MaxKeyValuePairs:2"
        );
    }

    #[test]
    fn config_max_kv_pairs_higher() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // MaxKeyValuePairs:5 should flag merge! with up to 5 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(5.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag merge! with 3 pairs when MaxKeyValuePairs:5"
        );
    }

    #[test]
    fn non_pure_receiver_multi_pair_not_flagged() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        // Method call receiver — not a local variable, should not be flagged with 2 pairs
        let source = b"obj.options.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag non-pure receiver with multiple pairs"
        );
    }
}
