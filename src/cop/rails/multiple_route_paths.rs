use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MultipleRoutePaths;

const HTTP_METHODS: &[&[u8]] = &[b"get", b"post", b"put", b"patch", b"delete"];

impl Cop for MultipleRoutePaths {
    fn name(&self) -> &'static str {
        "Rails/MultipleRoutePaths"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/config/routes.rb", "**/config/routes/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless HTTP method
        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        if !HTTP_METHODS.contains(&name) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let route_paths: Vec<_> = args
            .arguments()
            .iter()
            .filter(|arg| {
                arg.as_array_node().is_none()
                    && arg.as_hash_node().is_none()
                    && arg.as_keyword_hash_node().is_none()
            })
            .collect();

        if route_paths.len() < 2 {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use separate routes instead of combining multiple route paths in a single route."
                .to_string(),
        );

        if let Some(ref mut corr) = corrections {
            let method = std::str::from_utf8(name).unwrap_or("get");
            let last = route_paths.last().expect("len checked").location();
            let rest = source
                .byte_slice(last.end_offset(), loc.end_offset(), "")
                .to_string();
            let indent = " ".repeat(column);

            let replacement = route_paths
                .iter()
                .map(|rp| {
                    let rloc = rp.location();
                    let rsrc = source.byte_slice(rloc.start_offset(), rloc.end_offset(), "");
                    format!("{method} {rsrc}{rest}")
                })
                .collect::<Vec<_>>()
                .join(&format!("\n{indent}"));

            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diagnostic.corrected = true;
        }

        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleRoutePaths, "cops/rails/multiple_route_paths");

    #[test]
    fn autocorrects_two_paths_into_two_routes() {
        crate::testutil::assert_cop_autocorrect(
            &MultipleRoutePaths,
            b"get '/users', '/other_path', to: 'users#index'\n",
            b"get '/users', to: 'users#index'\nget '/other_path', to: 'users#index'\n",
        );
    }

    #[test]
    fn autocorrects_three_paths_with_indentation() {
        crate::testutil::assert_cop_autocorrect(
            &MultipleRoutePaths,
            b"  put '/x', '/y', '/z', to: 'w#v'\n",
            b"  put '/x', to: 'w#v'\n  put '/y', to: 'w#v'\n  put '/z', to: 'w#v'\n",
        );
    }
}
