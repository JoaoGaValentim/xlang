//! Recursive-descent parser: token stream to `xl-ast` items.
//!
//! Structure: items, statements, and blocks live here; the Pratt
//! expression parser is in `expr`; types, generics, and patterns are in
//! `types`. All modules share the [`Parser`] state via `impl` blocks.
//!
//! # Recovery conventions
//!
//! Recovered/missing constructs are **placeholders**: zero-width spans at
//! the deterministic insertion point (`parser-ast-contract.md` §2.3(3)).
//! Placeholder payload shapes are documented at each constructor. A
//! recovered tree is never a successful parse: diagnostics are the
//! authority, and callers must not use the canonical printer to certify
//! input whose diagnostics contain errors.
//!
//! # Determinism and progress
//!
//! Diagnostics are emitted in parse order; node identifiers are allocated
//! in construction order per AST instance. Every recovery cycle either
//! consumes input or terminates, and the block/item loops additionally arm
//! a progress watchdog so no token stream can cause a non-terminating
//! parse.

use xl_ast::{
    Block, EnumDecl, Expr, ExprId, Field, FnDecl, FnSig, Ident, ImplDecl, ImplItem, ImportDecl,
    Item, ItemId, ItemKind, LetStmt, Param, Pat, PatKind, Stmt, StmtKind, StructDecl, TraitDecl,
    TraitItem, Type, TypeKind, Variant, Visibility,
};
use xl_diag::Diagnostic;
use xl_lexer::{Token, TokenKind};
use xl_span::{FileId, Span};

use crate::messages;
use crate::stream::{KindTag, Stream};

use KindTag as T;

/// Top-level item synchronization set (`parser-ast-contract.md` §4).
pub(crate) const ITEM_SYNC: &[KindTag] = &[
    T::Semi,
    T::RBrace,
    T::Fn,
    T::Struct,
    T::Enum,
    T::Impl,
    T::Trait,
    T::Import,
    T::Pub,
    T::Eof,
];

/// Statement synchronization set (`;`, `}`, item keywords; `}` is left for
/// the enclosing block).
pub(crate) const STMT_SYNC: &[KindTag] = &[
    T::Semi,
    T::RBrace,
    T::Fn,
    T::Struct,
    T::Enum,
    T::Impl,
    T::Trait,
    T::Import,
    T::Pub,
    T::Eof,
];

/// Expression synchronization set; delimiters are never consumed.
pub(crate) const EXPR_SYNC: &[KindTag] = &[
    T::Comma,
    T::Semi,
    T::RBrace,
    T::RParen,
    T::RBracket,
    T::FatArrow,
    T::Eof,
];

/// Parameter-list recovery set (`parser-ast-contract.md` §4; `{` marks the
/// body so the list can be abandoned without extra noise).
pub(crate) const PARAM_SYNC: &[KindTag] = &[T::Comma, T::RParen, T::LBrace, T::RBrace, T::Semi];

/// Import recovery: sync at `;` (conformance N14/N15); the statement's
/// terminator is then consumed when present.
const IMPORT_SYNC: &[KindTag] = &[T::Semi, T::Eof];
const LET_SYNC: &[KindTag] = &[T::Semi, T::RBrace, T::Eof];
const FIELD_SYNC: &[KindTag] = &[T::Comma, T::RBrace, T::Eof];
const IMPL_SYNC: &[KindTag] = &[T::Fn, T::Pub, T::RBrace, T::Eof];

/// Result of parsing one statement position inside a block.
pub(crate) enum StmtOrTail {
    /// A complete statement (boxed: a `Stmt` is large and moves into the
    /// statement list).
    Stmt(Box<Stmt>),
    /// The block's tail (value) expression (boxed: the tail moves directly
    /// into `Block::tail`).
    Tail(Box<Expr>),
}

/// The parser state for one source file.
pub(crate) struct Parser {
    pub(crate) stream: Stream,
    pub(crate) file: FileId,
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// Shared nesting counter across all recursive parse paths (B-4).
    pub(crate) depth: u32,
    next_item_id: u32,
    next_expr_id: u32,
}

impl Parser {
    /// Creates a parser over an owned token stream.
    pub(crate) fn new(tokens: Vec<Token>, file: FileId) -> Self {
        Self {
            stream: Stream::new(tokens, file),
            file,
            diagnostics: Vec::new(),
            depth: 0,
            next_item_id: 0,
            next_expr_id: 0,
        }
    }

    /// Parses the whole token stream into items.
    pub(crate) fn parse_items(&mut self) -> Vec<Item> {
        let mut items = Vec::new();
        while !self.stream.at_eof() {
            let before = self.stream.mark();
            if self.at_item_start() {
                if let Some(item) = self.parse_item() {
                    items.push(item);
                }
            } else {
                let span = self.stream.current_span();
                let message = messages::expected("item");
                self.error(message, span);
                self.recover_top_level();
            }
            self.ensure_progress(before);
        }
        items
    }

