//! Statement, expression, pattern, type, and block lowering.
//!
//! Child indices follow the frozen schema in [`crate::origin`]; the
//! lowering traversal is the single source of those paths.

use xl_ast::{
    Block, Expr, ExprKind, Lit, Pat, PatKind, PathType, Stmt, StmtKind, StringPart, Type, TypeKind,
};

use super::{Lowerer, PatCtx};
use crate::fate::Aborted;
use crate::hir::{
    BodyId, BuiltinOp, DefKind, ExprKind as HirExpr, FieldSel, HirId, HirKind, HirLit,
    PatKind as HirPat, StmtKind as HirStmt, StringLit as HirStringLit, TypeRefId, TypeRefKind,
    Visibility,
};
use crate::messages;
use crate::origin::{AnchorId, AstOrigin};

/// Returns `true` when `expr` is a parser-recovered placeholder.
///
/// The accepted parser's only synthesized span convention is a zero-width
/// span; recovered expression placeholders are zero-width empty tuples
/// (`parser.rs`, "Placeholders, identity, spans"). A written `()` always
/// spans both parentheses, so the shape is unambiguous.
pub(super) fn is_placeholder_expr(expr: &Expr) -> bool {
    expr.span.is_empty() && matches!(&expr.kind, ExprKind::Tuple(elements) if elements.is_empty())
}

/// Returns `true` when `pat` is a parser-recovered placeholder (a
/// zero-width wildcard).
pub(super) fn is_placeholder_pat(pat: &Pat) -> bool {
    pat.span.is_empty() && matches!(&pat.kind, PatKind::Wildcard)
}

/// Returns `true` when `ty` is a parser-recovered placeholder (a
/// zero-width empty tuple, which is the placeholder type payload).
pub(super) fn is_placeholder_type(ty: &Type) -> bool {
    ty.span.is_empty() && matches!(&ty.kind, TypeKind::Tuple(elements) if elements.is_empty())
}

/// Returns `true` when a string literal contains interpolated parts.
fn has_interpolation(string: &xl_ast::StringLit) -> bool {
    string
        .parts
        .iter()
        .any(|part| matches!(part, StringPart::Expr(_)))
}

/// The merged decoded text of an all-literal string literal.
fn merged_text(string: &xl_ast::StringLit) -> String {
    let mut text = String::new();
    for part in &string.parts {
        if let StringPart::Text(segment) = part {
            text.push_str(&segment.text);
        }
    }
    text
}

impl Lowerer {
    /// The innermost body whose scopes contain the current construct.
    ///
    /// Pattern bindings are only recorded inside a lowered body; the
    /// fallback keeps the function panic-free.
    pub(super) fn active_body(&self) -> BodyId {
        self.current_body.unwrap_or(BodyId::from_index(0))
    }

    // ------------------------------------------------------------------
    // Blocks and statements
    // ------------------------------------------------------------------

    /// Lowers a block into a body.
    ///
    /// Bodies are structural glue between nodes and do not contribute to
    /// the nesting chain (`MAX_HIR_DEPTH` counts expressions, statements,
    /// patterns, and types; every nested block adds at least one counted
    /// node, so block nesting remains bounded).
    pub(super) fn lower_block(&mut self, block: &Block) -> Result<BodyId, Aborted> {
        let origin = self.structural_origin();
        let body_id = self.alloc_body(block.span, origin)?;
        let saved_body = self.current_body.replace(body_id);
        let mut stmts = Vec::with_capacity(block.stmts.len());
        for (index, stmt) in block.stmts.iter().enumerate() {
            self.push_child(index as u32);
            let lowered = self.lower_stmt(stmt);
            self.pop_child();
            stmts.push(lowered?);
        }
        let tail = match &block.tail {
            Some(tail) => {
                self.push_child(block.stmts.len() as u32);
                let lowered = self.lower_expr(tail);
                self.pop_child();
                Some(lowered?)
            }
            None => None,
        };
        self.current_body = saved_body;
        self.fill_body(body_id, stmts, tail);
        Ok(body_id)
    }

