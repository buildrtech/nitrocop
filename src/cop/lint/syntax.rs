use crate::cop::Cop;
use crate::diagnostic::Severity;

/// Checks for syntax errors.
///
/// This cop is a registration stub — the actual detection logic lives in
/// `emit_syntax_diagnostics()` in `src/linter.rs`. When a file has structural
/// parse errors (detected by Prism), each error is emitted as a Lint/Syntax
/// offense with Fatal severity, matching RuboCop's behavior of repacking
/// parser diagnostics into Lint/Syntax offenses.
///
/// ## Corpus investigation (2026-03-24)
///
/// FN=4708: nitrocop silently skipped files with parse errors (returning empty
/// diagnostics). RuboCop's Lint/Syntax reports each parser error/fatal diagnostic
/// as a separate offense. Fixed by adding `emit_syntax_diagnostics()` to the
/// linter pipeline that emits one Lint/Syntax diagnostic per structural Prism
/// error when the cop is enabled.
pub struct Syntax;

impl Cop for Syntax {
    fn name(&self) -> &'static str {
        "Lint/Syntax"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_severity(&self) -> Severity {
        Severity::Fatal
    }

    // Syntax errors are reported by the parser (Prism), not by this cop.
    // This struct exists for configuration compatibility with RuboCop.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cop_name() {
        assert_eq!(Syntax.name(), "Lint/Syntax");
    }

    #[test]
    fn default_severity_is_fatal() {
        assert_eq!(Syntax.default_severity(), Severity::Fatal);
    }

    #[test]
    fn no_offenses_on_valid_source() {
        use crate::testutil::run_cop_full;
        let source = b"x = 1\ny = 2\n";
        let diags = run_cop_full(&Syntax, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn supports_autocorrect_flag_enabled() {
        assert!(Syntax.supports_autocorrect());
    }
}
