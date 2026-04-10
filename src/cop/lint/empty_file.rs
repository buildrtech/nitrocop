use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation: 21 FPs on whitespace-only files.
/// Root cause: RuboCop's `empty_file?` checks `source.empty?` (0 bytes),
/// NOT whether the file contains only whitespace. Files with just newlines
/// or spaces are not flagged by RuboCop. The `contains_only_comments?`
/// check only runs when `AllowComments: false`.
/// Fix: only flag truly empty (0-byte) files, not whitespace-only files.
pub struct EmptyFile;

impl Cop for EmptyFile {
    fn name(&self) -> &'static str {
        "Lint/EmptyFile"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let src = source.as_bytes();

        // RuboCop only flags truly empty files (0 bytes).
        // Whitespace-only files are NOT flagged.
        if src.is_empty() {
            let mut diagnostic = self.diagnostic(source, 1, 0, "Empty file detected.".to_string());
            if let Some(corrections) = corrections.as_deref_mut() {
                corrections.push(crate::correction::Correction {
                    start: 0,
                    end: 0,
                    replacement: "nil\n".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
            diagnostics.push(diagnostic);
            return;
        }

        // When AllowComments is false, also flag files containing only comments/whitespace
        let allow_comments = config.get_bool("AllowComments", true);
        if !allow_comments {
            let has_code = source.lines().any(|line| {
                let trimmed = line
                    .iter()
                    .position(|&b| b != b' ' && b != b'\t' && b != b'\r')
                    .map(|start| &line[start..])
                    .unwrap_or(&[]);
                !trimmed.is_empty() && !trimmed.starts_with(b"#")
            });

            if !has_code {
                let mut diagnostic =
                    self.diagnostic(source, 1, 0, "Empty file detected.".to_string());
                if let Some(corrections) = corrections.as_deref_mut() {
                    let insert_at = src.len();
                    let prefix = if src.last().is_some_and(|b| *b == b'\n') {
                        ""
                    } else {
                        "\n"
                    };
                    corrections.push(crate::correction::Correction {
                        start: insert_at,
                        end: insert_at,
                        replacement: format!("{prefix}nil\n"),
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        EmptyFile,
        "cops/lint/empty_file",
        empty_file = "empty.rb",
        empty_no_newline = "empty_no_newline.rb",
        empty_crlf = "empty_crlf.rb",
    );

    #[test]
    fn supports_autocorrect() {
        assert!(EmptyFile.supports_autocorrect());
    }

    #[test]
    fn autocorrect_empty_file_inserts_nil() {
        crate::testutil::assert_cop_autocorrect(&EmptyFile, b"", b"nil\n");
    }
}
