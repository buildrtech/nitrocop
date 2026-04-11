use crate::cop::node_type::BEGIN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/ShadowedException — detects rescue clauses where a more specific exception
/// is shadowed by a less specific ancestor in the same or earlier rescue clause.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=5, FN=3.
///
/// FP=5: the previous Psych mapping treated `Psych::BadAlias` as a subclass of
/// `Psych::SyntaxError`. On Ruby 3.4, both `Psych::BadAlias` and
/// `Psych::DisallowedClass` are siblings under `Psych::Exception`, so rescue
/// groups like `Psych::SyntaxError, Psych::DisallowedClass, Psych::BadAlias`
/// are valid and should not be flagged.
///
/// FN=3: three gaps remained after the earlier hierarchy expansion:
/// - Leading `::` prefixes (`::Exception`, `::Psych::SyntaxError`) were compared
///   literally instead of being normalized to their constant names.
/// - `OpenSSL::PKey::RSAError` and `OpenSSL::PKey::DSAError` are aliases of the
///   same underlying `OpenSSL::PKey::PKeyError` class, so rescuing both should
///   count as shadowing even though the source strings differ.
/// - The older comment below remains directionally correct: RuboCop uses Ruby's
///   live class hierarchy rather than a pure hardcoded tree, so this cop needs
///   conservative static approximations of the runtime relationships RuboCop sees.
///
/// ## Corpus investigation (2026-03-16)
///
/// FP=2 remaining after prior fixes:
/// - Two consecutive bare `rescue` clauses (both resolve to StandardError) were
///   flagged because `groups_sorted` used `equivalent_exception_classes` which
///   treats identical classes as "shadowing". Bare rescue duplicates are
///   `Lint/DuplicateRescueException`, not `ShadowedException`.
/// - Unknown exception classes (e.g., `ActiveRecord::RecordInvalid` appearing in
///   both an earlier and later rescue clause) were flagged via the same
///   `equivalent_exception_classes` check in `groups_sorted`.
///
/// Fix: removed `equivalent_exception_classes` from `groups_sorted` so cross-clause
/// ordering only checks `is_ancestor_of` (true ancestor/descendant relationships).
/// The `equivalent_exception_classes` check remains in `contains_multiple_levels`
/// for within-group alias detection (e.g., RSAError/DSAError in same rescue).
pub struct ShadowedException;

// Known Ruby exception hierarchy — matches relationships that RuboCop's runtime
// can resolve via `Kernel.const_get` and `<=>` on exception classes.
//
// Each entry (parent, children) means parent is an ancestor of each child.
// `is_ancestor_of` does transitive lookup, so we only need direct parent-child.
//
// LoadError, NotImplementedError and SyntaxError are subclasses of ScriptError
// (NOT StandardError). This matters for Lint/ShadowedException correctness.
const EXCEPTION_HIERARCHY: &[(&str, &[&str])] = &[
    // Core Ruby hierarchy: Exception is root
    (
        "Exception",
        &[
            "StandardError",
            "ScriptError",
            "SecurityError",
            "SignalException",
            "SystemExit",
            "SystemStackError",
            "NoMemoryError",
        ],
    ),
    // StandardError subtree
    (
        "StandardError",
        &[
            "RuntimeError",
            "NameError",
            "TypeError",
            "ArgumentError",
            "RangeError",
            "IOError",
            "EOFError",
            "RegexpError",
            "ZeroDivisionError",
            "ThreadError",
            "SystemCallError",
            "Timeout::Error",
            "SocketError",
            "StopIteration",
            "IndexError",
        ],
    ),
    (
        "ScriptError",
        &["LoadError", "NotImplementedError", "SyntaxError"],
    ),
    ("SignalException", &["Interrupt"]),
    // StandardError deeper subtrees
    ("RuntimeError", &["Psych::SyntaxError"]),
    ("NameError", &["NoMethodError"]),
    (
        "ArgumentError",
        &["Date::Error", "IPAddr::InvalidAddressError"],
    ),
    ("RangeError", &["FloatDomainError"]),
    ("IOError", &["EOFError"]),
    ("IndexError", &["KeyError", "StopIteration"]),
    (
        "SystemCallError",
        &[
            "Errno::ENOENT",
            "Errno::EACCES",
            "Errno::EINVAL",
            "Errno::ECONNRESET",
            "Errno::ECONNREFUSED",
            "Errno::EPIPE",
            "Errno::EAGAIN",
            "Errno::EWOULDBLOCK",
            "Errno::EINTR",
        ],
    ),
    ("Errno::EAGAIN", &["IO::EWOULDBLOCKWaitReadable"]),
    ("Timeout::Error", &["Net::OpenTimeout", "Net::ReadTimeout"]),
    ("SocketError", &["Socket::ResolutionError"]),
    // Standard library exception hierarchies
    ("IPAddr::Error", &["IPAddr::InvalidAddressError"]),
    ("Net::HTTPError", &["Net::HTTPServerException"]),
    (
        "OpenSSL::PKey::PKeyError",
        &[
            "OpenSSL::PKey::RSAError",
            "OpenSSL::PKey::DSAError",
            "OpenSSL::PKey::ECError",
        ],
    ),
    ("Zlib::Error", &["Zlib::GzipFile::Error"]),
    (
        "Gem::Exception",
        &[
            "Gem::LoadError",
            "Gem::InstallError",
            "Gem::DependencyError",
            "Gem::FormatException",
            "Gem::CommandLineError",
        ],
    ),
    ("Gem::LoadError", &["Gem::MissingSpecError"]),
    ("Gem::MissingSpecError", &["Gem::MissingSpecVersionError"]),
];

