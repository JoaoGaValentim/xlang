//! Checked allocation, resource budgets, and depth accounting.
//!
//! Every entity allocation is checked against the ratified `u32` identity
//! space (`P2D-A2`: no reuse, no wrapping, no saturation, no panic) and
//! against the practical budgets (`P2D-A7`). Failures abort construction
//! through [`Aborted`]; the recorded reason is surfaced through
//! [`crate::StageFate`].

use xl_diag::Diagnostic;
use xl_span::Span;

use super::Lowerer;
use crate::fate::{AbortReason, Aborted, Budget, IdSpace};
use crate::hir::{
    Body, BodyId, Def, DefId, DefKind, FnSig, FnSigId, HirId, HirItem, HirItemId, HirItemKind,
    HirKind, HirNode, Resolution, TypeRef, TypeRefId, TypeRefKind, UnresolvedReason, Use, UseId,
    Visibility,
};
use crate::messages;
use crate::origin::AstOrigin;

impl Lowerer {
    // ------------------------------------------------------------------
    // Budget and identity checks
    // ------------------------------------------------------------------

    /// Aborts when the per-module node budget is already spent.
    pub(super) fn check_node_budget(&mut self, span: Span) -> Result<(), Aborted> {
        self.check_alive()?;
        if self.module.nodes.nodes.len() >= self.limits.max_hir_nodes as usize {
            let message = messages::node_limit(self.limits.max_hir_nodes);
            return Err(self.fail(
                AbortReason::BudgetExceeded {
                    budget: Budget::HirNodes,
                },
                Some(message),
                span,
            ));
        }
        Ok(())
    }

    /// Aborts when the per-module definition budget is already spent.
    pub(super) fn check_def_budget(&mut self, span: Span) -> Result<(), Aborted> {
        self.check_alive()?;
        if self.module.defs.defs.len() >= self.limits.max_defs as usize {
            let message = messages::def_limit(self.limits.max_defs);
            return Err(self.fail(
                AbortReason::BudgetExceeded {
                    budget: Budget::Defs,
                },
                Some(message),
                span,
            ));
        }
        Ok(())
    }

    /// Aborts when the per-module use budget is already spent.
    pub(super) fn check_use_budget(&mut self, span: Span) -> Result<(), Aborted> {
        self.check_alive()?;
        if self.module.uses.uses.len() >= self.limits.max_uses as usize {
            let message = messages::use_limit(self.limits.max_uses);
            return Err(self.fail(
                AbortReason::BudgetExceeded {
                    budget: Budget::Uses,
                },
                Some(message),
                span,
            ));
        }
        Ok(())
    }

    /// Converts a table length into the next `u32` index, aborting with
    /// the controlled id-space failure when the space is exhausted.
    pub(super) fn next_index(
        &mut self,
        len: usize,
        space: IdSpace,
        span: Span,
    ) -> Result<u32, Aborted> {
        self.check_alive()?;
        match u32::try_from(len) {
            Ok(index) => Ok(index),
            Err(_) => Err(self.fail(AbortReason::IdSpaceExhausted { space }, None, span)),
        }
    }

    // ------------------------------------------------------------------
    // Nodes
    // ------------------------------------------------------------------

    /// Allocates an expression, statement, or pattern node.
    pub(super) fn alloc_node(
        &mut self,
        kind: HirKind,
        span: Span,
        origin: AstOrigin,
    ) -> Result<HirId, Aborted> {
        self.check_node_budget(span)?;
        let index = self.next_index(self.module.nodes.nodes.len(), IdSpace::Nodes, span)?;
        self.module.nodes.nodes.push(HirNode {
            id: HirId::from_index(index),
            span,
            kind,
        });
        self.module.origins.nodes.push(origin);
        Ok(HirId::from_index(index))
    }

    /// Reserves a node whose payload becomes known later (loop nodes,
    /// whose bodies reference them). The slot holds an error node until
    /// [`Lowerer::fill_node`] overwrites it.
    pub(super) fn reserve_node(&mut self, span: Span, origin: AstOrigin) -> Result<HirId, Aborted> {
        self.alloc_node(HirKind::Expr(crate::hir::ExprKind::Error), span, origin)
    }

    /// Overwrites a reserved node's kind and span.
    pub(super) fn fill_node(&mut self, id: HirId, kind: HirKind, span: Span) {
        if let Some(node) = self.module.nodes.nodes.get_mut(id.index() as usize) {
            node.kind = kind;
            node.span = span;
        }
    }

    // ------------------------------------------------------------------
    // Definitions
    // ------------------------------------------------------------------

