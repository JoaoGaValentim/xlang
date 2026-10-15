//! Structural AST → HIR lowering.
//!
//! This module implements WP-A: the structural half of the ratified
//! `lower` contract (`lowering-contract.md`; Directive 011 §5). It builds
//! the arena-owned HIR, collects definitions, records every name use, and
//! performs the three ratified desugarings (`for`, `?`, string
//! interpolation).
//!
//! **Structural lowering performs no name resolution.** Every source use
//! is returned as [`Resolution::Unresolved`] with
//! [`UnresolvedReason::Pending`]; compiler-owned internal references
//! created by desugaring are established at creation and are never name
//! lookups. Import targets are left
//! [`crate::hir::ModuleBindingTarget::Unresolved`]. WP-B replaces those
//! states while composing this stage with the resolver; no successful
//! resolution result is ever fabricated here.
//!
//! # Deterministic allocation order
//!
//! Allocation order is fixed and documented (`hir-contract.md` §4.1):
//!
//! 1. **Module scan** — module items in source order; for each item: the
//!    item's own definition, then its generics and member definitions
//!    (parameters, fields, variants, methods, trait items) in source
//!    order.
//! 2. **Signature lowering** — items in source order; within one item,
//!    generic bounds, parameter types, return type, field types, variant
//!    payloads, and impl headers, all left to right.
//! 3. **Body lowering** — function and method bodies in source order;
//!    within a body, lexical order; nested items are completed at their
//!    source position.
//! 4. **Desugared entities** — at the point of the originating construct,
//!    in expansion order.

mod alloc;
mod desugar;
mod expr;
mod item;

use xl_ast::Item;
use xl_diag::Diagnostic;
use xl_span::{FileId, Span};

use crate::fate::{AbortReason, Aborted, StageFate};
use crate::hir::{
    BodyId, BodyTable, DefId, DefTable, FnSigId, HirId, HirModule, ModuleId, NodeArena, PackageId,
    Resolutions, SigTable, TypeRefId, TypeRefTable, UseTable,
};
use crate::limits::LowerLimits;
use crate::origin::{
    AnchorId, AstOrigin, AstOriginKey, AstOriginTable, DesugarKind, ExpansionId, SynthOrigin,
};

/// The result of structural lowering (WP-A).
///
/// This is the explicitly structural, pre-resolution output. It is shaped
/// for composition: WP-B consumes it, resolves the recorded uses and
/// import targets against the module registry, and fills the ratified
/// `LowerOutput` (adding `import_edges`), without re-traversing the AST.
///
/// The module is a complete representation for any structurally plausible
/// input (parser-recovered placeholders become error nodes); the only
/// non-source partial results are the resource failures reported through
/// [`StructuralLowerOutput::fate`], which make the output unusable for a
/// successful check.
#[derive(Debug)]
pub struct StructuralLowerOutput {
    /// The lowered module.
    pub module: HirModule,
    /// The syntactic resolution side table: one entry per use.
    pub resolutions: Resolutions,
    /// Diagnostics originating at this stage, in deterministic order.
    pub diagnostics: Vec<Diagnostic>,
    /// The stage outcome.
    pub fate: StageFate,
}

impl StructuralLowerOutput {
    /// Returns `true` when the output is usable for a successful
    /// validation (no resource abort occurred).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.fate.is_complete()
    }
}

/// Lowers `items` structurally into a new [`HirModule`] with the ratified
/// budgets (`P2D-A7`).
///
/// `file` must be the file every item's spans refer to; `module` and
/// `package` are the identities assigned by the caller's module registry
/// (`P2D-A5`). This function performs no I/O, never panics on any input,
/// and does not depend on the parser crate.
#[must_use]
pub fn lower_structure(
    items: &[Item],
    file: FileId,
    module: ModuleId,
    package: PackageId,
) -> StructuralLowerOutput {
    lower_structure_with_limits(items, file, module, package, LowerLimits::RATIFIED)
}

/// Lowers `items` structurally with explicit budgets.
///
/// The `with_limits` entry point exists so boundary behavior is testable
/// without building enormous inputs; it never changes which programs are
/// accepted at the ratified values except by failing earlier.
#[must_use]
pub fn lower_structure_with_limits(
    items: &[Item],
    file: FileId,
    module: ModuleId,
    package: PackageId,
    limits: LowerLimits,
) -> StructuralLowerOutput {
    Lowerer::new(file, module, package, limits).run(items)
}

