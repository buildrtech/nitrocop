use std::collections::{HashMap, HashSet};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AttributeAssignment;

impl Cop for AttributeAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/AttributeAssignment"
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
        let mut corrections = corrections;
        // Collect all lines with metadata
        let lines: Vec<(usize, String)> = source
            .lines()
            .enumerate()
            .filter_map(|(idx, line)| {
                std::str::from_utf8(line)
                    .ok()
                    .map(|s| (idx + 1, s.to_string()))
            })
            .collect();

        // Find gemspec block boundaries and process each independently
        let mut block_start_indices = Vec::new();
        for (i, (_line_num, line_str)) in lines.iter().enumerate() {
            let trimmed = line_str.trim();
            if trimmed.contains("Gem::Specification.new") && trimmed.contains("do") {
                block_start_indices.push(i);
            }
        }

        if block_start_indices.is_empty() {
            // No gemspec blocks found, process the whole file as one block
            self.process_block(source, &lines, diagnostics, corrections);
        } else {
            // Process each block independently
            for (bi, &start) in block_start_indices.iter().enumerate() {
                let end = if bi + 1 < block_start_indices.len() {
                    block_start_indices[bi + 1]
                } else {
                    lines.len()
                };
                self.process_block(
                    source,
                    &lines[start..end],
                    diagnostics,
                    corrections.as_deref_mut(),
                );
            }
        }
    }
}

impl AttributeAssignment {
    fn process_block(
        &self,
        source: &SourceFile,
        lines: &[(usize, String)],
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut direct_assignments: HashMap<String, usize> = HashMap::new();
        let mut indexed_assignments: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut corrected_lines = HashSet::new();
        let mut corrections = corrections;

        for (line_num, line_str) in lines {
            let trimmed = line_str.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let leading_spaces = line_str.len() - line_str.trim_start().len();

            match classify_assignment(trimmed) {
                Some((attr, AssignStyle::Direct)) => {
                    direct_assignments.entry(attr).or_insert(*line_num);
                }
                Some((attr, AssignStyle::Indexed)) => {
                    indexed_assignments
                        .entry(attr)
                        .or_default()
                        .push((*line_num, leading_spaces));
                }
                None => {}
            }
        }

        for (attr, locations) in &indexed_assignments {
            if direct_assignments.contains_key(attr) {
                for &(line_num, col) in locations {
                    let mut diagnostic = self.diagnostic(
                        source,
                        line_num,
                        col,
                        "Use consistent style for Gemspec attributes assignment.".to_string(),
                    );
                    if corrected_lines.insert(line_num) {
                        if let Some(corrections) = corrections.as_deref_mut() {
                            if let Some(start) = source.line_col_to_offset(line_num, 0) {
                                let line_len = source
                                    .lines()
                                    .nth(line_num - 1)
                                    .map(|l| l.len())
                                    .unwrap_or(0);
                                corrections.push(crate::correction::Correction {
                                    start,
                                    end: start + line_len,
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
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum AssignStyle {
    Direct,
    Indexed,
}

/// Classify an assignment line as direct or indexed.
/// Returns the attribute name and style, or None if not an assignment.
fn classify_assignment(trimmed: &str) -> Option<(String, AssignStyle)> {
    // Look for a dot after a variable name
    let dot_pos = trimmed.find('.')?;
    let after_dot = &trimmed[dot_pos + 1..];

    // Extract the attribute name (alphanumeric + underscore)
    let attr_end = after_dot
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(after_dot.len());
    if attr_end == 0 {
        return None;
    }
    let attr = &after_dot[..attr_end];
    let rest = &after_dot[attr_end..];

    // Check for indexed assignment: attr[...] = or attr [...] =
    let rest_trimmed = rest.trim_start();
    if rest_trimmed.starts_with('[') {
        if let Some(bracket_end) = rest_trimmed.find(']') {
            let after_bracket = rest_trimmed[bracket_end + 1..].trim_start();
            if after_bracket.starts_with('=') && !after_bracket.starts_with("==") {
                return Some((attr.to_string(), AssignStyle::Indexed));
            }
        }
        return None;
    }

    // Check for direct assignment: attr = (but not attr ==)
    if rest_trimmed.starts_with('=') && !rest_trimmed.starts_with("==") {
        return Some((attr.to_string(), AssignStyle::Direct));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AttributeAssignment, "cops/gemspec/attribute_assignment");

    #[test]
    fn autocorrect_removes_indexed_conflicting_assignment_line() {
        crate::testutil::assert_cop_autocorrect(
            &AttributeAssignment,
            b"Gem::Specification.new do |spec|\n  spec.metadata = { 'k0' => 'v0' }\n  spec.metadata['k1'] = 'v1'\nend\n",
            b"Gem::Specification.new do |spec|\n  spec.metadata = { 'k0' => 'v0' }\n\nend\n",
        );
    }
}
