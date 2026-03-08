use crate::cop::node_type::{ALIAS_METHOD_NODE, CALL_NODE, DEF_NODE};
use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Naming/MethodName cop — checks that method names use the configured naming style.
///
/// ## Investigation (2026-03-08)
/// FP=0, FN=590 in corpus. Root causes:
/// 1. AllowedPatterns used substring matching instead of regex — fixed to use regex::Regex.
/// 2. Only handled DEF_NODE — missing attr_reader/attr_writer/attr_accessor/attr (CALL_NODE),
///    define_method/define_singleton_method (CALL_NODE), Struct.new/Data.define member names
///    (CALL_NODE), alias keyword (ALIAS_METHOD_NODE), and alias_method (CALL_NODE).
/// 3. All of these are now handled, matching RuboCop's on_def/on_defs/on_send/on_alias handlers.
///
/// Follow-up (2026-03-08): FP=8 regressed at sites using
/// `# rubocop:disable Style/MethodName`. RuboCop still suppresses
/// `Naming/MethodName` for that moved legacy name because the short name stayed
/// `MethodName`. Fixed centrally in `parse/directives.rs`.
pub struct MethodName;

/// Bundles config values needed for method name checking.
struct MethodNameConfig {
    enforced_style: String,
    allowed_patterns: Option<Vec<String>>,
    forbidden_identifiers: Option<Vec<String>>,
    forbidden_patterns: Option<Vec<String>>,
}

impl MethodNameConfig {
    fn from_cop_config(config: &CopConfig) -> Self {
        Self {
            enforced_style: config.get_str("EnforcedStyle", "snake_case").to_string(),
            allowed_patterns: config.get_string_array("AllowedPatterns"),
            forbidden_identifiers: config.get_string_array("ForbiddenIdentifiers"),
            forbidden_patterns: config.get_string_array("ForbiddenPatterns"),
        }
    }
}

/// Returns true if the name consists entirely of non-alphabetic characters (operator methods).
fn is_operator_method(name: &[u8]) -> bool {
    !name.iter().any(|b| b.is_ascii_alphabetic())
}

/// Check if a method name matches AllowedPatterns using regex matching.
fn matches_allowed_pattern(name: &str, allowed_patterns: &Option<Vec<String>>) -> bool {
    if let Some(patterns) = allowed_patterns {
        for p in patterns {
            if let Ok(re) = regex::Regex::new(p) {
                if re.is_match(name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a method name is forbidden by ForbiddenIdentifiers or ForbiddenPatterns.
fn is_forbidden_name(name: &str, cfg: &MethodNameConfig) -> bool {
    if let Some(forbidden) = &cfg.forbidden_identifiers {
        if forbidden.iter().any(|f| f == name) {
            return true;
        }
    }
    if let Some(patterns) = &cfg.forbidden_patterns {
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check naming style compliance.
fn style_ok(name: &[u8], enforced_style: &str) -> bool {
    match enforced_style {
        "camelCase" => is_lower_camel_case(name),
        _ => is_snake_case(name),
    }
}

fn style_msg(enforced_style: &str) -> &str {
    match enforced_style {
        "camelCase" => "camelCase",
        _ => "snake_case",
    }
}

/// Extract a method name string from a symbol or string node.
fn extract_name_from_sym_or_str<'a>(
    node: &'a ruby_prism::Node<'a>,
) -> Option<(Vec<u8>, ruby_prism::Location<'a>)> {
    if let Some(sym) = node.as_symbol_node() {
        Some((sym.unescaped().to_vec(), sym.location()))
    } else {
        node.as_string_node()
            .map(|s| (s.unescaped().to_vec(), s.location()))
    }
}

impl Cop for MethodName {
    fn name(&self) -> &'static str {
        "Naming/MethodName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, CALL_NODE, ALIAS_METHOD_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let cfg = MethodNameConfig::from_cop_config(config);

        if let Some(def_node) = node.as_def_node() {
            check_def_node(self, source, &def_node, &cfg, diagnostics);
        } else if let Some(call_node) = node.as_call_node() {
            check_call_node(self, source, &call_node, &cfg, diagnostics);
        } else if let Some(alias_node) = node.as_alias_method_node() {
            check_alias_node(self, source, &alias_node, &cfg, diagnostics);
        }
    }
}

fn check_def_node(
    cop: &MethodName,
    source: &SourceFile,
    def_node: &ruby_prism::DefNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let method_name = def_node.name().as_slice();
    let method_name_str = std::str::from_utf8(method_name).unwrap_or("");

    if is_operator_method(method_name) {
        return;
    }

    if matches_allowed_pattern(method_name_str, &cfg.allowed_patterns) {
        return;
    }

    // Skip CamelCase singleton methods (def self.ClassName) — RuboCop allows these
    // as factory/constructor methods when a matching class exists in scope.
    // We approximate by skipping all uppercase-starting singleton method names.
    if def_node.receiver().is_some() && method_name.first().is_some_and(|b| b.is_ascii_uppercase())
    {
        return;
    }

    let loc = def_node.name_loc();
    let (line, column) = source.offset_to_line_col(loc.start_offset());

    if is_forbidden_name(method_name_str, cfg) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("`{method_name_str}` is forbidden, use another method name instead."),
        ));
        return;
    }

    if !style_ok(method_name, &cfg.enforced_style) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Use {} for method names.", style_msg(&cfg.enforced_style)),
        ));
    }
}

