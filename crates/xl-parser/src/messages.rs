//! Stable syntactic diagnostic message builders.
//!
//! Every message in this module is part of the parser's observable contract
//! (`docs/proposals/phase-1/conformance-plan.md` §2 pins the negative-corpus
//! texts; changing a builder's output is a contract change). Messages are
//! deterministic and payload-free apart from the quoted fragments.

/// `expected {what}` (for example `expected expression`, `expected ';'`).
pub(crate) fn expected(what: &str) -> String {
    format!("expected {what}")
}

/// `expected '{left}' or '{right}'`.
pub(crate) fn expected_or(left: &str, right: &str) -> String {
    format!("expected '{left}' or '{right}'")
}

/// `unexpected {what}` (for example `unexpected '>'`).
pub(crate) fn unexpected(what: &str) -> String {
    format!("unexpected {what}")
}

/// `invalid assignment target`.
pub(crate) fn invalid_assignment_target() -> String {
    "invalid assignment target".to_string()
}

/// `comparison operators cannot be chained; use parentheses`.
pub(crate) fn chained_comparison() -> String {
    "comparison operators cannot be chained; use parentheses".to_string()
}

/// `range cannot be chained; use parentheses`.
pub(crate) fn chained_range() -> String {
    "range cannot be chained; use parentheses".to_string()
}

/// `chained assignment; parenthesize the right-hand side`.
pub(crate) fn chained_assignment() -> String {
    "chained assignment; parenthesize the right-hand side".to_string()
}

/// `empty interpolation`.
pub(crate) fn empty_interpolation() -> String {
    "empty interpolation".to_string()
}

/// `expression nesting limit ({limit}) exceeded`.
pub(crate) fn nesting_limit(limit: u32) -> String {
    format!("expression nesting limit ({limit}) exceeded")
}

/// `visibility is not allowed on import declarations`.
pub(crate) fn visibility_on_import() -> String {
    "visibility is not allowed on import declarations".to_string()
}

/// `visibility is not allowed on impl blocks`.
pub(crate) fn visibility_on_impl() -> String {
    "visibility is not allowed on impl blocks".to_string()
}