    /// Allocates a definition with a complete payload.
    pub(super) fn alloc_def(
        &mut self,
        name: String,
        kind: DefKind,
        span: Span,
        visibility: Visibility,
        parent: Option<DefId>,
        origin: AstOrigin,
    ) -> Result<DefId, Aborted> {
        let id = self.reserve_def(name, span, visibility, parent, origin)?;
        self.fill_def(id, kind);
        Ok(id)
    }

    /// Reserves a definition whose payload becomes known later. The slot
    /// holds [`DefKind::Error`] until [`Lowerer::fill_def`] overwrites
    /// it; an unfilled slot can only survive a resource abort.
    pub(super) fn reserve_def(
        &mut self,
        name: String,
        span: Span,
        visibility: Visibility,
        parent: Option<DefId>,
        origin: AstOrigin,
    ) -> Result<DefId, Aborted> {
        self.check_def_budget(span)?;
        let index = self.next_index(self.module.defs.defs.len(), IdSpace::Defs, span)?;
        self.module.defs.defs.push(Def {
            id: DefId::from_index(index),
            name,
            kind: DefKind::Error,
            span,
            visibility,
            parent,
        });
        self.module.origins.defs.push(origin);
        Ok(DefId::from_index(index))
    }

    /// Overwrites a reserved definition's kind.
    pub(super) fn fill_def(&mut self, id: DefId, kind: DefKind) {
        if let Some(def) = self.module.defs.defs.get_mut(id.index() as usize) {
            def.kind = kind;
        }
    }

    // ------------------------------------------------------------------
    // Uses
    // ------------------------------------------------------------------

    /// Allocates a name use together with its initial resolution state.
    ///
    /// Source uses are allocated with [`Resolution::Unresolved`] and
    /// [`UnresolvedReason::Pending`]; a desugaring's internal references
    /// are allocated with their established resolution (an expansion-local
    /// definition or a compiler-owned internal definition). The latter is
    /// not a name lookup: nothing in an expansion is resolved by name
    /// (`lowering-contract.md` §2).
    pub(super) fn alloc_use(
        &mut self,
        name: String,
        span: Span,
        origin: AstOrigin,
        resolution: Resolution,
    ) -> Result<UseId, Aborted> {
        self.check_use_budget(span)?;
        let index = self.next_index(self.module.uses.uses.len(), IdSpace::Uses, span)?;
        self.module.uses.uses.push(Use { name, span });
        self.module.origins.uses.push(origin);
        self.resolutions.uses.push(resolution);
        Ok(UseId::from_index(index))
    }

    /// Allocates a source use: unresolved, pending name resolution.
    pub(super) fn alloc_source_use(
        &mut self,
        name: String,
        span: Span,
        origin: AstOrigin,
    ) -> Result<UseId, Aborted> {
        self.alloc_use(
            name,
            span,
            origin,
            Resolution::Unresolved(UnresolvedReason::Pending),
        )
    }

    // ------------------------------------------------------------------
    // Bodies
    // ------------------------------------------------------------------

    /// Allocates a body (initially empty; completed with
    /// [`Lowerer::fill_body`]).
    pub(super) fn alloc_body(&mut self, span: Span, origin: AstOrigin) -> Result<BodyId, Aborted> {
        self.check_alive()?;
        let index = self.next_index(self.module.bodies.bodies.len(), IdSpace::Bodies, span)?;
        self.module.bodies.bodies.push(Body::default());
        self.module.origins.bodies.push(origin);
        Ok(BodyId::from_index(index))
    }

    /// Overwrites a body's contents.
    pub(super) fn fill_body(&mut self, id: BodyId, stmts: Vec<HirId>, tail: Option<HirId>) {
        if let Some(body) = self.module.bodies.bodies.get_mut(id.index() as usize) {
            body.stmts = stmts;
            body.tail = tail;
        }
    }

    // ------------------------------------------------------------------
    // Type references
    // ------------------------------------------------------------------

    /// Allocates a type reference with a complete payload.
    pub(super) fn alloc_type(
        &mut self,
        kind: TypeRefKind,
        span: Span,
        origin: AstOrigin,
    ) -> Result<TypeRefId, Aborted> {
        let id = self.reserve_type(span, origin)?;
        self.fill_type(id, kind);
        Ok(id)
    }

