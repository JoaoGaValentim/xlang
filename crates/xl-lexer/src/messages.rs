//! Stable diagnostic message builders.
//!
//! Every lexical diagnostic message in this module is locked by snapshot
//! tests; changing a builder's output is a contract change
//! (`docs/proposals/phase-1/lexer-contract.md` §8).

/// `unexpected character '<c>'` (illegal character).
pub(crate) fn unexpected_character(c: char) -> String {
    format!("unexpected character '{c}'")
}

/// `unterminated string literal`.
pub(crate) fn unterminated_string() -> String {
    "unterminated string literal".to_string()
}

/// `unknown escape sequence '\<c>'`.
pub(crate) fn unknown_escape(c: char) -> String {
    format!("unknown escape sequence '\\{c}'")
}

/// `invalid unicode escape sequence`.
pub(crate) fn invalid_unicode_escape() -> String {
    "invalid unicode escape sequence".to_string()
}

/// `unterminated interpolation in string literal`.
pub(crate) fn unterminated_interpolation() -> String {
    "unterminated interpolation in string literal".to_string()
}

/// `interpolation nesting limit (<limit>) exceeded`.
pub(crate) fn interpolation_limit(limit: u32) -> String {
    format!("interpolation nesting limit ({limit}) exceeded")
}

/// `unterminated block comment`.
pub(crate) fn unterminated_block_comment() -> String {
    "unterminated block comment".to_string()
}

/// `expected digits after '0x'` / `expected digits after '0b'`.
pub(crate) fn expected_radix_digits(prefix: &str) -> String {
    format!("expected digits after '{prefix}'")
}

/// `'_' must appear between digits`.
pub(crate) fn misplaced_underscore() -> String {
    "'_' must appear between digits".to_string()
}

/// `integer literal is too large`.
pub(crate) fn integer_too_large() -> String {
    "integer literal is too large".to_string()
}

/// `floating-point literal is not representable`.
pub(crate) fn float_not_representable() -> String {
    "floating-point literal is not representable".to_string()
}

/// `a character literal must contain exactly one character`.
pub(crate) fn char_exactly_one() -> String {
    "a character literal must contain exactly one character".to_string()
}

/// `unterminated character literal`.
pub(crate) fn unterminated_char() -> String {
    "unterminated character literal".to_string()
}
