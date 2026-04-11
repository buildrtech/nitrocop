use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// **Round 1** — Corpus oracle reported FP=19, FN=2.
/// The core range tracking was correct, but directive parsing was too literal.
/// Real-world code uses both `rubocop:disable` and `rubocop: disable`, and
/// disable comments often carry a trailing explanation. Our parser kept that
/// explanatory text as part of the cop name, so later clean `rubocop:enable`
/// directives did not match and we reported missing enables incorrectly.
/// Fix improved the cop from 158 offenses to 146, leaving 5-6 excess FPs.
///
/// **Round 2** — FP=6, FN=0. Root cause: `# rubocop:enable all` was not
/// recognized as closing all individually opened disables. RuboCop expands
/// `enable all` to re-enable every currently-disabled cop. Our code treated
/// "all" as a literal cop name and only removed the "all" key from
/// `open_disables`, leaving individually disabled cops (e.g.
/// `Metrics/MethodLength`, `Layout`) unclosed. Fix: when the enable directive
/// contains "all", drain all open disables (checking MaximumRangeSize for each).
///
/// **Round 3** — FP=2, FN=0. Root cause: `--` trailing comment marker not
/// handled. RuboCop uses `--` as a delimiter between cop names and explanatory
/// text (e.g., `# rubocop:disable Style/Foo -- use bar, baz instead`). Our
/// `parse_directive` split the entire text after the action on commas without
/// first stripping the `--` suffix, so explanations containing commas produced
/// phantom cop names (e.g., `baz` from `bar, baz instead`). These phantom
/// disables had no matching enable and were reported as FPs. Fix: strip text
/// after `--` before splitting on commas.
///
/// **Round 4** — FP=1, FN=0. Root cause: malformed tokens such as `Metrics/`
/// were still accepted as cop names. In real-world directives like
/// `# rubocop:disable /BlockLength, Metrics/`, RuboCop ignores both malformed
/// tokens entirely, so no later enable is required. Fix: reject tokens that do
/// not start with an alphanumeric character or that end with `/`.
pub struct MissingCopEnableDirective;

impl Cop for MissingCopEnableDirective {
    fn name(&self) -> &'static str {
        "Lint/MissingCopEnableDirective"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Fast reject: most files have no rubocop directives.
        if !contains_subslice(source.as_bytes(), b"rubocop:") {
            return;
        }

        let max_range = get_max_range_size(config);
        // Track open disables: cop_name -> (line_number, column)
        let mut open_disables: HashMap<String, (usize, usize)> = HashMap::new();

        let mut byte_offset = 0usize;
        let mut line_count = 0usize;
        for (i, line) in source.lines().enumerate() {
            line_count = i + 1;
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => {
                    byte_offset += line.len() + 1;
                    continue;
                }
            };

            // Skip lines inside strings/heredocs
            if let Some(hash_pos) = line_str.find('#') {
                if !code_map.is_not_string(byte_offset + hash_pos) {
                    byte_offset += line.len() + 1;
                    continue;
                }
            }

            // Find rubocop directives
            let Some((action, cops, col)) = parse_directive(line_str) else {
                byte_offset += line.len() + 1;
                continue;
            };

