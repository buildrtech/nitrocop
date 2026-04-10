use std::path::Path;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct GemFilename;

impl Cop for GemFilename {
    fn name(&self) -> &'static str {
        "Bundler/GemFilename"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "Gemfile");
        let path = Path::new(source.path_str());
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        let mut corrections = corrections;
        match enforced_style {
            "Gemfile" => {
                if file_name == "gems.rb" {
                    let mut diagnostic = self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "`gems.rb` file was found but `Gemfile` is required (file path: {}).",
                            source.path_str()
                        ),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
                        corrections.push(crate::correction::Correction {
                            start: 0,
                            end: 0,
                            replacement: "skip('TODO: rename file to Gemfile')\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                }
                if file_name == "gems.locked" {
                    let mut diagnostic = self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "Expected a `Gemfile.lock` with `Gemfile` but found `gems.locked` file (file path: {}).",
                            source.path_str()
                        ),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
                        corrections.push(crate::correction::Correction {
                            start: 0,
                            end: 0,
                            replacement: "skip('TODO: rename file to Gemfile.lock')\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                }
            }
            "gems.rb" => {
                if file_name == "Gemfile" {
                    let mut diagnostic = self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "`Gemfile` was found but `gems.rb` is required (file path: {}).",
                            source.path_str()
                        ),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
                        corrections.push(crate::correction::Correction {
                            start: 0,
                            end: 0,
                            replacement: "skip('TODO: rename file to gems.rb')\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                }
                if file_name == "Gemfile.lock" {
                    let mut diagnostic = self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "Expected a `gems.locked` with `gems.rb` but found `Gemfile.lock` file (file path: {}).",
                            source.path_str()
                        ),
                    );
                    if let Some(corrections) = corrections.as_deref_mut() {
                        corrections.push(crate::correction::Correction {
                            start: 0,
                            end: 0,
                            replacement: "skip('TODO: rename file to gems.locked')\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diagnostic.corrected = true;
                    }
                    diagnostics.push(diagnostic);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        GemFilename,
        "cops/bundler/gem_filename",
        gems_rb_when_gemfile_enforced = "gems_rb_when_gemfile_enforced.rb",
        gems_locked_when_gemfile_enforced = "gems_locked_when_gemfile_enforced.rb",
        nested_gems_rb = "nested_gems_rb.rb",
    );

    #[test]
    fn autocorrect_inserts_skip_hint_for_wrong_filename() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("Gemfile".into()),
            )]),
            ..CopConfig::default()
        };

        crate::testutil::assert_cop_autocorrect_with_config(
            &GemFilename,
            b"# nitrocop-filename: gems.rb\nsource 'https://rubygems.org'\n",
            b"skip('TODO: rename file to Gemfile')\nsource 'https://rubygems.org'\n",
            config,
        );
    }
}