    /// Reserves a type reference whose payload becomes known later
    /// (generic bounds, parameter types).
    pub(super) fn reserve_type(
        &mut self,
        span: Span,
        origin: AstOrigin,
    ) -> Result<TypeRefId, Aborted> {
        self.check_alive()?;
        let index = self.next_index(self.module.types.types.len(), IdSpace::Types, span)?;
        self.module.types.types.push(TypeRef {
            id: TypeRefId::from_index(index),
            kind: TypeRefKind::Error,
            span,
        });
        self.module.origins.types.push(origin);
        Ok(TypeRefId::from_index(index))
    }

    /// Overwrites a reserved type reference's kind.
    pub(super) fn fill_type(&mut self, id: TypeRefId, kind: TypeRefKind) {
        if let Some(type_ref) = self.module.types.types.get_mut(id.index() as usize) {
            type_ref.kind = kind;
        }
    }

    // ------------------------------------------------------------------
    // Items and signatures
    // ------------------------------------------------------------------

    /// Allocates an item slot.
    pub(super) fn alloc_item(
        &mut self,
        kind: HirItemKind,
        def: DefId,
        span: Span,
        origin: AstOrigin,
    ) -> Result<HirItemId, Aborted> {
        self.check_alive()?;
        let index = self.next_index(self.module.items.len(), IdSpace::Items, span)?;
        self.module.items.push(HirItem {
            id: HirItemId::from_index(index),
            kind,
            def,
            span,
        });
        self.module.origins.items.push(origin);
        Ok(HirItemId::from_index(index))
    }

    /// Reserves a signature (initially empty; completed with
    /// [`Lowerer::fill_sig`]).
    pub(super) fn reserve_sig(&mut self, owner: DefId, span: Span) -> Result<FnSigId, Aborted> {
        self.check_alive()?;
        let index = self.next_index(self.module.sigs.sigs.len(), IdSpace::Sigs, span)?;
        self.module.sigs.sigs.push(FnSig {
            name: owner,
            params: Vec::new(),
            ret: None,
            span,
        });
        Ok(FnSigId::from_index(index))
    }

    /// Overwrites a reserved signature's parameter list and return type.
    pub(super) fn fill_sig(&mut self, id: FnSigId, params: Vec<DefId>, ret: Option<TypeRefId>) {
        if let Some(sig) = self.module.sigs.sigs.get_mut(id.index() as usize) {
            sig.params = params;
            sig.ret = ret;
        }
    }

    // ------------------------------------------------------------------
    // Depth accounting
    // ------------------------------------------------------------------

    /// Reports the depth-limit failure (once per run) as a fail-closed
    /// error diagnostic. The calling lowering function poisons the
    /// affected subtree with an error node.
    pub(super) fn report_depth_exceeded(&mut self, span: Span) {
        if !self.depth_reported {
            self.depth_reported = true;
            let message = messages::depth_limit(self.limits.max_hir_depth);
            self.diagnostics.push(Diagnostic::error(message, span));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{ModuleId, PackageId};
    use crate::limits::LowerLimits;
    use xl_span::FileId;

    fn lowerer() -> Lowerer {
        Lowerer::new(
            FileId::from_index(0),
            ModuleId::from_index(0),
            PackageId::from_index(0),
            LowerLimits::RATIFIED,
        )
    }

    #[test]
    fn index_conversion_accepts_the_last_representable_index() {
        let mut lowerer = lowerer();
        let span = Span {
            file: FileId::from_index(0),
            lo: 0,
            hi: 0,
        };
        assert_eq!(
            lowerer.next_index(u32::MAX as usize, IdSpace::Nodes, span),
            Ok(u32::MAX)
        );
        assert!(lowerer.fate.is_none());
    }

    #[test]
    fn index_conversion_aborts_at_the_representational_boundary() {
        let mut lowerer = lowerer();
        let span = Span {
            file: FileId::from_index(0),
            lo: 0,
            hi: 0,
        };
        let error = lowerer
            .next_index(u32::MAX as usize + 1, IdSpace::Defs, span)
            .expect_err("exhaustion must abort");
        assert_eq!(error, Aborted::SIGNAL);
        assert_eq!(
            lowerer.fate,
            Some(AbortReason::IdSpaceExhausted {
                space: IdSpace::Defs
            })
        );
        // No diagnostic: id exhaustion is a controlled internal error.
        assert!(lowerer.diagnostics.is_empty());
        // Once aborted, allocation stays refused and the first reason
        // wins.
        let error = lowerer
            .next_index(0, IdSpace::Uses, span)
            .expect_err("aborted construction refuses allocation");
        assert_eq!(error, Aborted::SIGNAL);
        assert_eq!(
            lowerer.fate,
            Some(AbortReason::IdSpaceExhausted {
                space: IdSpace::Defs
            })
        );
    }
}