fn check_call_node(
    cop: &MethodName,
    source: &SourceFile,
    call_node: &ruby_prism::CallNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let method = call_node.name().as_slice();
    let method_str = std::str::from_utf8(method).unwrap_or("");

    match method_str {
        "define_method" | "define_singleton_method" => {
            check_define_method(cop, source, call_node, cfg, diagnostics);
        }
        "alias_method" => {
            check_alias_method_call(cop, source, call_node, cfg, diagnostics);
        }
        "attr" | "attr_reader" | "attr_writer" | "attr_accessor" => {
            check_attr_accessor(cop, source, call_node, cfg, diagnostics);
        }
        "new" | "define" => {
            check_struct_or_data(cop, source, call_node, cfg, diagnostics);
        }
        _ => {}
    }
}

fn check_define_method(
    cop: &MethodName,
    source: &SourceFile,
    call_node: &ruby_prism::CallNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let args = match call_node.arguments() {
        Some(a) => a,
        None => return,
    };
    let args_list: Vec<_> = args.arguments().iter().collect();
    if args_list.is_empty() {
        return;
    }

    let (name_bytes, loc) = match extract_name_from_sym_or_str(&args_list[0]) {
        Some(v) => v,
        None => return,
    };

    let name_str = match std::str::from_utf8(&name_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    if is_operator_method(&name_bytes) {
        return;
    }

    emit_method_name_offense(cop, source, name_str, &name_bytes, &loc, cfg, diagnostics);
}

fn check_alias_method_call(
    cop: &MethodName,
    source: &SourceFile,
    call_node: &ruby_prism::CallNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let args = match call_node.arguments() {
        Some(a) => a,
        None => return,
    };
    let args_list: Vec<_> = args.arguments().iter().collect();

    // RuboCop requires exactly 2 arguments
    if args_list.len() != 2 {
        return;
    }

    let (name_bytes, loc) = match extract_name_from_sym_or_str(&args_list[0]) {
        Some(v) => v,
        None => return,
    };

    let name_str = match std::str::from_utf8(&name_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    emit_method_name_offense(cop, source, name_str, &name_bytes, &loc, cfg, diagnostics);
}

fn check_attr_accessor(
    cop: &MethodName,
    source: &SourceFile,
    call_node: &ruby_prism::CallNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must have no receiver (bare attr_reader, not obj.attr_reader)
    if call_node.receiver().is_some() {
        return;
    }

    let args = match call_node.arguments() {
        Some(a) => a,
        None => return,
    };

    for arg in args.arguments().iter() {
        let (name_bytes, loc) = match extract_name_from_sym_or_str(&arg) {
            Some(v) => v,
            None => continue,
        };

        let name_str = match std::str::from_utf8(&name_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if matches_allowed_pattern(name_str, &cfg.allowed_patterns) {
            continue;
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        if is_forbidden_name(name_str, cfg) {
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                format!("`{name_str}` is forbidden, use another method name instead."),
            ));
            continue;
        }

        if !style_ok(&name_bytes, &cfg.enforced_style) {
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                format!("Use {} for method names.", style_msg(&cfg.enforced_style)),
            ));
        }
    }
}

