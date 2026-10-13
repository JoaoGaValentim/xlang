#![no_main]

//! Fuzz target for the XLANG lexer.
//!
//! Raw bytes are converted safely to `&str` (the lexer API operates on
//! validated Rust strings); invalid UTF-8 input is skipped. The target
//! asserts the core contract: termination, a single final `Eof` token, and
//! UTF-8-boundary-valid spans.

use libfuzzer_sys::fuzz_target;
use xl_lexer::TokenKind;
use xl_span::SourceMap;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut sources = SourceMap::new();
    let Ok(file) = sources.add_file("fuzz.x", text) else {
        return;
    };
    let Ok(output) = xl_lexer::lex_file(&sources, file) else {
        return;
    };

    let last = output.tokens.last().expect("stream ends with Eof");
    assert!(matches!(last.kind, TokenKind::Eof));
    assert_eq!(
        output
            .tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Eof))
            .count(),
        1
    );

    let len = text.len() as u32;
    for token in &output.tokens {
        assert!(token.span.lo <= token.span.hi);
        assert!(token.span.hi <= len);
        assert!(text.is_char_boundary(token.span.lo as usize));
        assert!(text.is_char_boundary(token.span.hi as usize));
    }
});
