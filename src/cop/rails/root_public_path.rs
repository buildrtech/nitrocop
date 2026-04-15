use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RootPublicPath;

impl Cop for RootPublicPath {
    fn name(&self) -> &'static str {
        "Rails/RootPublicPath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"join" {
            return;
        }

        // Must have at least one argument, first must be a string starting with "public"
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }
        let first_str = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };
        let content = first_str.unescaped();
        // Match strings like "public", "public/file.pdf"
        if content != b"public" && !content.starts_with(b"public/") {
            return;
        }

        // Receiver should be a call to `root`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let root_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if root_call.name().as_slice() != b"root" {
            return;
        }

        // root's receiver should be constant `Rails` or `::Rails`
        let rails_recv = match root_call.receiver() {
            Some(r) => r,
            None => return,
        };
        if util::constant_name(&rails_recv) != Some(b"Rails") {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic =
            self.diagnostic(source, line, column, "Use `Rails.public_path`.".to_string());

        if let Some(ref mut corr) = corrections {
            let rails_source = source
                .byte_slice(
                    rails_recv.location().start_offset(),
                    rails_recv.location().end_offset(),
                    "",
                )
                .to_string();

            let first_path = String::from_utf8_lossy(content).to_string();
            let first_path_remainder = if first_path == "public" {
                "".to_string()
            } else {
                first_path
                    .strip_prefix("public/")
                    .unwrap_or_default()
                    .to_string()
            };

            let mut join_args: Vec<String> = arg_list[1..]
                .iter()
                .map(|arg| {
                    source
                        .byte_slice(
                            arg.location().start_offset(),
                            arg.location().end_offset(),
                            "",
                        )
                        .to_string()
                })
                .collect();

            if !first_path_remainder.is_empty() {
                join_args.insert(0, format!("'{}'", first_path_remainder));
            }

            let mut replacement = format!("{rails_source}.public_path");
            if !join_args.is_empty() {
                replacement.push_str(".join(");
                replacement.push_str(&join_args.join(", "));
                replacement.push(')');
            }

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPublicPath, "cops/rails/root_public_path");

    #[test]
    fn autocorrects_public_root_join_without_extra_args() {
        crate::testutil::assert_cop_autocorrect(
            &RootPublicPath,
            b"Rails.root.join(\"public\")\n",
            b"Rails.public_path\n",
        );
    }

    #[test]
    fn autocorrects_public_prefixed_path_to_join_tail() {
        crate::testutil::assert_cop_autocorrect(
            &RootPublicPath,
            b"Rails.root.join(\"public/file.pdf\")\n",
            b"Rails.public_path.join('file.pdf')\n",
        );
    }

    #[test]
    fn autocorrects_with_additional_join_args() {
        crate::testutil::assert_cop_autocorrect(
            &RootPublicPath,
            b"::Rails.root.join(\"public\", \"file.pdf\")\n",
            b"::Rails.public_path.join(\"file.pdf\")\n",
        );
    }
}
