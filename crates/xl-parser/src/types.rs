//! Type, generic-parameter, and pattern parsing.
//!
//! Generic closing implements the accepted G-3 rule: in type-closing
//! context a `>` closes one level; a `>=` token is split once into `>` plus
//! a residual `=` (span-exact, no token dropped or duplicated). The split
//! never applies in expression positions, so `a >= b` remains a single
//! comparison.
//!
//! Grouping is syntax-only for types: `(T)` yields the inner type node with
//! its own span (`parser-ast-contract.md` §2.3(6)).

use xl_ast::{GenericParam, Generics, Ident, Pat, PatKind, Path, PathType, Type, TypeKind};
use xl_lexer::TokenKind;
use xl_span::Span;

use crate::messages;
use crate::parser::Parser;
use crate::stream::KindTag;

use KindTag as Tag;

/// Type-position recovery set; delimiters and `=` are never consumed.
pub(crate) const TYPE_SYNC: &[KindTag] = &[
    Tag::Comma,
    Tag::Semi,
    Tag::RBrace,
    Tag::RParen,
    Tag::RBracket,
    Tag::FatArrow,
    Tag::Eq,
    Tag::Eof,
];

/// Type-list recovery set (inside `( )` payloads and `fn( )` types);
/// `}` is preserved for the enclosing block.
pub(crate) const TYPE_LIST_SYNC: &[KindTag] = &[Tag::Comma, Tag::RParen, Tag::RBrace, Tag::Eof];

/// Generic-argument/parameter recovery set.
const GENERIC_SYNC: &[KindTag] = &[
    Tag::Comma,
    Tag::Gt,
    Tag::Ge,
    Tag::RParen,
    Tag::RBracket,
    Tag::RBrace,
    Tag::LBrace,
    Tag::Semi,
    Tag::Eof,
];

/// Pattern-position recovery set.
const PAT_SYNC: &[KindTag] = &[
    Tag::Comma,
    Tag::Semi,
    Tag::RBrace,
    Tag::RParen,
    Tag::RBracket,
    Tag::FatArrow,
    Tag::Eq,
    Tag::In,
    Tag::Eof,
];

/// Constructor-payload pattern recovery set (`}` preserved for the block).
const PAT_LIST_SYNC: &[KindTag] = &[Tag::Comma, Tag::RParen, Tag::RBrace, Tag::Eof];

impl Parser {
    // ------------------------------------------------------------------
    // Types
    // ------------------------------------------------------------------

    /// Parses a type in a non-generic, non-cast context (depth-guarded).
    ///
    /// After the type completes, excess `>` closers are diagnosed and
    /// skipped (`unexpected '>'`, N27). Generic-argument lists and `as`
    /// casts call [`Parser::parse_type_inner`] directly: there a following
    /// `>` belongs to the enclosing list or to a comparison expression.
    pub(crate) fn parse_type_entry(&mut self) -> Type {
        let ty = self.parse_type_inner();
        self.skip_excess_closers();
        ty
    }

    /// Parses a type at any nesting position (depth-guarded).
    pub(crate) fn parse_type_inner(&mut self) -> Type {
        if !self.enter() {
            let at = self.stream.position();
            self.leave();
            return self.placeholder_type_at(at);
        }
        let result = self.parse_type_inner_unguarded();
        self.leave();
        result
    }

    /// Diagnoses and skips stray `>` closers in type position.
    fn skip_excess_closers(&mut self) {
        while self.stream.check_tag(Tag::Gt) {
            let span = self.stream.current_span();
            let message = messages::unexpected("'>'");
            self.error(message, span);
            self.stream.bump();
        }
    }

