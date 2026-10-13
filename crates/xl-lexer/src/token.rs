//! Token data model for the XLANG lexer.

use xl_span::Span;

/// Radix of an integer literal, preserved for diagnostics and printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radix {
    /// Decimal (`0`, `1_000`).
    Dec,
    /// Hexadecimal (`0xFF`).
    Hex,
    /// Binary (`0b1010`).
    Bin,
}

impl Radix {
    /// Returns the numeric base (2, 10, or 16).
    #[must_use]
    pub const fn base(self) -> u64 {
        match self {
            Self::Dec => 10,
            Self::Hex => 16,
            Self::Bin => 2,
        }
    }

    /// Returns the lowercase prefix used by the canonical printer.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Dec => "",
            Self::Hex => "0x",
            Self::Bin => "0b",
        }
    }
}

/// Integer literal payload: magnitude plus radix (amendment B-2).
///
/// The lexer never applies signed-range restrictions; magnitude is bounded
/// only by [`u64::MAX`] (see the crate documentation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntLit {
    /// The literal's magnitude, with `_` separators removed.
    pub magnitude: u64,
    /// The radix the literal was written in.
    pub radix: Radix,
}

/// A decoded literal-text segment inside a string literal.
///
/// `text` holds the decoded value (escapes resolved); `span` covers the raw
/// source bytes of the segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSegment {
    /// Decoded text.
    pub text: String,
    /// Raw source span of the segment (escape sequences included).
    pub span: Span,
}

/// An interpolated expression segment: `{ tokens }`.
///
/// The delimiter spans are accounted separately from the embedded tokens
/// (`Interpolation::open` covers `{`, `Interpolation::close` covers `}`).
/// Embedded tokens exclude the delimiters and trivia (`lexer-contract.md`
/// §5.5).
#[derive(Debug, Clone, PartialEq)]
pub struct Interpolation {
    /// Span of the opening `{`.
    pub open: Span,
    /// Embedded expression tokens, delimiters and trivia excluded.
    pub tokens: Vec<Token>,
    /// Span of the closing `}`; zero-width at EOF when unterminated.
    pub close: Span,
}

/// A part of a structured string literal.
#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    /// Decoded literal text.
    Text(TextSegment),
    /// An interpolated expression.
    Interp(Interpolation),
}

/// Structured string-literal payload: ordered text and interpolation parts.
#[derive(Debug, Clone, PartialEq)]
pub struct StrLit {
    /// The ordered parts of the literal (empty for `""`).
    pub parts: Vec<StrPart>,
}

/// The lexical category and payload of a token.
///
/// `Str` carries nested tokens (the interpolation design); f64 payloads in
/// `Float` prevent a total `Eq` derivation, so only `PartialEq` is derived.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// An identifier that is not a keyword (Unicode XID; `self` is an
    /// identifier by contract).
    Ident(String),
    /// An integer literal (magnitude + radix).
    Int(IntLit),
    /// A finite floating-point literal.
    Float(f64),
    /// A character literal.
    Char(char),
    /// A structured string literal.
    Str(StrLit),

    /// `fn`
    Fn,
    /// `let`
    Let,
    /// `mut`
    Mut,
    /// `if`
    If,
    /// `else`
    Else,
    /// `match`
    Match,
    /// `while`
    While,
    /// `for`
    For,
    /// `in`
    In,
    /// `loop`
    Loop,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `return`
    Return,
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
    /// `as`
    As,
    /// `true`
    True,
    /// `false`
    False,

    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `%=`
    PercentEq,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `?`
    Question,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `::`
    ColonColon,

    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `;`
    Semi,

    /// `_` (wildcard pattern token; not an expression).
    Underscore,
    /// Recovery token for malformed input; its span covers the offending
    /// source text and its diagnostic is reported separately.
    Error,
    /// End of input (top-level stream only; zero-width span at EOF).
    Eof,
}

/// A token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The token category and payload.
    pub kind: TokenKind,
    /// The token's source span (`[lo, hi)`, UTF-8 bytes).
    pub span: Span,
}
