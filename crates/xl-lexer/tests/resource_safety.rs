//! Suite I — resource safety: limits, progress, and adversarial input.

mod common;

use common::lex;
use xl_lexer::{MAX_INTERPOLATION_DEPTH, TokenKind};

/// Builds a string literal with `levels` nested interpolations wrapping a
/// one-character string: level 0 is `"x"`, level 1 is `"{ "x" }"`, etc.
fn nested_string(levels: usize) -> String {
    let mut s = String::from("\"x\"");
    for _ in 0..levels {
        s = format!("\"{{ {s} }}\"");
    }
    s
}

#[test]
fn nesting_up_to_the_limit_is_accepted() {
    let source = nested_string(MAX_INTERPOLATION_DEPTH as usize);
    let (_, output) = lex(&source);
    assert_eq!(
        output.diagnostics.len(),
        0,
        "64 levels must be accepted: {:?}",
        output
            .diagnostics
            .iter()
            .map(|d| d.message())
            .collect::<Vec<_>>()
    );
    assert_eq!(output.tokens.len(), 2);
    assert!(matches!(output.tokens[0].kind, TokenKind::Str(_)));
    assert_eq!(output.tokens[1].kind, TokenKind::Eof);
}

#[test]
fn exceeding_the_limit_is_diagnosed_and_recovered() {
    let source = nested_string(MAX_INTERPOLATION_DEPTH as usize + 1);
    let (_, output) = lex(&source);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "interpolation nesting limit (64) exceeded"
    );
    // The stream still terminates cleanly with a single string token.
    assert_eq!(output.tokens.len(), 2);
    assert!(matches!(output.tokens[0].kind, TokenKind::Str(_)));
    assert_eq!(output.tokens[1].kind, TokenKind::Eof);
}

#[test]
fn limit_recovery_skips_nested_strings_and_comments() {
    // The over-limit interpolation contains a quoted brace and a commented
    // brace before its true closing brace; the raw recovery must not stop
    // early.
    let base = "\"a { \"}\" /* } */ b } c\"";
    let mut source = base.to_string();
    for _ in 0..MAX_INTERPOLATION_DEPTH {
        source = format!("\"{{ {source} }}\"");
    }
    let (_, output) = lex(&source);
    assert_eq!(
        output.diagnostics.len(),
        1,
        "{:?}",
        output
            .diagnostics
            .iter()
            .map(|d| d.message())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        output.diagnostics[0].message(),
        "interpolation nesting limit (64) exceeded"
    );
    assert_eq!(output.tokens.len(), 2);
    let span = output.tokens[0].span;
    assert_eq!(span.lo, 0);
    assert_eq!(span.hi as usize, source.len());
}

#[test]
fn limit_recovery_hitting_eof_reports_both_conditions() {
    let deep = nested_string(MAX_INTERPOLATION_DEPTH as usize);
    let source = format!("\"{{ {deep} ");
    let (_, output) = lex(&source);
    let messages: Vec<&str> = output.diagnostics.iter().map(|d| d.message()).collect();
    assert_eq!(
        messages,
        [
            "interpolation nesting limit (64) exceeded",
            "unterminated interpolation in string literal",
        ]
    );
    assert_eq!(output.tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn thousands_of_nested_comments_terminate() {
    // Openers and closers are separated by spaces so that no `/*` or `*/`
    // pairs overlap across the junction.
    let mut source = String::new();
    for _ in 0..10_000 {
        source.push_str("/* ");
    }
    for _ in 0..10_000 {
        source.push_str("*/ ");
    }
    source.push('x');
    let (_, output) = lex(&source);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tokens[0].kind, TokenKind::Ident("x".to_string()));
    assert_eq!(output.tokens[1].kind, TokenKind::Eof);
}

#[test]
fn very_long_identifier_is_one_token() {
    let source = "a".repeat(100_000);
    let (_, output) = lex(&source);
    assert_eq!(output.tokens.len(), 2);
    assert_eq!(output.tokens[0].kind, TokenKind::Ident(source.clone()));
    assert!(output.diagnostics.is_empty());
}

#[test]
fn very_long_digit_run_is_rejected_once() {
    let source = "9".repeat(100_000);
    let (_, output) = lex(&source);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "integer literal is too large"
    );
    assert_eq!(output.tokens.len(), 2);
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
}

#[test]
fn consecutive_illegal_characters_progress() {
    let source = "~".repeat(1_000);
    let (_, output) = lex(&source);
    assert_eq!(output.tokens.len(), 1_001);
    assert_eq!(output.diagnostics.len(), 1_000);
    assert_eq!(output.tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn many_escaped_braces_do_not_open_interpolations() {
    let mut source = String::from("\"");
    source.push_str(&"\\{".repeat(5_000));
    source.push('"');
    let (_, output) = lex(&source);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tokens.len(), 2);
    let TokenKind::Str(lit) = &output.tokens[0].kind else {
        panic!("expected a string token");
    };
    assert_eq!(lit.parts.len(), 1);
    let xl_lexer::StrPart::Text(text) = &lit.parts[0] else {
        panic!("expected one text part");
    };
    assert_eq!(text.text.chars().count(), 5_000);
}

#[test]
fn multiple_independent_errors_are_all_reported() {
    // The newline terminates the unterminated string so that the remaining
    // errors on the following line are also reported.
    let (_, output) = lex("~ \"abc\n0x 1_ /* ~");
    assert_eq!(output.diagnostics.len(), 5);
}

#[test]
fn empty_and_tiny_inputs() {
    for source in ["", " ", "\n", "\u{FEFF}", "\"", "'", "/", "/*", "//"] {
        let (_, output) = lex(source);
        assert_eq!(
            output.tokens.last().unwrap().kind,
            TokenKind::Eof,
            "source {source:?}"
        );
        assert_eq!(
            output
                .tokens
                .iter()
                .filter(|t| t.kind == TokenKind::Eof)
                .count(),
            1,
            "source {source:?}"
        );
    }
}
