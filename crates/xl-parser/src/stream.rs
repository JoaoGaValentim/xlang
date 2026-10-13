//! Token cursor over a lexer token stream.
//!
//! The cursor owns a copy of the token list so that the generic-closer
//! split (accepted contract `parser-ast-contract.md` §3(3), packet G-3) can
//! rewrite one `>=` token into its `>` + `=` halves without touching the
//! caller's data. The rewrite is lossless: the synthesized `=` keeps the
//! exact residual byte range of the original token, and no token is dropped
//! or duplicated.

use xl_lexer::{Token, TokenKind};
use xl_span::{FileId, Span};

/// Discriminant-only description of a token kind, used by lookahead and
/// synchronization sets without constructing payload-carrying values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KindTag {
    /// Identifier.
    Ident,
    /// `fn`
    Fn,
    /// `struct`
    Struct,
    /// `enum`
    Enum,
    /// `impl`
    Impl,
    /// `trait`
    Trait,
    /// `import`
    Import,
    /// `pub`
    Pub,
    /// `let`
    Let,
    /// `mut`
    Mut,
    /// `if`
    If,
    /// `else`
    Else,
    /// `in`
    In,
    /// `for`
    For,
    /// `as`
    As,
    /// `_`
    Underscore,
    /// `;`
    Semi,
    /// `}`
    RBrace,
    /// `)`
    RParen,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `(`
    LParen,
    /// `[`
    LBracket,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `::`
    ColonColon,
    /// `.`
    Dot,
    /// `?`
    Question,
    /// `=>`
    FatArrow,
    /// `->`
    Arrow,
    /// `=`
    Eq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `Error` recovery token.
    Error,
    /// `Eof`
    Eof,
    /// Any other token category (literals, remaining operators/keywords);
    /// used only as a catch-all in category conversion, never matched by
    /// synchronization sets.
    Other,
}

impl KindTag {
    /// Returns `true` when `kind` has this tag's category.
    pub(crate) fn matches(self, kind: &TokenKind) -> bool {
        use KindTag as T;
        matches!(
            (self, kind),
            (T::Ident, TokenKind::Ident(_))
                | (T::Fn, TokenKind::Fn)
                | (T::Struct, TokenKind::Struct)
                | (T::Enum, TokenKind::Enum)
                | (T::Impl, TokenKind::Impl)
                | (T::Trait, TokenKind::Trait)
                | (T::Import, TokenKind::Import)
                | (T::Pub, TokenKind::Pub)
                | (T::Let, TokenKind::Let)
                | (T::Mut, TokenKind::Mut)
                | (T::If, TokenKind::If)
                | (T::Else, TokenKind::Else)
                | (T::In, TokenKind::In)
                | (T::For, TokenKind::For)
                | (T::As, TokenKind::As)
                | (T::Underscore, TokenKind::Underscore)
                | (T::Semi, TokenKind::Semi)
                | (T::RBrace, TokenKind::RBrace)
                | (T::RParen, TokenKind::RParen)
                | (T::RBracket, TokenKind::RBracket)
                | (T::LBrace, TokenKind::LBrace)
                | (T::LParen, TokenKind::LParen)
                | (T::LBracket, TokenKind::LBracket)
                | (T::Comma, TokenKind::Comma)
                | (T::Colon, TokenKind::Colon)
                | (T::ColonColon, TokenKind::ColonColon)
                | (T::Dot, TokenKind::Dot)
                | (T::Question, TokenKind::Question)
                | (T::FatArrow, TokenKind::FatArrow)
                | (T::Arrow, TokenKind::Arrow)
                | (T::Eq, TokenKind::Eq)
                | (T::Lt, TokenKind::Lt)
                | (T::Gt, TokenKind::Gt)
                | (T::Ge, TokenKind::Ge)
                | (T::Error, TokenKind::Error)
                | (T::Eof, TokenKind::Eof)
        )
    }
}

/// A cursor over a token stream.
///
/// The top-level stream always ends with a single `Eof` token
/// (`lexer-contract.md` §9); embedded interpolation streams contain no `Eof`
/// and are simply exhausted. [`Stream::position`] returns a valid insertion
/// point in both cases (the current token's start, or the configured
/// end-of-stream offset).
pub(crate) struct Stream {
    tokens: Vec<Token>,
    pos: usize,
    /// Offset reported when the stream is exhausted.
    end_pos: u32,
    /// File identifier reported when the stream is empty.
    file: FileId,
}