fn check_struct_or_data(
    cop: &MethodName,
    source: &SourceFile,
    call_node: &ruby_prism::CallNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let method = call_node.name().as_slice();
    let receiver = match call_node.receiver() {
        Some(r) => r,
        None => return,
    };

    let is_struct_new = method == b"new" && is_const_named(&receiver, b"Struct");
    let is_data_define = method == b"define" && is_const_named(&receiver, b"Data");

    if !is_struct_new && !is_data_define {
        return;
    }

    let args = match call_node.arguments() {
        Some(a) => a,
        None => return,
    };

    let args_list: Vec<_> = args.arguments().iter().collect();

    // For Struct.new, skip the first argument if it's a string (class name)
    let start_idx = if is_struct_new {
        if args_list
            .first()
            .is_some_and(|a| a.as_string_node().is_some())
        {
            1
        } else {
            0
        }
    } else {
        0
    };

    for arg in &args_list[start_idx..] {
        let (name_bytes, loc) = match extract_name_from_sym_or_str(arg) {
            Some(v) => v,
            None => continue,
        };

        let name_str = match std::str::from_utf8(&name_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        emit_method_name_offense(cop, source, name_str, &name_bytes, &loc, cfg, diagnostics);
    }
}

fn check_alias_node(
    cop: &MethodName,
    source: &SourceFile,
    alias_node: &ruby_prism::AliasMethodNode<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let new_name = alias_node.new_name();
    let sym = match new_name.as_symbol_node() {
        Some(s) => s,
        None => return,
    };

    let name_bytes = sym.unescaped().to_vec();
    let name_str = match std::str::from_utf8(&name_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    let loc = sym.location();
    emit_method_name_offense(cop, source, name_str, &name_bytes, &loc, cfg, diagnostics);
}

/// Emit an offense for a method name that violates naming rules.
fn emit_method_name_offense(
    cop: &MethodName,
    source: &SourceFile,
    name_str: &str,
    name_bytes: &[u8],
    loc: &ruby_prism::Location<'_>,
    cfg: &MethodNameConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if matches_allowed_pattern(name_str, &cfg.allowed_patterns) {
        return;
    }

    let (line, column) = source.offset_to_line_col(loc.start_offset());

    if is_forbidden_name(name_str, cfg) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("`{name_str}` is forbidden, use another method name instead."),
        ));
        return;
    }

    if is_operator_method(name_bytes) {
        return;
    }

    if !style_ok(name_bytes, &cfg.enforced_style) {
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Use {} for method names.", style_msg(&cfg.enforced_style)),
        ));
    }
}

/// Check if a node is a constant reference to the given name (handles both `Foo` and `::Foo`).
fn is_const_named(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        if cp.parent().is_none() {
            if let Some(child_name) = cp.name() {
                return child_name.as_slice() == name;
            }
        }
    }
    false
}

/// Returns true if the name is lowerCamelCase (starts lowercase, no underscores, has uppercase).
fn is_lower_camel_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    if name[0].is_ascii_uppercase() {
        return false;
    }
    let name_without_leading = name
        .iter()
        .skip_while(|&&b| b == b'_')
        .copied()
        .collect::<Vec<_>>();
    for &b in &name_without_leading {
        if b == b'_' {
            return false;
        }
        if !(b.is_ascii_alphanumeric() || b == b'?' || b == b'!' || b == b'=') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodName, "cops/naming/method_name");

    #[test]
    fn config_enforced_style_camel_case() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("camelCase".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def myMethod\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(
            diags.is_empty(),
            "camelCase method should not be flagged in camelCase mode"
        );
    }

    #[test]
    fn config_enforced_style_camel_case_flags_snake() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("camelCase".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def my_method\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(
            !diags.is_empty(),
            "snake_case method should be flagged in camelCase mode"
        );
    }

    #[test]
    fn config_forbidden_identifiers() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "ForbiddenIdentifiers".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("destroy".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"def destroy\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(!diags.is_empty(), "Forbidden identifier should be flagged");
        assert!(diags[0].message.contains("forbidden"));
    }

    #[test]
    fn config_forbidden_patterns() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "ForbiddenPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("_v1\\z".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"def release_v1\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(!diags.is_empty(), "Forbidden pattern should be flagged");
        assert!(diags[0].message.contains("forbidden"));
    }

    #[test]
    fn allowed_patterns_uses_regex() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String(
                    "\\AonSelectionBulkChange\\z".into(),
                )]),
            )]),
            ..CopConfig::default()
        };
        let source = b"def onSelectionBulkChange(arg)\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config.clone());
        assert!(
            diags.is_empty(),
            "Method matching AllowedPatterns regex should not be flagged"
        );

        let source2 = b"def otherCamelCase\nend\n";
        let diags2 = run_cop_full_with_config(&MethodName, source2, config);
        assert!(
            !diags2.is_empty(),
            "Non-matching camelCase should still be flagged"
        );
    }
}
