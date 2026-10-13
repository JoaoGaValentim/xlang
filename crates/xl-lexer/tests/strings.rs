//! Suite D — string literals: escapes, structured segments, interpolation,
//! and malformed input.

mod common;

use common::{dump_string_token, kinds, lex};
use xl_lexer::TokenKind;

#[test]
fn plain_string_has_one_text_segment() {
    let dump = dump_string_token("\"abc\"");
    assert_eq!(dump, "Str@0..5\n  Text@1..4 \"abc\"\n");
}

#[test]
fn empty_string_has_no_parts() {
    assert_eq!(dump_string_token("\"\""), "Str@0..2\n");
}

#[test]
fn decoded_escapes_and_segment_spans_cover_raw_bytes() {
    // `"a\nb"`: the text segment covers the raw 5 interior bytes even
    // though the decoded text is 3 characters.
    let dump = dump_string_token("\"a\\nb\"");
    assert_eq!(dump, "Str@0..6\n  Text@1..5 \"a\\nb\"\n");
}

#[test]
fn escaped_braces_are_literal_text() {
    let dump = dump_string_token("\"literal \\{brace\\}\"");
    assert_eq!(dump, "Str@0..19\n  Text@1..18 \"literal {brace}\"\n");
}

#[test]
fn lone_closing_brace_in_text_is_literal() {
    let dump = dump_string_token("\"a } b\"");
    assert_eq!(dump, "Str@0..7\n  Text@1..6 \"a } b\"\n");
}

#[test]
fn unicode_escapes_decode() {
    let dump = dump_string_token("\"\\u{1F980}\"");
    assert_eq!(dump, "Str@0..11\n  Text@1..10 \"🦀\"\n");
}

#[test]
fn overlong_unicode_escape_is_invalid() {
    let (_, output) = lex("\"\\u{1234567}\"");
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "invalid unicode escape sequence"
    );
    // The malformed escape is consumed; the literal still closes.
    assert_eq!(output.tokens.len(), 2);
}

#[test]
fn unicode_escape_beyond_range_is_invalid() {
    for source in ["\"\\u{110000}\"", "\"\\u{}\"", "\"\\uX\""] {
        let (_, output) = lex(source);
        assert_eq!(
            output.diagnostics.len(),
            1,
            "source {source:?} must produce one diagnostic"
        );
        assert_eq!(
            output.diagnostics[0].message(),
            "invalid unicode escape sequence",
            "source {source:?}"
        );
    }
}

#[test]
fn unknown_escape_is_recovered_literally() {
    let (_, output) = lex("\"\\q\"");
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "unknown escape sequence '\\q'"
    );
    let dump = dump_string_token("\"\\q\"");
    assert_eq!(dump, "Str@0..4\n  Text@1..3 \"q\"\n");
}

#[test]
fn simple_interpolation_structure() {
    let dump = dump_string_token("\"total = {total}\"");
    assert_eq!(
        dump,
        "Str@0..17\n  Text@1..9 \"total = \"\n  Interp open@9..10 close@15..16\n    10..15 Ident(total)\n"
    );
}

#[test]
fn interpolated_call_contains_embedded_tokens() {
    let dump = dump_string_token("\"value = {compute(1 + 2)}\"");
    assert_eq!(
        dump,
        "Str@0..26\n  \
         Text@1..9 \"value = \"\n  \
         Interp open@9..10 close@24..25\n    \
         10..17 Ident(compute)\n    \
         17..18 LParen\n    \
         18..19 Int(1, Dec)\n    \
         20..21 Plus\n    \
         22..23 Int(2, Dec)\n    \
         23..24 RParen\n"
    );
}

#[test]
fn nested_blocks_inside_interpolation() {
    let dump = dump_string_token("\"nested {if flag { 1 } else { 2 }}\"");
    assert_eq!(
        dump,
        "Str@0..35\n  \
         Text@1..8 \"nested \"\n  \
         Interp open@8..9 close@33..34\n    \
         9..11 If\n    \
         12..16 Ident(flag)\n    \
         17..18 LBrace\n    \
         19..20 Int(1, Dec)\n    \
         21..22 RBrace\n    \
         23..27 Else\n    \
         28..29 LBrace\n    \
         30..31 Int(2, Dec)\n    \
         32..33 RBrace\n"
    );
}

#[test]
fn empty_interpolation_is_structural_without_lexer_diagnostics() {
    // The parser owns the empty-interpolation diagnostic.
    let (_, output) = lex("\"{}\"");
    assert!(output.diagnostics.is_empty());
    let dump = dump_string_token("\"{}\"");
    assert_eq!(dump, "Str@0..4\n  Interp open@1..2 close@2..3\n");
}