    // ------------------------------------------------------------------
    // Diagnostics, identity, spans, placeholders
    // ------------------------------------------------------------------

    /// Emits an error diagnostic at `span`.
    pub(crate) fn error(&mut self, message: String, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    /// Allocates the next item identifier (saturating; instance-scoped).
    pub(crate) fn new_item_id(&mut self) -> ItemId {
        let id = ItemId::from_index(self.next_item_id);
        self.next_item_id = self.next_item_id.saturating_add(1);
        id
    }

    /// Allocates the next expression identifier (saturating; instance-scoped).
    pub(crate) fn new_expr_id(&mut self) -> ExprId {
        let id = ExprId::from_index(self.next_expr_id);
        self.next_expr_id = self.next_expr_id.saturating_add(1);
        id
    }

    /// A zero-width span at `at` (the only synthesized span convention).
    pub(crate) fn zero_span(&self, at: u32) -> Span {
        Span {
            file: self.file,
            lo: at,
            hi: at,
        }
    }

    /// Joins two spans into their smallest covering span.
    pub(crate) fn join(&self, a: Span, b: Span) -> Span {
        Span {
            file: self.file,
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
        }
    }

    /// Placeholder expression: zero-width span, `Tuple([])` payload.
    pub(crate) fn placeholder_expr_at(&mut self, at: u32) -> Expr {
        Expr {
            id: self.new_expr_id(),
            kind: xl_ast::ExprKind::Tuple(Vec::new()),
            span: self.zero_span(at),
        }
    }

    /// Placeholder pattern: zero-width span, `Wildcard` payload.
    pub(crate) fn placeholder_pat_at(&self, at: u32) -> Pat {
        Pat {
            kind: PatKind::Wildcard,
            span: self.zero_span(at),
        }
    }

    /// Placeholder type: zero-width span, `Tuple([])` (unit) payload.
    pub(crate) fn placeholder_type_at(&self, at: u32) -> Type {
        Type {
            kind: TypeKind::Tuple(Vec::new()),
            span: self.zero_span(at),
        }
    }

    /// Placeholder identifier: zero-width span, empty name.
    ///
    /// The canonical printer reports `InvalidStructure` for empty
    /// identifiers, so recovered trees can never be mistaken for
    /// successfully parsed ones.
    pub(crate) fn placeholder_ident_at(&self, at: u32) -> Ident {
        Ident {
            name: String::new(),
            span: self.zero_span(at),
        }
    }

    // ------------------------------------------------------------------
    // Depth guard, synchronization, progress
    // ------------------------------------------------------------------

    /// Enters one nesting level; on limit violation reports once, skips to
    /// the expression synchronization set, and returns `false`.
    pub(crate) fn enter(&mut self) -> bool {
        self.depth += 1;
        if self.depth > crate::MAX_PARSE_DEPTH {
            let span = self.stream.current_span();
            let message = messages::nesting_limit(crate::MAX_PARSE_DEPTH);
            self.error(message, span);
            self.synchronize(EXPR_SYNC);
            false
        } else {
            true
        }
    }

    /// Leaves one nesting level.
    pub(crate) fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Skips tokens until the current token is in `set` (never consuming a
    /// member of the set) or the stream is exhausted.
    ///
    /// Returns `true` when at least one token was consumed.
    pub(crate) fn synchronize(&mut self, set: &[KindTag]) -> bool {
        let mut consumed = false;
        loop {
            match self.stream.peek() {
                None => break,
                Some(token) => {
                    if set.iter().any(|tag| tag.matches(&token.kind)) {
                        break;
                    }
                    self.stream.bump();
                    consumed = true;
                }
            }
        }
        consumed
    }

    /// Guarantees forward progress for loop bodies: if the cursor did not
    /// move since `mark`, consumes one token.
    ///
    /// This is the safety net behind the "parsing terminates on every token
    /// stream" guarantee; well-formed recovery paths never rely on it.
    pub(crate) fn ensure_progress(&mut self, mark: usize) {
        if !self.stream.progressed_since(mark) {
            self.stream.bump();
        }
    }

    /// Consumes the current token when it has `tag`'s category; otherwise
    /// reports `expected {what}` at the current position.
    pub(crate) fn expect_tag(&mut self, tag: KindTag, what: &str) -> Option<Token> {
        if let Some(token) = self.stream.eat_tag(tag) {
            Some(token)
        } else {
            let span = self.stream.current_span();
            let message = messages::expected(what);
            self.error(message, span);
            None
        }
    }

    /// Consumes an identifier; on failure reports `expected identifier`.
    pub(crate) fn expect_ident(&mut self) -> Option<Ident> {
        if self.stream.check_tag(T::Ident) {
            match self.stream.bump() {
                Some(Token {
                    kind: TokenKind::Ident(name),
                    span,
                }) => Some(Ident { name, span }),
                _ => None,
            }
        } else {
            let span = self.stream.current_span();
            let message = messages::expected("identifier");
            self.error(message, span);
            None
        }
    }

    /// Recovers at top level: skip to the item synchronization set; consume
    /// the bad construct's trailing `;` when the skip stopped there, or the
    /// synchronization token itself when nothing was consumed (the caller's
    /// message already described the problem).
    fn recover_top_level(&mut self) {
        let consumed = self.synchronize(ITEM_SYNC);
        if !consumed || self.stream.check_tag(T::Semi) {
            self.stream.bump();
        }
    }

    // ------------------------------------------------------------------
    // Items
    // ------------------------------------------------------------------

    /// Returns `true` when the cursor is at a plausible item start.
    fn at_item_start(&self) -> bool {
        let item_keyword = |tag: KindTag| {
            matches!(
                tag,
                T::Fn | T::Struct | T::Enum | T::Impl | T::Trait | T::Import
            )
        };
        match self.stream.peek() {
            None => false,
            Some(token) => {
                let tag = token_tag(&token.kind);
                if item_keyword(tag) {
                    return true;
                }
                if tag == T::Pub {
                    return self
                        .stream
                        .peek_n(1)
                        .is_some_and(|next| item_keyword(token_tag(&next.kind)));
                }
                false
            }
        }
    }

    /// Parses one item (depth-guarded). Returns `None` after a recovery
    /// that produced no construct.
    pub(crate) fn parse_item(&mut self) -> Option<Item> {
        if !self.enter() {
            self.leave();
            return None;
        }
        let result = self.parse_item_inner();
        self.leave();
        result
    }

    fn parse_item_inner(&mut self) -> Option<Item> {
        let lo = self.stream.position();
        let vis = self.parse_visibility();
        let tag = self
            .stream
            .peek()
            .map_or(T::Eof, |token| token_tag(&token.kind));
        match tag {
            T::Import => {
                if let Visibility::Public(span) = vis {
                    let message = messages::visibility_on_import();
                    self.error(message, span);
                }
                self.parse_import(lo)
            }
            T::Fn => self.parse_fn_item(lo, vis).map(|decl| Item {
                id: self.new_item_id(),
                span: decl.span,
                kind: ItemKind::Fn(decl),
            }),
            T::Struct => self.parse_struct_item(lo, vis).map(|decl| Item {
                id: self.new_item_id(),
                span: decl.span,
                kind: ItemKind::Struct(decl),
            }),
            T::Enum => self.parse_enum_item(lo, vis).map(|decl| Item {
                id: self.new_item_id(),
                span: decl.span,
                kind: ItemKind::Enum(decl),
            }),
            T::Impl => {
                if let Visibility::Public(span) = vis {
                    let message = messages::visibility_on_impl();
                    self.error(message, span);
                }
                self.parse_impl_item(lo).map(|decl| Item {
                    id: self.new_item_id(),
                    span: decl.span,
                    kind: ItemKind::Impl(decl),
                })
            }
            T::Trait => self.parse_trait_item(lo, vis).map(|decl| Item {
                id: self.new_item_id(),
                span: decl.span,
                kind: ItemKind::Trait(decl),
            }),
            _ => {
                let span = self.stream.current_span();
                let message = messages::expected("item");
                self.error(message, span);
                let consumed = self.synchronize(ITEM_SYNC);
                if !consumed {
                    self.stream.bump();
                }
                None
            }
        }
    }

    /// Parses an optional `pub`.
    fn parse_visibility(&mut self) -> Visibility {
        match self.stream.eat_tag(T::Pub) {
            Some(token) => Visibility::Public(token.span),
            None => Visibility::Private,
        }
    }

    /// Parses a function declaration (at `fn`); `lo` is the item start.
    fn parse_fn_item(&mut self, lo: u32, vis: Visibility) -> Option<FnDecl> {
        self.stream.bump()?; // `fn`
        let name = self
            .expect_ident()
            .unwrap_or_else(|| self.placeholder_ident_at(self.stream.position()));
        let generics = self.parse_generics_decl();
        let (params, close, _broken) = self.parse_params();
        let param_end = match (&close, params.last()) {
            (Some(span), _) => span.hi,
            (None, Some(param)) => param_span(param).hi,
            (None, None) => name.span.hi,
        };
        let ret = if self.stream.eat_tag(T::Arrow).is_some() {
            Some(self.parse_type_entry())
        } else {
            None
        };
        let sig_hi = ret.as_ref().map_or(param_end, |ty| ty.span.hi);
        let sig_span = Span {
            file: self.file,
            lo: name.span.lo,
            hi: sig_hi.max(name.span.lo),
        };
        let sig = FnSig {
            name,
            params,
            ret,
            span: sig_span,
        };
        let body = self.parse_block();
        let span = Span {
            file: self.file,
            lo,
            hi: body.span.hi.max(lo),
        };
        Some(FnDecl {
            vis,
            generics,
            sig,
            body,
            span,
        })
    }

    /// Parses a parameter list for function declarations, signatures, and
    /// methods: `( [params] )`.
    ///
    /// Returns the parameters, the closing `)` span when present, and
    /// whether recovery already handled the closer position.
    fn parse_params(&mut self) -> (Vec<Param>, Option<Span>, bool) {
        let mut params = Vec::new();
        let mut broken = false;
        if self.stream.eat_tag(T::LParen).is_none() {
            let span = self.stream.current_span();
            let message = messages::expected("'('");
            self.error(message, span);
            broken = true;
        }
        if !broken {
            if self.stream.check_tag(T::RParen) {
                let close = self.stream.bump().map(|token| token.span);
                return (params, close, false);
            }
            loop {
                match self.parse_param() {
                    Some(param) => {
                        params.push(param);
                        if self.stream.check_tag(T::Comma) {
                            self.stream.bump();
                            if self.stream.check_tag(T::RParen) {
                                break;
                            }
                            continue;
                        }
                    }
                    None => {
                        // `parse_param` already reported; recover quietly so
                        // the parameter error is reported once (N3/N28).
                        if self.stream.check_tag(T::Comma) {
                            self.stream.bump();
                            if self.stream.check_tag(T::RParen) {
                                break;
                            }
                            continue;
                        }
                        if self.stream.check_tag(T::RParen) {
                            break;
                        }
                        self.synchronize(PARAM_SYNC);
                        if self.stream.check_tag(T::Comma) {
                            self.stream.bump();
                            if self.stream.check_tag(T::RParen) {
                                break;
                            }
                            continue;
                        }
                        if self.stream.check_tag(T::RParen) {
                            break;
                        }
                        broken = true;
                        break;
                    }
                }
                if self.stream.check_tag(T::RParen) {
                    break;
                }
                let span = self.stream.current_span();
                let message = messages::expected_or(",", ")");
                self.error(message, span);
                self.synchronize(PARAM_SYNC);
                if self.stream.check_tag(T::Comma) {
                    self.stream.bump();
                    if self.stream.check_tag(T::RParen) {
                        break;
                    }
                    continue;
                }
                if self.stream.check_tag(T::RParen) {
                    break;
                }
                broken = true;
                break;
            }
        }
        let close = if broken {
            self.stream.bump_if_tag(T::RParen).map(|token| token.span)
        } else {
            self.expect_tag(T::RParen, "')'").map(|token| token.span)
        };
        (params, close, broken)
    }

    /// Parses one function parameter.
    fn parse_param(&mut self) -> Option<Param> {
        let is_self = match self.stream.peek() {
            Some(Token {
                kind: TokenKind::Ident(name),
                ..
            }) => {
                name == "self"
                    && !self
                        .stream
                        .peek_n(1)
                        .is_some_and(|next| T::Colon.matches(&next.kind))
            }
            _ => false,
        };
        if is_self {
            let token = self.stream.bump()?;
            return Some(Param::Receiver(token.span));
        }
        if self.stream.check_tag(T::Ident) {
            let name = self.expect_ident()?;
            if self.stream.eat_tag(T::Colon).is_none() {
                let span = self.stream.current_span();
                let message = messages::expected("':'");
                self.error(message, span);
            }
            let ty = self.parse_type_entry();
            let span = Span {
                file: self.file,
                lo: name.span.lo,
                hi: ty.span.hi.max(name.span.lo),
            };
            return Some(Param::Named { name, ty, span });
        }
        let span = self.stream.current_span();
        let message = messages::expected("parameter");
        self.error(message, span);
        None
    }

    /// Parses a struct declaration (at `struct`).
    fn parse_struct_item(&mut self, lo: u32, vis: Visibility) -> Option<StructDecl> {
        self.stream.bump()?; // `struct`
        let name = self
            .expect_ident()
            .unwrap_or_else(|| self.placeholder_ident_at(self.stream.position()));
        let generics = self.parse_generics_decl();
        let mut fields = Vec::new();
        let mut end = name.span.hi;
        if self.expect_tag(T::LBrace, "'{'").is_some() {
            loop {
                if self.stream.check_tag(T::RBrace) || self.stream.at_eof() {
                    break;
                }
                let before = self.stream.mark();
                if let Some(field) = self.parse_field() {
                    end = field.span.hi;
                    fields.push(field);
                }
                if self.stream.check_tag(T::Comma) {
                    self.stream.bump();
                    continue;
                }
                if self.stream.check_tag(T::RBrace) {
                    break;
                }
                let span = self.stream.current_span();
                let message = messages::expected_or(",", "}");
                self.error(message, span);
                self.synchronize(FIELD_SYNC);
                if self.stream.check_tag(T::Comma) {
                    self.stream.bump();
                    continue;
                }
                self.ensure_progress(before);
                if self.stream.check_tag(T::RBrace) {
                    break;
                }
            }
            if let Some(close) = self.stream.eat_tag(T::RBrace) {
                end = close.span.hi;
            } else {
                let span = self.stream.current_span();
                let message = messages::expected("'}'");
                self.error(message, span);
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: end.max(lo),
        };
        Some(StructDecl {
            vis,
            name,
            generics,
            fields,
            span,
        })
    }

    /// Parses one struct field: `[pub] ident : type`.
    fn parse_field(&mut self) -> Option<Field> {
        let lo = self.stream.position();
        let vis = self.parse_visibility();
        if !self.stream.check_tag(T::Ident) {
            let span = self.stream.current_span();
            let message = messages::expected("identifier");
            self.error(message, span);
            self.synchronize(FIELD_SYNC);
            return None;
        }
        let name = self.expect_ident()?;
        let ty = if self.stream.eat_tag(T::Colon).is_some() {
            self.parse_type_entry()
        } else {
            let span = self.stream.current_span();
            let message = messages::expected("':'");
            self.error(message, span);
            self.placeholder_type_at(self.stream.position())
        };
        let span = Span {
            file: self.file,
            lo,
            hi: ty.span.hi.max(lo),
        };
        Some(Field {
            vis,
            name,
            ty,
            span,
        })
    }

    /// Parses an enum declaration (at `enum`).
    fn parse_enum_item(&mut self, lo: u32, vis: Visibility) -> Option<EnumDecl> {
        self.stream.bump()?; // `enum`
        let name = self
            .expect_ident()
            .unwrap_or_else(|| self.placeholder_ident_at(self.stream.position()));
        let generics = self.parse_generics_decl();
        let mut variants = Vec::new();
        let mut end = name.span.hi;
        if self.expect_tag(T::LBrace, "'{'").is_some() {
            loop {
                if self.stream.check_tag(T::RBrace) || self.stream.at_eof() {
                    break;
                }
                let before = self.stream.mark();
                if let Some(variant) = self.parse_variant() {
                    end = variant.span.hi;
                    variants.push(variant);
                }
                if self.stream.check_tag(T::Comma) {
                    self.stream.bump();
                    continue;
                }
                if self.stream.check_tag(T::RBrace) {
                    break;
                }
                let span = self.stream.current_span();
                let message = messages::expected_or(",", "}");
                self.error(message, span);
                self.synchronize(FIELD_SYNC);
                if self.stream.check_tag(T::Comma) {
                    self.stream.bump();
                    continue;
                }
                self.ensure_progress(before);
                if self.stream.check_tag(T::RBrace) {
                    break;
                }
            }
            if let Some(close) = self.stream.eat_tag(T::RBrace) {
                end = close.span.hi;
            } else {
                let span = self.stream.current_span();
                let message = messages::expected("'}'");
                self.error(message, span);
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: end.max(lo),
        };
        Some(EnumDecl {
            vis,
            name,
            generics,
            variants,
            span,
        })
    }

    /// Parses one enum variant: `Ident` or `Ident(T, …)`.
    fn parse_variant(&mut self) -> Option<Variant> {
        if !self.stream.check_tag(T::Ident) {
            let span = self.stream.current_span();
            let message = messages::expected("identifier");
            self.error(message, span);
            self.synchronize(FIELD_SYNC);
            return None;
        }
        let name = self.expect_ident()?;
        let mut payload = Vec::new();
        let mut hi = name.span.hi;
        if self.stream.eat_tag(T::LParen).is_some() {
            if self.stream.check_tag(T::RParen) {
                if let Some(close) = self.stream.bump() {
                    hi = close.span.hi;
                }
            } else {
                loop {
                    payload.push(self.parse_type_entry());
                    if self.stream.check_tag(T::Comma) {
                        self.stream.bump();
                        if self.stream.check_tag(T::RParen) {
                            break;
                        }
                        continue;
                    }
                    if self.stream.check_tag(T::RParen) {
                        break;
                    }
                    let span = self.stream.current_span();
                    let message = messages::expected_or(",", ")");
                    self.error(message, span);
                    self.synchronize(crate::types::TYPE_LIST_SYNC);
                    if self.stream.check_tag(T::Comma) {
                        self.stream.bump();
                        if self.stream.check_tag(T::RParen) {
                            break;
                        }
                        continue;
                    }
                    break;
                }
                if let Some(close) = self.stream.eat_tag(T::RParen) {
                    hi = close.span.hi;
                } else {
                    let span = self.stream.current_span();
                    let message = messages::expected("')'");
                    self.error(message, span);
                }
            }
        }
        let span = Span {
            file: self.file,
            lo: name.span.lo,
            hi: hi.max(name.span.lo),
        };
        Some(Variant {
            name,
            payload,
            span,
        })
    }

    /// Parses an `impl` block (at `impl`).
    fn parse_impl_item(&mut self, lo: u32) -> Option<ImplDecl> {
        self.stream.bump()?; // `impl`
        let generics = self.parse_generics_decl();
        let first = self.parse_path_type_entry();
        let (trait_, self_ty) = if self.stream.eat_tag(T::For).is_some() {
            (Some(first), self.parse_path_type_entry())
        } else {
            (None, first)
        };
        let mut items = Vec::new();
        let mut end = self_ty.span.hi;
        if self.expect_tag(T::LBrace, "'{'").is_some() {
            loop {
                if self.stream.check_tag(T::RBrace) || self.stream.at_eof() {
                    break;
                }
                let before = self.stream.mark();
                if let Some(item) = self.parse_impl_member() {
                    end = impl_item_span(&item).hi;
                    items.push(item);
                }
                self.ensure_progress(before);
            }
            if let Some(close) = self.stream.eat_tag(T::RBrace) {
                end = close.span.hi;
            } else {
                let span = self.stream.current_span();
                let message = messages::expected("'}'");
                self.error(message, span);
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: end.max(lo),
        };
        Some(ImplDecl {
            generics,
            trait_,
            self_ty,
            items,
            span,
        })
    }

    /// Parses one `impl` member: `[pub] fn … { … }`.
    fn parse_impl_member(&mut self) -> Option<ImplItem> {
        let lo = self.stream.position();
        let vis = self.parse_visibility();
        if self.stream.check_tag(T::Fn) {
            let decl = self.parse_fn_item(lo, vis)?;
            return Some(ImplItem::Fn(decl));
        }
        let span = self.stream.current_span();
        let message = messages::expected("'fn'");
        self.error(message, span);
        let consumed = self.synchronize(IMPL_SYNC);
        if !consumed {
            self.stream.bump();
        }
        None
    }

    /// Parses a trait declaration (at `trait`).
    fn parse_trait_item(&mut self, lo: u32, vis: Visibility) -> Option<TraitDecl> {
        self.stream.bump()?; // `trait`
        let name = self
            .expect_ident()
            .unwrap_or_else(|| self.placeholder_ident_at(self.stream.position()));
        let generics = self.parse_generics_decl();
        let mut items = Vec::new();
        let mut end = name.span.hi;
        if self.expect_tag(T::LBrace, "'{'").is_some() {
            loop {
                if self.stream.check_tag(T::RBrace) || self.stream.at_eof() {
                    break;
                }
                let before = self.stream.mark();
                if let Some(item) = self.parse_trait_member() {
                    end = item.span.hi;
                    items.push(item);
                }
                self.ensure_progress(before);
            }
            if let Some(close) = self.stream.eat_tag(T::RBrace) {
                end = close.span.hi;
            } else {
                let span = self.stream.current_span();
                let message = messages::expected("'}'");
                self.error(message, span);
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: end.max(lo),
        };
        Some(TraitDecl {
            vis,
            name,
            generics,
            items,
            span,
        })
    }

    /// Parses one signature-only trait member: `fn … ;`.
    fn parse_trait_member(&mut self) -> Option<TraitItem> {
        let lo = self.stream.position();
        if self.stream.eat_tag(T::Fn).is_none() {
            let span = self.stream.current_span();
            let message = messages::expected("'fn'");
            self.error(message, span);
            let consumed = self.synchronize(IMPL_SYNC);
            if !consumed {
                self.stream.bump();
            }
            return None;
        }
        let sig = self.parse_fn_sig();
        let mut hi = sig.span.hi;
        if let Some(semi) = self.stream.eat_tag(T::Semi) {
            hi = semi.span.hi;
        } else {
            let span = self.stream.current_span();
            let message = messages::expected("';'");
            self.error(message, span);
            self.synchronize(IMPL_SYNC);
            if let Some(semi) = self.stream.eat_tag(T::Semi) {
                hi = semi.span.hi;
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: hi.max(lo),
        };
        Some(TraitItem { sig, span })
    }

    /// Parses `IDENT [generics] ( params ) [-> type]` (after `fn`).
    fn parse_fn_sig(&mut self) -> FnSig {
        let name = self
            .expect_ident()
            .unwrap_or_else(|| self.placeholder_ident_at(self.stream.position()));
        let _generics = self.parse_generics_decl();
        let (params, close, _broken) = self.parse_params();
        let param_end = match (&close, params.last()) {
            (Some(span), _) => span.hi,
            (None, Some(param)) => param_span(param).hi,
            (None, None) => name.span.hi,
        };
        let ret = if self.stream.eat_tag(T::Arrow).is_some() {
            Some(self.parse_type_entry())
        } else {
            None
        };
        let sig_hi = ret.as_ref().map_or(param_end, |ty| ty.span.hi);
        let span = Span {
            file: self.file,
            lo: name.span.lo,
            hi: sig_hi.max(name.span.lo),
        };
        FnSig {
            name,
            params,
            ret,
            span,
        }
    }

    /// Parses an import declaration (at `import`).
    fn parse_import(&mut self, lo: u32) -> Option<Item> {
        self.stream.bump()?; // `import`
        let mut path = Vec::new();
        let first = match self.expect_ident() {
            Some(ident) => ident,
            None => {
                self.synchronize(IMPORT_SYNC);
                self.stream.bump_if_tag(T::Semi);
                return None;
            }
        };
        path.push(first);
        loop {
            if self.stream.check_tag(T::Dot) {
                self.stream.bump();
                match self.expect_ident() {
                    Some(ident) => path.push(ident),
                    None => {
                        self.synchronize(IMPORT_SYNC);
                        break;
                    }
                }
            } else if self.stream.check_tag(T::ColonColon) {
                let span = self.stream.current_span();
                let message = messages::expected_or(".", ";");
                self.error(message, span);
                self.synchronize(IMPORT_SYNC);
                break;
            } else {
                break;
            }
        }
        let alias = if self.stream.eat_tag(T::As).is_some() {
            self.expect_ident()
        } else {
            None
        };
        let mut hi = path.last().map_or(lo, |ident| ident.span.hi);
        if let Some(ident) = &alias {
            hi = ident.span.hi;
        }
        if let Some(semi) = self.stream.eat_tag(T::Semi) {
            hi = semi.span.hi;
        } else {
            let span = self.stream.current_span();
            let message = messages::expected("';'");
            self.error(message, span);
            self.synchronize(IMPORT_SYNC);
            if let Some(semi) = self.stream.eat_tag(T::Semi) {
                hi = semi.span.hi;
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: hi.max(lo),
        };
        Some(Item {
            id: self.new_item_id(),
            kind: ItemKind::Import(ImportDecl { path, alias, span }),
            span,
        })
    }

    // ------------------------------------------------------------------
    // Blocks and statements
    // ------------------------------------------------------------------

    /// Parses a brace-delimited block (depth-guarded).
    pub(crate) fn parse_block(&mut self) -> Block {
        if !self.enter() {
            let at = self.stream.position();
            self.leave();
            return Block {
                stmts: Vec::new(),
                tail: None,
                span: self.zero_span(at),
            };
        }
        let result = self.parse_block_inner();
        self.leave();
        result
    }

    fn parse_block_inner(&mut self) -> Block {
        let open = self.stream.eat_tag(T::LBrace);
        let lo = open
            .as_ref()
            .map_or_else(|| self.stream.position(), |token| token.span.lo);
        if open.is_none() {
            let span = self.stream.current_span();
            let message = messages::expected("'{'");
            self.error(message, span);
        }
        let mut stmts = Vec::new();
        let mut tail = None;
        loop {
            if self.stream.at_eof() {
                if open.is_some() {
                    let span = self.stream.current_span();
                    let message = messages::expected("'}'");
                    self.error(message, span);
                }
                break;
            }
            if self.stream.check_tag(T::RBrace) {
                break;
            }
            let before = self.stream.mark();
            match self.parse_stmt_or_tail() {
                Some(StmtOrTail::Stmt(stmt)) => stmts.push(*stmt),
                Some(StmtOrTail::Tail(expr)) => {
                    tail = Some(expr);
                    break;
                }
                None => {}
            }
            self.ensure_progress(before);
        }
        let hi = match self.stream.eat_tag(T::RBrace) {
            Some(close) => close.span.hi,
            None => self.stream.position(),
        };
        let span = Span {
            file: self.file,
            lo,
            hi: hi.max(lo),
        };
        Block { stmts, tail, span }
    }

    /// Parses one statement position, applying the accepted classification
    /// order: `;` → statement; `}`/EOF → tail; block-like → semicolon-free
    /// statement; otherwise `expected ';'` with recovery.
    fn parse_stmt_or_tail(&mut self) -> Option<StmtOrTail> {
        if self.at_item_start() {
            let item = self.parse_item()?;
            let span = item.span;
            return Some(StmtOrTail::Stmt(Box::new(Stmt {
                kind: StmtKind::Item(item),
                span,
            })));
        }
        if self.stream.check_tag(T::Let) {
            return Some(StmtOrTail::Stmt(Box::new(self.parse_let_stmt())));
        }
        if self.stream.check_tag(T::Semi) {
            // No empty statements (`;;`): report and skip.
            let span = self.stream.current_span();
            let message = messages::expected("expression");
            self.error(message, span);
            self.stream.bump();
            return None;
        }
        let expr = self.parse_expr_bp(0);
        if let Some(semi) = self.stream.eat_tag(T::Semi) {
            let span = Span {
                file: self.file,
                lo: expr.span.lo,
                hi: semi.span.hi.max(expr.span.lo),
            };
            return Some(StmtOrTail::Stmt(Box::new(Stmt {
                kind: StmtKind::Expr(expr),
                span,
            })));
        }
        if self.stream.check_tag(T::RBrace) || self.stream.at_eof() {
            return Some(StmtOrTail::Tail(Box::new(expr)));
        }
        if is_block_like(&expr) {
            let span = expr.span;
            return Some(StmtOrTail::Stmt(Box::new(Stmt {
                kind: StmtKind::Expr(expr),
                span,
            })));
        }
        let span = self.stream.current_span();
        let message = messages::expected("';'");
        self.error(message, span);
        self.synchronize(STMT_SYNC);
        self.stream.bump_if_tag(T::Semi);
        let span = expr.span;
        Some(StmtOrTail::Stmt(Box::new(Stmt {
            kind: StmtKind::Expr(expr),
            span,
        })))
    }

    /// Parses a `let` declaration.
    fn parse_let_stmt(&mut self) -> Stmt {
        let lo = self.stream.bump().map_or(0, |token| token.span.lo);
        let mutable = self.stream.eat_tag(T::Mut).is_some();
        let pat = self.parse_pattern();
        let ty = if self.stream.eat_tag(T::Colon).is_some() {
            Some(self.parse_type_entry())
        } else {
            None
        };
        let init = if self.stream.eat_tag(T::Eq).is_some() {
            self.parse_expr_bp(0)
        } else {
            let span = self.stream.current_span();
            let message = messages::expected("'='");
            self.error(message, span);
            // Initializer placeholder; synchronize at the statement end.
            self.synchronize(LET_SYNC);
            self.placeholder_expr_at(self.stream.position())
        };
        let mut hi = init.span.hi.max(pat.span.hi).max(lo);
        if let Some(semi) = self.stream.eat_tag(T::Semi) {
            hi = semi.span.hi;
        } else if !self.stream.check_tag(T::RBrace) && !self.stream.at_eof() {
            let span = self.stream.current_span();
            let message = messages::expected("';'");
            self.error(message, span);
            self.synchronize(STMT_SYNC);
            if let Some(semi) = self.stream.eat_tag(T::Semi) {
                hi = semi.span.hi;
            }
        }
        let span = Span {
            file: self.file,
            lo,
            hi: hi.max(lo),
        };
        Stmt {
            kind: StmtKind::Let(LetStmt {
                mutable,
                pat,
                ty,
                init,
                span,
            }),
            span,
        }
    }
}

/// Returns the `KindTag` category of a token kind.
pub(crate) fn token_tag(kind: &TokenKind) -> KindTag {
    use KindTag as Tag;
    match kind {
        TokenKind::Ident(_) => Tag::Ident,
        TokenKind::Fn => Tag::Fn,
        TokenKind::Struct => Tag::Struct,
        TokenKind::Enum => Tag::Enum,
        TokenKind::Impl => Tag::Impl,
        TokenKind::Trait => Tag::Trait,
        TokenKind::Import => Tag::Import,
        TokenKind::Pub => Tag::Pub,
        TokenKind::Let => Tag::Let,
        TokenKind::Mut => Tag::Mut,
        TokenKind::If => Tag::If,
        TokenKind::Else => Tag::Else,
        TokenKind::In => Tag::In,
        TokenKind::For => Tag::For,
        TokenKind::As => Tag::As,
        TokenKind::Underscore => Tag::Underscore,
        TokenKind::Semi => Tag::Semi,
        TokenKind::RBrace => Tag::RBrace,
        TokenKind::RParen => Tag::RParen,
        TokenKind::RBracket => Tag::RBracket,
        TokenKind::LBrace => Tag::LBrace,
        TokenKind::LParen => Tag::LParen,
        TokenKind::Comma => Tag::Comma,
        TokenKind::Colon => Tag::Colon,
        TokenKind::ColonColon => Tag::ColonColon,
        TokenKind::Dot => Tag::Dot,
        TokenKind::Question => Tag::Question,
        TokenKind::LBracket => Tag::LBracket,
        TokenKind::FatArrow => Tag::FatArrow,
        TokenKind::Arrow => Tag::Arrow,
        TokenKind::Eq => Tag::Eq,
        TokenKind::Lt => Tag::Lt,
        TokenKind::Gt => Tag::Gt,
        TokenKind::Ge => Tag::Ge,
        TokenKind::Error => Tag::Error,
        TokenKind::Eof => Tag::Eof,
        _ => Tag::Other,
    }
}

/// Returns the span of a parameter.
pub(crate) fn param_span(param: &Param) -> Span {
    match param {
        Param::Receiver(span) => *span,
        Param::Named { span, .. } => *span,
    }
}

/// Returns the span of an `impl` item.
pub(crate) fn impl_item_span(item: &ImplItem) -> Span {
    match item {
        ImplItem::Fn(decl) => decl.span,
    }
}

/// Returns `true` for the block-like expression kinds of the accepted
/// classification rule (`parser-ast-contract.md` §3(2) rule 3).
pub(crate) fn is_block_like(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        xl_ast::ExprKind::If(..)
            | xl_ast::ExprKind::Match(..)
            | xl_ast::ExprKind::While(..)
            | xl_ast::ExprKind::For(..)
            | xl_ast::ExprKind::Loop(..)
            | xl_ast::ExprKind::Block(..)
    )
}
