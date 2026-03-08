use crate::cop::node_type::{ARRAY_NODE, ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/RakeEnvironment cop - checks that rake tasks depend on :environment.
///
/// ## Investigation findings (2026-03-08)
///
/// **30 FP:** The `:default` task was not excluded from the check. RuboCop skips
/// `task :default` / `task default: [...]` because the default task is just a
/// dispatcher and doesn't need `:environment`.
///
/// **63 FN:** The cop only handled `task :name` (SymbolNode/StringNode as first arg)
/// but not the hash-first-arg form `task name: :dep do ... end` where the first arg
/// is a KeywordHashNode. In this form, the key is the task name and the value is the
/// dependency list. Tasks like `task foo: :environment` were not recognized at all,
/// and tasks like `task foo: []` (hash form, no deps) were not flagged.
///
/// **Fix:** Extract task name from both symbol/string first-arg and hash-first-arg
/// forms. Skip if task name is "default". For hash-first-arg, check the value for
/// dependencies (symbol = has dep, non-empty array = has dep, empty array = no dep).
pub struct RakeEnvironment;

impl Cop for RakeEnvironment {
    fn name(&self) -> &'static str {
        "Rails/RakeEnvironment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            ASSOC_NODE,
            CALL_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Start from CallNode `task`, then check if it has a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless `task` call
        if call.receiver().is_some() {
            return;
        }

        if call.name().as_slice() != b"task" {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Determine if this is a simple task definition (first arg is symbol/string)
        // or a hash-first-arg form (first arg is KeywordHashNode like `task foo: :dep`).
        let first = &arg_list[0];
        let task_name_is_default;
        let hash_first_arg;

        if let Some(sym) = first.as_symbol_node() {
            // task :foo do ... end
            task_name_is_default = sym.unescaped() == b"default";
            hash_first_arg = false;
        } else if let Some(s) = first.as_string_node() {
            // task 'foo' do ... end
            task_name_is_default = s.unescaped() == b"default";
            hash_first_arg = false;
        } else if let Some(kw) = first.as_keyword_hash_node() {
            // task foo: :dep do ... end  (hash-first-arg form)
            // Extract the key as the task name and check value for dependencies.
            let mut elements = kw.elements().iter();
            if let Some(first_elem) = elements.next() {
                if let Some(assoc) = first_elem.as_assoc_node() {
                    // Check task name from the key
                    let key = assoc.key();
                    task_name_is_default = is_name_default(&key);

                    // Check if value represents dependencies
                    let value = assoc.value();
                    if has_dependencies(&value) {
                        if task_name_is_default {
                            return;
                        }
                        // Has dependencies — not an offense (unless we also need to
                        // check remaining hash entries, but RuboCop doesn't).
                        return;
                    }
                    hash_first_arg = true;
                } else {
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        }

        // Skip :default task
        if task_name_is_default {
            return;
        }

        // For non-hash-first-arg form, check remaining args for dependency hashes.
        if !hash_first_arg {
            for arg in &arg_list[1..] {
                if let Some(kw) = arg.as_keyword_hash_node() {
                    for elem in kw.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node() {
                            let value = assoc.value();
                            if has_dependencies(&value) {
                                return;
                            }
                        }
                    }
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Add `:environment` dependency to the rake task.".to_string(),
        ));
    }
}

/// Check if a node represents the name "default" (as symbol or string).
fn is_name_default(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(sym) = node.as_symbol_node() {
        return sym.unescaped() == b"default";
    }
    if let Some(s) = node.as_string_node() {
        return s.unescaped() == b"default";
    }
    false
}

/// Check if a node represents non-empty dependencies.
/// A symbol value means a single dependency. A non-empty array means multiple deps.
/// An empty array means no dependencies.
fn has_dependencies(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_symbol_node().is_some() {
        return true;
    }
    if let Some(arr) = node.as_array_node() {
        return arr.elements().iter().next().is_some();
    }
    // String value could also be a dependency
    if node.as_string_node().is_some() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RakeEnvironment, "cops/rails/rake_environment");
}
