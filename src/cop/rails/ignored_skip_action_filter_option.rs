use crate::cop::node_type::CALL_NODE;
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IgnoredSkipActionFilterOption;

const SKIP_METHODS: &[&[u8]] = &[
    b"skip_after_action",
    b"skip_around_action",
    b"skip_before_action",
    b"skip_action_callback",
];

impl Cop for IgnoredSkipActionFilterOption {
    fn name(&self) -> &'static str {
        "Rails/IgnoredSkipActionFilterOption"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb", "**/app/mailers/**/*.rb"]
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

        // Must be receiverless skip_*_action call
        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        if !SKIP_METHODS.contains(&name) {
            return;
        }

        // Check for keyword arguments
        let has_if = keyword_arg_value(&call, b"if").is_some();
        let has_only = keyword_arg_value(&call, b"only").is_some();
        let has_except = keyword_arg_value(&call, b"except").is_some();

        let target_key = if has_if && has_only {
            Some(b"if".as_slice())
        } else if has_if && has_except {
            Some(b"except".as_slice())
        } else {
            None
        };

        let Some(target_key) = target_key else {
            return;
        };

        let message = if target_key == b"if" {
            "`if` option will be ignored when `only` and `if` are used together."
        } else {
            "`except` option will be ignored when `if` and `except` are used together."
        };

        let start_offset = crate::cop::util::keyword_arg_pair_start_offset(&call, target_key)
            .unwrap_or_else(|| node.location().start_offset());
        let (line, column) = source.offset_to_line_col(start_offset);
        let mut diagnostic = self.diagnostic(source, line, column, message.to_string());

        if let Some(ref mut corr) = corrections
            && let Some(args) = call.arguments()
        {
            for arg in args.arguments().iter() {
                let Some(kw) = arg.as_keyword_hash_node() else {
                    continue;
                };

                let elements: Vec<_> = kw.elements().iter().collect();
                let target_index = elements.iter().position(|elem| {
                    elem.as_assoc_node()
                        .and_then(|assoc| assoc.key().as_symbol_node())
                        .is_some_and(|sym| sym.unescaped() == target_key)
                });

                let Some(target_index) = target_index else {
                    continue;
                };

                let target_assoc = elements[target_index]
                    .as_assoc_node()
                    .expect("target index should be assoc node");
                let target_loc = target_assoc.location();

                let (start, end) = if target_index + 1 < elements.len() {
                    (
                        target_loc.start_offset(),
                        elements[target_index + 1].location().start_offset(),
                    )
                } else if target_index > 0 {
                    (
                        elements[target_index - 1].location().end_offset(),
                        target_loc.end_offset(),
                    )
                } else {
                    (target_loc.start_offset(), target_loc.end_offset())
                };

                if start < end {
                    corr.push(crate::correction::Correction {
                        start,
                        end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diagnostic.corrected = true;
                }
                break;
            }
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IgnoredSkipActionFilterOption,
        "cops/rails/ignored_skip_action_filter_option"
    );

    #[test]
    fn autocorrects_if_when_only_and_if_used_together() {
        crate::testutil::assert_cop_autocorrect(
            &IgnoredSkipActionFilterOption,
            b"skip_before_action :login_required, only: :show, if: :trusted_origin?\n",
            b"skip_before_action :login_required, only: :show\n",
        );
    }

    #[test]
    fn autocorrects_except_when_if_and_except_used_together() {
        crate::testutil::assert_cop_autocorrect(
            &IgnoredSkipActionFilterOption,
            b"skip_before_action :login_required, except: :admin, if: :trusted_origin?\n",
            b"skip_before_action :login_required, if: :trusted_origin?\n",
        );
    }
}
