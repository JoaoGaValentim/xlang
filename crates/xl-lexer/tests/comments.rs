//! Suite E — comments: line, block, nested, termination, and EOF.

mod common;

use common::{kinds, lex};
use xl_lexer::TokenKind;

#[test]
fn line_comments_run_to_end_of_line() {
    assert_eq!(
        kinds("a // b\nc"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Ident("c".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn line_comment_at_eof_is_fine() {
    assert_eq!(
        kinds("a // trailing"),
        [TokenKind::Ident("a".to_string()), TokenKind::Eof]
    );
}

#[test]
fn block_comments_are_skipped() {
    assert_eq!(
        kinds("a /* comment */ b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn block_comments_nest() {
    assert_eq!(
        kinds("a /* outer /* inner */ still */ b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("/* /* /* three */ */ */ x"),
        [TokenKind::Ident("x".to_string()), TokenKind::Eof]
    );
}

#[test]
fn comment_markers_inside_comments_do_not_terminate_early() {
    let (_, output) = lex("/* /* */ */ let");
    assert_eq!(output.tokens[0].kind, TokenKind::Let);
    assert!(output.diagnostics.is_empty());
}

#[test]
fn unterminated_block_comment_is_diagnosed_once() {
    let (_, output) = lex("a /* never closed");
    assert_eq!(output.tokens[0].kind, TokenKind::Ident("a".to_string()));
    assert_eq!(output.tokens[1].kind, TokenKind::Eof);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "unterminated block comment"
    );
    let span = output.diagnostics[0].primary().span;
    assert_eq!((span.lo, span.hi), (2, 17));
}

#[test]
fn block_comment_spans_multiple_lines() {
    assert_eq!(
        kinds("/* line one\nline two */ x"),
        [TokenKind::Ident("x".to_string()), TokenKind::Eof]
    );
}

#[test]
fn empty_block_comment() {
    assert_eq!(kinds("/**/"), [TokenKind::Eof]);
}