#[test]
fn nested_string_inside_interpolation() {
    let dump = dump_string_token("\"a { \"b {c} d\" } e\"");
    assert_eq!(
        dump,
        "Str@0..19\n  \
         Text@1..3 \"a \"\n  \
         Interp open@3..4 close@15..16\n    \
         nested Str@5..14\n      \
         Text@6..8 \"b \"\n      \
         Interp open@8..9 close@10..11\n        \
         9..10 Ident(c)\n      \
         Text@11..13 \" d\"\n  \
         Text@16..18 \" e\"\n"
    );
}

#[test]
fn comment_inside_interpolation_does_not_close_it() {
    let dump = dump_string_token("\"{ f() /* } */ }\"");
    assert_eq!(
        dump,
        "Str@0..17\n  \
         Interp open@1..2 close@15..16\n    \
         3..4 Ident(f)\n    \
         4..5 LParen\n    \
         5..6 RParen\n"
    );
}

#[test]
fn unterminated_string_at_line_end() {
    let (_, output) = lex("\"abc\nnext");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(output.tokens[0].span.lo, 0);
    assert_eq!(output.tokens[0].span.hi, 4);
    assert_eq!(
        output.diagnostics[0].message(),
        "unterminated string literal"
    );
    // Lexing resumes at the newline; `next` is a fresh identifier.
    assert_eq!(output.tokens[1].kind, TokenKind::Ident("next".to_string()));
}

#[test]
fn unterminated_string_at_eof() {
    let (_, output) = lex("\"abc");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "unterminated string literal"
    );
}

#[test]
fn source_ending_immediately_after_escape() {
    let (_, output) = lex("\"abc\\");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "unterminated string literal"
    );
}

#[test]
fn unterminated_interpolation_at_eof_keeps_the_string_token() {
    let (_, output) = lex("\"abc {foo");
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "unterminated interpolation in string literal"
    );
    let dump = dump_string_token("\"abc {foo");
    assert_eq!(
        dump,
        "Str@0..9\n  \
         Text@1..5 \"abc \"\n  \
         Interp open@5..6 close@9..9\n    \
         6..9 Ident(foo)\n    \
         9..9 Error\n"
    );
}

#[test]
fn unterminated_inner_string_and_outer_interpolation() {
    // No closing quote anywhere: the inner string runs to EOF, and the
    // outer interpolation is likewise unterminated.
    let (_, output) = lex("\"{ \"abc }");
    let messages: Vec<&str> = output.diagnostics.iter().map(|d| d.message()).collect();
    assert_eq!(
        messages,
        [
            "unterminated string literal",
            "unterminated interpolation in string literal",
        ]
    );
}

#[test]
fn inner_string_consumes_brace_until_its_own_close() {
    // `"{ "abc }"`: the `}` is inside the inner string; the inner string
    // closes at the final quote, so only the outer interpolation is
    // unterminated at EOF. (Erratum recorded for lexer-contract §5.3.)
    let (_, output) = lex("\"{ \"abc }\"");
    let messages: Vec<&str> = output.diagnostics.iter().map(|d| d.message()).collect();
    assert_eq!(messages, ["unterminated interpolation in string literal"]);
}

#[test]
fn raw_newline_inside_interpolation_terminates_the_string() {
    let (_, output) = lex("\"{ a\n}\"");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(
        output.diagnostics[0].message(),
        "unterminated string literal"
    );
}

#[test]
fn interpolations_can_contain_question_and_cast_tokens() {
    assert_eq!(
        kinds("\"{ s.to_int()? as i64 }\""),
        [
            TokenKind::Str(xl_lexer::StrLit {
                parts: vec![xl_lexer::StrPart::Interp(xl_lexer::Interpolation {
                    open: xl_span::Span {
                        file: xl_span::FileId::from_index(0),
                        lo: 1,
                        hi: 2,
                    },
                    tokens: vec![
                        tok(TokenKind::Ident("s".into()), 3, 4),
                        tok(TokenKind::Dot, 4, 5),
                        tok(TokenKind::Ident("to_int".into()), 5, 11),
                        tok(TokenKind::LParen, 11, 12),
                        tok(TokenKind::RParen, 12, 13),
                        tok(TokenKind::Question, 13, 14),
                        tok(TokenKind::As, 15, 17),
                        tok(TokenKind::Ident("i64".into()), 18, 21),
                    ],
                    close: xl_span::Span {
                        file: xl_span::FileId::from_index(0),
                        lo: 22,
                        hi: 23,
                    },
                })],
            }),
            TokenKind::Eof,
        ]
    );
}

fn tok(kind: TokenKind, lo: u32, hi: u32) -> xl_lexer::Token {
    xl_lexer::Token {
        kind,
        span: xl_span::Span {
            file: xl_span::FileId::from_index(0),
            lo,
            hi,
        },
    }
}
