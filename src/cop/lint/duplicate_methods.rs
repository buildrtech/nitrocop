use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for duplicated instance (or singleton) method definitions.
///
/// Tracks `def`, `defs` (self.method), `alias`, `alias_method`, `attr_reader`,
/// `attr_writer`, `attr_accessor`, `attr`, `def_delegator`, `def_instance_delegator`,
/// `def_delegators`, and `def_instance_delegators`.
///
/// Root causes of previous FN (1,203):
/// - Only checked direct `def` children of class/module StatementsNode
/// - Missed `private def`, `protected def` (CallNode wrapping DefNode)
/// - Missed `alias_method`, `attr_*`, `def_delegator*` call patterns
/// - Missed top-level definitions (Object scope)
/// - Missed reopened class/module blocks (separate `class A...end` blocks)
/// - Missed nested method scoping (method_key with ancestor def name)
/// - Missed `class << self` / `class << expr` singleton class patterns
/// - Wrong message format (was "Duplicated method definition." instead of RuboCop format)
///
/// Root causes of previous FP (47):
/// - Did not skip definitions inside `if`/`unless`/`case` ancestors
/// - Did not handle rescue/ensure scope reset
pub struct DuplicateMethods;

impl Cop for DuplicateMethods {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = DupMethodVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            definitions: HashMap::new(),
            scope_stack: Vec::new(),
            def_stack: Vec::new(),
            if_depth: 0,
            plain_block_depth: 0,
            rescue_ensure_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Stored definition location for a method.
#[derive(Clone)]
struct DefLocation {
    line: usize,
}

struct DupMethodVisitor<'a, 'src> {
    cop: &'a DuplicateMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Global definitions map: qualified method key -> first definition location
    definitions: HashMap<String, DefLocation>,
    /// Stack of scope names for building qualified method names.
    /// Each entry tracks the scope name and whether we are inside a singleton class.
    scope_stack: Vec<ScopeEntry>,
    /// Stack of enclosing def method names (for method_key scoping of nested defs)
    def_stack: Vec<String>,
    /// Depth inside if/unless/case nodes — skip definitions when > 0
    if_depth: usize,
    /// Depth inside non-scope blocks (DSL blocks, describe blocks, etc.)
    /// Methods inside these are ignored per RuboCop (parent_module_name returns nil).
    plain_block_depth: usize,
    /// Stack of rescue/ensure scope markers. When > 0, the first redefinition of
    /// a method key is allowed (different execution path). Inner entries track
    /// which keys have already been "first-seen" in this scope.
    rescue_ensure_stack: Vec<Vec<String>>,
}

#[derive(Clone)]
struct ScopeEntry {
    name: String,
    is_singleton: bool,
}

impl DupMethodVisitor<'_, '_> {
    /// Build the qualified method name like RuboCop's `found_instance_method`.
    /// For instance methods: `ClassName#method_name`
    /// For singleton methods: `ClassName.method_name`
    /// For top-level: `Object#method_name`
    fn qualified_method_name(&self, method_name: &str, is_singleton: bool) -> String {
        let scope = self.current_scope_name();
        let separator = if is_singleton { "." } else { "#" };
        format!("{scope}{separator}{method_name}")
    }

    /// Get the current scope name from the scope stack.
    fn current_scope_name(&self) -> String {
        if self.scope_stack.is_empty() {
            return "Object".to_string();
        }

        let mut parts = Vec::new();
        for entry in &self.scope_stack {
            parts.push(entry.name.as_str());
        }
        parts.join("::")
    }

    /// Build the method key that includes ancestor def name for nested methods.
    /// This matches RuboCop's `method_key` method.
    fn method_key(&self, qualified_name: &str) -> String {
        if let Some(ancestor_def) = self.def_stack.last() {
            format!("{ancestor_def}.{qualified_name}")
        } else {
            qualified_name.to_string()
        }
    }

