//! Suite A — token correctness: kinds, spans, EOF, and the public API.

mod common;

use common::{describe, kinds, lex};
use xl_lexer::{MAX_INTERPOLATION_DEPTH, TokenKind};
use xl_span::FileId;

#[test]
fn empty_source_yields_only_eof() {
    let (_, output) = lex("");
    assert_eq!(output.tokens.len(), 1);
    assert_eq!(output.tokens[0].kind, TokenKind::Eof);
    assert_eq!(output.tokens[0].span.lo, 0);
    assert_eq!(output.tokens[0].span.hi, 0);
    assert!(output.diagnostics.is_empty());
}

#[test]
fn whitespace_only_source_yields_eof_at_end() {
    let (_, output) = lex("  \t\r\n  ");
    assert_eq!(output.tokens.len(), 1);
    assert_eq!(output.tokens[0].kind, TokenKind::Eof);
    assert_eq!(output.tokens[0].span.lo, 7);
    assert_eq!(output.tokens[0].span.hi, 7);
}

#[test]
fn all_keywords_are_recognized() {
    let source = "fn let mut if else match while for in loop break continue \
                  return struct enum impl trait import pub as true false";
    assert_eq!(
        kinds(source),
        [
            TokenKind::Fn,
            TokenKind::Let,
            TokenKind::Mut,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::Match,
            TokenKind::While,
            TokenKind::For,
            TokenKind::In,
            TokenKind::Loop,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Return,
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Impl,
            TokenKind::Trait,
            TokenKind::Import,
            TokenKind::Pub,
            TokenKind::As,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn keywords_are_case_sensitive() {
    assert_eq!(
        kinds("Fn LET ifx"),
        [
            TokenKind::Ident("Fn".to_string()),
            TokenKind::Ident("LET".to_string()),
            TokenKind::Ident("ifx".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn self_is_an_identifier_not_a_keyword() {
    assert_eq!(
        kinds("self"),
        [TokenKind::Ident("self".to_string()), TokenKind::Eof]
    );
}

#[test]
fn underscore_is_the_wildcard_token() {
    assert_eq!(
        kinds("_ __ _x x_"),
        [
            TokenKind::Underscore,
            TokenKind::Ident("__".to_string()),
            TokenKind::Ident("_x".to_string()),
            TokenKind::Ident("x_".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn simple_punctuation_battery() {
    assert_eq!(
        kinds("( ) { } [ ] , ; :"),
        [
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::Comma,
            TokenKind::Semi,
            TokenKind::Colon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn spans_are_exact_for_a_small_program_fragment() {
    let source = "let x = 1;";
    let (_, output) = lex(source);
    let spans: Vec<(u32, u32)> = output
        .tokens
        .iter()
        .map(|t| (t.span.lo, t.span.hi))
        .collect();
    assert_eq!(
        spans,
        vec![
            (0, 3),   // let
            (4, 5),   // x
            (6, 7),   // =
            (8, 9),   // 1
            (9, 10),  // ;
            (10, 10), // EOF
        ]
    );
    assert_eq!(describe(&output.tokens[0].kind), "Let");
}

#[test]
fn multibyte_spans_are_byte_exact() {
    let source = "let café = 1;";
    let (_, output) = lex(source);
    let ident = &output.tokens[1];
    assert_eq!(ident.kind, TokenKind::Ident("café".to_string()));
    // `café` starts at byte 4 and is 5 bytes long (`é` is two bytes).
    assert_eq!((ident.span.lo, ident.span.hi), (4, 9));
}

#[test]
fn eof_is_unique_and_final() {
    let (_, output) = lex("a b c");
    assert_eq!(
        output
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Eof)
            .count(),
        1
    );
    assert_eq!(output.tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn primary_lex_api_takes_file_and_source() {
    let mut sources = xl_span::SourceMap::new();
    let file = sources
        .add_file("main.x", "fn main() {}")
        .expect("registration succeeds");
    let source_file = sources.file(file).expect("registered");
    let output = xl_lexer::lex(source_file, file);
    assert_eq!(output.tokens[0].kind, TokenKind::Fn);
    assert!(output.tokens.iter().all(|t| t.span.file == file));
}

#[test]
fn lex_file_rejects_unknown_file_ids() {
    let sources = xl_span::SourceMap::new();
    let err = xl_lexer::lex_file(&sources, FileId::from_index(7)).unwrap_err();
    assert_eq!(err, xl_span::SpanError::UnknownFile(FileId::from_index(7)));
}

#[test]
fn interpolation_depth_limit_is_64() {
    assert_eq!(MAX_INTERPOLATION_DEPTH, 64);
}
