use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OrderedGems;

impl Cop for OrderedGems {
    fn name(&self) -> &'static str {
        "Bundler/OrderedGems"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let treat_comments_as_separators = config.get_bool("TreatCommentsAsGroupSeparators", true);
        let consider_punctuation = config.get_bool("ConsiderPunctuation", false);

        let mut prev_gem: Option<(String, String)> = None; // (original_name, sort_key)
        let mut in_block_comment = false;

        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim();
            let line_num = i + 1;

            if in_block_comment {
                if trimmed.starts_with("=end") {
                    in_block_comment = false;
                    prev_gem = None;
                }
                continue;
            }

            if trimmed.starts_with("=begin") {
                in_block_comment = true;
                prev_gem = None;
                continue;
            }

            // Blank lines reset the ordering group
            if trimmed.is_empty() {
                prev_gem = None;
                continue;
            }

            // Comments may reset the ordering group
            if trimmed.starts_with('#') {
                if treat_comments_as_separators {
                    prev_gem = None;
                }
                continue;
            }

            // Non-gem, non-blank, non-comment lines (like `group`, `source`, etc.)
            // also reset the ordering group
            if let Some(gem_name) = extract_literal_gem_name(line_str) {
                let sort_key = make_sort_key(gem_name, consider_punctuation);

                if let Some((ref prev_name, ref prev_key)) = prev_gem {
                    if sort_key < *prev_key {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            0,
                            format!(
                                "Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `{}` should appear before `{}`.",
                                gem_name, prev_name
                            ),
                        ));
                    }
                }

                prev_gem = Some((gem_name.to_string(), sort_key));
            } else {
                // Non-gem declaration resets the group
                prev_gem = None;
            }
        }
    }
}

/// Extract the gem name from literal first-argument forms:
/// - `gem 'foo'`
/// - `gem "foo"`
/// - `gem('foo')`
///
/// Lines like `gem ENV['FOO'] || 'foo'` are intentionally ignored.
fn extract_literal_gem_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let after_gem = trimmed.strip_prefix("gem")?;
    let first = after_gem.chars().next()?;
    if !first.is_whitespace() && first != '(' {
        return None;
    }

    let mut rest = after_gem.trim_start();
    if let Some(after_paren) = rest.strip_prefix('(') {
        rest = after_paren.trim_start();
    }

    let quote = rest.as_bytes().first().copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }

    let content = &rest[1..];
    let end = content.find(quote as char)?;
    Some(&content[..end])
}

/// Create a sort key for case-insensitive comparison.
/// When consider_punctuation is false, strip `-` and `_` for comparison.
fn make_sort_key(name: &str, consider_punctuation: bool) -> String {
    let lower = name.to_lowercase();
    if consider_punctuation {
        lower
    } else {
        lower.replace(['-', '_'], "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrderedGems, "cops/bundler/ordered_gems");
}
