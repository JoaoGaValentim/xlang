//! Suite B — Unicode identifiers and UTF-8 boundaries.

mod common;

use common::{describe, kinds, lex};
use xl_lexer::TokenKind;

#[test]
fn accented_and_non_latin_identifiers() {
    assert_eq!(
        kinds("café δ 日本語 Δelta"),
        [
            TokenKind::Ident("café".to_string()),
            TokenKind::Ident("δ".to_string()),
            TokenKind::Ident("日本語".to_string()),
            TokenKind::Ident("Δelta".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn identifiers_may_combine_scripts() {
    assert_eq!(
        kinds("naïve2Straße"),
        [TokenKind::Ident("naïve2Straße".to_string()), TokenKind::Eof]
    );
}

#[test]
fn underscore_prefixed_identifiers_are_allowed() {
    assert_eq!(
        kinds("_unused _1"),
        [
            TokenKind::Ident("_unused".to_string()),
            TokenKind::Ident("_1".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn non_xid_characters_are_illegal() {
    // `²` (No), `①` (No), and emoji (So) are not XID_Start/XID_Continue.
    let (_, output) = lex("x² ① 🦀");
    assert_eq!(output.tokens[0].kind, TokenKind::Ident("x".to_string()));
    let errors: Vec<&xl_lexer::Token> = output
        .tokens
        .iter()
        .filter(|t| t.kind == TokenKind::Error)
        .collect();
    assert_eq!(errors.len(), 3);
    assert_eq!(output.diagnostics.len(), 3);
    assert!(output.diagnostics[0].message().contains("'²'"));
    assert!(output.diagnostics[1].message().contains("'①'"));
    assert!(output.diagnostics[2].message().contains("'🦀'"));
}

#[test]
fn illegal_char_spans_are_char_exact() {
    let (_, output) = lex("a🦀b");
    let error = output
        .tokens
        .iter()
        .find(|t| t.kind == TokenKind::Error)
        .expect("emoji rejected");
    // `🦀` occupies bytes 1..5.
    assert_eq!((error.span.lo, error.span.hi), (1, 5));
    assert_eq!(describe(&output.tokens[2].kind), "Ident(b)".to_string());
}

#[test]
fn bom_is_skipped_at_start_only() {
    let (_, output) = lex("\u{FEFF}let");
    assert_eq!(output.tokens[0].kind, TokenKind::Let);
    assert_eq!(output.tokens[0].span.lo, 3);

    let (_, mid) = lex("a\u{FEFF}b");
    assert_eq!(mid.diagnostics.len(), 1);
    assert!(
        mid.diagnostics[0]
            .message()
            .contains("unexpected character")
    );
}

#[test]
fn identifier_span_on_multibyte_text_is_byte_exact() {
    let (_, output) = lex("日本語");
    assert_eq!((output.tokens[0].span.lo, output.tokens[0].span.hi), (0, 9));
    assert_eq!(output.tokens[1].span.lo, 9);
}
