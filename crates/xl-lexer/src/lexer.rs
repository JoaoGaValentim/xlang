//! The XLANG lexical scanner.
//!
//! The scanner is a single pass over the source text. It never panics on
//! malformed input, always makes progress, and emits exactly one `Eof`
//! token at the end of the top-level stream. Structured string literals
//! contain recursively lexed embedded streams; nesting is bounded by
//! [`crate::MAX_INTERPOLATION_DEPTH`] so stack use is independent of input
//! length.

use unicode_ident::{is_xid_continue, is_xid_start};
use xl_diag::Diagnostic;
use xl_span::{FileId, Span};

use crate::LexOutput;
use crate::messages;
use crate::token::{IntLit, Interpolation, Radix, StrLit, StrPart, TextSegment, Token, TokenKind};

/// Outcome of skipping trivia in embedded (in-string) mode.
enum Trivia {
    /// Trivia skipped; lexing can continue.
    Ok,
    /// A raw CR/LF was reached at `pos` (not consumed).
    Newline(usize),
    /// End of input was reached.
    Eof,
}

/// Termination of an embedded expression token stream.
enum EmbeddedEnd {
    /// The closing `}` was consumed; carries its span bounds.
    Close(usize, usize),
    /// A raw CR/LF terminated the enclosing string at this position.
    Newline(usize),
    /// End of input before the closing `}`.
    Eof,
}

/// Outcome of scanning an escape sequence.
enum EscapeOutcome {
    /// A valid escape decoded to this character.
    Char(char),
    /// Unknown escape; carries the character to take literally.
    Unknown(char),
    /// Malformed `\u{...}` escape (already recovered as far as defined).
    InvalidUnicode,
    /// The escape ran into a raw newline or EOF.
    Unterminated,
}

/// Outcome of the raw recovery scan for an over-limit interpolation.
enum LimitRecovery {
    /// A balancing `}` was consumed at this position (one past its end).
    Close(usize),
    /// A raw CR/LF was reached at this position (not consumed).
    Newline(usize),
    /// End of input was reached.
    Eof,
}

/// The scanner state for one file.
struct Lexer<'a> {
    file: FileId,
    src: &'a str,
    pos: usize,
    depth: u32,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

