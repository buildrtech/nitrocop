use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle (run 22651309591) reported FP=0, FN=1.
///
/// FN=1: pagy `docs/gem/pagy.gemspec` — symlink path mismatch. Fixed by adding
/// `resolve_symlink_paths.py` to the CI workflow.
///
/// ## Corpus investigation (2026-03-19)
///
/// Corpus oracle (run 23302802988) reported FP=0, FN=1.
///
/// FN=1: DataDog/datadog-ci-rb `datadog-ci.gemspec:8` — multi-line array with
/// string interpolation (`[">= #{CONST}", "< #{CONST}"]`). `is_dynamic_rhs`
/// treated `[` as hash access and returned dynamic=true, skipping the offense.
/// RuboCop treats array literals as non-dynamic (no lvar/send descendants in
/// const paths), can't extract version digits from interpolated strings, and
/// fires a mismatch offense. Fixed by only treating `[` as dynamic when it's
/// not at the start of the RHS (hash access like `foo[...]` vs array literal).
///
/// ## Corpus investigation (2026-03-21, extended corpus)
///
/// Extended corpus reported FP=0, FN=2.
///
/// FN=1: net-http-persistent `net-http-persistent.gemspec:17` — `.freeze`
/// treated as dynamic. RuboCop's `dynamic_version?` does NOT treat bare
/// `"str".freeze` as dynamic (the .freeze IS the node itself, no send
/// descendants). Fixed by stripping trailing `.freeze` from RHS and extracting
/// version normally. Nested `.freeze` (inside `Gem::Requirement.new(...)`) is
/// still treated as dynamic (the .freeze is a send descendant there).
///
/// FN=2: twterm `twterm.gemspec:19` — double assignment
/// `spec.required_ruby_version = spec.required_ruby_version = value`.
/// `split(".required_ruby_version").nth(1)` truncated at the second occurrence,
/// yielding ` = spec` as the RHS, which was treated as a local variable
/// (dynamic). Fixed by using `find` + slice to get the full remainder after
/// the first `.required_ruby_version`, so the RHS includes the actual value.
pub struct RequiredRubyVersion;

/// Extract version digits from a version string like RuboCop does:
/// scan for digits and take the first two, joined with '.'.
/// Single-digit versions (e.g. ">= 3") return just that digit (e.g. "3").
/// e.g. ">= 2.7.0" → "2.7", "~> 3.4" → "3.4", ">= 3" → "3"
fn extract_version_digits(s: &str) -> Option<String> {
    let digits: Vec<char> = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 2 {
        Some(format!("{}.{}", digits[0], digits[1]))
    } else if digits.len() == 1 {
        Some(digits[0].to_string())
    } else {
        None
    }
}

