//! Suite F — operators and punctuation: longest-match tokenization.

mod common;

use common::{kinds, lex};
use xl_lexer::{IntLit, Radix, TokenKind};

fn int(magnitude: u64) -> TokenKind {
    TokenKind::Int(IntLit {
        magnitude,
        radix: Radix::Dec,
    })
}

#[test]
fn mandated_operator_cases_tokenize_as_expected() {
    assert_eq!(
        kinds("1..2"),
        [int(1), TokenKind::DotDot, int(2), TokenKind::Eof]
    );
    assert_eq!(kinds("1.5"), [TokenKind::Float(1.5), TokenKind::Eof]);
    assert_eq!(
        kinds("1.foo"),
        [
            int(1),
            TokenKind::Dot,
            TokenKind::Ident("foo".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("x.0"),
        [
            TokenKind::Ident("x".to_string()),
            TokenKind::Dot,
            int(0),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a>=b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Ge,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("Vec<Vec<T>>"),
        [
            TokenKind::Ident("Vec".to_string()),
            TokenKind::Lt,
            TokenKind::Ident("Vec".to_string()),
            TokenKind::Lt,
            TokenKind::Ident("T".to_string()),
            TokenKind::Gt,
            TokenKind::Gt,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a+=b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::PlusEq,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a==b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::EqEq,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a=>b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::FatArrow,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn full_operator_inventory() {
    let source = "+ - * / % += -= *= /= %= = == != < <= > >= && || ! -> => \
                  ? . .. ::";
    assert_eq!(
        kinds(source),
        [
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq,
            TokenKind::Eq,
            TokenKind::EqEq,
            TokenKind::NotEq,
            TokenKind::Lt,
            TokenKind::Le,
            TokenKind::Gt,
            TokenKind::Ge,
            TokenKind::AndAnd,
            TokenKind::OrOr,
            TokenKind::Bang,
            TokenKind::Arrow,
            TokenKind::FatArrow,
            TokenKind::Question,
            TokenKind::Dot,
            TokenKind::DotDot,
            TokenKind::ColonColon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn operators_adjacent_to_operands() {
    assert_eq!(
        kinds("a-b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Minus,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a->b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Arrow,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a!=b"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::NotEq,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("::a::b"),
        [
            TokenKind::ColonColon,
            TokenKind::Ident("a".to_string()),
            TokenKind::ColonColon,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn unsupported_operator_sequences_decompose_deterministically() {
    // `..=` lexes as `..` then `=` (no inclusive-range operator).
    assert_eq!(
        kinds("..="),
        [TokenKind::DotDot, TokenKind::Eq, TokenKind::Eof]
    );
    // `...` lexes as `..` then `.`.
    assert_eq!(
        kinds("..."),
        [TokenKind::DotDot, TokenKind::Dot, TokenKind::Eof]
    );
    // `>>` is always two `>` (no shift tokens).
    assert_eq!(kinds(">>"), [TokenKind::Gt, TokenKind::Gt, TokenKind::Eof]);
}

#[test]
fn single_ampersand_and_pipe_are_illegal() {
    for source in ["&x", "|x"] {
        let (_, output) = lex(source);
        assert_eq!(output.tokens[0].kind, TokenKind::Error);
        assert_eq!(output.diagnostics.len(), 1);
        assert!(
            output.diagnostics[0]
                .message()
                .contains("unexpected character")
        );
    }
    let (_, output) = lex("a & b");
    assert!(output.diagnostics[0].message().contains("'&'"));
}

#[test]
fn comment_markers_beat_division_operators() {
    assert_eq!(
        kinds("a / b // c"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Slash,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a /* b */ c"),
        [
            TokenKind::Ident("a".to_string()),
            TokenKind::Ident("c".to_string()),
            TokenKind::Eof,
        ]
    );
}
