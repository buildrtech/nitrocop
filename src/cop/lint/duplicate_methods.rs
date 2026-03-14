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
/// ## Investigation history
///
/// ### Round 1 (initial implementation)
/// Root causes of FN (1,203):
/// - Only checked direct `def` children of class/module StatementsNode
/// - Missed `private def`, `protected def` (CallNode wrapping DefNode)
/// - Missed `alias_method`, `attr_*`, `def_delegator*` call patterns
/// - Missed top-level definitions (Object scope)
/// - Missed reopened class/module blocks (separate `class A...end` blocks)
/// - Missed nested method scoping (method_key with ancestor def name)
/// - Missed `class << self` / `class << expr` singleton class patterns
/// - Wrong message format (was "Duplicated method definition." instead of RuboCop format)
///
/// Root causes of FP (47):
/// - Did not skip definitions inside `if`/`unless`/`case` ancestors
/// - Did not handle rescue/ensure scope reset
///
/// ### Round 2 (FP=16, FN=40)
/// Root causes of FN:
/// - `Struct.new do ... end` not recognized as scope-creating (only Class.new/Module.new
///   were handled). Fixed by adding Struct to scope_creating_call_name and
///   visit_constant_write_node.
/// - `def ConstName.method` where ConstName is an outer scope (not innermost). The old
///   `scope_matches_const` only checked innermost scope. Fixed by implementing
///   `lookup_constant` that traverses the full scope stack, matching RuboCop behavior.
///
/// Root causes of FP:
/// - rescue/ensure handling used per-block scope (stack of Vecs) instead of global
///   per-type scope like RuboCop. RuboCop uses `@scopes[:rescue]` and `@scopes[:ensure]`
///   as global sets — first redefinition across ALL rescue blocks is forgiven, second is
///   offense. Fixed by replacing `rescue_ensure_stack: Vec<Vec<String>>` with global
///   `rescue_forgiven` and `ensure_forgiven` sets plus a type stack.
///
/// Remaining items not implemented (likely not significant for corpus):
/// - `delegate :foo, to: :bar` (ActiveSupport) — only active when
///   ActiveSupportExtensionsEnabled is true, which is off by default in corpus baseline.
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
            in_rescue_or_ensure: false,
            rescue_forgiven: Vec::new(),
            ensure_forgiven: Vec::new(),
            rescue_ensure_type_stack: Vec::new(),
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
    /// Whether we are currently inside a rescue or ensure node (any depth).
    /// Matches RuboCop's `node.each_ancestor(:rescue, :ensure).first&.type`.
    in_rescue_or_ensure: bool,
    /// Global per-type scope for rescue blocks. Keys forgiven in any rescue block
    /// are tracked here. Matches RuboCop's `@scopes[:rescue]`.
    rescue_forgiven: Vec<String>,
    /// Global per-type scope for ensure blocks. Keys forgiven in any ensure block
    /// are tracked here. Matches RuboCop's `@scopes[:ensure]`.
    ensure_forgiven: Vec<String>,
    /// Which scope type the current rescue/ensure ancestors belong to.
    /// Used to decide which forgiven set to check.
    rescue_ensure_type_stack: Vec<RescueEnsureType>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RescueEnsureType {
    Rescue,
    Ensure,
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

        // Handle rescue/ensure scope: first occurrence of a method key inside
        // ANY rescue (or ensure) body is allowed to "redefine" — it's a different
        // execution path. RuboCop uses @scopes[:rescue] / @scopes[:ensure] as
        // global per-type sets (not per-block). So the first redefinition across
        // ALL rescue blocks is forgiven, but the second is an offense.
        if self.in_rescue_or_ensure && self.definitions.contains_key(&key) {
            if let Some(&scope_type) = self.rescue_ensure_type_stack.last() {
                let forgiven = match scope_type {
                    RescueEnsureType::Rescue => &self.rescue_forgiven,
                    RescueEnsureType::Ensure => &self.ensure_forgiven,
                };
                if !forgiven.contains(&key) {
                    // First time this key is redefined in this scope type — forgive it
                    self.definitions
                        .insert(key.clone(), DefLocation { line: def_line });
                    match scope_type {
                        RescueEnsureType::Rescue => self.rescue_forgiven.push(key),
                        RescueEnsureType::Ensure => self.ensure_forgiven.push(key),
                    }
                    return;
                }
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
                if let Some(qualified_scope) = self.lookup_constant(const_name) {
                    // Save current scope, temporarily replace with resolved scope
                    let saved_scopes = self.scope_stack.clone();
                    self.scope_stack.clear();
                    self.scope_stack.push(ScopeEntry {
                        name: qualified_scope,
                        is_singleton: true,
                    });
                    let keyword_offset = node.def_keyword_loc().start_offset();
                    self.found_method(name, true, def_line, keyword_offset);
                    self.scope_stack = saved_scopes;
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

    /// Look up a constant name in the scope stack, matching RuboCop's `lookup_constant`.
    /// Returns the fully qualified scope name if found, or None if not found.
    fn lookup_constant(&self, const_name: &str) -> Option<String> {
        // Walk the scope stack from innermost to outermost, looking for a match.
        // Each scope entry may be a simple name ("A") or a constant path ("A::B").
        // Check each component of the name.
        for (i, entry) in self.scope_stack.iter().enumerate().rev() {
            // Check if this entry's name matches directly or as a component
            let parts: Vec<&str> = entry.name.split("::").collect();
            for part in &parts {
                if *part == const_name {
                    // Found the matching constant. Build the qualified name
                    // up to and including this scope entry.
                    let mut result_parts = Vec::new();
                    for e in &self.scope_stack[..=i] {
                        result_parts.push(e.name.as_str());
                    }
                    return Some(result_parts.join("::"));
                }
            }
        }
        None
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
                if name == "Class" || name == "Module" || name == "Struct" {
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
                        if recv_name == "Class" || recv_name == "Module" || recv_name == "Struct" {
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
        let was = self.in_rescue_or_ensure;
        self.in_rescue_or_ensure = true;
        self.rescue_ensure_type_stack.push(RescueEnsureType::Rescue);
        ruby_prism::visit_rescue_node(self, node);
        self.rescue_ensure_type_stack.pop();
        self.in_rescue_or_ensure = was;
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        let was = self.in_rescue_or_ensure;
        self.in_rescue_or_ensure = true;
        self.rescue_ensure_type_stack.push(RescueEnsureType::Ensure);
        ruby_prism::visit_ensure_node(self, node);
        self.rescue_ensure_type_stack.pop();
        self.in_rescue_or_ensure = was;
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

    use crate::testutil::run_cop_full;

    fn count_offenses(source: &[u8]) -> usize {
        run_cop_full(&DuplicateMethods, source).len()
    }

    // Hypothesis tests for identifying FP/FN root causes

    #[test]
    fn test_rescue_ensure_global_scope() {
        // RuboCop uses a global per-type scope for rescue/ensure.
        // Nested defs inside `setup` are scoped as "setup.Foo#bar"
        // so the two defs inside setup are the same key. Rescue forgives.
        let n = count_offenses(b"class Foo\n  def bar; 1; end\n\n  def setup\n    def bar; 2; end\n  rescue\n    def bar; 3; end\n  end\nend\n");
        assert_eq!(n, 0, "rescue should forgive first redefinition");
    }

    #[test]
    fn test_struct_new_as_scope() {
        // Struct.new do ... end assigned to constant should create a scope
        let n =
            count_offenses(b"A = Struct.new(:x) do\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(n, 1, "Struct.new should create scope and detect duplicates");
    }

    #[test]
    fn test_class_eval_with_string() {
        // class_eval with a string argument (not block) should not create scope
        let n = count_offenses(
            b"class Foo\n  def bar; 1; end\nend\nFoo.class_eval(\"def baz; end\")\n",
        );
        assert_eq!(n, 0, "class_eval with string should not create scope");
    }

    #[test]
    fn test_module_eval_scope() {
        // module_eval should work same as class_eval
        let n = count_offenses(b"Foo.module_eval do\n  def bar; 1; end\n  def bar; 2; end\nend\n");
        assert_eq!(
            n, 1,
            "module_eval should create scope and detect duplicates"
        );
    }

    #[test]
    fn test_class_open_constant_path() {
        // class A::B should be handled
        let n = count_offenses(b"class A::B\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(n, 1, "class with constant path should detect duplicates");
    }

    #[test]
    fn test_dup_inside_class_with_block() {
        // Methods inside `included do` or similar blocks should be ignored
        let n = count_offenses(
            b"class Foo\n  included do\n    def bar; 1; end\n    def bar; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 0, "methods in DSL blocks should be ignored");
    }

    #[test]
    fn test_sclass_inside_sclass() {
        // class << self inside class has nested class B - different scopes
        let n = count_offenses(b"class Foo\n  class << self\n    def bar; 1; end\n\n    class B\n      def bar; 2; end\n    end\n  end\nend\n");
        assert_eq!(n, 0, "different scopes should not conflict");
    }

    #[test]
    fn test_rescue_ensure_rubocop_behavior() {
        // Ensure scope should forgive first redefinition of alias_method :save
        let n = count_offenses(b"module FooTest\n  def make_save_always_fail\n    Foo.class_eval do\n      def failed_save\n        raise\n      end\n      alias_method :original_save, :save\n      alias_method :save, :failed_save\n    end\n\n    yield\n  ensure\n    Foo.class_eval do\n      alias_method :save, :original_save\n    end\n  end\nend\n");
        assert_eq!(n, 0, "ensure scope should forgive first redefinition");
    }

    #[test]
    fn test_rescue_global_type_scope() {
        // RuboCop: @scopes[:rescue] is shared across ALL rescue blocks.
        // Second rescue should NOT forgive a method already forgiven by first rescue.
        let n = count_offenses(b"class Foo\n  def bar; 1; end\n\n  def test1\n    def bar; 2; end\n  rescue\n    def bar; 3; end\n  end\n\n  def test2\n    x = 1\n  rescue\n    def bar; 4; end\n  end\nend\n");
        // bar is defined at line 2. In test1: nested bar is "test1.Foo#bar" (different key).
        // In test2: nested bar is "test2.Foo#bar" (different key). No conflicts.
        assert_eq!(n, 0, "different enclosing methods create different keys");
    }

    #[test]
    fn test_multiple_rescue_same_method() {
        // In the same enclosing scope (no def wrapper), multiple rescue blocks share type scope
        // RuboCop uses @scopes[:rescue] globally - first forgiven, second is offense
        let n = count_offenses(b"class Foo\n  def bar; 1; end\n\n  begin\n    x = 1\n  rescue\n    def bar; 2; end\n  end\n\n  begin\n    y = 1\n  rescue\n    def bar; 3; end\n  end\nend\n");
        // bar defined at line 2. First rescue: bar redef forgiven. Second rescue: offense.
        // RuboCop: @scopes[:rescue] = ["Foo#bar"] after first rescue
        // Second rescue: key "Foo#bar" already in @scopes[:rescue], so offense.
        assert_eq!(
            n, 1,
            "second rescue should report offense (global type scope)"
        );
    }

    #[test]
    fn test_constant_path_class_name() {
        // class A::B::C should produce scope "A::B::C"
        let n = count_offenses(b"class A::B::C\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(
            n, 1,
            "class with deep constant path should detect duplicates"
        );
    }

    #[test]
    fn test_local_var_struct_new_isolated() {
        // local = Struct.new do ... end should isolate scope (like Class.new)
        let n = count_offenses(b"a = Struct.new(:x) do\n  def foo; 1; end\nend\nb = Struct.new(:x) do\n  def foo; 2; end\nend\n");
        assert_eq!(n, 0, "local var Struct.new should isolate scopes");
    }

    #[test]
    fn test_constant_write_struct_new_separate() {
        // Two different constants with Struct.new should have separate scopes
        let n = count_offenses(b"A = Struct.new(:x) do\n  def foo; 1; end\nend\nB = Struct.new(:y) do\n  def foo; 2; end\nend\n");
        assert_eq!(n, 0, "separate Struct.new constants have separate scopes");
    }

    #[test]
    fn test_reopened_struct_new_detects_dup() {
        // Same constant Struct.new opened twice should detect dup
        let n = count_offenses(b"A = Struct.new(:x) do\n  def foo; 1; end\nend\nA = Struct.new(:y) do\n  def foo; 2; end\nend\n");
        assert_eq!(n, 1, "reopened Struct.new constant should detect dup");
    }

    #[test]
    fn test_case_should_suppress() {
        // Methods inside case/when should be suppressed (like if/unless)
        let n = count_offenses(b"class Foo\n  case RUBY_VERSION\n  when '3.0'\n    def bar; 1; end\n  when '2.7'\n    def bar; 2; end\n  end\nend\n");
        assert_eq!(n, 0, "case/when should suppress duplicate detection");
    }

    #[test]
    fn test_def_inside_for_loop() {
        // Methods inside for loops should NOT be suppressed by nitrocop
        // (RuboCop only suppresses if_type? ancestors, not for loops)
        let n = count_offenses(
            b"class Foo\n  def bar; 1; end\n  for x in items\n    def bar; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "for loop should not suppress duplicate detection");
    }

    #[test]
    fn test_class_eval_constant_path() {
        // A::B.class_eval do should create scope "A::B"
        let n = count_offenses(b"A::B.class_eval do\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        assert_eq!(
            n, 1,
            "class_eval with constant path should detect duplicates"
        );
    }

    #[test]
    fn test_constant_write_with_non_new_method() {
        // A = SomeThing.build do...end should NOT create scope
        // Only Class.new/Module.new/Struct.new create scopes
        let n =
            count_offenses(b"A = SomeThing.build do\n  def foo; 1; end\n  def foo; 2; end\nend\n");
        // This is a plain block — methods inside should be ignored
        assert_eq!(n, 0, "non-new method blocks should be ignored");
    }

    #[test]
    fn test_class_reopened_with_sclass() {
        // Reopened class with class << self should detect dups
        let n = count_offenses(b"class A\n  class << self\n    def foo; 1; end\n  end\nend\nclass A\n  class << self\n    def foo; 2; end\n  end\nend\n");
        assert_eq!(n, 1, "reopened class with sclass should detect dups");
    }

    #[test]
    fn test_mixed_def_self_and_sclass() {
        // def self.foo and class << self; def foo should be same scope
        let n = count_offenses(
            b"class A\n  def self.foo; 1; end\n  class << self\n    def foo; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "def self.foo and sclass def foo should be same scope");
    }

    #[test]
    fn test_def_const_outer_scope() {
        // def M.foo inside class A within module M should resolve to M.foo
        let n = count_offenses(
            b"module M\n  class A\n    def M.foo; 1; end\n    def M.foo; 2; end\n  end\nend\n",
        );
        assert_eq!(n, 1, "def M.foo should resolve M to outer scope");
    }

    #[test]
    fn test_def_const_and_self_same_class() {
        // def A.foo and def self.foo inside class A should be same
        let n = count_offenses(b"class A\n  def A.foo; 1; end\n  def self.foo; 2; end\nend\n");
        assert_eq!(n, 1, "def A.foo and def self.foo should be same");
    }
}
