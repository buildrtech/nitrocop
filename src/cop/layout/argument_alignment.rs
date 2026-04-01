use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/ArgumentAlignment cop.
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause — `**splat` with aligned continuation kwargs:**
/// When a KeywordHashNode is expanded for alignment checking, `AssocSplatNode`
/// elements must be excluded from both the alignment items and the minimum-count
/// check. RuboCop's `first_arg.pairs` returns only `pair` nodes (not `kwsplat`),
/// and `multiple_arguments?` checks `pairs.count >= 2`. So `**splat` + 1 keyword
/// pair → skip (not enough args). `**splat` + 2+ keyword pairs → check pairs only,
/// using the first pair as the alignment reference (not the splat).
///
/// **FN root cause — block args (`&block`, `&handler`):**
/// In Prism, block arguments (`&block`) are stored on `call_node.block()` as a
/// `BlockArgumentNode`, NOT in `call_node.arguments()`. The cop was only iterating
/// `arguments()`, so block args were invisible to alignment checking. Fix: append
/// the block argument to the effective args list when present.
///
/// ## Investigation findings (2026-03-16)
///
/// **FP root cause — incorrect keyword hash expansion in `with_first_argument` mode:**
/// When multiple arguments exist and the last one is a `KeywordHashNode`, nitrocop
/// was ALWAYS expanding the hash's elements into the effective_args list. But
/// RuboCop's `arguments_or_first_arg_pairs` (used for `with_first_argument` style)
/// only expands when the FIRST argument is a bare keyword hash (sole arg case).
/// For multi-arg calls, it returns `node.arguments` as-is, keeping the
/// KeywordHashNode as a single item. This caused ~19k false positives because
/// continuation keyword pairs (e.g., `branch: "v349"` on line 2 of a `gem` call)
/// were individually checked against the first positional arg's column.
///
/// Fix: only expand the last KeywordHashNode for `with_fixed_indentation` style
/// (which uses `arguments_with_last_arg_pairs`). For `with_first_argument`, keep
/// the kwHash as a single item so its internal pairs aren't individually checked.
///
/// **Previous FN note (2026-03-14) revised:** The earlier finding about expanding
/// keyword hash elements in multi-arg calls was only correct for
/// `with_fixed_indentation` style. For `with_first_argument` (default), expansion
/// was causing massive FPs. The fix keeps expansion only for fixed indentation.
///
/// ## Investigation findings (2026-03-16, second pass)
///
/// **FP root cause — block pass promoting single-arg call to multi-arg:**
/// When a call has a sole keyword hash argument plus a block pass
/// (e.g., `add_listener(:before => :x, &method(:y))`), the block pass
/// (`BlockArgumentNode`) was appended to `effective_args` BEFORE the
/// `len() < 2` gate. This made the cop treat it as a 2-arg call and check
/// alignment, but RuboCop's `multiple_arguments?` uses `node.arguments`
/// which excludes block pass nodes. Fix: move the block-arg append AFTER
/// the minimum-count check, so block pass can't promote a single-arg call.
pub struct ArgumentAlignment;

