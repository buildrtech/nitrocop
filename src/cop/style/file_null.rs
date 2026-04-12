use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Corpus FN investigation for the 0 FP / 71 FN run found misses like
/// `logger: Logger.new("/dev/null")` inside keyword-hash values. RuboCop only
/// exempts strings whose direct parent is an array or pair, so this cop tracks
/// the immediate parent node instead of suppressing every descendant.
pub struct FileNull;

impl Cop for FileNull {
    fn name(&self) -> &'static str {
        "Style/FileNull"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // First pass: check if the file contains any "/dev/null" string
        // (needed for bare "NUL" detection)
        let root = parse_result.node();
        let mut dev_null_finder = DevNullFinder { found: false };
        dev_null_finder.visit(&root);
        let contain_dev_null = dev_null_finder.found;

        // Second pass: find offenses
        let mut visitor = FileNullVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            offense_ranges: Vec::new(),
            contain_dev_null,
            parent_stack: Vec::new(),
        };
        visitor.visit(&root);

        if let Some(ref mut corr) = corrections {
            for &(start, end) in &visitor.offense_ranges {
                corr.push(crate::correction::Correction {
                    start,
                    end,
                    replacement: "File::NULL".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
            for diag in &mut visitor.diagnostics {
                diag.corrected = true;
            }
        }

        diagnostics.extend(visitor.diagnostics);
    }
}

struct DevNullFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for DevNullFinder {
    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        let content = node.unescaped();
        if let Ok(s) = std::str::from_utf8(content) {
            if s.eq_ignore_ascii_case("/dev/null") {
                self.found = true;
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ParentKind {
    Array,
    Assoc,
    Other,
}

impl ParentKind {
    fn from_node(node: &ruby_prism::Node<'_>) -> Self {
        match node {
            ruby_prism::Node::ArrayNode { .. } => Self::Array,
            ruby_prism::Node::AssocNode { .. } => Self::Assoc,
            _ => Self::Other,
        }
    }
}

struct FileNullVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a FileNull,
    diagnostics: Vec<Diagnostic>,
    offense_ranges: Vec<(usize, usize)>,
    contain_dev_null: bool,
    parent_stack: Vec<ParentKind>,
}

impl<'a, 'pr> Visit<'pr> for FileNullVisitor<'a> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.parent_stack.push(ParentKind::from_node(&node));
    }

    fn visit_branch_node_leave(&mut self) {
        self.parent_stack.pop();
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        // RuboCop only accepts strings directly contained by the array/pair.
        if matches!(
            self.parent_stack.last(),
            Some(ParentKind::Array | ParentKind::Assoc)
        ) {
            return;
        }

        let content_bytes = node.unescaped();
        let content_str = match std::str::from_utf8(content_bytes) {
            Ok(s) => s,
            Err(_) => return,
        };

        if content_str.is_empty() {
            return;
        }

        let lower = content_str.to_lowercase();

        let matched =
            if lower == "/dev/null" || lower == "nul:" || (lower == "nul" && self.contain_dev_null)
            {
                Some(content_str)
            } else {
                None
            };

        if let Some(matched_str) = matched {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let mut diag = self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `File::NULL` instead of `{}`.", matched_str),
            );
            diag.corrected = true;
            self.diagnostics.push(diag);
            self.offense_ranges
                .push((loc.start_offset(), loc.end_offset()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileNull, "cops/style/file_null");
    crate::cop_autocorrect_fixture_tests!(FileNull, "cops/style/file_null");
}