            match action {
                "disable" | "todo" => {
                    // Check if this is an inline disable (code before the comment)
                    let before = &line_str[..col];
                    let is_inline = !before.trim().is_empty();
                    if !is_inline {
                        for cop in &cops {
                            open_disables.insert(cop.to_string(), (i + 1, col));
                        }
                    }
                }
                "enable" => {
                    // `rubocop:enable all` closes ALL open disables
                    let is_enable_all = cops.iter().any(|c| c == "all");
                    if is_enable_all {
                        let all_open: Vec<(String, (usize, usize))> =
                            open_disables.drain().collect();
                        for (cop, (start_line, _)) in &all_open {
                            if max_range.is_finite() {
                                let range_size = (i + 1) - start_line - 1;
                                if range_size > max_range as usize {
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        *start_line,
                                        0,
                                        format_message(cop, Some(max_range as usize)),
                                    ));
                                }
                            }
                        }
                    } else {
                        for cop in &cops {
                            if let Some((start_line, _)) = open_disables.remove(cop.as_str()) {
                                // Check if range exceeds MaximumRangeSize
                                if max_range.is_finite() {
                                    let range_size = (i + 1) - start_line - 1; // lines between disable and enable
                                    if range_size > max_range as usize {
                                        diagnostics.push(self.diagnostic(
                                            source,
                                            start_line,
                                            0,
                                            format_message(cop, Some(max_range as usize)),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            byte_offset += line.len() + 1;
        }

        if open_disables.is_empty() {
            return;
        }

        let mut missing_enable_cops: Vec<String> = Vec::new();

        // Report all remaining open disables (never re-enabled)
        for (cop, (line, _col)) in &open_disables {
            // RuboCop's acceptable_range? skips offenses when the disabled cop is
            // itself not enabled in config — the disable is harmless.
            if is_cop_disabled_in_config(config, cop) {
                continue;
            }
            if max_range.is_finite() {
                let range_size = line_count.saturating_sub(*line);
                if range_size > max_range as usize {
                    diagnostics.push(self.diagnostic(
                        source,
                        *line,
                        0,
                        format_message(cop, Some(max_range as usize)),
                    ));
                    missing_enable_cops.push(cop.clone());
                }
            } else {
                diagnostics.push(self.diagnostic(source, *line, 0, format_message(cop, None)));
                missing_enable_cops.push(cop.clone());
            }
        }

        if let Some(corrections) = corrections {
            if !missing_enable_cops.is_empty() {
                missing_enable_cops.sort();
                missing_enable_cops.dedup();
                let mut appendix = String::new();
                if !source.as_bytes().ends_with(b"\n") {
                    appendix.push('\n');
                }
                for cop in &missing_enable_cops {
                    appendix.push_str(&format!("# rubocop:enable {cop}\n"));
                }
                let eof = source.as_bytes().len();
                corrections.push(crate::correction::Correction {
                    start: eof,
                    end: eof,
                    replacement: appendix,
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
        }

        // Sort by line number for deterministic output
        diagnostics.sort_by_key(|d| d.location.line);
    }
}

fn format_message(cop: &str, max_range: Option<usize>) -> String {
    // Determine if it's a department (no `/`) or a specific cop
    let kind = if cop.contains('/') {
        "cop"
    } else {
        "department"
    };
    match max_range {
        Some(n) => format!(
            "Re-enable {} {} within {} lines after disabling it.",
            cop, kind, n
        ),
        None => format!(
            "Re-enable {} {} with `# rubocop:enable` after disabling it.",
            cop, kind,
        ),
    }
}

fn is_cop_disabled_in_config(config: &CopConfig, cop: &str) -> bool {
    config
        .options
        .get("DisabledCopNames")
        .and_then(|v| v.as_sequence())
        .is_some_and(|seq| seq.iter().any(|item| item.as_str() == Some(cop)))
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn get_max_range_size(config: &CopConfig) -> f64 {
    config
        .options
        .get("MaximumRangeSize")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_u64().map(|u| u as f64))
                .or_else(|| {
                    v.as_str().and_then(|s| {
                        if s == ".inf" || s == "Infinity" {
                            Some(f64::INFINITY)
                        } else {
                            s.parse::<f64>().ok()
                        }
                    })
                })
        })
        .unwrap_or(f64::INFINITY)
}

fn parse_directive(line: &str) -> Option<(&str, Vec<String>, usize)> {
    let hash_pos = line.find('#')?;
    let after_hash = &line[hash_pos + 1..].trim_start();

    let after_prefix = after_hash.strip_prefix("rubocop:")?.trim_start();

    // Extract action
    let action_end = after_prefix
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(after_prefix.len());
    let action = &after_prefix[..action_end];

    if !matches!(action, "disable" | "enable" | "todo") {
        return None;
    }

    // Strip `--` trailing comment marker (RuboCop convention) before parsing cop names.
    // Without this, explanations like `-- use bar, baz instead` would split on the comma
    // and incorrectly parse `baz` as a cop name.
    let cops_str = after_prefix[action_end..].split("--").next().unwrap_or("");
    let cops: Vec<String> = cops_str.split(',').filter_map(parse_cop_token).collect();
    if cops.is_empty() {
        return None;
    }

    // We need to return a reference to the action string within the original line.
    // Since we trimmed, use the computed action.
    // But we can't return a reference to a substring we created.
    // Let's compute the action from the original line instead.
    let action_str = match action {
        "disable" => "disable",
        "enable" => "enable",
        "todo" => "todo",
        _ => return None,
    };

    Some((action_str, cops, hash_pos))
}

fn parse_cop_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start();
    let first = *trimmed.as_bytes().first()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }
    let end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '/' || c == '_'))
        .unwrap_or(trimmed.len());
    let token = trimmed[..end].trim_end_matches('.');
    if token.is_empty() || token.ends_with('/') {
        None
    } else {
        Some(token.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        MissingCopEnableDirective,
        "cops/lint/missing_cop_enable_directive",
        missing_enable_cop = "missing_enable_cop.rb",
        missing_enable_dept = "missing_enable_dept.rb",
        missing_enable_two = "missing_enable_two.rb",
        missing_enable_spaced = "missing_enable_spaced.rb",
    );

    #[test]
    fn autocorrect_appends_missing_enable_directive() {
        crate::testutil::assert_cop_autocorrect(
            &MissingCopEnableDirective,
            b"# rubocop:disable Layout/LineLength\nx =   0\n",
            b"# rubocop:disable Layout/LineLength\nx =   0\n# rubocop:enable Layout/LineLength\n",
        );
    }
}
