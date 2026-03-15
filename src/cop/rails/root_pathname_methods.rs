use crate::cop::node_type::CALL_NODE;
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-15):
///
/// **FPs fixed (4):** All 4 FPs were `File.open(Rails.root.join(...))` used as part of a larger
/// expression (argument to another method or receiver of a chain like `.read`). RuboCop skips
/// `File.open(...)` when `node.parent&.send_type?` because these patterns are handled by
/// `Style/FileRead` / `Style/FileWrite` instead. Since Prism lacks parent pointers, we detect
/// this by checking the source byte after the node's end offset (`.`, `)`, `,` indicate the
/// node is part of a larger expression).
///
/// **FNs fixed (21):** All 21 FNs used `Rails.public_path` instead of `Rails.root`. RuboCop
/// checks both `{:root :public_path}` in its `rails_root?` matcher. Added `public_path`
/// support to `rails_root_method_from_node`.
///
/// **Corpus investigation (2026-03-15, round 2):**
///
/// **FP fixed (1):** `obj.attr = File.open(Rails.root.join(...))` — setter assignment context.
/// RuboCop skips because `node.parent` is a send (`attr=`). Fixed by also checking the byte
/// before the node for `=` (but not `==`).
///
/// **FNs fixed (127):** Nearly all were `File.open(Rails.root.join(...))` inside hash literals,
/// e.g. `{io: File.open(Rails.root.join("public", "photo.png")), ...}`. The old after-node
/// heuristic checked for `,` and `)` which incorrectly skipped these (hash commas, not call
/// args). Refined to only check `.` after the node (chain receiver) and `(`/`,`/`=` before
/// the node (argument to call / setter).
pub struct RootPathnameMethods;

const FILE_METHODS: &[&[u8]] = &[
    b"read",
    b"write",
    b"binread",
    b"binwrite",
    b"readlines",
    b"exist?",
    b"exists?",
    b"directory?",
    b"file?",
    b"empty?",
    b"size",
    b"delete",
    b"unlink",
    b"open",
    b"expand_path",
    b"realpath",
    b"realdirpath",
    b"basename",
    b"dirname",
    b"extname",
    b"join",
    b"stat",
    b"lstat",
    b"ftype",
    b"atime",
    b"ctime",
    b"mtime",
    b"readable?",
    b"writable?",
    b"executable?",
    b"symlink?",
    b"pipe?",
    b"socket?",
    b"zero?",
    b"size?",
    b"owned?",
    b"grpowned?",
    b"chmod",
    b"chown",
    b"truncate",
    b"rename",
    b"split",
    b"fnmatch",
    b"fnmatch?",
    b"blockdev?",
    b"chardev?",
    b"setuid?",
    b"setgid?",
    b"sticky?",
    b"readable_real?",
    b"writable_real?",
    b"executable_real?",
    b"world_readable?",
    b"world_writable?",
    b"readlink",
    b"sysopen",
    b"birthtime",
    b"lchmod",
    b"lchown",
    b"utime",
];

const DIR_METHODS: &[&[u8]] = &[
    b"glob",
    b"[]",
    b"exist?",
    b"exists?",
    b"mkdir",
    b"rmdir",
    b"children",
    b"each_child",
    b"entries",
    b"empty?",
    b"open",
    b"delete",
    b"unlink",
];

const FILE_TEST_METHODS: &[&[u8]] = &[
    b"blockdev?",
    b"chardev?",
    b"directory?",
    b"empty?",
    b"executable?",
    b"executable_real?",
    b"exist?",
    b"file?",
    b"grpowned?",
    b"owned?",
    b"pipe?",
    b"readable?",
    b"readable_real?",
    b"setgid?",
    b"setuid?",
    b"size",
    b"size?",
    b"socket?",
    b"sticky?",
    b"symlink?",
    b"world_readable?",
    b"world_writable?",
    b"writable?",
    b"writable_real?",
    b"zero?",
];

const FILE_UTILS_METHODS: &[&[u8]] =
    &[b"chmod", b"chown", b"mkdir", b"mkpath", b"rmdir", b"rmtree"];

