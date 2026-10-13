#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Lexical analysis for the XLANG compiler: source text to tokens.
//!
//! This crate implements the accepted lexical contract
//! (`docs/proposals/phase-1/lexer-contract.md`, ratified by Directive 006;
//! ADR-0004). It depends only on `xl-span` (files, spans, positions) and
//! `xl-diag` (diagnostics); it never depends on the parser or any later
//! compiler stage, and it never performs name resolution, typing, or
//! desugaring.
//!
//! # Contract
//!
//! - [`lex`] is a deterministic, total function of the source text: the
//!   same input always yields the same token stream and diagnostics.
//! - It **never panics** on malformed input, always makes progress, and
//!   emits exactly one [`TokenKind::Eof`] token, zero-width, at the end of
//!   the top-level stream.
//! - Whitespace and comments are trivia and produce no tokens.
//! - Malformed input produces [`TokenKind::Error`] tokens plus structured
//!   [`Diagnostic`]s; the caller decides whether and how to render them.
//! - All spans are UTF-8 byte offsets on character boundaries (ADR-0002).
//!
//! # Structured strings
//!
//! A string literal is a single [`TokenKind::Str`] token. Its payload
//! partitions the literal interior into decoded text segments and
//! interpolations (`lexer-contract.md` §5.5):
//!
//! - text-segment spans cover raw source bytes (escapes included);
//! - interpolation delimiter spans (`open`/`close`) are accounted
//!   separately, and embedded tokens exclude the braces and trivia;
//! - embedded token streams contain no `Eof` token; if EOF interrupts an
//!   interpolation, the parent records the `close` span as zero-width at
//!   EOF, appends one zero-width `Error` marker to the embedded tokens, and
//!   reports `unterminated interpolation in string literal`.
//!
//! # Resource safety
//!
//! Interpolation nesting is bounded by [`MAX_INTERPOLATION_DEPTH`]; exceeding
//! it produces a controlled diagnostic and a raw recovery scan (balanced
//! braces, skipping nested strings and comments) — never a stack overflow.
//! Comment nesting is counted iteratively. All scanning is linear in the
//! consumed input.
//!
//! # Dependencies
//!
//! - Runtime: [`unicode_ident`] supplies standards-compliant
//!   `XID_Start`/`XID_Continue` classification (UAX #31); the crate is tiny,
//!   dependency-free, and holds no unsafe code this crate relies on.
//! - Dev-only: `insta` (snapshots; recommended by the specification §9) and
//!   `proptest` (determinism/span properties). They add no runtime weight.
//!
//! # Symbol storage
//!
//! Identifiers and decoded text carry owned `String` payloads; the lexer
//! performs **no** global interning and requires no shared mutable state
//! (Directive 006 §5). Tokens own their data, so the parser never has to
//! borrow from lexer memory; a session-level interner can replace the
//! payload representation later behind the same observable contract.

mod lexer;
mod messages;
mod token;

pub use token::{IntLit, Interpolation, Radix, StrLit, StrPart, TextSegment, Token, TokenKind};

use xl_diag::Diagnostic;
use xl_span::{FileId, SourceFile, SourceMap, SpanError};

/// Maximum supported interpolation nesting depth inside string literals.
///
/// Exceeding this limit yields a recovered `Error` token and the diagnostic
/// `interpolation nesting limit (64) exceeded`; the constant is an
/// implementation-safety policy, not language semantics (amendment B-4).
pub const MAX_INTERPOLATION_DEPTH: u32 = 64;

/// The result of lexing one source file.
#[derive(Debug, Clone, PartialEq)]
pub struct LexOutput {
    /// The token stream; the final element is always the `Eof` token.
    pub tokens: Vec<Token>,
    /// Lexical diagnostics in scan (event) order, one per error occurrence.
    pub diagnostics: Vec<Diagnostic>,
}

/// Lexes `source` (registered as `file`) into tokens and diagnostics.
///
/// `file` must be the identifier under which `source` is registered in the
/// caller's [`SourceMap`]; the lexer only uses it to build spans (it never
/// touches the map, matching `xl-span`'s map-scoping contract).
#[must_use]
pub fn lex(source: &SourceFile, file: FileId) -> LexOutput {
    lexer::run(source.source(), file)
}

/// Looks `file` up in `sources` and lexes it.
///
/// # Errors
///
/// Returns [`SpanError::UnknownFile`] if `file` is not registered in
/// `sources`.
pub fn lex_file(sources: &SourceMap, file: FileId) -> Result<LexOutput, SpanError> {
    let source = sources.file(file)?;
    Ok(lex(source, file))
}