/// One body-lowering context: how pattern bindings are recorded.
#[derive(Debug, Clone, Copy)]
struct PatCtx {
    /// `true` when the enclosing statement is a `let mut` (every binding
    /// of its pattern is mutable; `P2D-B17`).
    mutable: bool,
    /// The body the binding belongs to.
    body: BodyId,
    /// The `let` type annotation shared by every binding of the pattern.
    ty: Option<TypeRefId>,
}

/// One desugaring expansion in progress.
struct ExpansionFrame {
    /// The kind of desugaring.
    kind: DesugarKind,
    /// The originating construct.
    source: AstOriginKey,
    /// The expansion's depth (0 = outermost).
    expansion: u32,
    /// The next occurrence index within this expansion.
    occurrence: u32,
}

/// A per-item definition skeleton, produced by the module-scan pass and
/// consumed by signature and body lowering.
struct ItemSkeleton {
    /// The item's own definition.
    def: DefId,
    /// The skeleton payload.
    kind: SkeletonKind,
}

/// The per-kind payload of an [`ItemSkeleton`].
enum SkeletonKind {
    /// A function item.
    Fn(FnSkeleton),
    /// A struct item.
    Struct {
        /// The struct's generic parameter definitions.
        generics: Vec<DefId>,
        /// The struct's field definitions.
        fields: Vec<DefId>,
    },
    /// An enum item.
    Enum {
        /// The enum's generic parameter definitions.
        generics: Vec<DefId>,
        /// The enum's variant definitions.
        variants: Vec<DefId>,
    },
    /// A trait item.
    Trait {
        /// The trait's generic parameter definitions.
        generics: Vec<DefId>,
        /// The trait's item definitions.
        items: Vec<SigSkeleton>,
    },
    /// An `impl` block.
    Impl {
        /// The impl's generic parameter definitions.
        generics: Vec<DefId>,
        /// The impl's method definitions.
        methods: Vec<FnSkeleton>,
    },
    /// An import declaration (fully lowered by the module scan).
    Import,
}

/// The skeleton of a function-like definition.
struct FnSkeleton {
    /// The definition itself.
    def: DefId,
    /// Its generic parameter definitions.
    generics: Vec<DefId>,
    /// Its parameter (and receiver) definitions.
    params: Vec<DefId>,
    /// Its reserved signature.
    sig: FnSigId,
}

/// A trait-item skeleton.
struct SigSkeleton {
    /// The definition itself.
    def: DefId,
    /// Its parameter (and receiver) definitions.
    params: Vec<DefId>,
    /// Its reserved signature.
    sig: FnSigId,
}

/// The lowering driver. One instance lowers one module.
struct Lowerer {
    file: FileId,
    limits: LowerLimits,
    module: HirModule,
    resolutions: Resolutions,
    diagnostics: Vec<Diagnostic>,
    fate: Option<AbortReason>,
    /// Desugaring-stack depth (see [`ExpansionId`]).
    expansion_depth: u32,
    /// Current HIR nesting depth (expressions, statements, patterns,
    /// bodies, types).
    hir_depth: u32,
    /// Whether the depth-limit diagnostic was already emitted (at most
    /// one per run, deterministic).
    depth_reported: bool,
    /// Enclosing loop nodes of the current function/method body.
    loop_stack: Vec<HirId>,
    /// The anchor for structural origins.
    anchor: Option<AnchorId>,
    /// The child-index path from the anchor.
    path: Vec<u32>,
    /// Syntactic `Self` context depth (trait/impl items and their
    /// members; reset inside nested items).
    self_ctx: u32,
    /// The innermost body being lowered (for local ownership).
    current_body: Option<BodyId>,
}

impl Lowerer {
    pub(super) fn new(
        file: FileId,
        module: ModuleId,
        package: PackageId,
        limits: LowerLimits,
    ) -> Self {
        Self {
            file,
            limits,
            module: HirModule {
                module,
                package,
                file,
                items: Vec::new(),
                imports: Vec::new(),
                defs: DefTable::default(),
                uses: UseTable::default(),
                nodes: NodeArena::default(),
                bodies: BodyTable::default(),
                types: TypeRefTable::default(),
                sigs: SigTable::default(),
                origins: AstOriginTable::default(),
            },
            resolutions: Resolutions::default(),
            diagnostics: Vec::new(),
            fate: None,
            expansion_depth: 0,
            hir_depth: 0,
            depth_reported: false,
            loop_stack: Vec::new(),
            anchor: None,
            path: Vec::new(),
            self_ctx: 0,
            current_body: None,
        }
    }

