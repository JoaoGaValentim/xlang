//! Shared helpers for the `xl-lexer` integration test suites.
//!
//! Each integration-test binary compiles this module independently, so not
//! every helper is used by every suite.
#![allow(dead_code)]

use xl_lexer::{LexOutput, TokenKind};
use xl_span::SourceMap;

/// Lexes `src` as `test.x`; returns the map (for rendering) and the output.
pub fn lex(src: &str) -> (SourceMap, LexOutput) {
    let mut sources = SourceMap::new();
    let file = sources
        .add_file("test.x", src)
        .expect("registration succeeds");
    let output = xl_lexer::lex_file(&sources, file).expect("file is registered");
    (sources, output)
}

/// Returns all token kinds, including the final `Eof`.
pub fn kinds(src: &str) -> Vec<TokenKind> {
    let (_, output) = lex(src);
    output.tokens.into_iter().map(|t| t.kind).collect()
}

/// Stable textual dump of the token stream: `lo..hi Kind` per token.
pub fn dump_tokens(src: &str) -> String {
    let (_, output) = lex(src);
    let mut lines = Vec::new();
    for token in &output.tokens {
        lines.push(format!(
            "{:>4}..{:<4} {}",
            token.span.lo,
            token.span.hi,
            describe(&token.kind)
        ));
    }
    lines.join("\n")
}

/// Stable one-line description of a token kind.
pub fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(text) => format!("Ident({text})"),
        TokenKind::Int(int) => format!("Int({}, {:?})", int.magnitude, int.radix),
        TokenKind::Float(value) => format!("Float({value:?})"),
        TokenKind::Char(value) => format!("Char({value:?})"),
        TokenKind::Str(lit) => format!("Str({} part(s))", lit.parts.len()),
        other => format!("{other:?}"),
    }
}

/// Renders every diagnostic against the source map, one block per
/// diagnostic, joined with `---` separators.
pub fn dump_diagnostics(src: &str) -> String {
    let (sources, output) = lex(src);
    let mut rendered = Vec::new();
    for diagnostic in &output.diagnostics {
        rendered.push(xl_diag::render(diagnostic, &sources));
    }
    rendered.join("---\n")
}

/// Detailed structured dump of the first `Str` token: parts, decoded text,
/// delimiter and segment spans, and embedded token descriptions.
pub fn dump_string_token(src: &str) -> String {
    let (_, output) = lex(src);
    let token = output
        .tokens
        .iter()
        .find(|t| matches!(t.kind, TokenKind::Str(_)))
        .expect("source must contain a string token");
    let TokenKind::Str(lit) = &token.kind else {
        unreachable!()
    };
    let mut out = String::new();
    out.push_str(&format!("Str@{}..{}\n", token.span.lo, token.span.hi));
    dump_parts(lit, 1, &mut out);
    out
}

fn dump_parts(lit: &xl_lexer::StrLit, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    for part in &lit.parts {
        match part {
            xl_lexer::StrPart::Text(text) => {
                out.push_str(&format!(
                    "{pad}Text@{}..{} {:?}\n",
                    text.span.lo, text.span.hi, text.text
                ));
            }
            xl_lexer::StrPart::Interp(interp) => {
                out.push_str(&format!(
                    "{pad}Interp open@{}..{} close@{}..{}\n",
                    interp.open.lo, interp.open.hi, interp.close.lo, interp.close.hi
                ));
                for token in &interp.tokens {
                    if let TokenKind::Str(inner) = &token.kind {
                        out.push_str(&format!(
                            "{pad}  nested Str@{}..{}\n",
                            token.span.lo, token.span.hi
                        ));
                        dump_parts(inner, indent + 2, out);
                    } else {
                        out.push_str(&format!(
                            "{pad}  {}..{} {}\n",
                            token.span.lo,
                            token.span.hi,
                            describe(&token.kind)
                        ));
                    }
                }
            }
        }
    }
}