impl Cop for ArgumentAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArgumentAlignment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "with_first_argument");
        let indent_width = config.get_usize("IndentationWidth", 2);
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop skips []= calls (bracket assignment)
        if call_node.name().as_slice() == b"[]=" {
            return;
        }

        let arguments = match call_node.arguments() {
            Some(args) => args,
            None => return,
        };

        let arg_list = arguments.arguments();
        if arg_list.is_empty() {
            return;
        }

        // Collect effective arguments, matching RuboCop's behavior:
        //
        // with_first_argument style (arguments_or_first_arg_pairs):
        //   - If the first arg is a bare KeywordHashNode (sole arg), expand to
        //     its .pairs only (excludes AssocSplatNode). Need >= 2 pairs.
        //   - Otherwise, use node.arguments with the last arg's KeywordHashNode
        //     expanded to its .pairs.
        //
        // with_fixed_indentation style (arguments_with_last_arg_pairs):
        //   - All args except last, plus last arg's KeywordHashNode .pairs
        //     (or the last arg itself if not a keyword hash).
        //
        // In both cases, block arguments from call_node.block() (BlockArgumentNode)
        // are included as additional alignment targets.
        let args_vec: Vec<ruby_prism::Node<'_>> = arg_list.iter().collect();
        let is_sole_keyword_hash =
            args_vec.len() == 1 && args_vec[0].as_keyword_hash_node().is_some();

        let mut effective_args: Vec<ruby_prism::Node<'_>> = Vec::new();

        if is_sole_keyword_hash && style != "with_fixed_indentation" {
            // with_first_argument: expand first (sole) arg's pairs only
            let kw_hash = args_vec[0].as_keyword_hash_node().unwrap();
            for elem in kw_hash.elements().iter() {
                // Only include AssocNode (pair), skip AssocSplatNode
                if elem.as_assoc_splat_node().is_none() {
                    effective_args.push(elem);
                }
            }
        } else {
            // Expand the last arg if it's a KeywordHashNode.
            //
            // RuboCop behavior differs by style:
            // - with_first_argument uses `arguments_or_first_arg_pairs` which only
            //   expands the FIRST arg when it's a sole keyword hash (handled above).
            //   For multi-arg calls, it returns `node.arguments` as-is, keeping the
            //   KeywordHashNode as a single item.
            // - with_fixed_indentation uses `arguments_with_last_arg_pairs` which
            //   always expands the last arg's keyword hash elements.
            let last_idx = args_vec.len() - 1;
            for (i, arg) in args_vec.into_iter().enumerate() {
                if i == last_idx && style == "with_fixed_indentation" {
                    if let Some(kw_hash) = arg.as_keyword_hash_node() {
                        for elem in kw_hash.elements().iter() {
                            // Only include AssocNode (pair), skip AssocSplatNode
                            if elem.as_assoc_splat_node().is_none() {
                                effective_args.push(elem);
                            }
                        }
                        continue;
                    }
                }
                effective_args.push(arg);
            }
        }

        // RuboCop's multiple_arguments? check uses node.arguments (which excludes
        // block pass). We must check the count BEFORE adding block arguments,
        // because a block pass should not promote a single-arg call into a
        // multi-arg call for alignment purposes.
        if effective_args.len() < 2 {
            return;
        }

        // Include block argument (&block, &handler, etc.) from call_node.block().
        // In Prism, BlockArgumentNode is on call_node.block(), not in arguments().
        // Added AFTER the multiple_arguments? gate so that a sole keyword hash arg
        // with a block pass (e.g., `add_listener(:before => :x, &method(:y))`)
        // doesn't trigger alignment checking when there's only one real argument.
        if let Some(block) = call_node.block() {
            if block.as_block_argument_node().is_some() {
                effective_args.push(block);
            }
        }

        let first_arg = &effective_args[0];
        let (first_line, first_col) =
            source.offset_to_line_col(first_arg.location().start_offset());

        let mut checked_lines = std::collections::HashSet::new();
        checked_lines.insert(first_line);

        // For "with_fixed_indentation", the expected column is the call line's
        // indentation + indent_width
        let expected_col = match style {
            "with_fixed_indentation" => {
                // Use the line containing the method selector (or opening paren),
                // NOT the full call expression start (which includes the receiver
                // chain). For chained calls like `Foo.bar.baz("str", arg)`, the
                // call node starts at `Foo` but we want the indentation of the
                // line containing `.baz(`.
                let base_line = if let Some(open_loc) = call_node.opening_loc() {
                    source.offset_to_line_col(open_loc.start_offset()).0
                } else if let Some(msg_loc) = call_node.message_loc() {
                    source.offset_to_line_col(msg_loc.start_offset()).0
                } else {
                    source
                        .offset_to_line_col(call_node.location().start_offset())
                        .0
                };
                let base_line_bytes = source.lines().nth(base_line - 1).unwrap_or(b"");
                crate::cop::util::indentation_of(base_line_bytes) + indent_width
            }
            _ => first_col, // "with_first_argument" (default)
        };

        for arg in effective_args.iter().skip(1) {
            let (arg_line, arg_col) = source.offset_to_line_col(arg.location().start_offset());
            // Only check the FIRST argument on each new line
            if !checked_lines.contains(&arg_line) {
                checked_lines.insert(arg_line);
                // Skip arguments that don't begin their line (matching RuboCop's
                // begins_its_line? check). For example, in `}, suffix: :action`
                // the suffix: argument is not first on its line.
                if !crate::cop::util::begins_its_line(source, arg.location().start_offset()) {
                    continue;
                }
                if arg_col != expected_col {
                    let mut diagnostic = self.diagnostic(
                        source,
                        arg_line,
                        arg_col,
                        "Align the arguments of a method call if they span more than one line."
                            .to_string(),
                    );

                    if let Some(corrections) = corrections.as_mut() {
                        let line_start = source.line_start_offset(arg_line);
                        corrections.push(Correction {
                            start: line_start,
                            end: line_start + arg_col,
                            replacement: " ".repeat(expected_col),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ArgumentAlignment, "cops/layout/argument_alignment");
    crate::cop_autocorrect_fixture_tests!(ArgumentAlignment, "cops/layout/argument_alignment");

    #[test]
    fn single_line_call_no_offense() {
        let source = b"foo(1, 2, 3)\n";
        let diags = run_cop_full(&ArgumentAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_fixed_indentation_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("with_fixed_indentation".into()),
            )]),
            ..CopConfig::default()
        };
        // Args aligned with first arg (column 4) but with_fixed_indentation expects column 2
        let src = b"foo(1,\n    2)\n";
        let diags = run_cop_full_with_config(&ArgumentAlignment, src, config.clone());
        assert_eq!(
            diags.len(),
            1,
            "with_fixed_indentation should flag args aligned with first arg"
        );

        // Args at fixed indentation (2 spaces from call)
        let src2 = b"foo(1,\n  2)\n";
        let diags2 = run_cop_full_with_config(&ArgumentAlignment, src2, config);
        assert!(
            diags2.is_empty(),
            "with_fixed_indentation should accept fixed-indent args"
        );
    }
}
