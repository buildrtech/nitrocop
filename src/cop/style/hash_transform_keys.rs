use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE,
    LOCAL_VARIABLE_READ_NODE, MULTI_TARGET_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashTransformKeys;

impl Cop for HashTransformKeys {
    fn name(&self) -> &'static str {
        "Style/HashTransformKeys"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            LOCAL_VARIABLE_READ_NODE,
            MULTI_TARGET_NODE,
            STATEMENTS_NODE,
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
        // Look for CallNode `each_with_object({})` with a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"each_with_object" {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check that the argument to each_with_object is an empty hash
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1
            || (arg_list[0].as_hash_node().is_none()
                && arg_list[0].as_keyword_hash_node().is_none())
        {
            return;
        }

        // Check empty hash by looking at source between { and }
        if let Some(hash) = arg_list[0].as_hash_node() {
            let hash_src = hash.location().as_slice();
            let trimmed: Vec<u8> = hash_src
                .iter()
                .filter(|&&b| b != b' ' && b != b'{' && b != b'}')
                .copied()
                .collect();
            if !trimmed.is_empty() {
                return;
            }
        }

        // RuboCop requires destructured block parameters: |(k, v), h|
        // This ensures the receiver is iterated as key-value pairs (i.e. a hash).
        // Simple params like |klass, classes| indicate an array/enumerable, not a hash.
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let bp_params = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        // Need exactly 2 params: first must be destructured (mlhs), second is the hash accumulator
        let reqs: Vec<_> = bp_params.requireds().iter().collect();
        if reqs.len() != 2 {
            return;
        }
        // First param must be destructured (MultiTargetNode) with exactly 2 targets
        let multi_target = match reqs[0].as_multi_target_node() {
            Some(mt) => mt,
            None => return,
        };
        let targets: Vec<_> = multi_target.lefts().iter().collect();
        if targets.len() != 2 {
            return;
        }

        // Extract key/value parameter names from the destructured pair.
        let key_param_name = match targets[0].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return,
        };
        let value_param_name = match targets[1].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return,
        };

        // Check body has a single statement that looks like h[expr] = v
        // where expr is NOT a simple variable (key is transformed)
        // and v is specifically the VALUE parameter from the destructured pair
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // Check for h[key_expr] = v pattern (CallNode with name []=)
        if let Some(assign_call) = body_nodes[0].as_call_node() {
            if assign_call.name().as_slice() == b"[]=" {
                if let Some(assign_args) = assign_call.arguments() {
                    let aargs: Vec<_> = assign_args.arguments().iter().collect();
                    if aargs.len() == 2 {
                        let key_is_simple = aargs[0].as_local_variable_read_node().is_some();
                        if key_is_simple {
                            return;
                        }
                        // The assigned value must be a local variable matching
                        // the VALUE parameter from the destructured pair.
                        // This prevents flagging hash-inversion patterns like
                        // |(id, attrs), h| h[attrs[:code]] = id
                        // where `id` is the KEY param, not the VALUE param.
                        if let Some(val_lvar) = aargs[1].as_local_variable_read_node() {
                            if val_lvar.name().as_slice() == value_param_name.as_slice() {
                                let loc = call.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                let mut diag = self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Prefer `transform_keys` over `each_with_object`.".to_string(),
                                );

                                if let Some(corr) = corrections.as_mut() {
                                    let receiver_src = call.receiver().map_or_else(
                                        || "".to_string(),
                                        |recv| {
                                            String::from_utf8_lossy(
                                                &source.as_bytes()[recv.location().start_offset()
                                                    ..recv.location().end_offset()],
                                            )
                                            .to_string()
                                        },
                                    );
                                    let key_expr_src = String::from_utf8_lossy(
                                        &source.as_bytes()[aargs[0].location().start_offset()
                                            ..aargs[0].location().end_offset()],
                                    )
                                    .to_string();
                                    let key_name =
                                        String::from_utf8_lossy(key_param_name.as_slice());
                                    let replacement = if receiver_src.is_empty() {
                                        format!(
                                            "transform_keys {{ |{}| {} }}",
                                            key_name, key_expr_src
                                        )
                                    } else {
                                        format!(
                                            "{}.transform_keys {{ |{}| {} }}",
                                            receiver_src, key_name, key_expr_src
                                        )
                                    };
                                    corr.push(crate::correction::Correction {
                                        start: loc.start_offset(),
                                        end: loc.end_offset(),
                                        replacement,
                                        cop_name: self.name(),
                                        cop_index: 0,
                                    });
                                    diag.corrected = true;
                                }

                                diagnostics.push(diag);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashTransformKeys, "cops/style/hash_transform_keys");
    crate::cop_autocorrect_fixture_tests!(HashTransformKeys, "cops/style/hash_transform_keys");
}
