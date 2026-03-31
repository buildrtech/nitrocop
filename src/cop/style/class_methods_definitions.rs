use crate::cop::node_type::{DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ClassMethodsDefinitions cop.
///
/// ## Investigation findings (round 1)
/// FP root cause: The cop did not recognize `private :method_name` or
/// `protected :method_name` calls (symbol arguments) as making methods
/// non-public. It only handled standalone modifiers (`private` with no args)
/// and inline `private def foo` forms. Fixed by treating `private`/`protected`
/// calls with non-def arguments (symbol args) as marking at least one method
/// non-public, so the block is not flagged.
///
/// ## Investigation findings (round 2)
/// FP root causes:
/// 1. Single-line `class << self; def meth; end; end` — RuboCop does not flag
///    these. The criterion is: if any plain `def` node starts on the same line
///    as the `class << self` keyword, no offense is reported.
/// 2. `def self.x` and `def Receiver.method` inside `class << self` were
///    counted as plain defs. RuboCop's `each_child_node(:def)` only collects
///    `def` nodes (no receiver), not `defs` nodes (with receiver). Fixed by
///    checking `def_node.receiver().is_none()`.
///
/// FN root causes:
/// 1. `private :symbol_name` with arguments caused an immediate `return false`,
///    but RuboCop only considers a method private if: (a) standalone `private`
///    (block style) precedes it, (b) `private def foo` wraps it, or (c)
///    `private :foo` appears as a RIGHT sibling (after the def). `private :foo`
///    BEFORE `def foo` does NOT make the subsequent `def foo` private — the new
///    definition is public. Fixed by tracking per-method-name visibility from
///    post-hoc `private :name` calls (right siblings only).
/// 2. `include`/`extend`/`attr_reader` etc. alongside `def` nodes — these were
///    already handled correctly but the FN was masked by the `private :symbol`
///    early return bug above.
pub struct ClassMethodsDefinitions;

impl Cop for ClassMethodsDefinitions {
    fn name(&self) -> &'static str {
        "Style/ClassMethodsDefinitions"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "def_self");

        if enforced_style == "def_self" {
            // Check for `class << self` with public methods
            if let Some(sclass) = node.as_singleton_class_node() {
                let expr = sclass.expression();
                if expr.as_self_node().is_some() {
                    // Check if body has defs and ALL are public
                    if let Some(body) = sclass.body() {
                        let sclass_line = source
                            .offset_to_line_col(sclass.location().start_offset())
                            .0;
                        if all_defs_public(source, &body, sclass_line) {
                            let loc = sclass.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            let mut diag = self.diagnostic(
                                source,
                                line,
                                column,
                                "Do not define public methods within class << self.".to_string(),
                            );

                            if let Some(corrections) = corrections.as_mut() {
                                if let Some(replacement) =
                                    rewrite_simple_sclass_to_def_self(source, &sclass)
                                {
                                    corrections.push(crate::correction::Correction {
                                        start: loc.start_offset(),
                                        end: loc.end_offset(),
                                        replacement,
                                        cop_name: self.name(),
                                        cop_index: 0,
                                    });
                                    diag.corrected = true;
                                }
                            }

                            diagnostics.push(diag);
                        }
                    }
                }
            }
        }
    }
}