    /// Runs the three lowering passes over the module items.
    pub(super) fn run(mut self, items: &[Item]) -> StructuralLowerOutput {
        let mut skeletons: Vec<ItemSkeleton> = Vec::with_capacity(items.len());
        for item in items {
            match self.item_skeleton(item) {
                Ok(skeleton) => skeletons.push(skeleton),
                Err(_) => return self.finish(),
            }
        }
        for (item, skeleton) in items.iter().zip(&skeletons) {
            if self.item_signature(item, skeleton).is_err() {
                return self.finish();
            }
        }
        for (item, skeleton) in items.iter().zip(&skeletons) {
            if self.item_body(item, skeleton).is_err() {
                return self.finish();
            }
        }
        self.finish()
    }

    /// Completes the output, reporting the recorded fate.
    pub(super) fn finish(self) -> StructuralLowerOutput {
        let fate = match self.fate {
            Some(reason) => StageFate::Aborted { reason },
            None => StageFate::Complete,
        };
        StructuralLowerOutput {
            module: self.module,
            resolutions: self.resolutions,
            diagnostics: self.diagnostics,
            fate,
        }
    }

    // ------------------------------------------------------------------
    // Site state
    // ------------------------------------------------------------------

    /// Returns `Err` once construction has been aborted.
    pub(super) fn check_alive(&self) -> Result<(), Aborted> {
        if self.fate.is_some() {
            Err(Aborted::SIGNAL)
        } else {
            Ok(())
        }
    }

    /// Records an abort reason (with an optional fail-closed diagnostic)
    /// and returns the abort signal. The first reason wins.
    pub(super) fn fail(
        &mut self,
        reason: AbortReason,
        diagnostic: Option<String>,
        span: Span,
    ) -> Aborted {
        if self.fate.is_none() {
            self.fate = Some(reason);
            if let Some(message) = diagnostic {
                self.diagnostics.push(Diagnostic::error(message, span));
            }
        }
        Aborted::SIGNAL
    }

    /// Runs `f` with `anchor` as the structural anchor and an empty child
    /// path, restoring the previous site state afterwards.
    pub(super) fn with_anchor<R>(&mut self, anchor: AnchorId, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved_anchor = self.anchor.replace(anchor);
        let saved_path = std::mem::take(&mut self.path);
        let result = f(self);
        self.anchor = saved_anchor;
        self.path = saved_path;
        result
    }

    /// Pushes one child slot onto the current path.
    pub(super) fn push_child(&mut self, index: u32) {
        self.path.push(index);
    }

    /// Pops the current child slot.
    pub(super) fn pop_child(&mut self) {
        self.path.pop();
    }

    /// The structural origin at the current site.
    ///
    /// A `Structural` origin is only requested while an anchor is active
    /// (every structural entity descends from an item or expression); the
    /// fallback keeps the function panic-free if that invariant were ever
    /// broken.
    pub(super) fn structural_origin(&self) -> AstOrigin {
        match self.anchor {
            Some(anchor) => AstOrigin::Structural {
                anchor,
                path: self.path.clone(),
            },
            None => {
                debug_assert!(false, "structural origin requested without an anchor");
                AstOrigin::Structural {
                    anchor: AnchorId::Item(xl_ast::ItemId::from_index(0)),
                    path: self.path.clone(),
                }
            }
        }
    }

    /// Enters a desugaring expansion.
    pub(super) fn enter_expansion(
        &mut self,
        kind: DesugarKind,
        source: AstOriginKey,
    ) -> ExpansionFrame {
        let frame = ExpansionFrame {
            kind,
            source,
            expansion: self.expansion_depth,
            occurrence: 0,
        };
        self.expansion_depth = self.expansion_depth.saturating_add(1);
        frame
    }

    /// Leaves the current desugaring expansion.
    pub(super) fn leave_expansion(&mut self) {
        self.expansion_depth = self.expansion_depth.saturating_sub(1);
    }

    /// Produces the next synthesized origin within `frame`.
    pub(super) fn synth_origin(&mut self, frame: &mut ExpansionFrame) -> AstOrigin {
        let occurrence = frame.occurrence;
        frame.occurrence = frame.occurrence.saturating_add(1);
        AstOrigin::Synthesized {
            construct: SynthOrigin {
                kind: frame.kind,
                source: frame.source.clone(),
            },
            expansion: ExpansionId::from_index(frame.expansion),
            occurrence,
        }
    }
}
