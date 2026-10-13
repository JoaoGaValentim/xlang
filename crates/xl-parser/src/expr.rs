//! Expression parsing: Pratt loop, unary/postfix chains, primaries, and
//! structured-string interpolation.
//!
//! Binding powers are the accepted values (`grammar-rfc.md` §4.1): the
//! Pratt loop tracks the tier of the operator it applied last, so a second
//! operator of a non-associative tier is diagnosed and recovered with the
//! deterministic shapes (right-nested for assignment, left-nested for
//! ranges and comparisons). Grouping is transparent in the AST but is not
//! transparent to chain detection: a parenthesized operand resets the
//! provenance tracker, so `(a < b) < c` is accepted while `a < b < c` is
//! diagnosed.

use xl_ast::{
    Arm, AssignOp, BinOp, Expr, ExprKind, Ident, Lit, Path, StringLit, StringPart, TextSegment,
    UnOp,
};
use xl_lexer::{Token, TokenKind};
use xl_span::Span;

use crate::messages;
use crate::parser::{Parser, token_tag};
use crate::stream::KindTag;

use KindTag as Tag;

/// Call/argument list synchronization set; `}` is never consumed so an
/// enclosing block can still close (its brace is left for the block).
const CALL_SYNC: &[KindTag] = &[Tag::Comma, Tag::RParen, Tag::RBrace, Tag::Eof];

/// Array-literal synchronization set (`}` left for the block).
const ARRAY_SYNC: &[KindTag] = &[Tag::Comma, Tag::RBracket, Tag::RBrace, Tag::Eof];

/// Match-arm synchronization set.
const MATCH_SYNC: &[KindTag] = &[Tag::Comma, Tag::RBrace, Tag::Eof];

/// The precedence tiers of the accepted table, lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    Assign,
    Range,
    Or,
    And,
    Cmp,
    Add,
    Mul,
}

impl Tier {
    /// Returns `true` for the non-associative tiers (assignment, range,
    /// comparison), which are enforced explicitly by the Pratt loop.
    fn is_non_assoc(self) -> bool {
        matches!(self, Tier::Assign | Tier::Range | Tier::Cmp)
    }
}

/// The operator token flavor at an infix position.
#[derive(Debug, Clone, Copy)]
enum OpToken {
    Assign(AssignOp),
    Binary(BinOp),
    Range,
}

/// An infix operator with its tier and binding powers.
#[derive(Debug, Clone, Copy)]
struct OpInfo {
    tier: Tier,
    lbp: u8,
    rbp: u8,
    op: OpToken,
}

/// Maps the current token category to its infix operator info, when any.
fn peek_op(kind: &TokenKind) -> Option<OpInfo> {
    use TokenKind as K;
    let (tier, lbp, rbp, op) = match kind {
        K::Eq => (Tier::Assign, 2, 3, OpToken::Assign(AssignOp::Assign)),
        K::PlusEq => (Tier::Assign, 2, 3, OpToken::Assign(AssignOp::AddAssign)),
        K::MinusEq => (Tier::Assign, 2, 3, OpToken::Assign(AssignOp::SubAssign)),
        K::StarEq => (Tier::Assign, 2, 3, OpToken::Assign(AssignOp::MulAssign)),
        K::SlashEq => (Tier::Assign, 2, 3, OpToken::Assign(AssignOp::DivAssign)),
        K::PercentEq => (Tier::Assign, 2, 3, OpToken::Assign(AssignOp::RemAssign)),
        K::DotDot => (Tier::Range, 4, 5, OpToken::Range),
        K::OrOr => (Tier::Or, 6, 7, OpToken::Binary(BinOp::Or)),
        K::AndAnd => (Tier::And, 8, 9, OpToken::Binary(BinOp::And)),
        K::EqEq => (Tier::Cmp, 10, 11, OpToken::Binary(BinOp::Eq)),
        K::NotEq => (Tier::Cmp, 10, 11, OpToken::Binary(BinOp::Ne)),
        K::Lt => (Tier::Cmp, 10, 11, OpToken::Binary(BinOp::Lt)),
        K::Le => (Tier::Cmp, 10, 11, OpToken::Binary(BinOp::Le)),
        K::Gt => (Tier::Cmp, 10, 11, OpToken::Binary(BinOp::Gt)),
        K::Ge => (Tier::Cmp, 10, 11, OpToken::Binary(BinOp::Ge)),
        K::Plus => (Tier::Add, 12, 13, OpToken::Binary(BinOp::Add)),
        K::Minus => (Tier::Add, 12, 13, OpToken::Binary(BinOp::Sub)),
        K::Star => (Tier::Mul, 14, 15, OpToken::Binary(BinOp::Mul)),
        K::Slash => (Tier::Mul, 14, 15, OpToken::Binary(BinOp::Div)),
        K::Percent => (Tier::Mul, 14, 15, OpToken::Binary(BinOp::Rem)),
        _ => return None,
    };
    Some(OpInfo { tier, lbp, rbp, op })
}

