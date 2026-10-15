//! The three ratified desugarings: `for`, `?`, and string interpolation
//! (`lowering-contract.md` §4–6; `P2D-B8`, `P2D-B9`, `P2D-B10`).
//!
//! Every synthesized entity is created inside an [`ExpansionFrame`] and
//! receives a [`crate::origin::AstOrigin::Synthesized`] origin whose
//! complete occurrence key `(construct.source, expansion, occurrence)` is
//! unique. Internal bindings are fresh definitions referenced by identity
//! (never by name), so no source identifier can capture them; internal
//! prelude references carry fixed [`InternalDef`] identities and are
//! established at creation — nothing in an expansion is resolved by name.

use xl_ast::{Block, Expr, Pat, StringPart};

use super::{ExpansionFrame, Lowerer, PatCtx};
use crate::fate::Aborted;
use crate::hir::{
    Arm, AssignOp, BodyId, BuiltinOp, DefId, DefKind, ExprKind, HirId, HirKind, InternalDef,
    PatKind, Resolution, StmtKind, StringLit as HirStringLit, UseId, Visibility,
};
use crate::origin::{AstOrigin, AstOriginKey, DesugarKind};
use xl_span::Span;

impl Lowerer {
    // ------------------------------------------------------------------
    // Synthesis helpers (each consumes one occurrence in its frame)
    // ------------------------------------------------------------------

    /// Checks that an expansion's always-present overhead still fits the
    /// nesting budget.
    ///
    /// The overhead is the fixed depth of the expansion's synthesized
    /// skeleton beyond the originating construct (verified against the
    /// validator's real-depth counting by the boundary tests). User
    /// content inside the expansion is counted through the offsets applied
    /// around the corresponding lowering calls.
    pub(super) fn expansion_overhead_fits(&mut self, overhead: u32, span: Span) -> bool {
        if self.hir_depth.saturating_add(overhead) > self.limits.max_hir_depth {
            self.report_depth_exceeded(span);
            false
        } else {
            true
        }
    }

    /// Synthesizes one node.
    pub(super) fn synth_node(
        &mut self,
        frame: &mut ExpansionFrame,
        kind: HirKind,
        span: Span,
    ) -> Result<HirId, Aborted> {
        let origin = self.synth_origin(frame);
        self.alloc_node(kind, span, origin)
    }

    /// Synthesizes one internal local binding.
    pub(super) fn synth_local(
        &mut self,
        frame: &mut ExpansionFrame,
        name: &str,
        mutable: bool,
        body: BodyId,
        span: Span,
    ) -> Result<DefId, Aborted> {
        let origin = self.synth_origin(frame);
        self.alloc_def(
            name.to_string(),
            DefKind::Local {
                mutable,
                ty: None,
                body,
            },
            span,
            Visibility::Private,
            None,
            origin,
        )
    }

    /// Synthesizes one internal reference use.
    pub(super) fn synth_use(
        &mut self,
        frame: &mut ExpansionFrame,
        name: &str,
        span: Span,
        resolution: Resolution,
    ) -> Result<UseId, Aborted> {
        let origin = self.synth_origin(frame);
        self.alloc_use(name.to_string(), span, origin, resolution)
    }

    /// Synthesizes a path expression referencing one use.
    pub(super) fn synth_path(
        &mut self,
        frame: &mut ExpansionFrame,
        use_: UseId,
        span: Span,
    ) -> Result<HirId, Aborted> {
        self.synth_node(
            frame,
            HirKind::Expr(ExprKind::Path {
                segments: vec![use_],
            }),
            span,
        )
    }

    /// Synthesizes a decided binding pattern for one internal local (its
    /// candidate use is pre-established as the local itself).
    pub(super) fn synth_binding_pat(
        &mut self,
        frame: &mut ExpansionFrame,
        local: DefId,
        name: &str,
        span: Span,
    ) -> Result<HirId, Aborted> {
        let candidate = self.synth_use(frame, name, span, Resolution::Def(local))?;
        self.synth_node(
            frame,
            HirKind::Pat(PatKind::Binding { local, candidate }),
            span,
        )
    }