    fn parse_type_inner_unguarded(&mut self) -> Type {
        let Some(token) = self.stream.peek().cloned() else {
            let span = self.stream.current_span();
            let message = messages::expected("type");
            self.error(message, span);
            return self.placeholder_type_at(self.stream.position());
        };
        let span = token.span;
        match token.kind {
            TokenKind::LParen => self.parse_paren_type(span),
            TokenKind::LBracket => self.parse_bracket_type(span),
            TokenKind::Fn => self.parse_fn_type(span),
            TokenKind::Ident(_) => {
                let path = self.parse_path_type_inner();
                let path_span = path.span;
                Type {
                    kind: TypeKind::Path(path),
                    span: path_span,
                }
            }
            _ => {
                let message = messages::expected("type");
                self.error(message, span);
                self.synchronize(TYPE_SYNC);
                self.placeholder_type_at(self.stream.position())
            }
        }
    }

    /// Parses `()`, `(T)`, or `(T, …)`; grouping is transparent.
    fn parse_paren_type(&mut self, open: Span) -> Type {
        self.stream.bump(); // `(`
        if self.stream.check_tag(Tag::RParen) {
            let close = self.stream.bump().map_or(open, |token| token.span);
            let span = self.join(open, close);
            return Type {
                kind: TypeKind::Tuple(Vec::new()),
                span,
            };
        }
        let first = self.parse_type_entry();
        if self.stream.check_tag(Tag::Comma) {
            let mut elements = vec![first];
            loop {
                self.stream.bump(); // `,`
                if self.stream.check_tag(Tag::RParen) {
                    break;
                }
                elements.push(self.parse_type_entry());
                if !self.stream.check_tag(Tag::Comma) {
                    break;
                }
            }
            let close = self.expect_tag(Tag::RParen, "')'");
            let hi = close.map_or_else(|| self.stream.position(), |token| token.span.hi);
            let span = Span {
                file: self.file,
                lo: open.lo,
                hi: hi.max(open.lo),
            };
            Type {
                kind: TypeKind::Tuple(elements),
                span,
            }
        } else {
            let _ = self.expect_tag(Tag::RParen, "')'");
            first
        }
    }

    /// Parses `[T]` or `[T; N]`.
    fn parse_bracket_type(&mut self, open: Span) -> Type {
        self.stream.bump(); // `[`
        let element = self.parse_type_entry();
        let (kind, hi) = if self.stream.eat_tag(Tag::Semi).is_some() {
            let length = self.parse_expr_bp(0);
            let hi = length.span.hi;
            (TypeKind::Array(Box::new(element), Box::new(length)), hi)
        } else {
            let hi = element.span.hi;
            (TypeKind::Slice(Box::new(element)), hi)
        };
        let close = self.expect_tag(Tag::RBracket, "']'");
        let hi = close.map_or(hi, |token| token.span.hi);
        Type {
            kind,
            span: Span {
                file: self.file,
                lo: open.lo,
                hi: hi.max(open.lo),
            },
        }
    }