impl Cop for RootPathnameMethods {
    fn name(&self) -> &'static str {
        "Rails/RootPathnameMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Receiver must be a known constant (File, Dir, FileTest, FileUtils, IO)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_name = util::constant_name(&recv);
        let is_relevant = match recv_name {
            Some(b"File") | Some(b"IO") => FILE_METHODS.contains(&method_name),
            Some(b"Dir") => DIR_METHODS.contains(&method_name),
            Some(b"FileTest") => FILE_TEST_METHODS.contains(&method_name),
            Some(b"FileUtils") => FILE_UTILS_METHODS.contains(&method_name),
            _ => false,
        };

        if !is_relevant {
            return;
        }

        // RuboCop skips `File.open(...)` / `IO.open(...)` when the parent is a send node.
        // This handles cases like `File.open(Rails.root.join(...)).read` (receiver of chain),
        // `YAML.safe_load(File.open(Rails.root.join(...)))` (argument to another call),
        // or `obj.attr = File.open(Rails.root.join(...))` (argument to setter method).
        // These are handled by Style/FileRead and Style/FileWrite instead.
        // Since Prism doesn't provide parent pointers, we check the source bytes around
        // the node to detect if it's part of a larger expression (i.e., has a send parent).
        if method_name == b"open" {
            let src = source.as_bytes();
            let start_offset = node.location().start_offset();
            let end_offset = node.location().end_offset();

            // Check after the node: `.` means receiver of method chain
            // e.g., File.open(Rails.root.join(...)).read
            // We intentionally do NOT check for `)` or `,` after the node, because
            // those often indicate hash entry separators (e.g., `io: File.open(...),`)
            // where the parent is a `pair` node, not a send, and RuboCop flags those.
            let after = &src[end_offset..];
            let next_meaningful = after.iter().find(|&&b| b != b' ' && b != b'\t');
            if next_meaningful == Some(&b'.') {
                return;
            }

            // Check before the node for nesting indicators
            // `(` means argument to a call: YAML.safe_load(File.open(...))
            // `,` means second+ arg: foo(x, File.open(...))
            // `=` means RHS of setter method: obj.attr = File.open(...)
            //   (but NOT `==` comparison, which would be `== File.open(...)`)
            if start_offset > 0 {
                let before = &src[..start_offset];
                let prev_meaningful_pos = before.iter().rposition(|&b| b != b' ' && b != b'\t');
                if let Some(pos) = prev_meaningful_pos {
                    match before[pos] {
                        b'(' | b',' => return,
                        b'=' => {
                            // Skip `=` (setter assignment) but not `==` (comparison)
                            if pos == 0 || before[pos - 1] != b'=' {
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // First argument should be a Rails.root pathname:
        // Either `Rails.root.join(...)` or `Rails.root` directly
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];

        // Check if first arg is Rails.root or Rails.public_path directly
        if let Some(rails_label) = rails_root_method_from_node(first_arg) {
            let method_str = std::str::from_utf8(method_name).unwrap_or("method");
            let recv_str = std::str::from_utf8(recv_name.unwrap_or(b"File")).unwrap_or("File");
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("`{rails_label}` is a `Pathname`, so you can use `{rails_label}.{method_str}` instead of `{recv_str}.{method_str}({rails_label}, ...)`.",),
            ));
        }

        // Check if first arg is Rails.root.join(...) or Rails.public_path.join(...)
        if let Some(arg_call) = first_arg.as_call_node() {
            if arg_call.name().as_slice() == b"join" {
                if let Some(rails_label) = rails_root_method(arg_call.receiver()) {
                    let method_str = std::str::from_utf8(method_name).unwrap_or("method");
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("`{rails_label}` is a `Pathname`, so you can use `{rails_label}.join(...).{method_str}` instead.",),
                    ));
                }
            }
        }
    }
}

/// Check if a node is `Rails.root` or `Rails.public_path`, returning the method name.
fn rails_root_method(node: Option<ruby_prism::Node<'_>>) -> Option<&'static str> {
    let node = node?;
    rails_root_method_from_node(&node)
}

fn rails_root_method_from_node(node: &ruby_prism::Node<'_>) -> Option<&'static str> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();
    let label = match method {
        b"root" => "Rails.root",
        b"public_path" => "Rails.public_path",
        _ => return None,
    };
    let recv = call.receiver()?;
    if util::constant_name(&recv) == Some(b"Rails") {
        Some(label)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPathnameMethods, "cops/rails/root_pathname_methods");
}