const EQUIVALENT_EXCEPTION_GROUPS: &[&[&str]] = &[&[
    "OpenSSL::PKey::PKeyError",
    "OpenSSL::PKey::RSAError",
    "OpenSSL::PKey::DSAError",
    "OpenSSL::PKey::ECError",
]];

fn normalize_exception_name(name: &str) -> &str {
    name.trim().trim_start_matches("::")
}

fn equivalent_exception_classes(a: &str, b: &str) -> bool {
    let a = normalize_exception_name(a);
    let b = normalize_exception_name(b);

    // Don't treat identical names as "equivalent" here — duplicate exception
    // names in the same rescue clause are Lint/DuplicateRescueException, not
    // Lint/ShadowedException. Only match known alias groups (e.g. OpenSSL::PKey::*).
    EQUIVALENT_EXCEPTION_GROUPS
        .iter()
        .any(|group| group.contains(&a) && group.contains(&b))
}

fn is_ancestor_of(ancestor: &str, descendant: &str) -> bool {
    let ancestor = normalize_exception_name(ancestor);
    let descendant = normalize_exception_name(descendant);

    if ancestor == descendant {
        return false;
    }
    is_ancestor_of_recursive(ancestor, descendant, 0)
}

fn is_ancestor_of_recursive(ancestor: &str, descendant: &str, depth: usize) -> bool {
    if depth > 10 {
        return false; // prevent infinite recursion
    }
    for &(parent, children) in EXCEPTION_HIERARCHY {
        if parent == ancestor {
            if children.contains(&descendant) {
                return true;
            }
            // Check transitively: ancestor -> child -> ... -> descendant
            for &child in children {
                if is_ancestor_of_recursive(child, descendant, depth + 1) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a single group contains multiple levels of exceptions (ancestor/descendant pair).
fn contains_multiple_levels(group: &[String]) -> bool {
    if group.len() < 2 {
        return false;
    }
    // If group includes Exception and anything else, it has multiple levels
    if group
        .iter()
        .any(|e| normalize_exception_name(e) == "Exception")
    {
        return true;
    }
    for i in 0..group.len() {
        for j in (i + 1)..group.len() {
            if equivalent_exception_classes(&group[i], &group[j])
                || is_ancestor_of(&group[i], &group[j])
                || is_ancestor_of(&group[j], &group[i])
            {
                return true;
            }
        }
    }
    false
}

/// Check if two consecutive groups are in sorted order (more specific first).
fn groups_sorted(earlier: &[String], later: &[String]) -> bool {
    // If earlier group includes Exception, it's always wrong order
    if earlier
        .iter()
        .any(|e| normalize_exception_name(e) == "Exception")
    {
        return false;
    }
    // If later includes Exception or either group is empty, consider sorted
    if later
        .iter()
        .any(|e| normalize_exception_name(e) == "Exception")
        || earlier.is_empty()
        || later.is_empty()
    {
        return true;
    }
    // Check that no earlier exception is an ancestor of a later one.
    // Note: we intentionally do NOT check equivalent_exception_classes here.
    // Exact duplicates across rescue clauses are Lint/DuplicateRescueException,
    // not Lint/ShadowedException. Shadowing requires an ancestor/descendant
    // relationship, and both classes must be in the known hierarchy.
    for e in earlier {
        for l in later {
            if is_ancestor_of(e, l) {
                return false;
            }
        }
    }
    true
}

impl Cop for ShadowedException {
    fn name(&self) -> &'static str {
        "Lint/ShadowedException"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };
        let diagnostics_start = diagnostics.len();

        let mut rescue_opt = begin_node.rescue_clause();
        let mut all_clauses: Vec<(Vec<String>, usize)> = Vec::new();

        while let Some(rescue_node) = rescue_opt {
            let exceptions: Vec<String> = rescue_node
                .exceptions()
                .iter()
                .filter_map(|e| {
                    std::str::from_utf8(e.location().as_slice())
                        .ok()
                        .map(|s| normalize_exception_name(s).to_string())
                })
                .collect();

            let kw_loc = rescue_node.keyword_loc();
            all_clauses.push((exceptions, kw_loc.start_offset()));
            rescue_opt = rescue_node.subsequent();
        }

        if all_clauses.len() < 2 && all_clauses.iter().all(|(excs, _)| excs.len() <= 1) {
            return;
        }

        let groups: Vec<&Vec<String>> = all_clauses.iter().map(|(excs, _)| excs).collect();

        // Check if any single group has multiple levels
        let has_multi_level = groups.iter().any(|g| contains_multiple_levels(g));

        // Check if groups are sorted
        let all_sorted = groups.windows(2).all(|w| {
            let earlier = if w[0].is_empty() {
                vec!["StandardError".to_string()]
            } else {
                w[0].clone()
            };
            let later = if w[1].is_empty() {
                vec!["StandardError".to_string()]
            } else {
                w[1].clone()
            };
            groups_sorted(&earlier, &later)
        });

        if !has_multi_level && all_sorted {
            return;
        }

        // Find the first offending rescue clause (matching RuboCop's find_shadowing_rescue)
        // First check: any group with multiple levels
        for (excs, offset) in all_clauses.iter() {
            let group = if excs.is_empty() {
                vec!["StandardError".to_string()]
            } else {
                excs.clone()
            };
            if contains_multiple_levels(&group) {
                let (line, column) = source.offset_to_line_col(*offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not shadow rescued Exceptions.".to_string(),
                ));
            }
        }

        // Second check: first clause that makes ordering unsorted
        let resolved_groups: Vec<Vec<String>> = all_clauses
            .iter()
            .map(|(excs, _)| {
                if excs.is_empty() {
                    vec!["StandardError".to_string()]
                } else {
                    excs.clone()
                }
            })
            .collect();

        for i in 0..resolved_groups.len().saturating_sub(1) {
            if !groups_sorted(&resolved_groups[i], &resolved_groups[i + 1]) {
                let (line, column) = source.offset_to_line_col(all_clauses[i].1);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not shadow rescued Exceptions.".to_string(),
                ));
            }
        }

        if diagnostics.len() > diagnostics_start {
            if let Some(corrections) = corrections {
                let loc = begin_node.location();
                corrections.push(crate::correction::Correction {
                    start: loc.start_offset(),
                    end: loc.end_offset(),
                    replacement: "nil".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                for diagnostic in diagnostics.iter_mut().skip(diagnostics_start) {
                    diagnostic.corrected = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShadowedException, "cops/lint/shadowed_exception");

    #[test]
    fn autocorrect_rewrites_shadowed_exception_begin_block_to_nil() {
        crate::testutil::assert_cop_autocorrect(
            &ShadowedException,
            b"begin\n  something\nrescue Exception\n  handle_exception\nrescue StandardError\n  handle_standard_error\nend\n",
            b"nil\n",
        );
    }
}