impl Parser {
    /// Parses an expression at the given minimum binding power
    /// (depth-guarded; the single expression recursion entry point).
    pub(crate) fn parse_expr_bp(&mut self, min_bp: u8) -> Expr {
        if !self.enter() {
            let at = self.stream.position();
            self.leave();
            return self.placeholder_expr_at(at);
        }
        let result = self.parse_expr_bp_inner(min_bp);
        self.leave();
        result
    }

    fn parse_expr_bp_inner(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();
        let mut last_tier: Option<Tier> = None;
        while let Some(info) = self.stream.peek().and_then(|token| peek_op(&token.kind)) {
            if info.lbp < min_bp {
                break;
            }
            let op_span = self.stream.current_span();
            if info.tier.is_non_assoc() && last_tier == Some(info.tier) {
                // Same-tier repetition on a non-associative tier: diagnose
                // and recover with the accepted deterministic shape.
                self.stream.bump();
                let rhs = self.parse_expr_bp(info.rbp);
                match info.op {
                    OpToken::Assign(op) => {
                        self.error(messages::chained_assignment(), op_span);
                        lhs = self.restructure_right_nested(lhs, op, rhs);
                    }
                    OpToken::Binary(op) => {
                        self.error(messages::chained_comparison(), op_span);
                        lhs = self.make_binary(op, lhs, rhs);
                    }
                    OpToken::Range => {
                        self.error(messages::chained_range(), op_span);
                        lhs = self.make_range(lhs, rhs);
                    }
                }
                last_tier = Some(info.tier);
                continue;
            }
            self.stream.bump();
            let rhs = self.parse_expr_bp(info.rbp);
            match info.op {
                OpToken::Assign(op) => {
                    self.check_assignment_target(&lhs);
                    lhs = self.make_assign(op, lhs, rhs);
                }
                OpToken::Binary(op) => {
                    lhs = self.make_binary(op, lhs, rhs);
                }
                OpToken::Range => {
                    lhs = self.make_range(lhs, rhs);
                }
            }
            last_tier = Some(info.tier);
        }
        lhs
    }

