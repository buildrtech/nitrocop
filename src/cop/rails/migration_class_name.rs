use crate::cop::node_type::CLASS_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MigrationClassName;

impl Cop for MigrationClassName {
    fn name(&self) -> &'static str {
        "Rails/MigrationClassName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        // Check if class inherits from ActiveRecord::Migration
        let superclass = match class_node.superclass() {
            Some(s) => s,
            None => return,
        };

        let super_loc = superclass.location();
        let super_bytes = &source.as_bytes()[super_loc.start_offset()..super_loc.end_offset()];

        // Match ActiveRecord::Migration or ActiveRecord::Migration[x.y]
        if !super_bytes.starts_with(b"ActiveRecord::Migration") {
            return;
        }

        // Get the class name
        let class_name = class_node.name().as_slice();

        // Match existing behavior: names with underscores or non-CamelCase are offenses.
        let has_lowercase = class_name.iter().any(|&b| b.is_ascii_lowercase());
        let starts_upper = class_name.first().is_some_and(|&b| b.is_ascii_uppercase());
        let has_underscore = class_name.contains(&b'_');
        let is_offense = !starts_upper || !has_lowercase || has_underscore;

        if !is_offense {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Migration class name should be CamelCase and match the migration filename."
                .to_string(),
        );

        if has_underscore {
            if let Some(ref mut corr) = corrections {
                let class_loc = class_node.constant_path().location();
                let class_name_str = std::str::from_utf8(class_name).unwrap_or("");
                corr.push(crate::correction::Correction {
                    start: class_loc.start_offset(),
                    end: class_loc.end_offset(),
                    replacement: camelize(class_name_str),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diagnostic.corrected = true;
            }
        }

        diagnostics.push(diagnostic);
    }
}

fn camelize(name: &str) -> String {
    let mut out = String::new();
    for segment in name.split('_') {
        if segment.is_empty() {
            continue;
        }
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(&chars.as_str().to_ascii_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MigrationClassName, "cops/rails/migration_class_name");
    crate::cop_autocorrect_fixture_tests!(MigrationClassName, "cops/rails/migration_class_name");

    #[test]
    fn supports_autocorrect() {
        assert!(MigrationClassName.supports_autocorrect());
    }
}
