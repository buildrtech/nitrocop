use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DuplicatedAssignment;

impl Cop for DuplicatedAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/DuplicatedAssignment"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Track: attribute_name -> first occurrence line
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut corrected_lines = std::collections::HashSet::new();
        let mut corrections = corrections;

        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            // Skip comments and blank lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let line_num = line_idx + 1;
            let attrs = extract_assignment_attrs(trimmed);
            for attr in &attrs {
                match seen.entry(attr.clone()) {
                    std::collections::hash_map::Entry::Occupied(e) => {
                        // Find the position of the attribute in the original line
                        let dot_pos = line_str.find('.').unwrap_or(0);
                        let mut diagnostic = self.diagnostic(
                            source,
                            line_num,
                            dot_pos + 1, // after the dot
                            format!(
                                "Attribute `{}` is already set on line {}.",
                                e.key(),
                                e.get()
                            ),
                        );
                        if corrected_lines.insert(line_num) {
                            if let Some(corrections) = corrections.as_deref_mut() {
                                if let Some(start) = source.line_col_to_offset(line_num, 0) {
                                    let end = start + line.len();
                                    corrections.push(crate::correction::Correction {
                                        start,
                                        end,
                                        replacement: "".to_string(),
                                        cop_name: self.name(),
                                        cop_index: 0,
                                    });
                                    diagnostic.corrected = true;
                                }
                            }
                        }
                        diagnostics.push(diagnostic);
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(line_num);
                    }
                }
            }
        }
    }
}

/// Extract all assignment attribute names from a line like `spec.name = 'foo'`.
/// Returns empty vec for non-assignment lines or append operations (<<).
/// Returns multiple entries for self-assignment patterns like `spec.name = spec.name = 'foo'`.
fn extract_assignment_attrs(trimmed: &str) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut search_from = 0;

    while search_from < trimmed.len() {
        let remaining = &trimmed[search_from..];
        let rel_dot_pos = match remaining.find('.') {
            Some(p) => p,
            None => break,
        };
        let after_dot = &remaining[rel_dot_pos + 1..];

        // Extract attribute name (alphanumeric + underscore)
        let attr_end = after_dot
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(after_dot.len());
        if attr_end == 0 {
            search_from += rel_dot_pos + 1;
            continue;
        }
        let attr_name = &after_dot[..attr_end];
        let after_attr = &after_dot[attr_end..];

        // Check for bracket-style access: spec.metadata['key'] = val
        let (full_attr, rest_str) = if after_attr.starts_with('[') {
            if let Some(bracket_end) = after_attr.find(']') {
                let bracket_part = &after_attr[..=bracket_end];
                let full = format!("{}{}", attr_name, bracket_part);
                let after_bracket = after_attr[bracket_end + 1..].trim_start();
                (full, after_bracket)
            } else {
                search_from += rel_dot_pos + 1 + attr_end;
                continue;
            }
        } else {
            (attr_name.to_string(), after_attr.trim_start())
        };

        // Must be followed by `=` but not `==` or `<<`
        let is_assignment = if rest_str.starts_with("==") {
            false
        } else if rest_str.starts_with("= ") || rest_str.starts_with("=\n") || rest_str == "=" {
            true
        } else {
            rest_str.starts_with('=') && rest_str.len() > 1 && rest_str.as_bytes()[1] != b'='
        };

        if is_assignment {
            attrs.push(full_attr);
            // Continue searching after `=` for self-assignment patterns
            let abs_dot = search_from + rel_dot_pos;
            if let Some(eq_offset) = trimmed[abs_dot..].find('=') {
                search_from = abs_dot + eq_offset + 1;
            } else {
                break;
            }
        } else {
            search_from += rel_dot_pos + 1 + attr_end;
        }
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedAssignment, "cops/gemspec/duplicated_assignment");

    #[test]
    fn autocorrect_removes_duplicate_assignment_line() {
        crate::testutil::assert_cop_autocorrect(
            &DuplicatedAssignment,
            b"Gem::Specification.new do |spec|\n  spec.name = 'example'\n  spec.name = 'other'\nend\n",
            b"Gem::Specification.new do |spec|\n  spec.name = 'example'\n\nend\n",
        );
    }
}
