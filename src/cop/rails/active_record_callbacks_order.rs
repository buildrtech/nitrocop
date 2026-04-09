use crate::cop::node_type::CLASS_NODE;
use crate::cop::util::{class_body_calls, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActiveRecordCallbacksOrder;

const CALLBACK_ORDER: &[&[u8]] = &[
    b"after_initialize",
    b"before_validation",
    b"after_validation",
    b"before_save",
    b"around_save",
    b"before_create",
    b"around_create",
    b"after_create",
    b"before_update",
    b"around_update",
    b"after_update",
    b"before_destroy",
    b"around_destroy",
    b"after_destroy",
    b"after_save",
    b"after_commit",
    b"after_rollback",
    b"after_find",
    b"after_touch",
];

fn callback_order_index(name: &[u8]) -> Option<usize> {
    CALLBACK_ORDER.iter().position(|&c| c == name)
}

impl Cop for ActiveRecordCallbacksOrder {
    fn name(&self) -> &'static str {
        "Rails/ActiveRecordCallbacksOrder"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let calls = class_body_calls(&class);

        // Collect (callback_name, order_index, selector_offset, start, end)
        let mut callbacks: Vec<(&[u8], usize, usize, usize, usize)> = Vec::new();

        for call in &calls {
            // RuboCop only considers send nodes (callbacks without blocks).
            // Callbacks with blocks (before_save do...end, after_commit { }) are skipped.
            if call.block().is_some() {
                continue;
            }
            for &cb_name in CALLBACK_ORDER {
                if is_dsl_call(call, cb_name) {
                    if let Some(idx) = callback_order_index(cb_name) {
                        let selector = call.message_loc().unwrap_or(call.location());
                        let loc = call.location();
                        callbacks.push((
                            cb_name,
                            idx,
                            selector.start_offset(),
                            loc.start_offset(),
                            loc.end_offset(),
                        ));
                    }
                    break;
                }
            }
        }

        let mut prev_idx: isize = -1;
        let mut prev_name: &[u8] = b"";
        let mut prev_call_idx: Option<usize> = None;
        let mut violations: Vec<(usize, usize)> = Vec::new();

        for (i, &(name, idx, selector_offset, _, _)) in callbacks.iter().enumerate() {
            let idx_signed = idx as isize;
            if idx_signed < prev_idx {
                let (line, column) = source.offset_to_line_col(selector_offset);
                let name_str = String::from_utf8_lossy(name);
                let other_str = String::from_utf8_lossy(prev_name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("`{name_str}` is supposed to appear before `{other_str}`."),
                ));
                if let Some(prev_i) = prev_call_idx {
                    violations.push((prev_i, i));
                }
            }
            prev_idx = idx_signed;
            prev_name = name;
            prev_call_idx = Some(i);
        }

        // Conservative baseline autocorrect: single adjacent callback swap only.
        if violations.len() == 1
            && let Some(ref mut corr) = corrections
        {
            let (prev_i, curr_i) = violations[0];
            let (_, _, _, prev_start, prev_end) = callbacks[prev_i];
            let (_, _, _, curr_start, curr_end) = callbacks[curr_i];

            if prev_start < curr_start {
                let prev_src = source.byte_slice(prev_start, prev_end, "").to_string();
                let middle = source.byte_slice(prev_end, curr_start, "").to_string();
                let curr_src = source.byte_slice(curr_start, curr_end, "").to_string();

                corr.push(crate::correction::Correction {
                    start: prev_start,
                    end: curr_end,
                    replacement: format!("{curr_src}{middle}{prev_src}"),
                    cop_name: self.name(),
                    cop_index: 0,
                });

                if let Some(diag) = diagnostics.last_mut() {
                    diag.corrected = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ActiveRecordCallbacksOrder,
        "cops/rails/active_record_callbacks_order"
    );

    #[test]
    fn supports_autocorrect() {
        assert!(ActiveRecordCallbacksOrder.supports_autocorrect());
    }

    #[test]
    fn autocorrects_simple_adjacent_callback_order() {
        let input = b"class User < ApplicationRecord\n  after_save :do_something\n  before_save :prepare\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&ActiveRecordCallbacksOrder, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);

        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"class User < ApplicationRecord\n  before_save :prepare\n  after_save :do_something\nend\n"
        );
    }
}
