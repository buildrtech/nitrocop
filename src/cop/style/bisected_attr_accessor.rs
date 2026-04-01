use crate::cop::node_type::{
    CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::{HashMap, HashSet};

/// Checks for places where `attr_reader` and `attr_writer` for the same
/// method can be combined into a single `attr_accessor`.
///
/// ## Visibility scope tracking
///
/// RuboCop groups macros by their visibility scope (public/private/protected)
/// and only considers bisection within the same scope. For example:
///
/// ```ruby
/// class Foo
///   attr_reader :bar   # public scope
///   private
///   attr_writer :bar   # private scope -- NOT bisected
/// end
/// ```
///
/// This cop mirrors that behavior by tracking the current visibility as it
/// iterates through the class/module body statements. A bare `private`,
/// `protected`, or `public` call (with no arguments) changes the visibility
/// for all subsequent statements. Calls with arguments (e.g., `private :foo`)
/// do not change the ambient visibility.
///
/// ## Root cause of historical FPs (92 FP in corpus)
///
/// The original implementation did not track visibility scopes at all. It
/// collected all `attr_reader` and `attr_writer` calls in a class body
/// regardless of their position relative to `private`/`protected`/`public`
/// calls, then bisected them. This caused false positives whenever a reader
/// and writer for the same attribute were in different visibility scopes.
pub struct BisectedAttrAccessor;

/// Visibility scope for attr macros
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Visibility {
    Public,
    Private,
    Protected,
}

/// An attr_reader or attr_writer occurrence with its visibility and location
struct AttrOccurrence {
    name: String,
    visibility: Visibility,
    line: usize,
    column: usize,
    call_start: usize,
    call_end: usize,
    message_start: usize,
    message_end: usize,
    arg_count: usize,
    is_reader: bool,
}

impl Cop for BisectedAttrAccessor {
    fn name(&self) -> &'static str {
        "Style/BisectedAttrAccessor"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            MODULE_NODE,
            SINGLETON_CLASS_NODE,
            STATEMENTS_NODE,
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.body()
        } else if let Some(sclass_node) = node.as_singleton_class_node() {
            sclass_node.body()
        } else {
            return;
        };

        let body = match body {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let mut readers: Vec<AttrOccurrence> = Vec::new();
        let mut writers: Vec<AttrOccurrence> = Vec::new();
        let mut current_visibility = Visibility::Public;

        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
                if call.receiver().is_some() {
                    continue;
                }

                // Check for visibility-changing calls (bare private/protected/public with no args)
                if (name == "private" || name == "protected" || name == "public")
                    && call.arguments().is_none()
                    && call.block().is_none()
                {
                    current_visibility = match name {
                        "private" => Visibility::Private,
                        "protected" => Visibility::Protected,
                        "public" => Visibility::Public,
                        _ => unreachable!(),
                    };
                    continue;
                }

                let is_reader = name == "attr_reader" || name == "attr";
                let is_writer = name == "attr_writer";

                if !is_reader && !is_writer {
                    continue;
                }

                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        let attr_name = if let Some(sym) = arg.as_symbol_node() {
                            std::str::from_utf8(sym.unescaped())
                                .unwrap_or("")
                                .to_string()
                        } else {
                            continue;
                        };

                        let loc = arg.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());

                        let call_loc = call.location();
                        let Some(msg_loc) = call.message_loc() else {
                            continue;
                        };
                        let occurrence = AttrOccurrence {
                            name: attr_name,
                            visibility: current_visibility,
                            line,
                            column,
                            call_start: call_loc.start_offset(),
                            call_end: call_loc.end_offset(),
                            message_start: msg_loc.start_offset(),
                            message_end: msg_loc.end_offset(),
                            arg_count: args.arguments().iter().count(),
                            is_reader,
                        };

                        if is_reader {
                            readers.push(occurrence);
                        } else {
                            writers.push(occurrence);
                        }
                    }
                }
            }
        }

        // Group by visibility and find bisections within each scope
        let mut reader_names_by_vis: HashMap<Visibility, HashSet<String>> = HashMap::new();
        let mut writer_names_by_vis: HashMap<Visibility, HashSet<String>> = HashMap::new();

        for r in &readers {
            reader_names_by_vis
                .entry(r.visibility)
                .or_default()
                .insert(r.name.clone());
        }
        for w in &writers {
            writer_names_by_vis
                .entry(w.visibility)
                .or_default()
                .insert(w.name.clone());
        }

        // Find common names within each visibility scope
        let mut common: HashSet<(Visibility, String)> = HashSet::new();
        for (vis, reader_names) in &reader_names_by_vis {
            if let Some(writer_names) = writer_names_by_vis.get(vis) {
                for name in reader_names.intersection(writer_names) {
                    common.insert((*vis, name.clone()));
                }
            }
        }

        // Report diagnostics for bisected attrs
        for occ in readers.iter().chain(writers.iter()) {
            if common.contains(&(occ.visibility, occ.name.clone())) {
                diagnostics.push(self.diagnostic(
                    source,
                    occ.line,
                    occ.column,
                    format!("Combine both accessors into `attr_accessor :{}`.", occ.name),
                ));
            }
        }

        if let Some(corrections) = corrections.as_mut() {
            for (vis, name) in common {
                let reader = readers.iter().find(|r| {
                    r.visibility == vis && r.name == name && r.arg_count == 1 && r.is_reader
                });
                let writer = writers.iter().find(|w| {
                    w.visibility == vis && w.name == name && w.arg_count == 1 && !w.is_reader
                });

                let (Some(reader), Some(writer)) = (reader, writer) else {
                    continue;
                };

                let (keep, remove) = if reader.call_start <= writer.call_start {
                    (reader, writer)
                } else {
                    (writer, reader)
                };

                if !line_contains_only_call(source, remove.call_start, remove.call_end) {
                    continue;
                }

                corrections.push(crate::correction::Correction {
                    start: keep.message_start,
                    end: keep.message_end,
                    replacement: "attr_accessor".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });

                let delete_end = extend_to_line_break(source, remove.call_end);
                let delete_start = line_start_offset(source, remove.call_start);
                corrections.push(crate::correction::Correction {
                    start: delete_start,
                    end: delete_end,
                    replacement: "".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
        }
    }
}

fn line_start_offset(source: &SourceFile, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    line_start
}

fn line_contains_only_call(source: &SourceFile, call_start: usize, call_end: usize) -> bool {
    let bytes = source.as_bytes();
    let line_start = line_start_offset(source, call_start);

    let mut line_end = call_end;
    while line_end < bytes.len() && bytes[line_end] != b'\n' {
        line_end += 1;
    }

    bytes[line_start..call_start]
        .iter()
        .all(|b| b.is_ascii_whitespace())
        && bytes[call_end..line_end]
            .iter()
            .all(|b| b.is_ascii_whitespace())
}

fn extend_to_line_break(source: &SourceFile, end: usize) -> usize {
    let bytes = source.as_bytes();
    if end >= bytes.len() {
        return end;
    }

    if bytes[end] == b'\r' && end + 1 < bytes.len() && bytes[end + 1] == b'\n' {
        end + 2
    } else if bytes[end] == b'\n' {
        end + 1
    } else {
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BisectedAttrAccessor, "cops/style/bisected_attr_accessor");
    crate::cop_autocorrect_fixture_tests!(
        BisectedAttrAccessor,
        "cops/style/bisected_attr_accessor"
    );
}