    /// Synthesizes a constructor pattern.
    pub(super) fn synth_variant_pat(
        &mut self,
        frame: &mut ExpansionFrame,
        path: Vec<UseId>,
        args: Vec<HirId>,
        span: Span,
    ) -> Result<HirId, Aborted> {
        self.synth_node(frame, HirKind::Pat(PatKind::Variant { path, args }), span)
    }

    // ------------------------------------------------------------------
    // `?` — error propagation (P2D-B9)
    // ------------------------------------------------------------------

    /// Lowers `operand?` to the approved structural model:
    ///
    /// ```text
    /// match ⟨operand⟩ {
    ///     Ok(__v)  => __v,
    ///     Err(__e) => return Err(__e),
    /// }
    /// ```
    ///
    /// The `Ok`/`Err` references are compiler-owned internal identities,
    /// never name lookups. Typing rules (error-type equality, enclosing
    /// return type) belong to WP-C; this stage only builds the structure.
    ///
    /// The expansion's always-present skeleton reaches depth
    /// `construct + 3` (the `return Err(__e)` chain); the operand sits at
    /// `construct + 1`.
    pub(super) fn lower_try(&mut self, expr: &Expr, operand: &Expr) -> Result<HirId, Aborted> {
        if !self.expansion_overhead_fits(3, expr.span) {
            return self.alloc_node(
                HirKind::Expr(ExprKind::Error),
                expr.span,
                AstOrigin::Expr(expr.id),
            );
        }
        let mut frame = self.enter_expansion(DesugarKind::Try, AstOriginKey::Expr(expr.id));
        let result = (|| -> Result<HirId, Aborted> {
            let span = expr.span;
            // The operand is lowered inside the expansion: a nested
            // desugaring inside it observes this expansion on the stack.
            self.push_child(0);
            let value = self.lower_expr(operand);
            self.pop_child();
            let value = value?;
            let body = self.active_body();

            // `Ok(__v) => __v`
            let v_def = self.synth_local(&mut frame, "__v", false, body, span)?;
            let ok_use = self.synth_use(
                &mut frame,
                "Ok",
                span,
                Resolution::Internal(InternalDef::ResultOk),
            )?;
            let v_binding = self.synth_binding_pat(&mut frame, v_def, "__v", span)?;
            let ok_pat = self.synth_variant_pat(&mut frame, vec![ok_use], vec![v_binding], span)?;
            let v_use = self.synth_use(&mut frame, "__v", span, Resolution::Def(v_def))?;
            let v_path = self.synth_path(&mut frame, v_use, span)?;

            // `Err(__e) => return Err(__e)`
            let e_def = self.synth_local(&mut frame, "__e", false, body, span)?;
            let err_pat_use = self.synth_use(
                &mut frame,
                "Err",
                span,
                Resolution::Internal(InternalDef::ResultErr),
            )?;
            let e_binding = self.synth_binding_pat(&mut frame, e_def, "__e", span)?;
            let err_pat =
                self.synth_variant_pat(&mut frame, vec![err_pat_use], vec![e_binding], span)?;
            let err_callee_use = self.synth_use(
                &mut frame,
                "Err",
                span,
                Resolution::Internal(InternalDef::ResultErr),
            )?;
            let err_callee = self.synth_path(&mut frame, err_callee_use, span)?;
            let e_use = self.synth_use(&mut frame, "__e", span, Resolution::Def(e_def))?;
            let e_path = self.synth_path(&mut frame, e_use, span)?;
            let err_call = self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::Call(err_callee, vec![e_path])),
                span,
            )?;
            let ret = self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::Return(Some(err_call))),
                span,
            )?;

            let arms = vec![
                Arm {
                    pat: ok_pat,
                    body: v_path,
                    span,
                },
                Arm {
                    pat: err_pat,
                    body: ret,
                    span,
                },
            ];
            self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::Match(value, arms)),
                span,
            )
        })();
        self.leave_expansion();
        result
    }

    // ------------------------------------------------------------------
    // `for` — iteration over the internal range protocol (P2D-B8)
    // ------------------------------------------------------------------

    /// Lowers `for pat in iterable { body }` to the approved structural
    /// model (state advanced before the body, so `continue` is correct):
    ///
    /// ```text
    /// {
    ///     let mut __state = ⟨iterable⟩;            // evaluated exactly once
    ///     loop {
    ///         match range_next(__state) {
    ///             Some(__pair) => {
    ///                 let (__item, __next) = __pair;
    ///                 let pat = __item;            // user pattern binds here
    ///                 __state = __next;            // advanced BEFORE the body
    ///                 { ⟨body⟩ }                   // nested block scope
    ///             }
    ///             None => break,
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// The body block keeps its own scope inside the expanded block
    /// (`lowering-contract.md` §4.3), so a same-named `let` in the body
    /// shadows the pattern binding instead of colliding with it.
    pub(super) fn lower_for(
        &mut self,
        expr: &Expr,
        pat: &Pat,
        iterable: &Expr,
        block: &Block,
    ) -> Result<HirId, Aborted> {
        // A recovered iterable poisons the whole construct (no new
        // diagnostics; lowering-contract.md §4.5).
        if super::expr::is_placeholder_expr(iterable) {
            return self.alloc_node(
                HirKind::Expr(ExprKind::Error),
                expr.span,
                AstOrigin::Expr(expr.id),
            );
        }
        // The expansion's always-present skeleton reaches depth
        // `construct + 7` (the deepest fixed chains are the destructuring
        // pattern's elements and the state-advance assignment's operands,
        // at arm-statement + 2); user content is lowered with explicit
        // offsets below so the depth counter tracks the real HIR nesting
        // the validator sees.
        if !self.expansion_overhead_fits(7, expr.span) {
            return self.alloc_node(
                HirKind::Expr(ExprKind::Error),
                expr.span,
                AstOrigin::Expr(expr.id),
            );
        }
        let mut frame = self.enter_expansion(DesugarKind::For, AstOriginKey::Expr(expr.id));
        let result = (|| -> Result<HirId, Aborted> {
            let span = expr.span;

            // `{ let mut __state = ⟨iterable⟩; loop { ... } }`
            let outer_origin = self.synth_origin(&mut frame);
            let outer_body = self.alloc_body(span, outer_origin)?;
            let state_def = self.synth_local(&mut frame, "__state", true, outer_body, span)?;
            // The iterable is the state initializer at real depth
            // `construct + 2`.
            self.push_child(1);
            self.hir_depth = self.hir_depth.saturating_add(1);
            let iterable_lowered = self.lower_expr(iterable);
            self.hir_depth = self.hir_depth.saturating_sub(1);
            self.pop_child();
            let iterable_lowered = iterable_lowered?;
            let state_pat = self.synth_binding_pat(&mut frame, state_def, "__state", span)?;
            let state_let = self.synth_node(
                &mut frame,
                HirKind::Stmt(StmtKind::Let {
                    pattern: state_pat,
                    init: iterable_lowered,
                }),
                span,
            )?;

            // The loop node is reserved: the expansion's body and the
            // user's `break`/`continue` target it.
            let loop_origin = self.synth_origin(&mut frame);
            let loop_node = self.reserve_node(span, loop_origin)?;
            self.loop_stack.push(loop_node);
            let loop_body_origin = self.synth_origin(&mut frame);
            let loop_body = self.alloc_body(span, loop_body_origin)?;

            // `match range_next(__state) { ... }`
            let next_call_use =
                self.synth_use(&mut frame, "__state", span, Resolution::Def(state_def))?;
            let next_call_arg = self.synth_path(&mut frame, next_call_use, span)?;
            let next_call = self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::BuiltinCall {
                    op: BuiltinOp::RangeNext,
                    args: vec![next_call_arg],
                }),
                span,
            )?;

            // `Some(__pair) => { ... }`
            let arm_body_origin = self.synth_origin(&mut frame);
            let arm_body = self.alloc_body(span, arm_body_origin)?;
            let pair_def = self.synth_local(&mut frame, "__pair", false, arm_body, span)?;
            let item_def = self.synth_local(&mut frame, "__item", false, arm_body, span)?;
            let next_def = self.synth_local(&mut frame, "__next", false, arm_body, span)?;

            // `let (__item, __next) = __pair;`
            let pair_use = self.synth_use(&mut frame, "__pair", span, Resolution::Def(pair_def))?;
            let pair_path = self.synth_path(&mut frame, pair_use, span)?;
            let item_pat = self.synth_binding_pat(&mut frame, item_def, "__item", span)?;
            let next_pat = self.synth_binding_pat(&mut frame, next_def, "__next", span)?;
            let pair_pat = self.synth_node(
                &mut frame,
                HirKind::Pat(PatKind::Tuple(vec![item_pat, next_pat])),
                span,
            )?;
            let pair_let = self.synth_node(
                &mut frame,
                HirKind::Stmt(StmtKind::Let {
                    pattern: pair_pat,
                    init: pair_path,
                }),
                span,
            )?;

            // `let pat = __item;` — the user pattern (child 0 of the for).
            // The pattern sits at real depth `construct + 6`.
            self.push_child(0);
            self.hir_depth = self.hir_depth.saturating_add(5);
            let user_pat = self.lower_pat(
                pat,
                PatCtx {
                    mutable: false,
                    body: arm_body,
                    ty: None,
                },
                true,
            );
            self.hir_depth = self.hir_depth.saturating_sub(5);
            self.pop_child();
            let user_pat = user_pat?;
            let item_use = self.synth_use(&mut frame, "__item", span, Resolution::Def(item_def))?;
            let item_path = self.synth_path(&mut frame, item_use, span)?;
            let user_let = self.synth_node(
                &mut frame,
                HirKind::Stmt(StmtKind::Let {
                    pattern: user_pat,
                    init: item_path,
                }),
                span,
            )?;

            // `__state = __next;` (state advanced before the body).
            let assign_lhs_use =
                self.synth_use(&mut frame, "__state", span, Resolution::Def(state_def))?;
            let assign_lhs = self.synth_path(&mut frame, assign_lhs_use, span)?;
            let assign_rhs_use =
                self.synth_use(&mut frame, "__next", span, Resolution::Def(next_def))?;
            let assign_rhs = self.synth_path(&mut frame, assign_rhs_use, span)?;
            let assign = self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::Assign(AssignOp::Assign, assign_lhs, assign_rhs)),
                span,
            )?;
            let assign_stmt =
                self.synth_node(&mut frame, HirKind::Stmt(StmtKind::Expr(assign)), span)?;

            // `{ ⟨body⟩ }` — the body keeps its own nested block scope.
            // The body's statements sit at real depth `construct + 7`.
            self.push_child(2);
            self.hir_depth = self.hir_depth.saturating_add(6);
            let user_body = self.lower_block(block);
            self.hir_depth = self.hir_depth.saturating_sub(6);
            self.pop_child();
            let user_body = user_body?;
            let user_block =
                self.synth_node(&mut frame, HirKind::Expr(ExprKind::Block(user_body)), span)?;
            let user_block_stmt =
                self.synth_node(&mut frame, HirKind::Stmt(StmtKind::Expr(user_block)), span)?;

            self.fill_body(
                arm_body,
                vec![pair_let, user_let, assign_stmt, user_block_stmt],
                None,
            );
            let pair_binding = self.synth_binding_pat(&mut frame, pair_def, "__pair", span)?;
            let some_use = self.synth_use(
                &mut frame,
                "Some",
                span,
                Resolution::Internal(InternalDef::OptionSome),
            )?;
            let some_pat =
                self.synth_variant_pat(&mut frame, vec![some_use], vec![pair_binding], span)?;
            let arm_block =
                self.synth_node(&mut frame, HirKind::Expr(ExprKind::Block(arm_body)), span)?;

            // `None => break`
            let none_use = self.synth_use(
                &mut frame,
                "None",
                span,
                Resolution::Internal(InternalDef::OptionNone),
            )?;
            let none_pat = self.synth_variant_pat(&mut frame, vec![none_use], Vec::new(), span)?;
            let break_node = self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::Break { target: loop_node }),
                span,
            )?;

            let match_node = self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::Match(
                    next_call,
                    vec![
                        Arm {
                            pat: some_pat,
                            body: arm_block,
                            span,
                        },
                        Arm {
                            pat: none_pat,
                            body: break_node,
                            span,
                        },
                    ],
                )),
                span,
            )?;
            let match_stmt =
                self.synth_node(&mut frame, HirKind::Stmt(StmtKind::Expr(match_node)), span)?;

            self.fill_body(loop_body, vec![match_stmt], None);
            self.fill_node(loop_node, HirKind::Expr(ExprKind::Loop(loop_body)), span);
            self.loop_stack.pop();
            self.fill_body(outer_body, vec![state_let], Some(loop_node));
            self.synth_node(&mut frame, HirKind::Expr(ExprKind::Block(outer_body)), span)
        })();
        self.leave_expansion();
        result
    }

    // ------------------------------------------------------------------
    // String interpolation (P2D-B10)
    // ------------------------------------------------------------------

    /// Lowers an interpolated string literal to the approved structural
    /// model: consecutive text parts merged into `Lit(Str)` nodes,
    /// interpolated parts wrapped in `FormatPrim`, and all parts combined
    /// with the internal n-ary `StrConcat`. Parts are evaluated left to
    /// right, exactly once, in source order.
    ///
    /// Empty text groups are dropped in the concatenation; a string with
    /// no interpolation never reaches this function (it stays a
    /// `Lit(Str)`).
    pub(super) fn lower_interpolation(
        &mut self,
        expr: &Expr,
        string: &xl_ast::StringLit,
    ) -> Result<HirId, Aborted> {
        // The always-present skeleton reaches `construct + 2`
        // (`StrConcat` → `FormatPrim` → part expression).
        if !self.expansion_overhead_fits(2, expr.span) {
            return self.alloc_node(
                HirKind::Expr(ExprKind::Error),
                expr.span,
                AstOrigin::Expr(expr.id),
            );
        }
        let mut frame =
            self.enter_expansion(DesugarKind::Interpolation, AstOriginKey::Expr(expr.id));
        let result = (|| -> Result<HirId, Aborted> {
            let span = expr.span;
            let mut args: Vec<HirId> = Vec::new();
            let mut pending_text = String::new();
            let mut pending_lo: Option<u32> = None;
            let mut pending_hi: u32 = 0;

            for (index, part) in string.parts.iter().enumerate() {
                match part {
                    StringPart::Text(segment) => {
                        if pending_lo.is_none() {
                            pending_lo = Some(segment.span.lo);
                        }
                        pending_hi = segment.span.hi;
                        pending_text.push_str(&segment.text);
                    }
                    StringPart::Expr(part_expr) => {
                        self.flush_text_group(
                            &mut frame,
                            &mut args,
                            &mut pending_text,
                            &mut pending_lo,
                            pending_hi,
                        )?;
                        self.push_child(index as u32);
                        // The part sits at real depth `construct + 2`
                        // (inside its `FormatPrim` wrapper).
                        self.hir_depth = self.hir_depth.saturating_add(1);
                        let lowered = self.lower_expr(part_expr);
                        self.hir_depth = self.hir_depth.saturating_sub(1);
                        self.pop_child();
                        let lowered = lowered?;
                        // The wrapper carries the part's span so a
                        // formatting diagnostic points at user text.
                        let wrapped = self.synth_node(
                            &mut frame,
                            HirKind::Expr(ExprKind::BuiltinCall {
                                op: BuiltinOp::FormatPrim,
                                args: vec![lowered],
                            }),
                            part_expr.span,
                        )?;
                        args.push(wrapped);
                    }
                }
            }
            let _ = self.flush_text_group(
                &mut frame,
                &mut args,
                &mut pending_text,
                &mut pending_lo,
                pending_hi,
            )?;
            self.synth_node(
                &mut frame,
                HirKind::Expr(ExprKind::BuiltinCall {
                    op: BuiltinOp::StrConcat,
                    args,
                }),
                span,
            )
        })();
        self.leave_expansion();
        result
    }

    /// Emits the pending text group (when non-empty) as a synthesized
    /// `Lit(Str)` part. Returns `true` when a group was emitted.
    pub(super) fn flush_text_group(
        &mut self,
        frame: &mut ExpansionFrame,
        args: &mut Vec<HirId>,
        pending_text: &mut String,
        pending_lo: &mut Option<u32>,
        pending_hi: u32,
    ) -> Result<bool, Aborted> {
        let Some(lo) = pending_lo.take() else {
            return Ok(false);
        };
        if pending_text.is_empty() {
            return Ok(false);
        }
        let text = std::mem::take(pending_text);
        let span = Span {
            file: self.file,
            lo,
            hi: pending_hi.max(lo),
        };
        let node = self.synth_node(
            frame,
            HirKind::Expr(ExprKind::Lit(crate::hir::HirLit::Str(HirStringLit {
                text,
                span,
            }))),
            span,
        )?;
        args.push(node);
        Ok(true)
    }
}
