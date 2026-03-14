use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
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
/// **FN root cause — keyword hash elements in multi-arg calls:**
/// When a `KeywordHashNode` appeared alongside other positional args, only the
/// KeywordHashNode as a whole was checked, not its individual elements. RuboCop's
/// `arguments_with_last_arg_pairs` expands the last arg's pairs. Fix: always expand
/// the last arg's KeywordHashNode elements into the effective args list.
pub struct ArgumentAlignment;

impl Cop for ArgumentAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArgumentAlignment"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
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
            // Expand the last arg if it's a KeywordHashNode
            let last_idx = args_vec.len() - 1;
            for (i, arg) in args_vec.into_iter().enumerate() {
                if i == last_idx {
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

        // Include block argument (&block, &handler, etc.) from call_node.block().
        // In Prism, BlockArgumentNode is on call_node.block(), not in arguments().
        if let Some(block) = call_node.block() {
            if block.as_block_argument_node().is_some() {
                effective_args.push(block);
            }
        }

        if effective_args.len() < 2 {
            return;
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
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            arg_line,
                            arg_col,
                            "Align the arguments of a method call if they span more than one line."
                                .to_string(),
                        ),
                    );
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
