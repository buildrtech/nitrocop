use crate::cop::node_type::CALL_NODE;
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AddColumnIndex;

fn find_index_pair_range_and_value(
    call: &ruby_prism::CallNode<'_>,
) -> Option<(usize, usize, usize, usize, bool)> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node()
                    && let Some(sym) = assoc.key().as_symbol_node()
                    && sym.unescaped() == b"index"
                {
                    let ploc = assoc.location();
                    let vloc = assoc.value().location();
                    return Some((
                        ploc.start_offset(),
                        ploc.end_offset(),
                        vloc.start_offset(),
                        vloc.end_offset(),
                        assoc.value().as_hash_node().is_some(),
                    ));
                }
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node()
                    && let Some(sym) = assoc.key().as_symbol_node()
                    && sym.unescaped() == b"index"
                {
                    let ploc = assoc.location();
                    let vloc = assoc.value().location();
                    return Some((
                        ploc.start_offset(),
                        ploc.end_offset(),
                        vloc.start_offset(),
                        vloc.end_offset(),
                        assoc.value().as_hash_node().is_some(),
                    ));
                }
            }
        }
    }
    None
}

fn expand_pair_removal_range(source: &SourceFile, start: usize, end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let mut left = start;
    let mut right = end;

    while left > 0 && matches!(bytes[left - 1], b' ' | b'\t') {
        left -= 1;
    }
    if left > 0 && bytes[left - 1] == b',' {
        left -= 1;
        while left > 0 && matches!(bytes[left - 1], b' ' | b'\t') {
            left -= 1;
        }
        return (left, right);
    }

    while right < bytes.len() && matches!(bytes[right], b' ' | b'\t') {
        right += 1;
    }
    if right < bytes.len() && bytes[right] == b',' {
        right += 1;
        while right < bytes.len() && matches!(bytes[right], b' ' | b'\t') {
            right += 1;
        }
    }

    (left, right)
}

impl Cop for AddColumnIndex {
    fn name(&self) -> &'static str {
        "Rails/AddColumnIndex"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        if call.name().as_slice() != b"add_column" {
            return;
        }

        // Check if there's an `index` keyword argument
        if keyword_arg_value(&call, b"index").is_none() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "`add_column` does not accept an `index` key, use `add_index` instead.".to_string(),
        );

        if let Some(ref mut corr) = corrections
            && let Some((pair_start, pair_end, val_start, val_end, value_is_hash)) =
                find_index_pair_range_and_value(&call)
            && let Some(args) = call.arguments()
        {
            let args_list: Vec<_> = args.arguments().iter().collect();
            if args_list.len() >= 2 {
                let table = source
                    .byte_slice(
                        args_list[0].location().start_offset(),
                        args_list[0].location().end_offset(),
                        "",
                    )
                    .to_string();
                let column = source
                    .byte_slice(
                        args_list[1].location().start_offset(),
                        args_list[1].location().end_offset(),
                        "",
                    )
                    .to_string();

                let (rm_start, rm_end) = expand_pair_removal_range(source, pair_start, pair_end);
                corr.push(crate::correction::Correction {
                    start: rm_start,
                    end: rm_end,
                    replacement: "".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });

                let add_index_opts = if value_is_hash {
                    let raw = source.byte_slice(val_start, val_end, "").to_string();
                    let trimmed = raw
                        .trim()
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .trim();
                    if trimmed.is_empty() {
                        String::new()
                    } else {
                        format!(", {trimmed}")
                    }
                } else {
                    String::new()
                };

                corr.push(crate::correction::Correction {
                    start: loc.end_offset(),
                    end: loc.end_offset(),
                    replacement: format!("\nadd_index {table}, {column}{add_index_opts}"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AddColumnIndex, "cops/rails/add_column_index");
    crate::cop_autocorrect_fixture_tests!(AddColumnIndex, "cops/rails/add_column_index");
}
