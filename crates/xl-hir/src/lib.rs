#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! High-level intermediate representation and structural lowering for the
//! XLANG compiler.
//!
//! This crate implements Phase 2 Work Package A as authorized by
//! Engineering Directive 011: the arena-owned HIR data model, checked
//! identity allocation, AST-origin mapping, structural lowering of every
//! accepted AST construct, the three ratified desugarings (`for`, `?`,
//! string interpolation), the structural validator, and resource safety —
//! per ADR-0005 and the owner-ratified Phase 2 contracts.
//!
//! # Pipeline position
//!
//! ```text
//! source → lexer → parser → AST → [xl-hir structural lowering] ← this crate
//!   → name resolution (WP-B) → type checking (WP-C) → xc check (WP-D)
//! ```
//!
//! # Structural lowering versus name resolution
//!
//! [`lower_structure`] is the explicitly **structural, pre-resolution**
//! operation of WP-A. It builds the complete HIR representation,
//! desugars, records every name use, and performs no name resolution:
//!
//! - every source use is recorded (`UseId`) with resolution
//!   [`Resolution::Unresolved`] and reason
//!   [`UnresolvedReason::Pending`](crate::UnresolvedReason::Pending);
//! - every `import` binding is recorded with target
//!   [`ModuleBindingTarget::Unresolved`];
//! - desugaring-internal references are established by identity at
//!   creation (expansion-local definitions or compiler-owned
//!   [`InternalDef`] prelude references) and are never resolved through
//!   user scopes, so no source identifier can capture them.
//!
//! WP-B composes this stage with the module registry and name resolver to
//! fill the ratified `lower(&[Item], FileId, &ModuleRegistry) -> LowerOutput`
//! contract **without re-traversing the AST**; WP-A does not implement the
//! symbol table, the module registry, import resolution, or cyclic-import
//! detection.
//!
//! # Guarantees
//!
//! - **Total and pure.** Lowering never panics on any input (including
//!   parser-recovered ASTs and hand-built malformed trees) and performs no
//!   I/O and no runtime work.
//! - **Deterministic.** Identical inputs produce identical modules,
//!   identifiers, origins, and diagnostics; no hash-order dependence, no
//!   address-based identity, no global state.
//! - **Checked identity.** Every identifier is a `u32` index allocated in
//!   a fixed traversal order; exhaustion aborts construction with a
//!   controlled error ([`StageFate::Aborted`]) and never wraps, reuses, or
//!   saturates an id.
//! - **Resource-bounded.** The ratified budgets (`P2D-A7`) are enforced
//!   deterministically: [`MAX_HIR_DEPTH`] poisons the affected subtree
//!   with a fail-closed diagnostic; [`MAX_HIR_NODES`], [`MAX_DEFS`], and
//!   [`MAX_USES`] abort construction. No unbounded native-stack recursion
//!   is used to build or drop the IR (all arenas are flat).
//! - **Validated.** [`validate`] checks the structural invariants
//!   (`hir-contract.md` §11) and rejects deliberately malformed modules.
//!
//! # Example
//!
//! ```no_run
//! use xl_hir::{lower_structure, ModuleId, PackageId};
//! use xl_span::FileId;
//!
//! # fn example(items: &[xl_ast::Item]) {
//! let output = lower_structure(items, FileId::from_index(0), ModuleId::from_index(0), PackageId::from_index(0));
//! assert!(output.is_complete());
//! assert!(xl_hir::validate(&output.module).is_empty());
//! # }
//! ```

pub mod dump;
pub mod fate;
pub mod hir;
pub mod limits;
mod lower;
mod messages;
pub mod origin;
pub mod validate;

pub use fate::{AbortReason, Budget, IdSpace, StageFate};
pub use hir::{
    Arm, AssignOp, BinOp, Body, BodyId, BodyTable, BuiltinMember, BuiltinOp, Def, DefId, DefKind,
    DefTable, ExprKind, FieldSel, FnSig, FnSigId, GlobalDefId, HirId, HirItem, HirItemId,
    HirItemKind, HirKind, HirLit, HirModule, HirNode, ImportRecord, InternalDef, LocalId,
    ModuleBinding, ModuleBindingTarget, ModuleId, NodeArena, PackageId, PatKind, Radix, Resolution,
    Resolutions, SigTable, StmtKind, StringLit, TypeRef, TypeRefId, TypeRefKind, UnOp,
    UnresolvedReason, Use, UseId, UseTable, Visibility,
};
pub use limits::{LowerLimits, MAX_DEFS, MAX_HIR_DEPTH, MAX_HIR_NODES, MAX_USES};
pub use lower::{StructuralLowerOutput, lower_structure, lower_structure_with_limits};
pub use origin::{
    AnchorId, AstOrigin, AstOriginKey, AstOriginTable, DesugarKind, ExpansionId, OccurrenceKey,
    Origin, SynthOrigin,
};
pub use validate::{
    Violation, ViolationKind, validate, validate_resolutions, validate_with_limits,
};