/// Check if a trimmed RHS string looks like a bare local variable (e.g. `version`).
/// Local variables in Ruby start with a lowercase letter or underscore and contain
/// only alphanumeric characters and underscores. RuboCop treats these as dynamic.
fn is_local_variable(rhs: &str) -> bool {
    let ident = rhs.trim();
    if ident.is_empty() {
        return false;
    }
    let first = ident.as_bytes()[0];
    if !(first.is_ascii_lowercase() || first == b'_') {
        return false;
    }
    ident
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Format a TargetRubyVersion f64 as "X.Y".
fn format_target_version(v: f64) -> String {
    let major = v as u32;
    let minor = ((v * 10.0).round() as u32) % 10;
    format!("{major}.{minor}")
}

/// Check if a trimmed RHS string looks dynamic — contains method calls, hash
/// access, constant references, or other non-literal expressions. RuboCop's
/// `dynamic_version?` returns true when the expression has send descendants
/// (which includes `[]` calls), variables, or constant references.
fn is_dynamic_rhs(rhs: &str) -> bool {
    let trimmed = rhs.trim();
    // Hash access like gemspec['key'] or config[:key], but NOT array literals
    // starting with '['. Array literals (e.g. [">= 2.7", "< 4.0"]) are not
    // dynamic — they're non-literal if versions can't be extracted.
    if trimmed.contains('[') && !trimmed.starts_with('[') {
        return true;
    }
    // Method calls with parens like Foo.bar() or obj.method()
    if trimmed.contains('(') {
        return true;
    }
    // Method call on a local variable: e.g. `common_gemspec.required_ruby_version`.
    // RuboCop's dynamic_version? checks descendants for :send and variable types.
    // `(send (lvar :common_gemspec) :method)` has an lvar descendant → dynamic.
    // But `(send (const ...) :method)` has only const descendants → NOT dynamic.
    // So only treat as dynamic when the part before the first dot is a local variable
    // (starts with lowercase/underscore, all alphanumeric).
    if let Some(dot_pos) = trimmed.find('.') {
        let before_dot = &trimmed[..dot_pos];
        if is_local_variable(before_dot) {
            return true;
        }
    }
    false
}

fn assignment_rhs_cols(source: &SourceFile, line: usize) -> Option<(usize, usize)> {
    let line_bytes = source.lines().nth(line - 1)?;
    let line_str = std::str::from_utf8(line_bytes).ok()?;
    let method_pos = line_str.find(".required_ruby_version")?;
    let after_method = &line_str[method_pos + ".required_ruby_version".len()..];
    let eq_rel = after_method.find('=')?;
    let rhs_start = method_pos + ".required_ruby_version".len() + eq_rel + 1;
    Some((rhs_start, line_str.len()))
}

impl Cop for RequiredRubyVersion {
    fn name(&self) -> &'static str {
        "Gemspec/RequiredRubyVersion"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut corrections = corrections;
        let mut found = false;
        // Collect all required_ruby_version assignments for processing.
        // Each entry: (line_1based, col, extracted_version_string), or None if dynamic.
        let mut assignments: Vec<Option<(usize, usize, String)>> = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for required_ruby_version assignment
            if trimmed.contains(".required_ruby_version") {
                // Find the first .required_ruby_version and take everything after it.
                // Using find+slice instead of split().nth(1) so we get the full
                // remainder (split truncates at the next occurrence for double
                // assignments like `a.required_ruby_version = a.required_ruby_version = val`).
                let after = if let Some(pos) = trimmed.find(".required_ruby_version") {
                    &trimmed[pos + ".required_ruby_version".len()..]
                } else {
                    ""
                };
                let after_trimmed = after.trim_start();
                // Must be an assignment (= or >=) not just a method call check
                if after_trimmed.starts_with('=') || after_trimmed.is_empty() {
                    found = true;

                    if let Some(eq_pos) = after_trimmed.find('=') {
                        let rhs = &after_trimmed[eq_pos + 1..].trim_start();

                        // Handle .freeze: RuboCop's dynamic_version? checks
                        // descendants for send nodes. For bare `"str".freeze`,
                        // the .freeze IS the node (no send descendants) → NOT
                        // dynamic. For `Gem::Requirement.new("str".freeze)`,
                        // .freeze is a descendant → IS dynamic.
                        // We approximate: if .freeze is at the end of the RHS,
                        // strip it (bare string case). Otherwise (nested inside
                        // parens), treat as dynamic.
                        let rhs_str;
                        let rhs = if let Some(stripped) = rhs.strip_suffix(".freeze") {
                            rhs_str = stripped.trim_end().to_string();
                            rhs_str.as_str()
                        } else if rhs.contains(".freeze") {
                            assignments.push(None); // dynamic (nested .freeze)
                            continue;
                        } else {
                            rhs
                        };

                        // Check if RHS is a bare local variable (lowercase identifier, no
                        // quotes, no ::, no .). RuboCop treats these as dynamic.
                        if is_local_variable(rhs) {
                            assignments.push(None); // dynamic
                            continue;
                        }

                        // Try to extract the version string from a quoted string.
                        // This handles both plain strings ('>=2.7') and
                        // Gem::Requirement.new(">= 3.4") — both have quotes.
                        let mut extracted_version = false;
                        let quote_char = rhs
                            .find(['\'', '"'])
                            .map(|p| (p, rhs.as_bytes()[p] as char));
                        if let Some((start, qc)) = quote_char {
                            let after_open = &rhs[start + 1..];
                            if let Some(end) = after_open.find(qc) {
                                let ver_str = &after_open[..end];
                                if let Some(extracted) = extract_version_digits(ver_str) {
                                    let ver_literal = &rhs[start..=start + 1 + end];
                                    let col = line_str.find(ver_literal).unwrap_or(0);
                                    assignments.push(Some((line_idx + 1, col, extracted)));
                                    extracted_version = true;
                                }
                            }
                        }

                        if !extracted_version {
                            // No quoted version found. Check if RHS looks dynamic
                            // (hash access like gemspec['key']). RuboCop's
                            // dynamic_version? returns true for send descendants
                            // ([] is a send).
                            if is_dynamic_rhs(rhs) {
                                assignments.push(None); // dynamic
                            } else {
                                // Non-literal (constant, method call, etc.).
                                // RuboCop fires a mismatch offense for these.
                                let col = line_str.find(".required_ruby_version").unwrap_or(0)
                                    + ".required_ruby_version = ".len();
                                assignments.push(Some((line_idx + 1, col, String::new())));
                            }
                        }
                    }
                }
            }
        }

        if !found {
            diagnostics.push(self.diagnostic(
                source,
                1,
                0,
                "`required_ruby_version` should be specified.".to_string(),
            ));
            return;
        }

        // Check version mismatch against TargetRubyVersion for each non-dynamic assignment
        let target = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)));
        if let Some(target_ver) = target {
            let target_str = format_target_version(target_ver);
            for assignment in &assignments {
                if let Some((line, col, ref gemspec_version)) = *assignment {
                    if *gemspec_version != target_str {
                        let mut diagnostic = self.diagnostic(
                            source,
                            line,
                            col,
                            format!(
                                "`required_ruby_version` and `TargetRubyVersion` \
                                 ({target_str}, which may be specified in .rubocop.yml) should be equal."
                            ),
                        );
                        if let Some(corrections) = corrections.as_deref_mut() {
                            if let Some((rhs_start_col, rhs_end_col)) =
                                assignment_rhs_cols(source, line)
                            {
                                if let (Some(start), Some(end)) = (
                                    source.line_col_to_offset(line, rhs_start_col),
                                    source.line_col_to_offset(line, rhs_end_col),
                                ) {
                                    corrections.push(crate::correction::Correction {
                                        start,
                                        end,
                                        replacement: format!(" '>= {target_str}'"),
                                        cop_name: self.name(),
                                        cop_index: 0,
                                    });
                                    diagnostic.corrected = true;
                                }
                            }
                        }
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    crate::cop_scenario_fixture_tests!(
        RequiredRubyVersion,
        "cops/gemspec/required_ruby_version",
        missing_version = "missing_version.rb",
        empty_gemspec = "empty_gemspec.rb",
        only_other_attrs = "only_other_attrs.rb",
    );

    fn config_with_target_ruby(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn autocorrect_rewrites_mismatched_required_ruby_version() {
        crate::testutil::assert_cop_autocorrect_with_config(
            &RequiredRubyVersion,
            b"Gem::Specification.new do |spec|\n  spec.required_ruby_version = '>= 2.4'\nend\n",
            b"Gem::Specification.new do |spec|\n  spec.required_ruby_version = '>= 3.1'\nend\n",
            config_with_target_ruby(3.1),
        );
    }

    #[test]
    fn version_mismatch() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/version_mismatch.rb"
            ),
            config_with_target_ruby(3.1),
        );
    }

    #[test]
    fn single_digit_version() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/single_digit_version.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn dynamic_constant() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/dynamic_constant.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn dynamic_method_call() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/dynamic_method_call.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn version_match_no_offense() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense.rb"
            ),
            config_with_target_ruby(3.0),
        );
    }

    #[test]
    fn freeze_form_no_offense() {
        // Gem::Requirement.new("...".freeze) is treated as dynamic by RuboCop — no offense
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense_freeze.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn local_var_no_offense() {
        // Local variable assignment is treated as dynamic — no offense
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense_local_var.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn requirement_new_no_offense() {
        // Gem::Requirement.new("...") with matching version — no offense
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense_requirement_new.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn hash_access_no_offense() {
        // Hash access like gemspec['required_ruby_version'] is dynamic — no offense
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense_hash_access.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn method_call_no_offense() {
        // Method call like common_gemspec.required_ruby_version is dynamic — no offense
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense_method_call.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }

    #[test]
    fn freeze_version_mismatch() {
        // ">= 2.4".freeze — not dynamic per RuboCop, version "2.4" != target "4.0" → offense
        let source = crate::parse::source::SourceFile::from_bytes(
            "example.gemspec",
            b"Gem::Specification.new do |s|\n  s.required_ruby_version = \">= 2.4\".freeze\nend\n"
                .to_vec(),
        );
        let config = config_with_target_ruby(4.0);
        let mut diags = vec![];
        RequiredRubyVersion.check_lines(&source, &config, &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "should fire mismatch for .freeze version: {diags:?}"
        );
    }

    #[test]
    fn double_assignment_fires_offense() {
        // spec.required_ruby_version = spec.required_ruby_version = Gem::Requirement.new('~> 2.5')
        let source = crate::parse::source::SourceFile::from_bytes(
            "example.gemspec",
            b"Gem::Specification.new do |spec|\n  spec.required_ruby_version = spec.required_ruby_version = Gem::Requirement.new('~> 2.5')\nend\n"
                .to_vec(),
        );
        let config = config_with_target_ruby(4.0);
        let mut diags = vec![];
        RequiredRubyVersion.check_lines(&source, &config, &mut diags, None);
        assert!(
            !diags.is_empty(),
            "should fire mismatch for double assignment: {diags:?}"
        );
    }

    #[test]
    fn array_interpolation() {
        // Multi-line array with interpolated constants — not dynamic, fires mismatch
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/array_interpolation.rb"
            ),
            config_with_target_ruby(4.0),
        );
    }

    #[test]
    fn conditional_branches() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/conditional_branches.rb"
            ),
            config_with_target_ruby(3.4),
        );
    }
}
