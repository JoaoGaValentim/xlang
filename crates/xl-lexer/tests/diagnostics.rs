//! Suite G — diagnostics: stable messages, spans, ordering, and rendering.

mod common;

use common::{dump_diagnostics, lex};
use xl_lexer::TokenKind;

#[test]
fn illegal_character_renders_with_location() {
    let rendered = dump_diagnostics("~");
    assert_eq!(
        rendered,
        "error: unexpected character '~'\n --> test.x:1:1\n  |\n1 | ~\n  | ^\n"
    );
}

#[test]
fn misplaced_underscore_renders_at_the_offending_character() {
    let rendered = dump_diagnostics("let x = 1_;");
    assert_eq!(
        rendered,
        "error: '_' must appear between digits\n --> test.x:1:10\n  |\n1 | let x = 1_;\n  |          ^\n"
    );
}

#[test]
fn unterminated_string_reports_at_the_opening_quote() {
    let rendered = dump_diagnostics("\"abc");
    assert_eq!(
        rendered,
        "error: unterminated string literal\n --> test.x:1:1\n  |\n1 | \"abc\n  | ^^^^\n"
    );
}

#[test]
fn multiple_errors_are_reported_in_scan_order() {
    let (_, output) = lex("~ @ ~");
    assert_eq!(output.diagnostics.len(), 3);
    let messages: Vec<&str> = output.diagnostics.iter().map(|d| d.message()).collect();
    assert_eq!(
        messages,
        [
            "unexpected character '~'",
            "unexpected character '@'",
            "unexpected character '~'",
        ]
    );
    let spans: Vec<(u32, u32)> = output
        .diagnostics
        .iter()
        .map(|d| {
            let span = d.primary().span;
            (span.lo, span.hi)
        })
        .collect();
    assert_eq!(spans, vec![(0, 1), (2, 3), (4, 5)]);
}

#[test]
fn diagnostics_use_the_originating_file_id() {
    let (_, output) = lex("~");
    assert_eq!(
        output.diagnostics[0].primary().span.file,
        xl_span::FileId::from_index(0)
    );
}

#[test]
fn error_tokens_accompany_every_lexical_diagnostic() {
    for source in [
        "~",
        "\"abc",
        "0x",
        "1_",
        "18446744073709551616",
        "1e9999",
        "'ab'",
    ] {
        let (_, output) = lex(source);
        assert!(
            !output.diagnostics.is_empty(),
            "source {source:?} must produce a diagnostic"
        );
        assert!(
            output.tokens.iter().any(|t| t.kind == TokenKind::Error),
            "source {source:?} must produce an Error token"
        );
    }
}

#[test]
fn unterminated_block_comment_diagnostic_has_no_error_token() {
    // Contract: resume at EOF, diagnostic only.
    let (_, output) = lex("/* open");
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.tokens.iter().all(|t| t.kind != TokenKind::Error));
    assert_eq!(output.tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn unicode_escape_error_rendering() {
    let rendered = dump_diagnostics("\"\\u{}\"");
    assert_eq!(
        rendered,
        "error: invalid unicode escape sequence\n --> test.x:1:2\n  |\n1 | \"\\u{}\"\n  |  ^^^^\n"
    );
}
