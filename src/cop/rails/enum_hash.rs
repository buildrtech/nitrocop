use crate::cop::node_type::{ARRAY_NODE, ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnumHash;

fn element_source_for_hash(elem: &ruby_prism::Node<'_>, source: &SourceFile) -> String {
    if let Some(sym) = elem.as_symbol_node() {
        let value = String::from_utf8_lossy(sym.unescaped());
        let plain_symbol = value
            .chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
            && value.chars().all(|c| c == '_' || c.is_ascii_alphanumeric());

        if plain_symbol {
            return format!(":{value}");
        }

        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        return format!(":\"{escaped}\"");
    }

    if let Some(str_node) = elem.as_string_node() {
        let value = String::from_utf8_lossy(str_node.unescaped());
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        return format!("\"{escaped}\"");
    }

    let loc = elem.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

fn build_hash_from_array(array: &ruby_prism::ArrayNode<'_>, source: &SourceFile) -> String {
    let pairs = array
        .elements()
        .iter()
        .enumerate()
        .map(|(idx, elem)| format!("{} => {idx}", element_source_for_hash(&elem, source)))
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{{pairs}}}")
}

impl Cop for EnumHash {
    fn name(&self) -> &'static str {
        "Rails/EnumHash"
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

        if call.receiver().is_some() {
            return;
        }

        if call.name().as_slice() != b"enum" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Old syntax: enum status: [:active, :archived]
        // Parsed as: enum(KeywordHashNode { status: ArrayNode })
        for arg in &arg_list {
            if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node()
                        && let Some(array) = assoc.value().as_array_node()
                    {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            column,
                            "Use hash syntax for `enum` values: `enum status: { active: 0, archived: 1 }`."
                                .to_string(),
                        );

                        if let Some(ref mut corr) = corrections {
                            let arr_loc = array.location();
                            corr.push(crate::correction::Correction {
                                start: arr_loc.start_offset(),
                                end: arr_loc.end_offset(),
                                replacement: build_hash_from_array(&array, source),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diagnostic.corrected = true;
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // New syntax: enum :status, [:active, :archived]
        // Check if second arg is an array
        if arg_list.len() >= 2
            && arg_list[0].as_symbol_node().is_some()
            && arg_list[1].as_array_node().is_some()
        {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Use hash syntax for `enum` values: `enum status: { active: 0, archived: 1 }`."
                    .to_string(),
            );

            if let Some(array) = arg_list[1].as_array_node()
                && let Some(ref mut corr) = corrections
            {
                let arr_loc = array.location();
                corr.push(crate::correction::Correction {
                    start: arr_loc.start_offset(),
                    end: arr_loc.end_offset(),
                    replacement: build_hash_from_array(&array, source),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }

            diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnumHash, "cops/rails/enum_hash");
    crate::cop_autocorrect_fixture_tests!(EnumHash, "cops/rails/enum_hash");

    #[test]
    fn autocorrects_old_enum_string_array_syntax_to_hash() {
        crate::testutil::assert_cop_autocorrect(
            &EnumHash,
            b"enum status: ['active', 'archived']\n",
            b"enum status: {\"active\" => 0, \"archived\" => 1}\n",
        );
    }
}