    /// Record a found method definition and check for duplicates.
    fn found_method(
        &mut self,
        method_name: &str,
        is_singleton: bool,
        def_line: usize,
        offense_offset: usize,
    ) {
        let qualified = self.qualified_method_name(method_name, is_singleton);
        let key = self.method_key(&qualified);

        // Handle rescue/ensure scope: first occurrence in a rescue/ensure body
        // is allowed to "redefine" (it's a different execution path).
        if let Some(scope) = self.rescue_ensure_stack.last_mut() {
            if self.definitions.contains_key(&key) && !scope.contains(&key) {
                // First time in this rescue/ensure scope -- allow it
                self.definitions
                    .insert(key.clone(), DefLocation { line: def_line });
                scope.push(key);
                return;
            }
        }

        if let Some(first_def) = self.definitions.get(&key) {
            let first_line = first_def.line;
            let path = self.source.path_str();
            let (line, column) = self.source.offset_to_line_col(offense_offset);
            let message = format!(
                "Method `{qualified}` is defined at both {path}:{first_line} and {path}:{line}."
            );
            let diag = self.cop.diagnostic(self.source, line, column, message);
            self.diagnostics.push(diag);
        } else {
            self.definitions.insert(key, DefLocation { line: def_line });
        }
    }

    /// Process a def node (instance or singleton method).
    fn process_def(&mut self, node: &ruby_prism::DefNode<'_>) {
        if self.if_depth > 0 || self.plain_block_depth > 0 {
            return;
        }

        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());

        if let Some(receiver) = node.receiver() {
            // def self.method or def ConstName.method
            if receiver.as_self_node().is_some() {
                let keyword_offset = node.def_keyword_loc().start_offset();
                self.found_method(name, true, def_line, keyword_offset);
            } else if let Some(const_read) = receiver.as_constant_read_node() {
                let const_name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                if self.scope_matches_const(const_name) {
                    let keyword_offset = node.def_keyword_loc().start_offset();
                    self.found_method(name, true, def_line, keyword_offset);
                }
            }
        } else {
            // Instance method (or singleton method if inside `class << self`)
            let is_singleton = self.in_singleton_scope();
            let keyword_offset = node.def_keyword_loc().start_offset();
            self.found_method(name, is_singleton, def_line, keyword_offset);
        }
    }

    /// Check if we're currently inside a singleton class (class << self).
    fn in_singleton_scope(&self) -> bool {
        self.scope_stack.last().is_some_and(|e| e.is_singleton)
    }

    /// Check if a constant name matches the current scope's innermost name.
    fn scope_matches_const(&self, const_name: &str) -> bool {
        self.scope_stack
            .last()
            .is_some_and(|e| e.name == const_name)
    }

    /// Process an alias node.
    fn process_alias(&mut self, node: &ruby_prism::AliasMethodNode<'_>) {
        if self.if_depth > 0 || self.plain_block_depth > 0 {
            return;
        }

        let new_name_node = node.new_name();
        let old_name_node = node.old_name();

        let new_sym = match new_name_node.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        // Self-alias is allowed (alias foo foo)
        if let Some(old_sym) = old_name_node.as_symbol_node() {
            if new_sym.unescaped() == old_sym.unescaped() {
                return;
            }
        }

        let name = std::str::from_utf8(new_sym.unescaped()).unwrap_or("");
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();
        self.found_method(name, is_singleton, def_line, offset);
    }

    /// Process a call node for alias_method, attr_*, def_delegator*, etc.
    fn process_call(&mut self, node: &ruby_prism::CallNode<'_>) {
        if self.if_depth > 0 || self.plain_block_depth > 0 {
            return;
        }

        let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        let args = match node.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();

        match method_name {
            "alias_method" => self.process_alias_method(node, &arg_list),
            "attr_reader" => self.process_attr(node, &arg_list, true, false),
            "attr_writer" => self.process_attr(node, &arg_list, false, true),
            "attr_accessor" => self.process_attr(node, &arg_list, true, true),
            "attr" => self.process_attr_legacy(node, &arg_list),
            "def_delegator" | "def_instance_delegator" => {
                self.process_def_delegator(node, &arg_list);
            }
            "def_delegators" | "def_instance_delegators" => {
                self.process_def_delegators(node, &arg_list);
            }
            _ => {}
        }
    }