    /// Lowers one statement.
    pub(super) fn lower_stmt(&mut self, stmt: &Stmt) -> Result<HirId, Aborted> {
        let origin = self.structural_origin();
        self.hir_depth += 1;
        let result = if self.hir_depth > self.limits.max_hir_depth {
            self.report_depth_exceeded(stmt.span);
            self.alloc_node(HirKind::Expr(HirExpr::Error), stmt.span, origin.clone())
                .and_then(|error| {
                    self.alloc_node(
                        HirKind::Stmt(HirStmt::Expr(error)),
                        stmt.span,
                        origin.clone(),
                    )
                })
        } else {
            self.lower_stmt_kind(stmt, origin)
        };
        self.hir_depth -= 1;
        result
    }

    pub(super) fn lower_stmt_kind(
        &mut self,
        stmt: &Stmt,
        origin: AstOrigin,
    ) -> Result<HirId, Aborted> {
        match &stmt.kind {
            StmtKind::Let(let_stmt) => {
                // child 1: the type annotation (lowered first so every
                // binding of the pattern can share it; the schema index is
                // fixed either way).
                let annotation = match &let_stmt.ty {
                    Some(ty) => {
                        self.push_child(1);
                        let lowered = self.lower_type(ty);
                        self.pop_child();
                        Some(lowered?)
                    }
                    None => None,
                };
                // child 0: the pattern
                self.push_child(0);
                let pattern = self.lower_pat(
                    &let_stmt.pat,
                    PatCtx {
                        mutable: let_stmt.mutable,
                        body: self.active_body(),
                        ty: annotation,
                    },
                    false,
                );
                self.pop_child();
                let pattern = pattern?;
                // child 2: the initializer
                self.push_child(2);
                let init = self.lower_expr(&let_stmt.init);
                self.pop_child();
                let init = init?;
                self.alloc_node(
                    HirKind::Stmt(HirStmt::Let { pattern, init }),
                    stmt.span,
                    origin,
                )
            }
            StmtKind::Expr(expr) => {
                self.push_child(0);
                let lowered = self.lower_expr(expr);
                self.pop_child();
                let expr = lowered?;
                self.alloc_node(HirKind::Stmt(HirStmt::Expr(expr)), stmt.span, origin)
            }
            StmtKind::Item(item) => {
                self.push_child(0);
                let lowered = self.lower_nested_item(item);
                self.pop_child();
                lowered?;
                let item_id =
                    crate::hir::HirItemId::from_index((self.module.items.len() - 1) as u32);
                self.alloc_node(HirKind::Stmt(HirStmt::Item(item_id)), stmt.span, origin)
            }
        }
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    /// Lowers one expression.
    ///
    /// Recovered placeholders and depth-poisoned subtrees become error
    /// nodes, which are poison markers and do not contribute to the
    /// nesting chain (the validator counts them the same way, so every
    /// lowered module validates at its limit boundary).
    pub(super) fn lower_expr(&mut self, expr: &Expr) -> Result<HirId, Aborted> {
        let span = expr.span;
        let origin = AstOrigin::Expr(expr.id);
        if is_placeholder_expr(expr) {
            return self.alloc_node(HirKind::Expr(HirExpr::Error), span, origin);
        }
        self.hir_depth += 1;
        let result = if self.hir_depth > self.limits.max_hir_depth {
            self.report_depth_exceeded(span);
            self.alloc_node(HirKind::Expr(HirExpr::Error), span, origin)
        } else {
            self.with_anchor(AnchorId::Expr(expr.id), |lowerer| {
                lowerer.lower_expr_kind(expr)
            })
        };
        self.hir_depth -= 1;
        result
    }

    pub(super) fn lower_expr_kind(&mut self, expr: &Expr) -> Result<HirId, Aborted> {
        let span = expr.span;
        let origin = AstOrigin::Expr(expr.id);
        match &expr.kind {
            ExprKind::Lit(Lit::Str(string)) if has_interpolation(string) => {
                self.lower_interpolation(expr, string)
            }
            ExprKind::Lit(lit) => {
                let lowered = HirLit::from_ast(lit);
                self.alloc_node(HirKind::Expr(HirExpr::Lit(lowered)), span, origin)
            }
            ExprKind::Path(path) => {
                let mut segments = Vec::with_capacity(path.segments.len());
                for (index, segment) in path.segments.iter().enumerate() {
                    self.push_child(index as u32);
                    let use_ = self.alloc_source_use(
                        segment.name.clone(),
                        segment.span,
                        self.structural_origin(),
                    );
                    self.pop_child();
                    segments.push(use_?);
                }
                self.alloc_node(HirKind::Expr(HirExpr::Path { segments }), span, origin)
            }
            ExprKind::Unary(op, operand) => {
                self.push_child(0);
                let operand = self.lower_expr(operand);
                self.pop_child();
                let operand = operand?;
                self.alloc_node(HirKind::Expr(HirExpr::Unary(*op, operand)), span, origin)
            }
            ExprKind::Binary(op, left, right) => {
                self.push_child(0);
                let left = self.lower_expr(left);
                self.pop_child();
                let left = left?;
                self.push_child(1);
                let right = self.lower_expr(right);
                self.pop_child();
                let right = right?;
                self.alloc_node(
                    HirKind::Expr(HirExpr::Binary(*op, left, right)),
                    span,
                    origin,
                )
            }
            ExprKind::Assign(op, target, value) => {
                self.push_child(0);
                let target = self.lower_expr(target);
                self.pop_child();
                let target = target?;
                self.push_child(1);
                let value = self.lower_expr(value);
                self.pop_child();
                let value = value?;
                self.alloc_node(
                    HirKind::Expr(HirExpr::Assign(*op, target, value)),
                    span,
                    origin,
                )
            }
            ExprKind::Call(callee, args) => {
                self.push_child(0);
                let callee = self.lower_expr(callee);
                self.pop_child();
                let callee = callee?;
                let mut lowered_args = Vec::with_capacity(args.len());
                for (index, arg) in args.iter().enumerate() {
                    self.push_child(1 + index as u32);
                    let lowered = self.lower_expr(arg);
                    self.pop_child();
                    lowered_args.push(lowered?);
                }
                self.alloc_node(
                    HirKind::Expr(HirExpr::Call(callee, lowered_args)),
                    span,
                    origin,
                )
            }
            ExprKind::Field(base, name) => {
                self.push_child(0);
                let base = self.lower_expr(base);
                self.pop_child();
                let base = base?;
                self.push_child(1);
                let name_use =
                    self.alloc_source_use(name.name.clone(), name.span, self.structural_origin());
                self.pop_child();
                let name_use = name_use?;
                self.alloc_node(
                    HirKind::Expr(HirExpr::Field(
                        base,
                        FieldSel::Unresolved { name: name_use },
                    )),
                    span,
                    origin,
                )
            }
            ExprKind::Index(base, index) => {
                self.push_child(0);
                let base = self.lower_expr(base);
                self.pop_child();
                let base = base?;
                self.push_child(1);
                let index = self.lower_expr(index);
                self.pop_child();
                let index = index?;
                self.alloc_node(HirKind::Expr(HirExpr::Index(base, index)), span, origin)
            }
            ExprKind::Range(low, high) => {
                self.push_child(0);
                let low = self.lower_expr(low);
                self.pop_child();
                let low = low?;
                self.push_child(1);
                let high = self.lower_expr(high);
                self.pop_child();
                let high = high?;
                self.alloc_node(
                    HirKind::Expr(HirExpr::BuiltinCall {
                        op: BuiltinOp::RangeNew,
                        args: vec![low, high],
                    }),
                    span,
                    origin,
                )
            }
            ExprKind::Cast(inner, ty) => {
                self.push_child(0);
                let inner = self.lower_expr(inner);
                self.pop_child();
                let inner = inner?;
                self.push_child(1);
                let ty = self.lower_type(ty);
                self.pop_child();
                let ty = ty?;
                self.alloc_node(HirKind::Expr(HirExpr::Cast(inner, ty)), span, origin)
            }
            ExprKind::Try(operand) => self.lower_try(expr, operand),
            ExprKind::If(cond, then_block, else_expr) => {
                self.push_child(0);
                let cond = self.lower_expr(cond);
                self.pop_child();
                let cond = cond?;
                self.push_child(1);
                let then_body = self.lower_block(then_block);
                self.pop_child();
                let then_body = then_body?;
                let else_node = match else_expr {
                    Some(else_expr) => {
                        self.push_child(2);
                        let lowered = self.lower_expr(else_expr);
                        self.pop_child();
                        Some(lowered?)
                    }
                    None => None,
                };
                self.alloc_node(
                    HirKind::Expr(HirExpr::If(cond, then_body, else_node)),
                    span,
                    origin,
                )
            }
            ExprKind::Match(scrutinee, arms) => {
                self.push_child(0);
                let scrutinee = self.lower_expr(scrutinee);
                self.pop_child();
                let scrutinee = scrutinee?;
                let mut lowered_arms = Vec::with_capacity(arms.len());
                for (index, arm) in arms.iter().enumerate() {
                    self.push_child(1 + index as u32);
                    // child 0: the pattern
                    self.push_child(0);
                    let pat = self.lower_pat(
                        &arm.pat,
                        PatCtx {
                            mutable: false,
                            body: self.active_body(),
                            ty: None,
                        },
                        false,
                    );
                    self.pop_child();
                    let pat = pat?;
                    // child 1: the body
                    self.push_child(1);
                    let body = self.lower_expr(&arm.body);
                    self.pop_child();
                    self.pop_child();
                    lowered_arms.push(crate::hir::Arm {
                        pat,
                        body: body?,
                        span: arm.span,
                    });
                }
                self.alloc_node(
                    HirKind::Expr(HirExpr::Match(scrutinee, lowered_arms)),
                    span,
                    origin,
                )
            }
            ExprKind::Block(block) => {
                self.push_child(0);
                let body = self.lower_block(block);
                self.pop_child();
                let body = body?;
                self.alloc_node(HirKind::Expr(HirExpr::Block(body)), span, origin)
            }
            ExprKind::While(cond, block) => {
                self.push_child(0);
                let cond = self.lower_expr(cond);
                self.pop_child();
                let cond = cond?;
                // The loop node is reserved first: the body's
                // `break`/`continue` target it.
                let loop_node = self.reserve_node(span, origin.clone())?;
                self.loop_stack.push(loop_node);
                self.push_child(1);
                let body = self.lower_block(block);
                self.pop_child();
                self.loop_stack.pop();
                let body = body?;
                self.fill_node(loop_node, HirKind::Expr(HirExpr::While(cond, body)), span);
                Ok(loop_node)
            }
            ExprKind::Loop(block) => {
                let loop_node = self.reserve_node(span, origin.clone())?;
                self.loop_stack.push(loop_node);
                self.push_child(0);
                let body = self.lower_block(block);
                self.pop_child();
                self.loop_stack.pop();
                let body = body?;
                self.fill_node(loop_node, HirKind::Expr(HirExpr::Loop(body)), span);
                Ok(loop_node)
            }
            ExprKind::For(pat, iterable, block) => self.lower_for(expr, pat, iterable, block),
            ExprKind::Return(operand) => {
                let operand_node = match operand {
                    Some(operand) => {
                        self.push_child(0);
                        let lowered = self.lower_expr(operand);
                        self.pop_child();
                        Some(lowered?)
                    }
                    None => None,
                };
                self.alloc_node(HirKind::Expr(HirExpr::Return(operand_node)), span, origin)
            }
            ExprKind::Break => self.lower_loop_jump(span, origin, true),
            ExprKind::Continue => self.lower_loop_jump(span, origin, false),
            ExprKind::Tuple(elements) => {
                let mut lowered = Vec::with_capacity(elements.len());
                for (index, element) in elements.iter().enumerate() {
                    self.push_child(index as u32);
                    let element = self.lower_expr(element);
                    self.pop_child();
                    lowered.push(element?);
                }
                self.alloc_node(HirKind::Expr(HirExpr::Tuple(lowered)), span, origin)
            }
            ExprKind::Array(elements) => {
                let mut lowered = Vec::with_capacity(elements.len());
                for (index, element) in elements.iter().enumerate() {
                    self.push_child(index as u32);
                    let element = self.lower_expr(element);
                    self.pop_child();
                    lowered.push(element?);
                }
                self.alloc_node(HirKind::Expr(HirExpr::Array(lowered)), span, origin)
            }
        }
    }

    /// Lowers `break`/`continue`: the target is the nearest enclosing loop
    /// node in the same body (recorded structurally, never across a
    /// function boundary).
    ///
    /// The grammar admits these expressions anywhere; outside any loop the
    /// construct cannot be represented, so lowering poisons it with an
    /// error node and a fail-closed diagnostic instead of fabricating a
    /// target.
    pub(super) fn lower_loop_jump(
        &mut self,
        span: xl_span::Span,
        origin: AstOrigin,
        is_break: bool,
    ) -> Result<HirId, Aborted> {
        match self.loop_stack.last().copied() {
            Some(target) => {
                let kind = if is_break {
                    HirExpr::Break { target }
                } else {
                    HirExpr::Continue { target }
                };
                self.alloc_node(HirKind::Expr(kind), span, origin)
            }
            None => {
                let message = if is_break {
                    messages::break_outside_loop()
                } else {
                    messages::continue_outside_loop()
                };
                self.diagnostics
                    .push(xl_diag::Diagnostic::error(message, span));
                self.alloc_node(HirKind::Expr(HirExpr::Error), span, origin)
            }
        }
    }

    // ------------------------------------------------------------------
    // Patterns
    // ------------------------------------------------------------------

    /// Lowers one pattern.
    ///
    /// `recovered_as_wildcard` selects the `for`-desugaring behavior for
    /// parser-recovered placeholder patterns (`lowering-contract.md`
    /// §4.5): placeholders become wildcards there and error nodes
    /// everywhere else.
    pub(super) fn lower_pat(
        &mut self,
        pat: &Pat,
        ctx: PatCtx,
        recovered_as_wildcard: bool,
    ) -> Result<HirId, Aborted> {
        let origin = self.structural_origin();
        if is_placeholder_pat(pat) {
            let kind = if recovered_as_wildcard {
                HirPat::Wildcard
            } else {
                HirPat::Error
            };
            return self.alloc_node(HirKind::Pat(kind), pat.span, origin);
        }
        self.hir_depth += 1;
        let result = if self.hir_depth > self.limits.max_hir_depth {
            self.report_depth_exceeded(pat.span);
            self.alloc_node(HirKind::Pat(HirPat::Error), pat.span, origin)
        } else {
            self.lower_pat_kind(pat, ctx, recovered_as_wildcard, origin)
        };
        self.hir_depth -= 1;
        result
    }

    pub(super) fn lower_pat_kind(
        &mut self,
        pat: &Pat,
        ctx: PatCtx,
        recovered_as_wildcard: bool,
        origin: AstOrigin,
    ) -> Result<HirId, Aborted> {
        match &pat.kind {
            PatKind::Wildcard => self.alloc_node(HirKind::Pat(HirPat::Wildcard), pat.span, origin),
            PatKind::Binding(ident) => {
                let local = self.alloc_def(
                    ident.name.clone(),
                    DefKind::Local {
                        mutable: ctx.mutable,
                        ty: ctx.ty,
                        body: ctx.body,
                    },
                    ident.span,
                    Visibility::Private,
                    None,
                    self.structural_origin(),
                )?;
                let candidate = self.alloc_source_use(
                    ident.name.clone(),
                    ident.span,
                    self.structural_origin(),
                )?;
                self.alloc_node(
                    HirKind::Pat(HirPat::Binding { local, candidate }),
                    pat.span,
                    origin,
                )
            }
            PatKind::Lit(Lit::Str(string)) if has_interpolation(string) => {
                // A pattern must be a compile-time constant; interpolation
                // is not representable in one. Poison with a fail-closed
                // diagnostic (new at this stage).
                self.diagnostics.push(xl_diag::Diagnostic::error(
                    messages::interpolation_in_pattern(),
                    string.span,
                ));
                self.alloc_node(HirKind::Pat(HirPat::Error), pat.span, origin)
            }
            PatKind::Lit(lit) => {
                let lowered = HirLit::from_ast(lit);
                self.alloc_node(HirKind::Pat(HirPat::Lit(lowered)), pat.span, origin)
            }
            PatKind::Path { path, args } => {
                let mut segments = Vec::with_capacity(path.segments.len());
                for (index, segment) in path.segments.iter().enumerate() {
                    self.push_child(index as u32);
                    let use_ = self.alloc_source_use(
                        segment.name.clone(),
                        segment.span,
                        self.structural_origin(),
                    );
                    self.pop_child();
                    segments.push(use_?);
                }
                let mut lowered_args = Vec::new();
                if let Some(args) = args {
                    for (index, arg) in args.iter().enumerate() {
                        self.push_child(segments.len() as u32 + index as u32);
                        let lowered = self.lower_pat(arg, ctx, recovered_as_wildcard);
                        self.pop_child();
                        lowered_args.push(lowered?);
                    }
                }
                self.alloc_node(
                    HirKind::Pat(HirPat::Variant {
                        path: segments,
                        args: lowered_args,
                    }),
                    pat.span,
                    origin,
                )
            }
            PatKind::Tuple(elements) => {
                let mut lowered = Vec::with_capacity(elements.len());
                for (index, element) in elements.iter().enumerate() {
                    self.push_child(index as u32);
                    let element = self.lower_pat(element, ctx, recovered_as_wildcard);
                    self.pop_child();
                    lowered.push(element?);
                }
                self.alloc_node(HirKind::Pat(HirPat::Tuple(lowered)), pat.span, origin)
            }
        }
    }

    // ------------------------------------------------------------------
    // Types
    // ------------------------------------------------------------------

    /// Lowers one syntactic type.
    pub(super) fn lower_type(&mut self, ty: &Type) -> Result<TypeRefId, Aborted> {
        let origin = self.structural_origin();
        if is_placeholder_type(ty) {
            return self.alloc_type(TypeRefKind::Error, ty.span, origin);
        }
        self.hir_depth += 1;
        let result = if self.hir_depth > self.limits.max_hir_depth {
            self.report_depth_exceeded(ty.span);
            self.alloc_type(TypeRefKind::Error, ty.span, origin)
        } else {
            self.lower_type_kind(ty, origin)
        };
        self.hir_depth -= 1;
        result
    }

    pub(super) fn lower_type_kind(
        &mut self,
        ty: &Type,
        origin: AstOrigin,
    ) -> Result<TypeRefId, Aborted> {
        match &ty.kind {
            TypeKind::Path(path_type) => self.lower_path_type_at(path_type, origin),
            TypeKind::Tuple(elements) => {
                let mut lowered = Vec::with_capacity(elements.len());
                for (index, element) in elements.iter().enumerate() {
                    self.push_child(index as u32);
                    let element = self.lower_type(element);
                    self.pop_child();
                    lowered.push(element?);
                }
                self.alloc_type(TypeRefKind::Tuple(lowered), ty.span, origin)
            }
            TypeKind::Slice(element) => {
                self.push_child(0);
                let element = self.lower_type(element);
                self.pop_child();
                let element = element?;
                self.alloc_type(TypeRefKind::Slice(element), ty.span, origin)
            }
            TypeKind::Array(element, length) => {
                self.push_child(0);
                let element = self.lower_type(element);
                self.pop_child();
                let element = element?;
                self.push_child(1);
                let length = self.lower_expr(length);
                self.pop_child();
                let length = length?;
                self.alloc_type(
                    TypeRefKind::Array {
                        elem: element,
                        len: length,
                    },
                    ty.span,
                    origin,
                )
            }
            TypeKind::Fn { params, ret } => {
                let mut lowered_params = Vec::with_capacity(params.len());
                for (index, param) in params.iter().enumerate() {
                    self.push_child(index as u32);
                    let lowered = self.lower_type(param);
                    self.pop_child();
                    lowered_params.push(lowered?);
                }
                let lowered_ret = match ret {
                    Some(ret) => {
                        self.push_child(params.len() as u32);
                        let lowered = self.lower_type(ret);
                        self.pop_child();
                        Some(lowered?)
                    }
                    None => None,
                };
                self.alloc_type(
                    TypeRefKind::Fn {
                        params: lowered_params,
                        ret: lowered_ret,
                    },
                    ty.span,
                    origin,
                )
            }
        }
    }

    /// Lowers a type path, applying the contextual `Self` rule
    /// (`P2D-B14`): inside a trait or impl, a lone `Self` is the internal
    /// `SelfTy` (not a name use); everywhere else it is an ordinary path.
    pub(super) fn lower_path_type(&mut self, path_type: &PathType) -> Result<TypeRefId, Aborted> {
        let origin = self.structural_origin();
        self.lower_path_type_at(path_type, origin)
    }

    pub(super) fn lower_path_type_at(
        &mut self,
        path_type: &PathType,
        origin: AstOrigin,
    ) -> Result<TypeRefId, Aborted> {
        if self.self_ctx > 0
            && path_type.segments.len() == 1
            && path_type.segments[0].name == "Self"
            && path_type.generics.is_none()
        {
            return self.alloc_type(TypeRefKind::SelfTy, path_type.span, origin);
        }
        let segment_count = path_type.segments.len() as u32;
        let mut segments = Vec::with_capacity(path_type.segments.len());
        for (index, segment) in path_type.segments.iter().enumerate() {
            self.push_child(index as u32);
            let use_ =
                self.alloc_source_use(segment.name.clone(), segment.span, self.structural_origin());
            self.pop_child();
            segments.push(use_?);
        }
        let mut generics = Vec::new();
        if let Some(arguments) = &path_type.generics {
            for (index, argument) in arguments.iter().enumerate() {
                self.push_child(segment_count + index as u32);
                let lowered = self.lower_type(argument);
                self.pop_child();
                generics.push(lowered?);
            }
        }
        self.alloc_type(
            TypeRefKind::Path { segments, generics },
            path_type.span,
            origin,
        )
    }
}

impl HirLit {
    /// Lowers an accepted literal payload, merging all-literal string
    /// parts. (Interpolated strings never reach this function; they are
    /// expanded by the interpolation desugaring.)
    pub(crate) fn from_ast(lit: &Lit) -> Self {
        match lit {
            Lit::Int(int) => HirLit::Int {
                magnitude: int.magnitude,
                radix: int.radix,
            },
            Lit::Float(value) => HirLit::Float(*value),
            Lit::Bool(value) => HirLit::Bool(*value),
            Lit::Char(value) => HirLit::Char(*value),
            Lit::Str(string) => HirLit::Str(HirStringLit {
                text: merged_text(string),
                span: string.span,
            }),
        }
    }
}
