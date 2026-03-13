use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/StringHashKeys checks for the use of strings as keys in hashes.
///
/// ## Investigation findings (2026-03-13)
///
/// Root cause of 239 FPs: RuboCop has a `receive_environments_method?` matcher
/// that exempts string hash keys when the hash is passed to methods that
/// commonly use string keys for environment variables or replacement mappings:
/// - `IO.popen({"FOO" => "bar"}, ...)`
/// - `Open3.capture2/capture2e/capture3/popen2/popen2e/popen3({"FOO" => "bar"}, ...)`
/// - `Open3.pipeline/pipeline_r/pipeline_rw/pipeline_start/pipeline_w([{"FOO" => "bar"}, ...], ...)`
/// - `Kernel.spawn/system({"FOO" => "bar"}, ...)` (including bare `spawn`/`system`)
/// - `str.gsub/gsub!(pattern, {"old" => "new"})`
///
/// Fix: Converted from `check_node` to `check_source` with a visitor that
/// tracks whether we're inside an exempted method call's arguments.
pub struct StringHashKeys;

impl Cop for StringHashKeys {
    fn name(&self) -> &'static str {
        "Style/StringHashKeys"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = StringHashKeysVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            exempt_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct StringHashKeysVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a StringHashKeys,
    diagnostics: Vec<Diagnostic>,
    /// Depth counter: when > 0, we're inside arguments of an exempted method call.
    exempt_depth: usize,
}

/// Check if a call matches one of the environment-method patterns that RuboCop exempts.
fn is_env_method_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method = call.name();
    let method_name = method.as_slice();

    match call.receiver() {
        Some(ref receiver) => {
            // IO.popen
            if method_name == b"popen" && is_const(receiver, b"IO") {
                return true;
            }
            // Open3.capture2/capture2e/capture3/popen2/popen2e/popen3
            if is_const(receiver, b"Open3")
                && matches!(
                    method_name,
                    b"capture2"
                        | b"capture2e"
                        | b"capture3"
                        | b"popen2"
                        | b"popen2e"
                        | b"popen3"
                        | b"pipeline"
                        | b"pipeline_r"
                        | b"pipeline_rw"
                        | b"pipeline_start"
                        | b"pipeline_w"
                )
            {
                return true;
            }
            // Kernel.spawn / Kernel.system
            if is_const(receiver, b"Kernel") && matches!(method_name, b"spawn" | b"system") {
                return true;
            }
            // anything.gsub / anything.gsub!
            if matches!(method_name, b"gsub" | b"gsub!") {
                return true;
            }
            false
        }
        None => {
            // Bare spawn/system (implicit Kernel receiver)
            matches!(method_name, b"spawn" | b"system")
        }
    }
}

/// Check if a node is a constant read (simple or path) with the given name.
fn is_const(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::IO or just IO — parent is nil (cbase) or absent
        if cp.parent().is_none()
            || cp.parent().is_some_and(|p| {
                p.as_constant_path_node().is_none() && p.as_constant_read_node().is_none()
            })
        {
            return cp.name().is_some_and(|n| n.as_slice() == name);
        }
    }
    false
}

impl StringHashKeysVisitor<'_> {
    fn check_hash_elements<'pr, I>(&mut self, elements: I)
    where
        I: Iterator<Item = ruby_prism::Node<'pr>>,
    {
        if self.exempt_depth > 0 {
            return;
        }
        for element in elements {
            if let Some(assoc) = element.as_assoc_node() {
                let key = assoc.key();
                if key.as_string_node().is_some() {
                    let loc = key.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer symbols instead of strings as hash keys.".to_string(),
                    ));
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for StringHashKeysVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if is_env_method_call(node) {
            self.exempt_depth += 1;
            ruby_prism::visit_call_node(self, node);
            self.exempt_depth -= 1;
        } else {
            ruby_prism::visit_call_node(self, node);
        }
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        self.check_hash_elements(node.elements().iter());
        ruby_prism::visit_hash_node(self, node);
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        self.check_hash_elements(node.elements().iter());
        ruby_prism::visit_keyword_hash_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringHashKeys, "cops/style/string_hash_keys");
}