    fn process_alias_method(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.len() < 2 {
            return;
        }
        let new_name = match extract_symbol_or_string(&args[0]) {
            Some(n) => n,
            None => return,
        };
        let orig_name = match extract_symbol_or_string(&args[1]) {
            Some(n) => n,
            None => return,
        };
        if new_name == orig_name {
            return;
        }

        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();
        self.found_method(&new_name, is_singleton, def_line, offset);
    }

    fn process_attr(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
        readable: bool,
        writable: bool,
    ) {
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        for arg in args {
            if let Some(name) = extract_symbol_or_string(arg) {
                if readable {
                    self.found_method(&name, is_singleton, def_line, offset);
                }
                if writable {
                    let setter = format!("{name}=");
                    self.found_method(&setter, is_singleton, def_line, offset);
                }
            }
        }
    }

    fn process_attr_legacy(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.is_empty() {
            return;
        }
        let name = match extract_symbol_or_string(&args[0]) {
            Some(n) => n,
            None => return,
        };

        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        // Always readable
        self.found_method(&name, is_singleton, def_line, offset);

        // Writable if second arg is `true`
        if args.len() == 2 && args[1].as_true_node().is_some() {
            let setter = format!("{name}=");
            self.found_method(&setter, is_singleton, def_line, offset);
        }
    }

    fn process_def_delegator(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.len() < 2 {
            return;
        }
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        if args.len() >= 3 {
            // Third arg is the alias name -- that's the method being defined
            if let Some(name) = extract_symbol_or_string(&args[2]) {
                self.found_method(&name, is_singleton, def_line, offset);
            }
        } else if let Some(name) = extract_symbol_or_string(&args[1]) {
            self.found_method(&name, is_singleton, def_line, offset);
        }
    }