    /// Builds a binary expression node.
    fn make_binary(&mut self, op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        let span = self.join(lhs.span, rhs.span);
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
            span,
        }
    }

    /// Builds a range expression node.
    fn make_range(&mut self, lhs: Expr, rhs: Expr) -> Expr {
        let span = self.join(lhs.span, rhs.span);
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Range(Box::new(lhs), Box::new(rhs)),
            span,
        }
    }

    /// Builds an assignment expression node.
    fn make_assign(&mut self, op: AssignOp, lhs: Expr, rhs: Expr) -> Expr {
        let span = self.join(lhs.span, rhs.span);
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Assign(op, Box::new(lhs), Box::new(rhs)),
            span,
        }
    }

    /// Restructures a just-built assignment into the right-nested recovery
    /// shape for chained assignment: `t op r1` + `op r2` becomes
    /// `t op (r1 op r2)`.
    fn restructure_right_nested(&mut self, lhs: Expr, op: AssignOp, rhs: Expr) -> Expr {
        let lhs_id = lhs.id;
        let lhs_span = lhs.span;
        let rhs_span = rhs.span;
        match lhs.kind {
            ExprKind::Assign(first_op, target, prev_rhs) => {
                let target_span = target.span;
                let prev_span = prev_rhs.span;
                let inner_span = self.join(prev_span, rhs_span);
                let inner = Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Assign(op, prev_rhs, Box::new(rhs)),
                    span: inner_span,
                };
                let outer_span = self.join(target_span, inner_span);
                Expr {
                    id: lhs_id,
                    kind: ExprKind::Assign(first_op, target, Box::new(inner)),
                    span: outer_span,
                }
            }
            other => {
                let lhs_expr = Expr {
                    id: lhs_id,
                    kind: other,
                    span: lhs_span,
                };
                let span = self.join(lhs_span, rhs_span);
                Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Assign(op, Box::new(lhs_expr), Box::new(rhs)),
                    span,
                }
            }
        }
    }

    /// Validates an assignment left-hand side: `Path`, `Field`, or `Index`
    /// (grouping is transparent because it produces no AST node).
    fn check_assignment_target(&mut self, expr: &Expr) {
        let ok = matches!(
            expr.kind,
            ExprKind::Path(..) | ExprKind::Field(..) | ExprKind::Index(..)
        );
        if !ok {
            let span = expr.span;
            self.error(messages::invalid_assignment_target(), span);
        }
    }

    /// Parses prefix unary operators (tier 8; operand at min-bp 16) or a
    /// postfix chain.
    pub(crate) fn parse_prefix(&mut self) -> Expr {
        let unary = match self.stream.peek() {
            Some(Token {
                kind: TokenKind::Minus,
                ..
            }) => Some(UnOp::Neg),
            Some(Token {
                kind: TokenKind::Bang,
                ..
            }) => Some(UnOp::Not),
            _ => None,
        };
        if let Some(op) = unary {
            if !self.enter() {
                let at = self.stream.position();
                self.leave();
                return self.placeholder_expr_at(at);
            }
            let lo = self.stream.bump().map_or(0, |token| token.span.lo);
            let operand = self.parse_prefix();
            self.leave();
            let span = Span {
                file: self.file,
                lo,
                hi: operand.span.hi.max(lo),
            };
            return Expr {
                id: self.new_expr_id(),
                kind: ExprKind::Unary(op, Box::new(operand)),
                span,
            };
        }
        self.parse_postfix()
    }

    /// Parses a primary expression followed by the left-to-right postfix
    /// chain: calls, field access, indexing, `?`, and `as` casts (tier 9).
    pub(crate) fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.stream.check_tag(Tag::LParen) {
                self.stream.bump();
                let (args, close) = self.parse_expr_list(Tag::RParen, ")", CALL_SYNC);
                let hi = close.map_or_else(|| self.stream.position(), |span| span.hi);
                let span = Span {
                    file: self.file,
                    lo: expr.span.lo,
                    hi: hi.max(expr.span.lo),
                };
                expr = Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Call(Box::new(expr), args),
                    span,
                };
            } else if self.stream.check_tag(Tag::Dot) {
                self.stream.bump();
                let ident = if self.stream.check_tag(Tag::Ident) {
                    match self.stream.bump() {
                        Some(Token {
                            kind: TokenKind::Ident(name),
                            span,
                        }) => Ident { name, span },
                        _ => self.placeholder_ident_at(self.stream.position()),
                    }
                } else {
                    // `x.0` / `1.` / `1.e3`: tuple-field access does not
                    // exist; report and continue with a placeholder name.
                    let span = self.stream.current_span();
                    self.error(messages::expected("field name"), span);
                    self.placeholder_ident_at(self.stream.position())
                };
                let span = Span {
                    file: self.file,
                    lo: expr.span.lo,
                    hi: ident.span.hi.max(expr.span.lo),
                };
                expr = Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Field(Box::new(expr), ident),
                    span,
                };
            } else if self.stream.check_tag(Tag::LBracket) {
                self.stream.bump();
                let index = self.parse_expr_bp(0);
                let close = self.expect_tag(Tag::RBracket, "']'");
                let hi = close.map_or_else(|| self.stream.position(), |token| token.span.hi);
                let span = Span {
                    file: self.file,
                    lo: expr.span.lo,
                    hi: hi.max(expr.span.lo),
                };
                expr = Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Index(Box::new(expr), Box::new(index)),
                    span,
                };
            } else if self.stream.check_tag(Tag::Question) {
                let hi = self
                    .stream
                    .bump()
                    .map_or_else(|| self.stream.position(), |token| token.span.hi);
                let span = Span {
                    file: self.file,
                    lo: expr.span.lo,
                    hi: hi.max(expr.span.lo),
                };
                expr = Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Try(Box::new(expr)),
                    span,
                };
            } else if self.stream.check_tag(Tag::As) {
                self.stream.bump();
                let ty = self.parse_type_inner();
                let span = Span {
                    file: self.file,
                    lo: expr.span.lo,
                    hi: ty.span.hi.max(expr.span.lo),
                };
                expr = Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Cast(Box::new(expr), ty),
                    span,
                };
            } else {
                break;
            }
        }
        expr
    }

    /// Parses a delimited expression list: `(item, … ,)` or `[item, … ,]`
    /// with the closing delimiter consumed when present.
    fn parse_expr_list(
        &mut self,
        close: Tag,
        close_name: &str,
        sync: &[Tag],
    ) -> (Vec<Expr>, Option<Span>) {
        let mut items = Vec::new();
        if self.stream.check_tag(close) {
            return (items, self.stream.bump().map(|token| token.span));
        }
        loop {
            items.push(self.parse_expr_bp(0));
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                if self.stream.check_tag(close) {
                    return (items, self.stream.bump().map(|token| token.span));
                }
                continue;
            }
            if self.stream.check_tag(close) {
                return (items, self.stream.bump().map(|token| token.span));
            }
            let span = self.stream.current_span();
            let message = messages::expected_or(",", close_name);
            self.error(message, span);
            self.synchronize(sync);
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                continue;
            }
            if self.stream.check_tag(close) {
                return (items, self.stream.bump().map(|token| token.span));
            }
            return (items, None);
        }
    }

    /// Parses a primary expression (tiers 1–9 atoms).
    pub(crate) fn parse_primary(&mut self) -> Expr {
        // `Error` tokens are lexical recovery artifacts whose diagnostics
        // were already emitted by the lexer; skip them and keep parsing.
        while self.stream.check_tag(Tag::Error) {
            self.stream.bump();
        }
        let Some(token) = self.stream.peek().cloned() else {
            let span = self.stream.current_span();
            self.error(messages::expected("expression"), span);
            return self.placeholder_expr_at(self.stream.position());
        };
        let span = token.span;
        match token.kind {
            TokenKind::Int(value) => {
                self.stream.bump();
                self.lit_expr(Lit::Int(convert_int(value)), span)
            }
            TokenKind::Float(value) => {
                self.stream.bump();
                self.lit_expr(Lit::Float(value), span)
            }
            TokenKind::Char(value) => {
                self.stream.bump();
                self.lit_expr(Lit::Char(value), span)
            }
            TokenKind::True => {
                self.stream.bump();
                self.lit_expr(Lit::Bool(true), span)
            }
            TokenKind::False => {
                self.stream.bump();
                self.lit_expr(Lit::Bool(false), span)
            }
            TokenKind::Str(literal) => {
                self.stream.bump();
                let parts = self.parse_string_parts(literal);
                self.lit_expr(Lit::Str(StringLit { parts, span }), span)
            }
            TokenKind::Ident(name) => {
                self.stream.bump();
                let first = Ident { name, span };
                let path = self.finish_path(first);
                let path_span = path.span;
                Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Path(path),
                    span: path_span,
                }
            }
            TokenKind::LParen => self.parse_paren_expr(span),
            TokenKind::LBracket => self.parse_array_expr(span),
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::While => self.parse_while_expr(),
            TokenKind::For => self.parse_for_expr(),
            TokenKind::Loop => self.parse_loop_expr(),
            TokenKind::LBrace => {
                let block = self.parse_block();
                let block_span = block.span;
                Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Block(block),
                    span: block_span,
                }
            }
            TokenKind::Return => {
                self.stream.bump();
                let operand = if self.at_expr_end() {
                    None
                } else {
                    Some(Box::new(self.parse_expr_bp(0)))
                };
                let hi = operand
                    .as_ref()
                    .map_or(span.hi, |expr| expr.span.hi.max(span.hi));
                Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Return(operand),
                    span: Span {
                        file: self.file,
                        lo: span.lo,
                        hi,
                    },
                }
            }
            TokenKind::Break => {
                self.stream.bump();
                Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Break,
                    span,
                }
            }
            TokenKind::Continue => {
                self.stream.bump();
                Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Continue,
                    span,
                }
            }
            TokenKind::Underscore => {
                // `_` is a pattern token, never an expression.
                self.stream.bump();
                let message = messages::expected("expression");
                self.error(message, span);
                self.placeholder_expr_at(self.stream.position())
            }
            _ => {
                let message = messages::expected("expression");
                self.error(message, span);
                self.synchronize(crate::parser::EXPR_SYNC);
                self.placeholder_expr_at(self.stream.position())
            }
        }
    }

    /// Builds a literal expression node.
    fn lit_expr(&mut self, lit: Lit, span: Span) -> Expr {
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Lit(lit),
            span,
        }
    }

    /// Finishes a `::`-separated path after its first identifier.
    pub(crate) fn finish_path(&mut self, first: Ident) -> Path {
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
        Path {
            segments,
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Parses `()`, `(e)`, or `(e, …)`; grouping is transparent.
    fn parse_paren_expr(&mut self, open: Span) -> Expr {
        self.stream.bump(); // `(`
        if self.stream.check_tag(Tag::RParen) {
            let close = self.stream.bump().map_or(open, |token| token.span);
            let span = self.join(open, close);
            return Expr {
                id: self.new_expr_id(),
                kind: ExprKind::Tuple(Vec::new()),
                span,
            };
        }
        let first = self.parse_expr_bp(0);
        if self.stream.check_tag(Tag::Comma) {
            let mut elements = vec![first];
            loop {
                self.stream.bump(); // `,`
                if self.stream.check_tag(Tag::RParen) {
                    break;
                }
                elements.push(self.parse_expr_bp(0));
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
            Expr {
                id: self.new_expr_id(),
                kind: ExprKind::Tuple(elements),
                span,
            }
        } else {
            let _ = self.expect_tag(Tag::RParen, "')'");
            // Grouping produces no node: the inner expression is returned
            // unchanged, span included (`parser-ast-contract.md` §2.3(6)).
            first
        }
    }

    /// Parses an array literal `[e, …]`.
    fn parse_array_expr(&mut self, open: Span) -> Expr {
        self.stream.bump(); // `[`
        let (elements, close) = self.parse_expr_list(Tag::RBracket, "]", ARRAY_SYNC);
        let hi = close.map_or_else(|| self.stream.position(), |span| span.hi);
        let span = Span {
            file: self.file,
            lo: open.lo,
            hi: hi.max(open.lo),
        };
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Array(elements),
            span,
        }
    }

    /// Parses `if cond block [else (if | block)]`.
    fn parse_if_expr(&mut self) -> Expr {
        let lo = self.stream.bump().map_or(0, |token| token.span.lo);
        let cond = self.parse_expr_bp(0);
        let then_block = self.parse_block();
        let mut hi = then_block.span.hi;
        let else_ = if self.stream.eat_tag(Tag::Else).is_some() {
            if self.stream.check_tag(Tag::If) {
                let nested = self.parse_if_expr();
                hi = nested.span.hi;
                Some(Box::new(nested))
            } else {
                let block = self.parse_block();
                hi = block.span.hi;
                let span = block.span;
                Some(Box::new(Expr {
                    id: self.new_expr_id(),
                    kind: ExprKind::Block(block),
                    span,
                }))
            }
        } else {
            None
        };
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::If(Box::new(cond), then_block, else_),
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Parses `match scrutinee { arms }`.
    fn parse_match_expr(&mut self) -> Expr {
        let lo = self.stream.bump().map_or(0, |token| token.span.lo);
        let scrutinee = self.parse_expr_bp(0);
        let _ = self.expect_tag(Tag::LBrace, "'{'");
        let mut arms = Vec::new();
        loop {
            if self.stream.check_tag(Tag::RBrace) || self.stream.at_eof() {
                break;
            }
            let before = self.stream.mark();
            if !self.at_pattern_start() {
                // The token cannot begin a pattern: report the arm once and
                // skip to the next arm boundary instead of cascading
                // arrow/body diagnostics for the same token.
                let span = self.stream.current_span();
                let message = messages::expected("pattern");
                self.error(message, span);
                self.synchronize(MATCH_SYNC);
                if self.stream.check_tag(Tag::Comma) {
                    self.stream.bump();
                    continue;
                }
                self.ensure_progress_public(before);
                break;
            }
            let pat = self.parse_pattern();
            let _ = self.expect_tag(Tag::FatArrow, "'=>'");
            let body = self.parse_expr_bp(0);
            let span = self.join(pat.span, body.span);
            arms.push(Arm { pat, body, span });
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                continue;
            }
            if self.stream.check_tag(Tag::RBrace) || self.stream.at_eof() {
                break;
            }
            let span = self.stream.current_span();
            let message = messages::expected_or(",", "}");
            self.error(message, span);
            self.synchronize(MATCH_SYNC);
            if self.stream.check_tag(Tag::Comma) {
                self.stream.bump();
                continue;
            }
            self.ensure_progress_public(before);
            break;
        }
        let hi = self
            .stream
            .eat_tag(Tag::RBrace)
            .map_or_else(|| self.stream.position(), |token| token.span.hi);
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Match(Box::new(scrutinee), arms),
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Parses `while cond block`.
    fn parse_while_expr(&mut self) -> Expr {
        let lo = self.stream.bump().map_or(0, |token| token.span.lo);
        let cond = self.parse_expr_bp(0);
        let body = self.parse_block();
        let hi = body.span.hi;
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::While(Box::new(cond), body),
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Parses `for pattern in iterable block`.
    fn parse_for_expr(&mut self) -> Expr {
        let lo = self.stream.bump().map_or(0, |token| token.span.lo);
        let pat = self.parse_pattern();
        let _ = self.expect_tag(Tag::In, "in");
        let iterable = self.parse_expr_bp(0);
        let body = self.parse_block();
        let hi = body.span.hi;
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::For(Box::new(pat), Box::new(iterable), body),
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Parses `loop block`.
    fn parse_loop_expr(&mut self) -> Expr {
        let lo = self.stream.bump().map_or(0, |token| token.span.lo);
        let body = self.parse_block();
        let hi = body.span.hi;
        Expr {
            id: self.new_expr_id(),
            kind: ExprKind::Loop(body),
            span: Span {
                file: self.file,
                lo,
                hi: hi.max(lo),
            },
        }
    }

    /// Returns `true` when the current token can begin a pattern (used by
    /// match-arm recovery to report a stray token once instead of
    /// cascading).
    fn at_pattern_start(&self) -> bool {
        match self.stream.peek() {
            None => false,
            Some(token) => matches!(
                token.kind,
                TokenKind::Ident(_)
                    | TokenKind::Underscore
                    | TokenKind::Int(_)
                    | TokenKind::Float(_)
                    | TokenKind::Char(_)
                    | TokenKind::Str(_)
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::LParen
            ),
        }
    }

    /// Returns `true` when the current token cannot continue a
    /// maximally-extending operand (expression end positions).
    fn at_expr_end(&self) -> bool {
        match self.stream.peek() {
            None => true,
            Some(token) => matches!(
                token_tag(&token.kind),
                Tag::Comma
                    | Tag::Semi
                    | Tag::RBrace
                    | Tag::RParen
                    | Tag::RBracket
                    | Tag::FatArrow
                    | Tag::Eof
            ),
        }
    }

    /// Converts a lexer string literal into ordered AST string parts,
    /// parsing each embedded interpolation with the normal expression
    /// grammar.
    pub(crate) fn parse_string_parts(&mut self, literal: xl_lexer::StrLit) -> Vec<StringPart> {
        let mut parts = Vec::with_capacity(literal.parts.len());
        for part in literal.parts {
            match part {
                xl_lexer::StrPart::Text(text) => parts.push(StringPart::Text(TextSegment {
                    text: text.text,
                    span: text.span,
                })),
                xl_lexer::StrPart::Interp(interpolation) => {
                    let expr = self.parse_interpolation(&interpolation);
                    parts.push(StringPart::Expr(expr));
                }
            }
        }
        parts
    }

    /// Parses one interpolation `{ tokens }` into an expression.
    ///
    /// Lexical `Error` tokens inside the embedded stream are dropped here:
    /// the lexer already reported them, and keeping the parser silent about
    /// them avoids duplicate diagnostics (the lexical ones remain in the
    /// caller's output). An empty interior yields the `empty interpolation`
    /// diagnostic and a zero-width placeholder expression.
    fn parse_interpolation(&mut self, interpolation: &xl_lexer::Interpolation) -> Expr {
        let had_error_markers = interpolation
            .tokens
            .iter()
            .any(|token| Tag::Error.matches(&token.kind));
        let tokens: Vec<Token> = interpolation
            .tokens
            .iter()
            .filter(|token| !Tag::Error.matches(&token.kind))
            .cloned()
            .collect();
        if tokens.is_empty() {
            // `"{}"` (truly empty) is a parser-level diagnostic; an
            // abnormal termination marker means the lexer already reported
            // the unterminated interpolation, so the parser stays silent.
            if !had_error_markers {
                let span = Span {
                    file: self.file,
                    lo: interpolation.open.lo,
                    hi: interpolation.close.hi.max(interpolation.open.lo),
                };
                self.error(messages::empty_interpolation(), span);
            }
            return self.placeholder_expr_at(interpolation.open.hi);
        }
        let saved = self.stream.replace(tokens, interpolation.close.lo);
        let expr = self.parse_expr_bp(0);
        if let Some(token) = self.stream.peek() {
            // Leftover tokens after a complete expression: the closing
            // delimiter is not part of the embedded stream, so this is a
            // malformed interpolation body. When the lexer already reported
            // an abnormal termination (unterminated interpolation/string),
            // the leftovers are its aftermath and are skipped silently.
            if !had_error_markers {
                let span = token.span;
                let message = messages::expected("'}'");
                self.error(message, span);
            }
            while self.stream.bump().is_some() {}
        }
        self.stream = saved;
        expr
    }

    /// Public progress watchdog hook used by loop-style parsers in this
    /// module.
    pub(crate) fn ensure_progress_public(&mut self, mark: usize) {
        self.ensure_progress(mark);
    }
}

/// Converts a lexer integer literal into the AST payload.
pub(crate) fn convert_int(value: xl_lexer::IntLit) -> xl_ast::IntLit {
    let radix = match value.radix {
        xl_lexer::Radix::Dec => xl_ast::Radix::Dec,
        xl_lexer::Radix::Hex => xl_ast::Radix::Hex,
        xl_lexer::Radix::Bin => xl_ast::Radix::Bin,
    };
    xl_ast::IntLit {
        magnitude: value.magnitude,
        radix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_associative_tiers_are_exactly_three() {
        assert!(Tier::Assign.is_non_assoc());
        assert!(Tier::Range.is_non_assoc());
        assert!(Tier::Cmp.is_non_assoc());
        for tier in [Tier::Or, Tier::And, Tier::Add, Tier::Mul] {
            assert!(!tier.is_non_assoc(), "{tier:?}");
        }
    }

    #[test]
    fn binding_powers_match_the_ratified_table() {
        let cases = [
            (TokenKind::Eq, Tier::Assign, 2, 3),
            (TokenKind::DotDot, Tier::Range, 4, 5),
            (TokenKind::OrOr, Tier::Or, 6, 7),
            (TokenKind::AndAnd, Tier::And, 8, 9),
            (TokenKind::Lt, Tier::Cmp, 10, 11),
            (TokenKind::Plus, Tier::Add, 12, 13),
            (TokenKind::Star, Tier::Mul, 14, 15),
        ];
        for (kind, tier, lbp, rbp) in cases {
            let info = peek_op(&kind).expect("operator");
            assert_eq!(info.tier, tier, "{kind:?}");
            assert_eq!(info.lbp, lbp, "{kind:?}");
            assert_eq!(info.rbp, rbp, "{kind:?}");
        }
        assert!(peek_op(&TokenKind::LParen).is_none());
        assert!(peek_op(&TokenKind::Ident(String::new())).is_none());
    }

    #[test]
    fn integer_radix_converts_one_to_one() {
        for (lexer_radix, ast_radix) in [
            (xl_lexer::Radix::Dec, xl_ast::Radix::Dec),
            (xl_lexer::Radix::Hex, xl_ast::Radix::Hex),
            (xl_lexer::Radix::Bin, xl_ast::Radix::Bin),
        ] {
            let converted = convert_int(xl_lexer::IntLit {
                magnitude: 42,
                radix: lexer_radix,
            });
            assert_eq!(converted.magnitude, 42);
            assert_eq!(converted.radix, ast_radix);
        }
    }
}
