use crate::cop::node_type::{
    ARRAY_NODE, ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DotSeparatedKeys;

impl Cop for DotSeparatedKeys {
    fn name(&self) -> &'static str {
        "Rails/DotSeparatedKeys"
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
            STRING_NODE,
            SYMBOL_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"t" && method_name != b"translate" {
            return;
        }

        if let Some(recv) = call.receiver() {
            if let Some(cr) = recv.as_constant_read_node() {
                if cr.name().as_slice() != b"I18n" {
                    return;
                }
            } else if let Some(cp) = recv.as_constant_path_node() {
                if cp.parent().is_some() {
                    return;
                }
                if cp.name().map(|n| n.as_slice()) != Some(b"I18n") {
                    return;
                }
            } else {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list = args.arguments();
        let first_arg = match arg_list.iter().next() {
            Some(a) => a,
            None => return,
        };

        let first_key = if let Some(sym) = first_arg.as_symbol_node() {
            String::from_utf8_lossy(sym.unescaped()).to_string()
        } else if let Some(s) = first_arg.as_string_node() {
            String::from_utf8_lossy(s.unescaped()).to_string()
        } else {
            return;
        };

        for arg in arg_list.iter() {
            let kw = match arg.as_keyword_hash_node() {
                Some(k) => k,
                None => continue,
            };

            let elements: Vec<_> = kw.elements().iter().collect();
            for (idx, elem) in elements.iter().enumerate() {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };

                let is_scope_key = assoc
                    .key()
                    .as_symbol_node()
                    .is_some_and(|sym| sym.unescaped() == b"scope");
                if !is_scope_key {
                    continue;
                }

                let scope_parts = match scope_parts(&assoc.value()) {
                    Some(parts) => parts,
                    None => continue,
                };

                let loc = assoc.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    column,
                    "Use dot-separated keys instead of the `:scope` option.".to_string(),
                );

                if let Some(ref mut corr) = corrections {
                    let joined_scope = scope_parts.join(".");
                    let key_replacement =
                        format!("'{}.{}'", joined_scope, first_key).replace("..", ".");

                    let first_loc = first_arg.location();
                    corr.push(crate::correction::Correction {
                        start: first_loc.start_offset(),
                        end: first_loc.end_offset(),
                        replacement: key_replacement,
                        cop_name: self.name(),
                        cop_index: 0,
                    });

                    let bytes = source.as_bytes();
                    let assoc_start = assoc.location().start_offset();
                    let assoc_end = assoc.location().end_offset();
                    let (rm_start, rm_end) = if idx > 0 {
                        let mut start = assoc_start;
                        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                            start -= 1;
                        }
                        if start > 0 && bytes[start - 1] == b',' {
                            start -= 1;
                        }
                        (start, assoc_end)
                    } else if elements.len() > 1 {
                        let next_start = elements[idx + 1].location().start_offset();
                        (assoc_start, next_start)
                    } else {
                        let mut start = assoc_start;
                        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                            start -= 1;
                        }
                        if start > 0 && bytes[start - 1] == b',' {
                            start -= 1;
                        }
                        (start, assoc_end)
                    };

                    corr.push(crate::correction::Correction {
                        start: rm_start,
                        end: rm_end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });

                    diagnostic.corrected = true;
                }

                diagnostics.push(diagnostic);
                break;
            }
        }
    }
}

fn scope_parts(value: &ruby_prism::Node<'_>) -> Option<Vec<String>> {
    if let Some(sym) = value.as_symbol_node() {
        return Some(vec![String::from_utf8_lossy(sym.unescaped()).to_string()]);
    }

    let array = value.as_array_node()?;
    let mut out = Vec::new();
    for elem in array.elements().iter() {
        if let Some(sym) = elem.as_symbol_node() {
            out.push(String::from_utf8_lossy(sym.unescaped()).to_string());
        } else if let Some(s) = elem.as_string_node() {
            out.push(String::from_utf8_lossy(s.unescaped()).to_string());
        } else {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DotSeparatedKeys, "cops/rails/dot_separated_keys");
    crate::cop_autocorrect_fixture_tests!(DotSeparatedKeys, "cops/rails/dot_separated_keys");

    #[test]
    fn supports_autocorrect() {
        assert!(DotSeparatedKeys.supports_autocorrect());
    }
}