    /// Parses `fn( [types] ) [-> type]`.
    fn parse_fn_type(&mut self, open: Span) -> Type {
        self.stream.bump(); // `fn`
        let _ = self.expect_tag(Tag::LParen, "'('");
        let mut params = Vec::new();
        if !self.stream.check_tag(Tag::RParen) {
            loop {
                params.push(self.parse_type_entry());
                if self.stream.check_tag(Tag::Comma) {
                    self.stream.bump();
                    if self.stream.check_tag(Tag::RParen) {
                        break;
                    }
                    continue;
                }
                if self.stream.check_tag(Tag::RParen) {
                    break;
                }
                let span = self.stream.current_span();
                let message = messages::expected_or(",", ")");
                self.error(message, span);
                self.synchronize(TYPE_LIST_SYNC);
                if self.stream.check_tag(Tag::Comma) {
                    self.stream.bump();
                    if self.stream.check_tag(Tag::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        let mut hi = self
            .stream
            .eat_tag(Tag::RParen)
            .map_or_else(|| self.stream.position(), |token| token.span.hi);
        let ret = if self.stream.eat_tag(Tag::Arrow).is_some() {
            let ty = self.parse_type_entry();
            hi = ty.span.hi;
            Some(Box::new(ty))
        } else {
            None
        };
        Type {
            kind: TypeKind::Fn { params, ret },
            span: Span {
                file: self.file,
                lo: open.lo,
                hi: hi.max(open.lo),
            },
        }
    }

    /// Parses a type path in a non-generic context (excess-closer check).
    pub(crate) fn parse_path_type_entry(&mut self) -> PathType {
        let path = self.parse_path_type_inner();
        self.skip_excess_closers();
        path
    }

    /// Parses `IDENT { :: IDENT } [ generic_args ]`.
    pub(crate) fn parse_path_type_inner(&mut self) -> PathType {
        let first = match self.expect_ident() {
            Some(ident) => ident,
            None => {
                let at = self.stream.position();
                self.synchronize(TYPE_SYNC);
                self.placeholder_ident_at(at)
            }
        };
        let lo = first.span.lo;
        let mut hi = first.span.hi;
        let mut segments = vec![first];
        while self.stream.check_tag(Tag::ColonColon) {
            self.stream.bump();
            match self.expect_ident() {
                Some(ident) => {
                    hi = ident.span.hi;
                    segments.push(ident);
                }
                None => break,
            }
        }
        let generics = if self.stream.check_tag(Tag::Lt) {
            match self.parse_generic_args() {
                Some((args, span)) => {
                    hi = span.hi;
                    Some(args)
                }
                None => None,
            }
        } else {
            None
        };
        PathType {
            segments,
            generics,
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Parses `< type {, type} [,] >` with the G-3 closing rules.
    ///
    /// Returns the argument list and the span from `<` through the closer.
    fn parse_generic_args(&mut self) -> Option<(Vec<Type>, Span)> {
        let lt = self.stream.eat_tag(Tag::Lt)?;
        let mut args = Vec::new();
        loop {
            if self.stream.check_tag(Tag::Gt) {
                if args.is_empty() {
                    let span = self.stream.current_span();
                    let message = messages::expected("type");
                    self.error(message, span);
                }
                let close = self.stream.bump().map_or(lt.span, |token| token.span);
                return Some((args, self.join(lt.span, close)));
            }
            if self.stream.check_tag(Tag::Ge) {
                if args.is_empty() {
                    let span = self.stream.current_span();
                    let message = messages::expected("type");
                    self.error(message, span);
                }
                let close = self.stream.split_ge().unwrap_or(lt.span);
                return Some((args, self.join(lt.span, close)));
            }
            args.push(self.parse_type_inner());
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                if self.stream.check_tag(Tag::Gt) {
                    let close = self.stream.bump().map_or(lt.span, |token| token.span);
                    return Some((args, self.join(lt.span, close)));
                }
                if self.stream.check_tag(Tag::Ge) {
                    let close = self.stream.split_ge().unwrap_or(lt.span);
                    return Some((args, self.join(lt.span, close)));
                }
                continue;
            }
            if self.stream.check_tag(Tag::Gt) {
                let close = self.stream.bump().map_or(lt.span, |token| token.span);
                return Some((args, self.join(lt.span, close)));
            }
            if self.stream.check_tag(Tag::Ge) {
                let close = self.stream.split_ge().unwrap_or(lt.span);
                return Some((args, self.join(lt.span, close)));
            }
            let span = self.stream.current_span();
            let message = messages::expected_or(",", ">");
            self.error(message, span);
            self.synchronize(GENERIC_SYNC);
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                continue;
            }
            if self.stream.check_tag(Tag::Gt) {
                let close = self.stream.bump().map_or(lt.span, |token| token.span);
                return Some((args, self.join(lt.span, close)));
            }
            if self.stream.check_tag(Tag::Ge) {
                let close = self.stream.split_ge().unwrap_or(lt.span);
                return Some((args, self.join(lt.span, close)));
            }
            return Some((args, self.join(lt.span, self.stream.current_span())));
        }
    }

    /// Parses a declaration generic-parameter list `<T: Bound, U>`.
    pub(crate) fn parse_generics_decl(&mut self) -> Option<Generics> {
        if !self.stream.check_tag(Tag::Lt) {
            return None;
        }
        let lt = self.stream.bump()?;
        let mut params = Vec::new();
        loop {
            if self.stream.check_tag(Tag::Gt) || self.stream.check_tag(Tag::Ge) {
                if params.is_empty() {
                    let span = self.stream.current_span();
                    let message = messages::expected("identifier");
                    self.error(message, span);
                }
                break;
            }
            let Some(name) = self.expect_ident() else {
                self.synchronize(GENERIC_SYNC);
                break;
            };
            let bound = if self.stream.eat_tag(Tag::Colon).is_some() {
                // The bound is a `path_type` inside the parameter list:
                // its own `>` closer is the declaration's closer, so the
                // excess-closer check of `parse_path_type_entry` must not
                // run here.
                Some(self.parse_path_type_inner())
            } else {
                None
            };
            let hi = bound.as_ref().map_or(name.span.hi, |path| path.span.hi);
            let span = Span {
                file: self.file,
                lo: name.span.lo,
                hi: hi.max(name.span.lo),
            };
            params.push(GenericParam { name, bound, span });
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                continue;
            }
            break;
        }
        let close = if self.stream.check_tag(Tag::Gt) {
            self.stream.bump().map(|token| token.span)
        } else if self.stream.check_tag(Tag::Ge) {
            self.stream.split_ge()
        } else {
            let span = self.stream.current_span();
            let message = messages::expected("'>'");
            self.error(message, span);
            self.synchronize(GENERIC_SYNC);
            if self.stream.check_tag(Tag::Gt) {
                self.stream.bump().map(|token| token.span)
            } else if self.stream.check_tag(Tag::Ge) {
                self.stream.split_ge()
            } else {
                Some(self.stream.current_span())
            }
        };
        let close_span = close.unwrap_or_else(|| self.stream.current_span());
        Some(Generics {
            params,
            span: self.join(lt.span, close_span),
        })
    }

    // ------------------------------------------------------------------
    // Patterns
    // ------------------------------------------------------------------

    /// Parses a pattern (depth-guarded).
    pub(crate) fn parse_pattern(&mut self) -> Pat {
        if !self.enter() {
            let at = self.stream.position();
            self.leave();
            return self.placeholder_pat_at(at);
        }
        let result = self.parse_pattern_unguarded();
        self.leave();
        result
    }

    fn parse_pattern_unguarded(&mut self) -> Pat {
        let Some(token) = self.stream.peek().cloned() else {
            let span = self.stream.current_span();
            let message = messages::expected("pattern");
            self.error(message, span);
            return self.placeholder_pat_at(self.stream.position());
        };
        let span = token.span;
        match token.kind {
            TokenKind::Underscore => {
                self.stream.bump();
                Pat {
                    kind: PatKind::Wildcard,
                    span,
                }
            }
            TokenKind::Int(value) => {
                self.stream.bump();
                self.lit_pattern(xl_ast::Lit::Int(crate::expr::convert_int(value)), span)
            }
            TokenKind::Float(value) => {
                self.stream.bump();
                self.lit_pattern(xl_ast::Lit::Float(value), span)
            }
            TokenKind::Char(value) => {
                self.stream.bump();
                self.lit_pattern(xl_ast::Lit::Char(value), span)
            }
            TokenKind::True => {
                self.stream.bump();
                self.lit_pattern(xl_ast::Lit::Bool(true), span)
            }
            TokenKind::False => {
                self.stream.bump();
                self.lit_pattern(xl_ast::Lit::Bool(false), span)
            }
            TokenKind::Str(literal) => {
                self.stream.bump();
                let parts = self.parse_string_parts(literal);
                self.lit_pattern(xl_ast::Lit::Str(xl_ast::StringLit { parts, span }), span)
            }
            TokenKind::Ident(name) => {
                self.stream.bump();
                let first = Ident { name, span };
                if self.stream.check_tag(Tag::ColonColon) {
                    let path = self.finish_path(first);
                    let path_span = path.span;
                    let (args, hi) = if self.stream.check_tag(Tag::LParen) {
                        let (args, close) = self.parse_pattern_list();
                        let hi = close.map_or_else(|| self.stream.position(), |s| s.hi);
                        (Some(args), hi)
                    } else {
                        (None, path_span.hi)
                    };
                    let full = Span {
                        file: self.file,
                        lo: path_span.lo,
                        hi: hi.max(path_span.lo),
                    };
                    Pat {
                        kind: PatKind::Path { path, args },
                        span: full,
                    }
                } else if self.stream.check_tag(Tag::LParen) {
                    // Single-segment constructor: `Ok(n)`.
                    let (args, close) = self.parse_pattern_list();
                    let hi = close.map_or_else(|| self.stream.position(), |s| s.hi);
                    let path = Path {
                        segments: vec![first],
                        span,
                    };
                    let full = Span {
                        file: self.file,
                        lo: span.lo,
                        hi: hi.max(span.lo),
                    };
                    Pat {
                        kind: PatKind::Path {
                            path,
                            args: Some(args),
                        },
                        span: full,
                    }
                } else {
                    Pat {
                        kind: PatKind::Binding(first),
                        span,
                    }
                }
            }
            TokenKind::LParen => {
                self.stream.bump(); // `(`
                if self.stream.check_tag(Tag::RParen) {
                    let close = self.stream.bump().map_or(span, |token| token.span);
                    let message = messages::expected("pattern");
                    self.error(message, close);
                    let full = self.join(span, close);
                    return Pat {
                        kind: PatKind::Tuple(Vec::new()),
                        span: full,
                    };
                }
                let mut elements = vec![self.parse_pattern()];
                loop {
                    if self.stream.check_tag(Tag::Comma) {
                        self.stream.bump();
                        if self.stream.check_tag(Tag::RParen) {
                            break;
                        }
                        elements.push(self.parse_pattern());
                    } else {
                        break;
                    }
                }
                let close = self.expect_tag(Tag::RParen, "')'");
                let hi = close.map_or_else(|| self.stream.position(), |token| token.span.hi);
                let full = Span {
                    file: self.file,
                    lo: span.lo,
                    hi: hi.max(span.lo),
                };
                Pat {
                    kind: PatKind::Tuple(elements),
                    span: full,
                }
            }
            _ => {
                let message = messages::expected("pattern");
                self.error(message, span);
                self.synchronize(PAT_SYNC);
                self.placeholder_pat_at(self.stream.position())
            }
        }
    }

    /// Builds a literal pattern node.
    fn lit_pattern(&self, lit: xl_ast::Lit, span: Span) -> Pat {
        Pat {
            kind: PatKind::Lit(lit),
            span,
        }
    }

    /// Parses a constructor payload list `( pat, … )`, consuming the
    /// closer when present.
    fn parse_pattern_list(&mut self) -> (Vec<Pat>, Option<Span>) {
        self.stream.bump(); // `(`
        let mut items = Vec::new();
        if self.stream.check_tag(Tag::RParen) {
            return (items, self.stream.bump().map(|token| token.span));
        }
        loop {
            items.push(self.parse_pattern());
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                if self.stream.check_tag(Tag::RParen) {
                    return (items, self.stream.bump().map(|token| token.span));
                }
                continue;
            }
            if self.stream.check_tag(Tag::RParen) {
                return (items, self.stream.bump().map(|token| token.span));
            }
            let span = self.stream.current_span();
            let message = messages::expected_or(",", ")");
            self.error(message, span);
            self.synchronize(PAT_LIST_SYNC);
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                continue;
            }
            if self.stream.check_tag(Tag::RParen) {
                return (items, self.stream.bump().map(|token| token.span));
            }
            return (items, None);
        }
    }
}