/// Lexes `src` (belonging to `file`) and returns the token stream with
/// diagnostics. See [`crate::lex`] for the public contract.
pub(crate) fn run(src: &str, file: FileId) -> LexOutput {
    let mut lexer = Lexer {
        file,
        src,
        pos: 0,
        depth: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
    };
    lexer.lex_top_level();
    LexOutput {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    /// Consumes and returns the next character.
    ///
    /// Panics only on internal misuse (called at end of input); every call
    /// site is guarded by an explicit `peek` check.
    fn bump(&mut self) -> char {
        let c = self.peek().expect("bump called at end of input");
        self.pos += c.len_utf8();
        c
    }

    fn span(&self, lo: usize, hi: usize) -> Span {
        debug_assert!(lo <= hi);
        debug_assert!(hi <= self.src.len());
        debug_assert!(self.src.is_char_boundary(lo));
        debug_assert!(self.src.is_char_boundary(hi));
        Span {
            file: self.file,
            lo: lo as u32,
            hi: hi as u32,
        }
    }

    fn error(&mut self, message: String, lo: usize, hi: usize) {
        self.diagnostics
            .push(Diagnostic::error(message, self.span(lo, hi)));
    }

    fn finish(&mut self, kind: TokenKind, lo: usize) -> Token {
        Token {
            kind,
            span: self.span(lo, self.pos),
        }
    }

    // ------------------------------------------------------------------
    // Top level
    // ------------------------------------------------------------------

    fn lex_top_level(&mut self) {
        if self.src.starts_with('\u{FEFF}') {
            self.pos = '\u{FEFF}'.len_utf8();
        }
        loop {
            let _ = self.skip_trivia(false);
            if self.pos >= self.src.len() {
                let len = self.src.len();
                self.tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: self.span(len, len),
                });
                return;
            }
            let before = self.pos;
            let token = self.lex_one_token();
            self.tokens.push(token);
            debug_assert!(self.pos > before, "lexer must make progress");
        }
    }

    // ------------------------------------------------------------------
    // Trivia (whitespace and comments)
    // ------------------------------------------------------------------

    /// Skips whitespace and comments.
    ///
    /// In embedded mode (`embedded == true`, i.e. inside a string
    /// interpolation), a raw CR/LF is *not* trivia: it terminates the
    /// enclosing single-line string and is reported back to the caller.
    fn skip_trivia(&mut self, embedded: bool) -> Trivia {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') => {
                    self.bump();
                }
                Some('\r') | Some('\n') => {
                    if embedded {
                        return Trivia::Newline(self.pos);
                    }
                    self.bump();
                }
                Some('/') => match self.peek_nth(1) {
                    Some('/') => {
                        self.bump();
                        self.bump();
                        loop {
                            match self.peek() {
                                None => return if embedded { Trivia::Eof } else { Trivia::Ok },
                                Some('\r') | Some('\n') => {
                                    if embedded {
                                        return Trivia::Newline(self.pos);
                                    }
                                    break;
                                }
                                Some(_) => {
                                    self.bump();
                                }
                            }
                        }
                    }
                    Some('*') => {
                        let open = self.pos;
                        self.bump();
                        self.bump();
                        let mut comment_depth: u32 = 1;
                        loop {
                            match self.peek() {
                                None => {
                                    let len = self.src.len();
                                    self.error(messages::unterminated_block_comment(), open, len);
                                    return if embedded { Trivia::Eof } else { Trivia::Ok };
                                }
                                Some('\r') | Some('\n') if embedded => {
                                    return Trivia::Newline(self.pos);
                                }
                                Some('/') if self.peek_nth(1) == Some('*') => {
                                    comment_depth += 1;
                                    self.bump();
                                    self.bump();
                                }
                                Some('*') if self.peek_nth(1) == Some('/') => {
                                    comment_depth -= 1;
                                    self.bump();
                                    self.bump();
                                    if comment_depth == 0 {
                                        break;
                                    }
                                }
                                Some(_) => {
                                    self.bump();
                                }
                            }
                        }
                    }
                    _ => return Trivia::Ok,
                },
                _ => return Trivia::Ok,
            }
        }
    }

    // ------------------------------------------------------------------
    // Single tokens
    // ------------------------------------------------------------------

    fn lex_one_token(&mut self) -> Token {
        let lo = self.pos;
        let c = self.peek().expect("lex_one_token called at end of input");
        match c {
            '0'..='9' => {
                self.bump();
                self.scan_number(lo)
            }
            _ if c == '_' || is_xid_start(c) => {
                self.bump();
                self.scan_ident_or_underscore(lo)
            }
            '"' => {
                self.bump();
                self.scan_string(lo)
            }
            '\'' => {
                self.bump();
                self.scan_char(lo)
            }
            '+' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::PlusEq, TokenKind::Plus)
            }
            '*' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::StarEq, TokenKind::Star)
            }
            '/' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::SlashEq, TokenKind::Slash)
            }
            '%' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::PercentEq, TokenKind::Percent)
            }
            '<' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::Le, TokenKind::Lt)
            }
            '>' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::Ge, TokenKind::Gt)
            }
            '!' => {
                self.bump();
                self.two_char(lo, '=', TokenKind::NotEq, TokenKind::Bang)
            }
            '.' => {
                self.bump();
                self.two_char(lo, '.', TokenKind::DotDot, TokenKind::Dot)
            }
            ':' => {
                self.bump();
                self.two_char(lo, ':', TokenKind::ColonColon, TokenKind::Colon)
            }
            '-' => {
                self.bump();
                match self.peek() {
                    Some('=') => {
                        self.bump();
                        self.finish(TokenKind::MinusEq, lo)
                    }
                    Some('>') => {
                        self.bump();
                        self.finish(TokenKind::Arrow, lo)
                    }
                    _ => self.finish(TokenKind::Minus, lo),
                }
            }
            '=' => {
                self.bump();
                match self.peek() {
                    Some('=') => {
                        self.bump();
                        self.finish(TokenKind::EqEq, lo)
                    }
                    Some('>') => {
                        self.bump();
                        self.finish(TokenKind::FatArrow, lo)
                    }
                    _ => self.finish(TokenKind::Eq, lo),
                }
            }
            '&' => {
                self.bump();
                if self.peek() == Some('&') {
                    self.bump();
                    self.finish(TokenKind::AndAnd, lo)
                } else {
                    self.error(messages::unexpected_character('&'), lo, self.pos);
                    self.finish(TokenKind::Error, lo)
                }
            }
            '|' => {
                self.bump();
                if self.peek() == Some('|') {
                    self.bump();
                    self.finish(TokenKind::OrOr, lo)
                } else {
                    self.error(messages::unexpected_character('|'), lo, self.pos);
                    self.finish(TokenKind::Error, lo)
                }
            }
            '?' => {
                self.bump();
                self.finish(TokenKind::Question, lo)
            }
            ',' => {
                self.bump();
                self.finish(TokenKind::Comma, lo)
            }
            ';' => {
                self.bump();
                self.finish(TokenKind::Semi, lo)
            }
            '(' => {
                self.bump();
                self.finish(TokenKind::LParen, lo)
            }
            ')' => {
                self.bump();
                self.finish(TokenKind::RParen, lo)
            }
            '{' => {
                self.bump();
                self.finish(TokenKind::LBrace, lo)
            }
            '}' => {
                self.bump();
                self.finish(TokenKind::RBrace, lo)
            }
            '[' => {
                self.bump();
                self.finish(TokenKind::LBracket, lo)
            }
            ']' => {
                self.bump();
                self.finish(TokenKind::RBracket, lo)
            }
            _ => {
                self.bump();
                self.error(messages::unexpected_character(c), lo, self.pos);
                self.finish(TokenKind::Error, lo)
            }
        }
    }

    fn two_char(&mut self, lo: usize, second: char, both: TokenKind, single: TokenKind) -> Token {
        if self.peek() == Some(second) {
            self.bump();
            self.finish(both, lo)
        } else {
            self.finish(single, lo)
        }
    }

    // ------------------------------------------------------------------
    // Identifiers, keywords, wildcard
    // ------------------------------------------------------------------

    fn scan_ident_or_underscore(&mut self, lo: usize) -> Token {
        while let Some(c) = self.peek() {
            if is_xid_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        let text = &self.src[lo..self.pos];
        let kind = match text {
            "_" => TokenKind::Underscore,
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "trait" => TokenKind::Trait,
            "import" => TokenKind::Import,
            "pub" => TokenKind::Pub,
            "as" => TokenKind::As,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident(text.to_string()),
        };
        self.finish(kind, lo)
    }

    // ------------------------------------------------------------------
    // Numeric literals
    // ------------------------------------------------------------------

    fn scan_number(&mut self, lo: usize) -> Token {
        if self.src[lo..].starts_with('0') {
            match self.peek() {
                Some('x') | Some('X') => return self.scan_radix_number(lo, Radix::Hex),
                Some('b') | Some('B') => return self.scan_radix_number(lo, Radix::Bin),
                _ => {}
            }
        }

        let mut runs: Vec<(usize, usize)> = Vec::with_capacity(3);
        self.scan_digit_run();
        runs.push((lo, self.pos));

        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            let frac_lo = self.pos;
            self.scan_digit_run();
            runs.push((frac_lo, self.pos));
        }
        if self.exponent_ahead() {
            is_float = true;
            self.bump();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.bump();
            }
            let exp_lo = self.pos;
            self.scan_digit_run();
            runs.push((exp_lo, self.pos));
        }

        let end = self.pos;
        if let Some(bad) = self.first_bad_underscore(&runs) {
            self.error(messages::misplaced_underscore(), bad, bad + 1);
            return self.finish(TokenKind::Error, lo);
        }
        let raw = &self.src[lo..end];
        let cleaned: String = raw.chars().filter(|&c| c != '_').collect();
        if is_float {
            match cleaned.parse::<f64>() {
                Ok(value) if value.is_finite() => self.finish(TokenKind::Float(value), lo),
                _ => {
                    self.error(messages::float_not_representable(), lo, end);
                    self.finish(TokenKind::Error, lo)
                }
            }
        } else {
            match accumulate(&cleaned, 10) {
                Some(magnitude) => self.finish(
                    TokenKind::Int(IntLit {
                        magnitude,
                        radix: Radix::Dec,
                    }),
                    lo,
                ),
                None => {
                    self.error(messages::integer_too_large(), lo, end);
                    self.finish(TokenKind::Error, lo)
                }
            }
        }
    }

    fn scan_radix_number(&mut self, lo: usize, radix: Radix) -> Token {
        let prefix = match radix {
            Radix::Hex => "0x",
            Radix::Bin => "0b",
            Radix::Dec => unreachable!("radix scanning is only used for 0x and 0b"),
        };
        self.bump();
        let body_lo = self.pos;
        while let Some(c) = self.peek() {
            let is_digit = match radix {
                Radix::Hex => c.is_ascii_hexdigit(),
                Radix::Bin => c == '0' || c == '1',
                Radix::Dec => unreachable!(),
            };
            if is_digit || c == '_' {
                self.bump();
            } else {
                break;
            }
        }
        let body = &self.src[body_lo..self.pos];
        if body.is_empty() {
            self.error(messages::expected_radix_digits(prefix), lo, self.pos);
            return self.finish(TokenKind::Error, lo);
        }
        if let Some(offset) = misplaced_in_run(body) {
            let bad = body_lo + offset;
            self.error(messages::misplaced_underscore(), bad, bad + 1);
            return self.finish(TokenKind::Error, lo);
        }
        let cleaned: String = body.chars().filter(|&c| c != '_').collect();
        match accumulate(&cleaned, radix.base()) {
            Some(magnitude) => self.finish(TokenKind::Int(IntLit { magnitude, radix }), lo),
            None => {
                self.error(messages::integer_too_large(), lo, self.pos);
                self.finish(TokenKind::Error, lo)
            }
        }
    }

    fn scan_digit_run(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn exponent_ahead(&self) -> bool {
        match self.peek() {
            Some('e') | Some('E') => match self.peek_nth(1) {
                Some(d) if d.is_ascii_digit() => true,
                Some('+') | Some('-') => {
                    matches!(self.peek_nth(2), Some(d) if d.is_ascii_digit())
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn first_bad_underscore(&self, runs: &[(usize, usize)]) -> Option<usize> {
        for &(lo, hi) in runs {
            if let Some(offset) = misplaced_in_run(&self.src[lo..hi]) {
                return Some(lo + offset);
            }
        }
        None
    }

    // ------------------------------------------------------------------
    // Character literals
    // ------------------------------------------------------------------

    fn scan_char(&mut self, lo: usize) -> Token {
        let value = match self.peek() {
            None | Some('\n') | Some('\r') => {
                self.error(messages::unterminated_char(), lo, self.pos);
                return self.finish(TokenKind::Error, lo);
            }
            Some('\'') => {
                self.bump();
                self.error(messages::char_exactly_one(), lo, self.pos);
                return self.finish(TokenKind::Error, lo);
            }
            Some('\\') => {
                let esc_lo = self.pos;
                match self.scan_escape(false) {
                    EscapeOutcome::Char(c) => c,
                    EscapeOutcome::Unknown(c) => {
                        self.error(messages::unknown_escape(c), esc_lo, self.pos);
                        c
                    }
                    EscapeOutcome::InvalidUnicode => {
                        self.error(messages::invalid_unicode_escape(), esc_lo, self.pos);
                        let _ = self.scan_char_tail();
                        return self.finish(TokenKind::Error, lo);
                    }
                    EscapeOutcome::Unterminated => {
                        self.error(messages::unterminated_char(), lo, self.pos);
                        return self.finish(TokenKind::Error, lo);
                    }
                }
            }
            Some(c) => {
                self.bump();
                c
            }
        };
        match self.peek() {
            Some('\'') => {
                self.bump();
                self.finish(TokenKind::Char(value), lo)
            }
            None | Some('\n') | Some('\r') => {
                self.error(messages::unterminated_char(), lo, self.pos);
                self.finish(TokenKind::Error, lo)
            }
            Some(_) => {
                let closed = self.scan_char_tail();
                if closed {
                    self.error(messages::char_exactly_one(), lo, self.pos);
                } else {
                    self.error(messages::unterminated_char(), lo, self.pos);
                }
                self.finish(TokenKind::Error, lo)
            }
        }
    }

    /// Scans to the closing `'` of a malformed character literal.
    /// Returns `true` if a closing quote was found and consumed.
    fn scan_char_tail(&mut self) -> bool {
        while let Some(c) = self.peek() {
            if c == '\'' {
                self.bump();
                return true;
            }
            if c == '\n' || c == '\r' {
                return false;
            }
            self.bump();
        }
        false
    }

    /// Scans one escape sequence; `self.pos` is at the backslash.
    fn scan_escape(&mut self, in_string: bool) -> EscapeOutcome {
        debug_assert_eq!(self.peek(), Some('\\'));
        self.bump();
        match self.peek() {
            None | Some('\n') | Some('\r') => EscapeOutcome::Unterminated,
            Some('n') => {
                self.bump();
                EscapeOutcome::Char('\n')
            }
            Some('r') => {
                self.bump();
                EscapeOutcome::Char('\r')
            }
            Some('t') => {
                self.bump();
                EscapeOutcome::Char('\t')
            }
            Some('\\') => {
                self.bump();
                EscapeOutcome::Char('\\')
            }
            Some('\'') => {
                self.bump();
                EscapeOutcome::Char('\'')
            }
            Some('"') => {
                self.bump();
                EscapeOutcome::Char('"')
            }
            Some('0') => {
                self.bump();
                EscapeOutcome::Char('\0')
            }
            Some('{') if in_string => {
                self.bump();
                EscapeOutcome::Char('{')
            }
            Some('}') if in_string => {
                self.bump();
                EscapeOutcome::Char('}')
            }
            Some('u') => self.scan_unicode_escape(),
            Some(c) => {
                self.bump();
                EscapeOutcome::Unknown(c)
            }
        }
    }

    /// Scans a `\u{...}` escape; `self.pos` is at the `u`.
    fn scan_unicode_escape(&mut self) -> EscapeOutcome {
        self.bump();
        if self.peek() != Some('{') {
            return EscapeOutcome::InvalidUnicode;
        }
        self.bump();
        let mut value: u32 = 0;
        let mut digits: u32 = 0;
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    if digits == 6 {
                        self.scan_to_escape_end();
                        return EscapeOutcome::InvalidUnicode;
                    }
                    self.bump();
                    value = value * 16 + c.to_digit(16).expect("checked hex digit");
                    digits += 1;
                }
                Some('}') => {
                    self.bump();
                    break;
                }
                _ => {
                    self.scan_to_escape_end();
                    return EscapeOutcome::InvalidUnicode;
                }
            }
        }
        if digits == 0 {
            return EscapeOutcome::InvalidUnicode;
        }
        match char::from_u32(value) {
            Some(c) => EscapeOutcome::Char(c),
            None => EscapeOutcome::InvalidUnicode,
        }
    }

    /// Recovery for a malformed unicode escape: consumes up to and
    /// including the next `}`, or stops before a string boundary.
    fn scan_to_escape_end(&mut self) {
        loop {
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return;
                }
                Some('"') | Some('\'') | Some('\n') | Some('\r') | None => return,
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // String literals and interpolation
    // ------------------------------------------------------------------

    fn scan_string(&mut self, lo: usize) -> Token {
        let mut parts: Vec<StrPart> = Vec::new();
        let mut text = String::new();
        let mut text_lo = self.pos;
        loop {
            match self.peek() {
                None | Some('\n') | Some('\r') => {
                    self.error(messages::unterminated_string(), lo, self.pos);
                    return self.finish(TokenKind::Error, lo);
                }
                Some('"') => {
                    self.flush_text(&mut parts, &mut text, text_lo);
                    self.bump();
                    return self.finish(TokenKind::Str(StrLit { parts }), lo);
                }
                Some('\\') => {
                    let esc_lo = self.pos;
                    match self.scan_escape(true) {
                        EscapeOutcome::Char(c) => text.push(c),
                        EscapeOutcome::Unknown(c) => {
                            self.error(messages::unknown_escape(c), esc_lo, self.pos);
                            text.push(c);
                        }
                        EscapeOutcome::InvalidUnicode => {
                            self.error(messages::invalid_unicode_escape(), esc_lo, self.pos);
                        }
                        EscapeOutcome::Unterminated => {
                            self.error(messages::unterminated_string(), lo, self.pos);
                            return self.finish(TokenKind::Error, lo);
                        }
                    }
                }
                Some('{') => {
                    self.flush_text(&mut parts, &mut text, text_lo);
                    let open_lo = self.pos;
                    self.bump();
                    let open_span = self.span(open_lo, self.pos);
                    if self.depth >= crate::MAX_INTERPOLATION_DEPTH {
                        self.error(
                            messages::interpolation_limit(crate::MAX_INTERPOLATION_DEPTH),
                            open_lo,
                            self.pos,
                        );
                        let interior_lo = self.pos;
                        match self.skip_interpolation_limit() {
                            LimitRecovery::Close(close_hi) => {
                                let close_lo = close_hi - 1;
                                let interior = self.span(interior_lo, close_lo);
                                parts.push(StrPart::Interp(Interpolation {
                                    open: open_span,
                                    tokens: vec![Token {
                                        kind: TokenKind::Error,
                                        span: interior,
                                    }],
                                    close: self.span(close_lo, close_hi),
                                }));
                                text_lo = self.pos;
                            }
                            LimitRecovery::Newline(pos) => {
                                self.error(messages::unterminated_string(), lo, pos);
                                return self.finish(TokenKind::Error, lo);
                            }
                            LimitRecovery::Eof => {
                                let len = self.src.len();
                                let interior = self.span(interior_lo, len);
                                self.error(messages::unterminated_interpolation(), open_lo, len);
                                parts.push(StrPart::Interp(Interpolation {
                                    open: open_span,
                                    tokens: vec![Token {
                                        kind: TokenKind::Error,
                                        span: interior,
                                    }],
                                    close: self.span(len, len),
                                }));
                                return self.finish(TokenKind::Str(StrLit { parts }), lo);
                            }
                        }
                    } else {
                        self.depth += 1;
                        let (tokens, end) = self.lex_embedded();
                        self.depth -= 1;
                        match end {
                            EmbeddedEnd::Close(close_lo, close_hi) => {
                                parts.push(StrPart::Interp(Interpolation {
                                    open: open_span,
                                    tokens,
                                    close: self.span(close_lo, close_hi),
                                }));
                                text_lo = self.pos;
                            }
                            EmbeddedEnd::Newline(pos) => {
                                self.error(messages::unterminated_string(), lo, pos);
                                return self.finish(TokenKind::Error, lo);
                            }
                            EmbeddedEnd::Eof => {
                                let len = self.src.len();
                                let eof_span = self.span(len, len);
                                let mut tokens = tokens;
                                tokens.push(Token {
                                    kind: TokenKind::Error,
                                    span: eof_span,
                                });
                                self.error(messages::unterminated_interpolation(), open_lo, len);
                                parts.push(StrPart::Interp(Interpolation {
                                    open: open_span,
                                    tokens,
                                    close: eof_span,
                                }));
                                return self.finish(TokenKind::Str(StrLit { parts }), lo);
                            }
                        }
                    }
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
    }

    fn flush_text(&mut self, parts: &mut Vec<StrPart>, text: &mut String, text_lo: usize) {
        if !text.is_empty() {
            parts.push(StrPart::Text(TextSegment {
                text: std::mem::take(text),
                span: self.span(text_lo, self.pos),
            }));
        }
    }

    /// Lexes the embedded token stream of an interpolation.
    fn lex_embedded(&mut self) -> (Vec<Token>, EmbeddedEnd) {
        let mut tokens: Vec<Token> = Vec::new();
        let mut brace_depth: u32 = 0;
        loop {
            match self.skip_trivia(true) {
                Trivia::Newline(pos) => return (tokens, EmbeddedEnd::Newline(pos)),
                Trivia::Eof => return (tokens, EmbeddedEnd::Eof),
                Trivia::Ok => {}
            }
            if self.pos >= self.src.len() {
                return (tokens, EmbeddedEnd::Eof);
            }
            if self.peek() == Some('}') && brace_depth == 0 {
                let lo = self.pos;
                self.bump();
                return (tokens, EmbeddedEnd::Close(lo, self.pos));
            }
            let before = self.pos;
            let token = self.lex_one_token();
            debug_assert!(self.pos > before, "lexer must make progress");
            match token.kind {
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => {
                    debug_assert!(brace_depth > 0);
                    brace_depth -= 1;
                }
                _ => {}
            }
            tokens.push(token);
        }
    }

    /// Raw recovery scan for an over-limit interpolation: consumes to the
    /// balancing `}` (skipping nested strings and comments), a raw newline,
    /// or EOF.
    fn skip_interpolation_limit(&mut self) -> LimitRecovery {
        let mut brace_depth: u32 = 1;
        loop {
            match self.peek() {
                None => return LimitRecovery::Eof,
                Some('\n') | Some('\r') => return LimitRecovery::Newline(self.pos),
                Some('"') | Some('\'') => {
                    if let Some(stop) = self.skip_raw_quoted() {
                        return stop;
                    }
                }
                Some('/') if self.peek_nth(1) == Some('/') => loop {
                    match self.peek() {
                        None => return LimitRecovery::Eof,
                        Some('\n') | Some('\r') => return LimitRecovery::Newline(self.pos),
                        Some(_) => {
                            self.bump();
                        }
                    }
                },
                Some('/') if self.peek_nth(1) == Some('*') => {
                    self.bump();
                    self.bump();
                    let mut comment_depth: u32 = 1;
                    loop {
                        match self.peek() {
                            None => return LimitRecovery::Eof,
                            Some('\n') | Some('\r') => {
                                return LimitRecovery::Newline(self.pos);
                            }
                            Some('/') if self.peek_nth(1) == Some('*') => {
                                comment_depth += 1;
                                self.bump();
                                self.bump();
                            }
                            Some('*') if self.peek_nth(1) == Some('/') => {
                                comment_depth -= 1;
                                self.bump();
                                self.bump();
                                if comment_depth == 0 {
                                    break;
                                }
                            }
                            Some(_) => {
                                self.bump();
                            }
                        }
                    }
                }
                Some('{') => {
                    brace_depth += 1;
                    self.bump();
                }
                Some('}') => {
                    brace_depth -= 1;
                    self.bump();
                    if brace_depth == 0 {
                        return LimitRecovery::Close(self.pos);
                    }
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    /// Skips a raw quoted run during limit recovery. Returns `Some(stop)`
    /// when the run ends abnormally (newline or EOF).
    fn skip_raw_quoted(&mut self) -> Option<LimitRecovery> {
        let quote = self.bump();
        loop {
            match self.peek() {
                None => return Some(LimitRecovery::Eof),
                Some('\\') => {
                    self.bump();
                    if self.peek().is_some_and(|c| c != '\n' && c != '\r') {
                        self.bump();
                    }
                }
                Some(c) if c == quote => {
                    self.bump();
                    return None;
                }
                Some('\n') | Some('\r') => return Some(LimitRecovery::Newline(self.pos)),
                Some(_) => {
                    self.bump();
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

/// Returns the byte offset of the first misplaced `_` in a digit run
/// (leading, trailing, or doubled underscores violate the contract).
fn misplaced_in_run(run: &str) -> Option<usize> {
    let bytes = run.as_bytes();
    for (index, &byte) in bytes.iter().enumerate() {
        if byte == b'_' {
            let prev_ok = index > 0 && bytes[index - 1] != b'_';
            let next_ok = index + 1 < bytes.len() && bytes[index + 1] != b'_';
            if !prev_ok || !next_ok {
                return Some(index);
            }
        }
    }
    None
}

/// Accumulates a cleaned digit string in `base`, returning `None` on
/// `u64` overflow.
fn accumulate(digits: &str, base: u64) -> Option<u64> {
    let mut acc: u64 = 0;
    for c in digits.chars() {
        let digit = c.to_digit(base as u32)?;
        acc = acc.checked_mul(base)?.checked_add(u64::from(digit))?;
    }
    Some(acc)
}