/// Returns true if the sclass body contains at least one plain `def` node
/// (no receiver) and ALL such `def` nodes are public. This matches RuboCop's
/// `all_methods_public?` which only flags `class << self` when every method
/// can be trivially converted to `def self.method_name`.
///
/// Also returns false (skip) if any plain `def` starts on the same line as
/// the `class << self` keyword — RuboCop does not flag compact single-line forms.
fn all_defs_public(source: &SourceFile, body: &ruby_prism::Node<'_>, sclass_line: usize) -> bool {
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
            // Single-statement body: check if it's a plain def node (no receiver)
            if let Some(def_node) = body.as_def_node() {
                if def_node.receiver().is_some() {
                    return false; // `def self.x` — not a plain def
                }
                // Check single-line: if def is on same line as class << self, skip
                let def_line = source
                    .offset_to_line_col(def_node.location().start_offset())
                    .0;
                return def_line != sclass_line;
            }
            return false;
        }
    };

    let stmts_vec: Vec<_> = stmts.body().iter().collect();
    let mut found_def = false;
    let mut in_private = false;

    for stmt in &stmts_vec {
        // Check for access modifier calls (private, protected, public)
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if call.receiver().is_none() {
                if call.arguments().is_none() {
                    // Standalone modifier: `private` / `protected` / `public`
                    if name == b"private" || name == b"protected" {
                        in_private = true;
                        continue;
                    }
                    if name == b"public" {
                        in_private = false;
                        continue;
                    }
                } else if name == b"private" || name == b"protected" {
                    if let Some(args) = call.arguments() {
                        // Check if this is `private def foo` (inline modifier)
                        for arg in args.arguments().iter() {
                            if arg.as_def_node().is_some() {
                                // Inline `private def foo` — the def is non-public
                                return false;
                            }
                        }
                    }
                    // `private :name` — handled per-def via right-sibling check below
                    continue;
                } else if name == b"public" {
                    // `public def foo` — the def is explicitly public.
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if let Some(def_node) = arg.as_def_node() {
                                if def_node.receiver().is_none() {
                                    let def_line = source
                                        .offset_to_line_col(def_node.location().start_offset())
                                        .0;
                                    if def_line == sclass_line {
                                        return false; // Same line as class << self
                                    }
                                    found_def = true;
                                }
                            }
                        }
                    }
                    continue;
                }
            }
        }

        if let Some(def_node) = stmt.as_def_node() {
            // Only consider plain defs (no receiver like `def self.x`)
            if def_node.receiver().is_some() {
                continue;
            }

            // Check if def is on the same line as class << self
            let def_line = source
                .offset_to_line_col(def_node.location().start_offset())
                .0;
            if def_line == sclass_line {
                return false; // Single-line form — RuboCop does not flag
            }

            if in_private {
                return false; // Non-public def found (block-style private)
            }

            // Check if this method name was made private/protected by a post-hoc
            // `private :name` call (right sibling). Only RIGHT siblings count —
            // `private :name` BEFORE `def name` does NOT make the new def private.
            let method_name = def_node.name().as_slice();
            let mut made_private_by_right_sibling = false;
            // Find this stmt's position and check only right siblings
            let stmt_offset = def_node.location().start_offset();
            for later_stmt in &stmts_vec {
                if let Some(later_call) = later_stmt.as_call_node() {
                    if later_call.location().start_offset() <= stmt_offset {
                        continue; // Skip left siblings and self
                    }
                    let later_name = later_call.name().as_slice();
                    if later_call.receiver().is_none()
                        && (later_name == b"private" || later_name == b"protected")
                    {
                        if let Some(args) = later_call.arguments() {
                            for arg in args.arguments().iter() {
                                if let Some(sym) = arg.as_symbol_node() {
                                    if sym.unescaped() == method_name {
                                        made_private_by_right_sibling = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if made_private_by_right_sibling {
                return false; // Non-public by post-hoc `private :name`
            }

            found_def = true;
        }
    }
    found_def
}

fn rewrite_simple_sclass_to_def_self(
    source: &SourceFile,
    sclass: &ruby_prism::SingletonClassNode<'_>,
) -> Option<String> {
    let body = sclass.body()?;
    let stmts = body.as_statements_node()?;
    let stmt_vec: Vec<_> = stmts.body().iter().collect();
    if stmt_vec.is_empty() {
        return None;
    }

    let mut rewritten = Vec::new();
    let sclass_col = source
        .offset_to_line_col(sclass.location().start_offset())
        .1;

    for stmt in stmt_vec {
        let def_node = stmt.as_def_node()?;
        if def_node.receiver().is_some() {
            return None;
        }

        let def_src = source.byte_slice(
            def_node.location().start_offset(),
            def_node.location().end_offset(),
            "",
        );

        let mut lines: Vec<String> = def_src.lines().map(|l| l.to_string()).collect();
        if lines.is_empty() {
            return None;
        }

        let def_col = source
            .offset_to_line_col(def_node.location().start_offset())
            .1;
        let dedent = def_col.saturating_sub(sclass_col);

        for line in &mut lines {
            if dedent > 0 && line.len() >= dedent {
                let mut remove = 0usize;
                for b in line.as_bytes().iter().take(dedent) {
                    if *b == b' ' {
                        remove += 1;
                    } else {
                        break;
                    }
                }
                line.drain(..remove);
            }
        }

        if let Some(first) = lines.first_mut() {
            if let Some(rest) = first.strip_prefix("def ") {
                *first = format!("def self.{rest}");
            } else {
                return None;
            }
        }

        rewritten.push(lines.join("\n"));
    }

    let mut out = rewritten.join("\n\n");
    out = out.replace("\n\ndef self.", "\n\n  def self.");
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ClassMethodsDefinitions,
        "cops/style/class_methods_definitions"
    );
    crate::cop_autocorrect_fixture_tests!(
        ClassMethodsDefinitions,
        "cops/style/class_methods_definitions"
    );
}