impl Stream {
    /// Creates a cursor over an owned token list.
    pub(crate) fn new(tokens: Vec<Token>, file: FileId) -> Self {
        let end_pos = tokens.last().map_or(0, |token| token.span.hi);
        Self {
            tokens,
            pos: 0,
            end_pos,
            file,
        }
    }

    /// Replaces this stream with one over `tokens`, returning the old stream.
    ///
    /// Used for interpolation interiors: the embedded stream has no `Eof`
    /// token and reports `end_pos` past its last token.
    pub(crate) fn replace(&mut self, tokens: Vec<Token>, end_pos: u32) -> Stream {
        let file = self.file;
        let mut embedded = Stream::new(tokens, file);
        embedded.end_pos = end_pos;
        std::mem::replace(self, embedded)
    }

    /// Returns the current token, or `None` when the stream is exhausted.
    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Returns the token `n` positions ahead of the cursor.
    pub(crate) fn peek_n(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }

    /// Returns `true` when no token is available or the current token is the
    /// top-level `Eof`.
    pub(crate) fn at_eof(&self) -> bool {
        match self.peek() {
            None => true,
            Some(_) => self.check_tag(KindTag::Eof),
        }
    }

    /// Returns `true` when the current token has `tag`'s category.
    pub(crate) fn check_tag(&self, tag: KindTag) -> bool {
        self.peek().is_some_and(|token| tag.matches(&token.kind))
    }

    /// Consumes and returns the current token.
    pub(crate) fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    /// Consumes the current token when it has `tag`'s category.
    pub(crate) fn eat_tag(&mut self, tag: KindTag) -> Option<Token> {
        if self.check_tag(tag) {
            self.bump()
        } else {
            None
        }
    }

    /// Consumes the current token when it has `tag`'s category, without
    /// reporting anything on failure (silent recovery helper).
    pub(crate) fn bump_if_tag(&mut self, tag: KindTag) -> Option<Token> {
        self.eat_tag(tag)
    }

    /// Returns the offset used for diagnostics at the current position.
    pub(crate) fn position(&self) -> u32 {
        match self.peek() {
            Some(token) => token.span.lo,
            None => self.end_pos,
        }
    }

    /// Returns the span used for a diagnostic at the current position.
    ///
    /// At the end of a stream this is a zero-width span at the configured
    /// end offset.
    pub(crate) fn current_span(&self) -> Span {
        match self.peek() {
            Some(token) => token.span,
            None => Span {
                file: self.file,
                lo: self.end_pos,
                hi: self.end_pos,
            },
        }
    }

    /// Returns the cursor position for progress tracking.
    pub(crate) fn mark(&self) -> usize {
        self.pos
    }

    /// Returns `true` when the cursor moved since `mark` was taken.
    pub(crate) fn progressed_since(&self, mark: usize) -> bool {
        self.pos != mark
    }

    /// Splits the current `>=` token into its `>` and `=` halves.
    ///
    /// The `>` half is consumed (its exact span `[lo, lo + 1)` is returned);
    /// the `=` half replaces the current token with the exact residual span
    /// `[lo + 1, hi)`. Only called in type-closing context (G-3); expressions
    /// never reach this path, so `a >= b` keeps its single `Ge` token.
    ///
    /// Returns `None` when the current token is not `Ge`.
    pub(crate) fn split_ge(&mut self) -> Option<Span> {
        let token = self.tokens.get(self.pos)?;
        if !KindTag::Ge.matches(&token.kind) {
            return None;
        }
        let span = token.span;
        // `>=` is two ASCII bytes; both halves are exact byte ranges.
        let gt = Span {
            file: span.file,
            lo: span.lo,
            hi: span.lo + 1,
        };
        self.tokens[self.pos] = Token {
            kind: TokenKind::Eq,
            span: Span {
                file: span.file,
                lo: span.lo + 1,
                hi: span.hi,
            },
        };
        Some(gt)
    }
}
