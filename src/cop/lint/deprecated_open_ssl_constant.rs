use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_READ_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE,
    GLOBAL_VARIABLE_READ_NODE, INSTANCE_VARIABLE_READ_NODE, LOCAL_VARIABLE_READ_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=8.
///
/// Previous FP fix:
/// - `OpenSSL::Digest::Digest.new(...)` is an old alias for `OpenSSL::Digest`
///   itself, not an algorithm-specific subclass, so it must be skipped.
///
/// FN fix:
/// - That alias guard was applied too broadly and also skipped
///   `OpenSSL::Cipher::Cipher.new(...)`, which RuboCop still flags as deprecated.
///   Limit the skip to the digest alias only.
pub struct DeprecatedOpenSSLConstant;

const NO_ARG_ALGORITHM: &[&str] = &["BF", "DES", "IDEA", "RC4"];

fn algo_name_for_cipher(algo_const: &str) -> String {
    if algo_const.len() <= 3 {
        return algo_const.to_string();
    }

    algo_const
        .as_bytes()
        .chunks(3)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect::<Vec<_>>()
        .join("-")
}

fn build_cipher_arguments(algo_const: &str, arg_sources: &[String]) -> String {
    if algo_const == "Cipher" {
        return arg_sources
            .first()
            .cloned()
            .unwrap_or_else(|| "''".to_string());
    }

    let algorithm_name = algo_name_for_cipher(algo_const);
    let mut algorithm_parts: Vec<String> = algorithm_name
        .split('-')
        .map(|s| s.to_ascii_lowercase())
        .collect();

    let mut size_and_mode = Vec::new();
    for arg in arg_sources {
        let cleaned = arg.trim_matches(|c| c == ':' || c == '\'' || c == '"');
        for part in cleaned.split('-') {
            if !part.is_empty() {
                size_and_mode.push(part.to_ascii_lowercase());
            }
        }
    }

    if NO_ARG_ALGORITHM.contains(&algorithm_parts[0].to_ascii_uppercase().as_str())
        && arg_sources.is_empty()
    {
        return format!("'{}'", algorithm_parts[0]);
    }

    if size_and_mode.is_empty() {
        size_and_mode.push("cbc".to_string());
    }

    algorithm_parts.extend(size_and_mode);
    let combined = algorithm_parts
        .into_iter()
        .take(3)
        .collect::<Vec<_>>()
        .join("-");
    format!("'{}'", combined)
}

fn build_replacement(
    parent_class: &str,
    algo_const: &str,
    method_name: &[u8],
    arg_sources: &[String],
) -> Option<String> {
    match parent_class {
        "OpenSSL::Cipher" => {
            if method_name != b"new" {
                return None;
            }
            let replacement_args = build_cipher_arguments(algo_const, arg_sources);
            Some(format!("OpenSSL::Cipher.new({replacement_args})"))
        }
        "OpenSSL::Digest" => {
            let mut args = vec![format!("'{}'", algo_const)];
            args.extend(arg_sources.iter().cloned());
            let args_joined = args.join(", ");
            if method_name == b"new" {
                Some(format!("OpenSSL::Digest.new({args_joined})"))
            } else if method_name == b"digest" {
                Some(format!("OpenSSL::Digest.digest({args_joined})"))
            } else {
                None
            }
        }
        _ => None,
    }
}

impl Cop for DeprecatedOpenSSLConstant {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedOpenSSLConstant"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_READ_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_READ_NODE,
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"new" && method_name != b"digest" {
            return;
        }

        // RuboCop skips when arguments contain variables, method calls, or constants
        // because autocorrection can't handle dynamic values
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if arg.as_local_variable_read_node().is_some()
                    || arg.as_instance_variable_read_node().is_some()
                    || arg.as_class_variable_read_node().is_some()
                    || arg.as_global_variable_read_node().is_some()
                    || arg.as_call_node().is_some()
                    || arg.as_constant_read_node().is_some()
                    || arg.as_constant_path_node().is_some()
                {
                    return;
                }
            }
        }

        // Check for pattern: OpenSSL::Cipher::XXX.new or OpenSSL::Digest::XXX.new/digest
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver should be a ConstantPathNode like OpenSSL::Cipher::AES
        let recv_path = match recv.as_constant_path_node() {
            Some(p) => p,
            None => return,
        };

        let algo_name = match recv_path.name() {
            Some(n) => n,
            None => return,
        };
        let algo_name_str = algo_name.as_slice();

        // Parent should be OpenSSL::Cipher or OpenSSL::Digest
        let parent = match recv_path.parent() {
            Some(p) => p,
            None => return,
        };

        let parent_path = match parent.as_constant_path_node() {
            Some(p) => p,
            None => return,
        };

        let parent_name = match parent_path.name() {
            Some(n) => n,
            None => return,
        };

        let parent_name_str = parent_name.as_slice();
        if parent_name_str != b"Cipher" && parent_name_str != b"Digest" {
            return;
        }

        if parent_name_str == b"Digest" && algo_name_str == b"Digest" {
            return;
        }

        // Grandparent should be OpenSSL
        let grandparent = match parent_path.parent() {
            Some(p) => p,
            None => return,
        };

        let is_openssl = if let Some(const_read) = grandparent.as_constant_read_node() {
            const_read.name().as_slice() == b"OpenSSL"
        } else if let Some(const_path) = grandparent.as_constant_path_node() {
            const_path
                .name()
                .is_some_and(|n| n.as_slice() == b"OpenSSL")
        } else {
            false
        };

        if !is_openssl {
            return;
        }

        let parent_class =
            std::str::from_utf8(parent_path.location().as_slice()).unwrap_or("OpenSSL::Cipher");

        let recv_src =
            std::str::from_utf8(recv.location().as_slice()).unwrap_or("OpenSSL::Cipher::AES");
        let algo_const = std::str::from_utf8(algo_name_str).unwrap_or("AES");

        let arg_sources: Vec<String> = call
            .arguments()
            .map(|args| {
                args.arguments()
                    .iter()
                    .map(|arg| {
                        let loc = arg.location();
                        source
                            .byte_slice(loc.start_offset(), loc.end_offset(), "")
                            .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            format!("Use `{parent_class}` instead of `{recv_src}`."),
        );

        if let Some(corr) = corrections.as_mut() {
            if let Some(replacement) =
                build_replacement(parent_class, algo_const, method_name, &arg_sources)
            {
                corr.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement,
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DeprecatedOpenSSLConstant,
        "cops/lint/deprecated_open_ssl_constant"
    );
    crate::cop_autocorrect_fixture_tests!(
        DeprecatedOpenSSLConstant,
        "cops/lint/deprecated_open_ssl_constant"
    );
}