    fn process_def_delegators(
        &mut self,
        node: &ruby_prism::CallNode<'_>,
        args: &[ruby_prism::Node<'_>],
    ) {
        if args.len() < 2 {
            return;
        }
        let is_singleton = self.in_singleton_scope();
        let (def_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let offset = node.location().start_offset();

        for arg in &args[1..] {
            if let Some(name) = extract_symbol_or_string(arg) {
                self.found_method(&name, is_singleton, def_line, offset);
            }
        }
    }
}

/// Extract a symbol or string value from a node.
fn extract_symbol_or_string(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(sym) = node.as_symbol_node() {
        return Some(
            std::str::from_utf8(sym.unescaped())
                .unwrap_or("")
                .to_string(),
        );
    }
    if let Some(s) = node.as_string_node() {
        return Some(std::str::from_utf8(s.unescaped()).unwrap_or("").to_string());
    }
    None
}

/// Check if a call node is a scope-creating pattern like `Class.new do`, `Module.new do`,
/// `class_eval do`, etc. Returns the scope name if so.
fn scope_creating_call_name(node: &ruby_prism::CallNode<'_>) -> Option<String> {
    let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");

    // class_eval / module_eval with block
    if matches!(method_name, "class_eval" | "module_eval") && node.block().is_some() {
        if let Some(recv) = node.receiver() {
            if let Some(const_read) = recv.as_constant_read_node() {
                let name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                return Some(name.to_string());
            }
            if let Some(const_path) = recv.as_constant_path_node() {
                return Some(constant_path_name(&const_path));
            }
        }
        // class_eval with no explicit receiver (implicit self inside module)
        if node.receiver().is_none() {
            return Some("__implicit_class_eval__".to_string());
        }
    }

    // Class.new do / Module.new do
    if method_name == "new" && node.block().is_some() {
        if let Some(recv) = node.receiver() {
            if let Some(const_read) = recv.as_constant_read_node() {
                let name = std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                if name == "Class" || name == "Module" {
                    return Some("__dynamic_class_new__".to_string());
                }
            }
        }
    }

    None
}

/// Build a full constant path name from a ConstantPathNode.
fn constant_path_name(node: &ruby_prism::ConstantPathNode<'_>) -> String {
    let child_name = node
        .name()
        .map(|n| std::str::from_utf8(n.as_slice()).unwrap_or(""))
        .unwrap_or("");

    if let Some(parent) = node.parent() {
        if let Some(parent_const) = parent.as_constant_read_node() {
            let parent_name = std::str::from_utf8(parent_const.name().as_slice()).unwrap_or("");
            return format!("{parent_name}::{child_name}");
        }
        if let Some(parent_path) = parent.as_constant_path_node() {
            return format!("{}::{child_name}", constant_path_name(&parent_path));
        }
    }
    child_name.to_string()
}

impl<'pr> Visit<'pr> for DupMethodVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = class_or_module_name_from_constant(node.constant_path());
        self.scope_stack.push(ScopeEntry {
            name,
            is_singleton: false,
        });
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let name = class_or_module_name_from_constant(node.constant_path());
        self.scope_stack.push(ScopeEntry {
            name,
            is_singleton: false,
        });
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let expr = node.expression();

        if expr.as_self_node().is_some() {
            // `class << self` — mark the current scope as singleton, don't nest
            let depth = self.scope_stack.len();
            if depth > 0 {
                let was_singleton = self.scope_stack[depth - 1].is_singleton;
                self.scope_stack[depth - 1].is_singleton = true;
                if let Some(body) = node.body() {
                    self.visit(&body);
                }
                self.scope_stack[depth - 1].is_singleton = was_singleton;
            } else {
                // At top level, `class << self` creates Object singleton
                self.scope_stack.push(ScopeEntry {
                    name: "Object".to_string(),
                    is_singleton: true,
                });
                if let Some(body) = node.body() {
                    self.visit(&body);
                }
                self.scope_stack.pop();
            }
        } else {
            // `class << SomeConst` or `class << expr`
            let scope_name = if let Some(const_read) = expr.as_constant_read_node() {
                std::str::from_utf8(const_read.name().as_slice())
                    .unwrap_or("")
                    .to_string()
            } else if let Some(call) = expr.as_call_node() {
                std::str::from_utf8(call.name().as_slice())
                    .unwrap_or("")
                    .to_string()
            } else {
                return;
            };

            // For `class << A` inside `class B`, the scope is just `A`
            // (not nested under B), since it's a different object's singleton class
            let saved_scopes = self.scope_stack.clone();
            self.scope_stack.clear();
            self.scope_stack.push(ScopeEntry {
                name: scope_name,
                is_singleton: true,
            });
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            self.scope_stack = saved_scopes;
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.process_def(node);

        // Push def name for nested method scoping
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.def_stack.push(name);

        // Visit body for nested defs
        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.def_stack.pop();
    }

    fn visit_alias_method_node(&mut self, node: &ruby_prism::AliasMethodNode<'pr>) {
        self.process_alias(node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");

        // Check for `private def foo` / `protected def foo` pattern
        if matches!(method_name, "private" | "protected" | "public") {
            if let Some(arguments) = node.arguments() {
                let args: Vec<ruby_prism::Node<'pr>> = arguments.arguments().iter().collect();
                if args.len() == 1 {
                    if let Some(def_node) = args[0].as_def_node() {
                        self.process_def(&def_node);
                        // Visit the def body for nested methods
                        let name = std::str::from_utf8(def_node.name().as_slice())
                            .unwrap_or("")
                            .to_string();
                        self.def_stack.push(name);
                        if let Some(body) = def_node.body() {
                            self.visit(&body);
                        }
                        self.def_stack.pop();
                        return;
                    }
                }
            }
        }

        // Check for scope-creating calls (Class.new, class_eval, etc.)
        if let Some(scope_name) = scope_creating_call_name(node) {
            let effective_name = if scope_name == "__implicit_class_eval__" {
                self.current_scope_name()
            } else if scope_name == "__dynamic_class_new__" {
                // Local variable assignment -- isolated scope per assignment
                // Constant assignment is handled by visit_constant_write_node.
                if let Some(block) = node.block() {
                    let saved_defs = std::mem::take(&mut self.definitions);
                    let saved_scopes = self.scope_stack.clone();
                    self.scope_stack.clear();
                    self.scope_stack.push(ScopeEntry {
                        name: "__anonymous__".to_string(),
                        is_singleton: false,
                    });
                    self.visit(&block);
                    self.definitions = saved_defs;
                    self.scope_stack = saved_scopes;
                }
                return;
            } else {
                scope_name
            };

            self.scope_stack.push(ScopeEntry {
                name: effective_name,
                is_singleton: false,
            });
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            self.scope_stack.pop();
            return;
        }

        // Check for alias_method, attr_*, def_delegator* calls
        self.process_call(node);

        // Visit children
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(arguments) = node.arguments() {
            for arg in arguments.arguments().iter() {
                self.visit(&arg);
            }
        }
        // Visit block as a plain (non-scope-creating) block.
        // Methods inside these blocks are ignored per RuboCop behavior.
        if let Some(block) = node.block() {
            self.plain_block_depth += 1;
            self.visit(&block);
            self.plain_block_depth -= 1;
        }
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let value = node.value();
        // Check for `A = Class.new do ... end` or `A = Module.new do ... end`
        if let Some(call) = value.as_call_node() {
            let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            if method_name == "new" && call.block().is_some() {
                if let Some(recv) = call.receiver() {
                    if let Some(const_read) = recv.as_constant_read_node() {
                        let recv_name =
                            std::str::from_utf8(const_read.name().as_slice()).unwrap_or("");
                        if recv_name == "Class" || recv_name == "Module" {
                            let const_name =
                                std::str::from_utf8(node.name().as_slice()).unwrap_or("");
                            self.scope_stack.push(ScopeEntry {
                                name: const_name.to_string(),
                                is_singleton: false,
                            });
                            if let Some(block) = call.block() {
                                self.visit(&block);
                            }
                            self.scope_stack.pop();
                            return;
                        }
                    }
                }
            }
        }
        // Default: visit children
        self.visit(&value);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.if_depth += 1;
        ruby_prism::visit_if_node(self, node);
        self.if_depth -= 1;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.if_depth += 1;
        ruby_prism::visit_unless_node(self, node);
        self.if_depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.if_depth += 1;
        ruby_prism::visit_case_node(self, node);
        self.if_depth -= 1;
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        self.if_depth += 1;
        ruby_prism::visit_case_match_node(self, node);
        self.if_depth -= 1;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        self.rescue_ensure_stack.push(Vec::new());
        ruby_prism::visit_rescue_node(self, node);
        self.rescue_ensure_stack.pop();
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        self.rescue_ensure_stack.push(Vec::new());
        ruby_prism::visit_ensure_node(self, node);
        self.rescue_ensure_stack.pop();
    }
}

/// Extract the name from a class/module constant path.
fn class_or_module_name_from_constant(constant_path: ruby_prism::Node<'_>) -> String {
    if let Some(const_read) = constant_path.as_constant_read_node() {
        return std::str::from_utf8(const_read.name().as_slice())
            .unwrap_or("")
            .to_string();
    }
    if let Some(const_path) = constant_path.as_constant_path_node() {
        return constant_path_name(&const_path);
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMethods, "cops/lint/duplicate_methods");
}
