#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Syntactic parsing for the XLANG compiler: tokens to `xl-ast` items.
//!
//! This crate implements the parser component of Phase 1 as established by
//! the accepted parser/AST contract
//! (`docs/proposals/phase-1/parser-ast-contract.md`, ratified by Directive
//! 006; ADR-0004) under Engineering Directive 008. It consumes the
//! `xl-lexer` token model, preserves lexical diagnostics, emits syntactic
//! diagnostics, and constructs existing `xl-ast` nodes with exact source
//! spans and deterministic instance-scoped identifiers.
//!
//! # Pipeline
//!
//! ```text
//! SourceFile + FileId → xl_lexer::lex → tokens → xl_parser → items
//! ```
//!
//! [`parse`] runs the whole pipeline for one source file; [`parse_tokens`]
//! parses an existing token stream (lexical diagnostics are then the
//! caller's responsibility) and is the testing/integration entry point.
//!
//! # Contract
//!
//! - Parsing is a deterministic, total function of the token stream: the
//!   same input always yields the same items and diagnostics.
//! - It **never panics** on malformed input and always terminates: every
//!   recovery cycle consumes input or returns to a caller that does.
//! - Recursion is bounded by [`MAX_PARSE_DEPTH`] across all mutually
//!   recursive paths (expressions, types, patterns, blocks, items,
//!   generics, interpolations); exceeding the limit produces a controlled
//!   diagnostic and bounded recovery (amendment B-4).
//! - All spans are exact UTF-8 byte ranges from the consumed tokens
//!   (ADR-0002); recovered nodes use zero-width spans at the deterministic
//!   insertion point and never masquerade as valid constructs.
//! - No name resolution, typing, or desugaring happens here.
//!
//! # Diagnostics
//!
//! [`ParseOutput::diagnostics`] contains, in order, the lexical
//! diagnostics from `xl-lexer` (scan order) followed by the syntactic
//! diagnostics (parse order); the ordering is deterministic. Error-level
//! diagnostics are the authority on parse failure: do not use the canonical
//! printer to certify input that has errors.
//!
//! # Dependencies
//!
//! `xl-parser → xl-lexer, xl-ast, xl-span, xl-diag` (ADR-0001 extended);
//! the parser never depends on the CLI or later compiler stages.

mod expr;
mod messages;
mod parser;
mod stream;
mod types;

use xl_ast::Item;
use xl_diag::Diagnostic;
use xl_lexer::Token;
use xl_span::{FileId, SourceFile, SourceMap, SpanError};

/// Maximum supported nesting depth for all recursive parse paths.
///
/// The limit is shared across expressions, types, patterns, blocks, nested
/// items, generic arguments, parenthesized constructs, and interpolated
/// expressions; it is an implementation-safety policy (amendment B-4), not
/// language semantics. Exceeding it yields the diagnostic
/// `expression nesting limit (64) exceeded` at the offending token, skips
/// to the expression synchronization set, and continues parsing.
pub const MAX_PARSE_DEPTH: u32 = 64;

/// The result of parsing one source file.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutput {
    /// The parsed items in source order.
    pub items: Vec<Item>,
    /// Lexical diagnostics (scan order) followed by syntactic diagnostics
    /// (parse order).
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseOutput {
    /// Returns `true` when any diagnostic has error severity.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == xl_diag::Severity::Error)
    }

    /// Returns the number of error-severity diagnostics.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity() == xl_diag::Severity::Error)
            .count()
    }
}

/// Lexes and parses `source` (registered as `file`).
///
/// `file` must be the identifier under which `source` is registered in the
/// caller's [`SourceMap`]; the parser only uses it to build spans, matching
/// `xl-span`'s map-scoping contract.
#[must_use]
pub fn parse(source: &SourceFile, file: FileId) -> ParseOutput {
    let lexed = xl_lexer::lex(source, file);
    let mut output = parse_tokens(&lexed.tokens, file);
    let mut diagnostics = lexed.diagnostics;
    diagnostics.append(&mut output.diagnostics);
    output.diagnostics = diagnostics;
    output
}

/// Looks `file` up in `sources`, then lexes and parses it.
///
/// # Errors
///
/// Returns [`SpanError::UnknownFile`] if `file` is not registered in
/// `sources`.
pub fn parse_file(sources: &SourceMap, file: FileId) -> Result<ParseOutput, SpanError> {
    let source = sources.file(file)?;
    Ok(parse(source, file))
}

/// Parses an existing token stream into items and syntactic diagnostics.
///
/// The lexical diagnostics that accompanied the tokens are **not** part of
/// the returned output; callers that lexed the tokens themselves remain
/// responsible for reporting those (ordering contract: lexical first, then
/// syntactic).
#[must_use]
pub fn parse_tokens(tokens: &[Token], file: FileId) -> ParseOutput {
    let mut parser = parser::Parser::new(tokens.to_vec(), file);
    let items = parser.parse_items();
    ParseOutput {
        items,
        diagnostics: parser.diagnostics,
    }
}
