//! Stage outcome and checked-allocation failure vocabulary (`P2D-A2`,
//! `P2D-A7`).
//!
//! Lowering never panics on any input. Two classes of failure exist:
//!
//! - **Source-representable problems** (parser-recovered placeholders, a
//!   construct that cannot be represented) become error nodes in a
//!   *complete* module, possibly with a diagnostic.
//! - **Resource failures** stop construction: the result is marked
//!   [`StageFate::Aborted`] and is unusable for a successful semantic
//!   validation. An id-space exhaustion is a controlled internal error; a
//!   practical budget exhaustion additionally carries a fail-closed
//!   diagnostic.

use std::fmt;

/// The outcome of a lowering stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageFate {
    /// The stage ran to the end of its input within its resource budgets.
    Complete,
    /// A non-recoverable resource failure stopped construction. The
    /// partial result must not be treated as a successful semantic
    /// validation.
    Aborted {
        /// The failure that stopped construction.
        reason: AbortReason,
    },
}

impl StageFate {
    /// Returns `true` when the stage completed.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Why construction stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbortReason {
    /// A `u32` identity space was exhausted. No id is reused, wrapped, or
    /// saturated; this is a controlled internal error (never a panic), and
    /// it is not a source error.
    IdSpaceExhausted {
        /// The exhausted identity space.
        space: IdSpace,
    },
    /// A practical resource budget (`P2D-A7`) was exceeded. The failure
    /// carries a fail-closed error diagnostic.
    BudgetExceeded {
        /// The exceeded budget.
        budget: Budget,
    },
}

impl fmt::Display for AbortReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdSpaceExhausted { space } => {
                write!(f, "{} id space is exhausted", space.label())
            }
            Self::BudgetExceeded { budget } => write!(f, "{} exceeded", budget.label()),
        }
    }
}

/// A `u32` identity space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdSpace {
    /// Expression/statement/pattern nodes ([`crate::hir::HirId`]).
    Nodes,
    /// Item slots ([`crate::hir::HirItemId`]).
    Items,
    /// Definitions ([`crate::hir::DefId`]).
    Defs,
    /// Name uses ([`crate::hir::UseId`]).
    Uses,
    /// Bodies ([`crate::hir::BodyId`]).
    Bodies,
    /// Type references ([`crate::hir::TypeRefId`]).
    Types,
    /// Signatures ([`crate::hir::FnSigId`]).
    Sigs,
}

impl IdSpace {
    /// Returns the human-readable space label used in reports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nodes => "HIR node",
            Self::Items => "HIR item",
            Self::Defs => "HIR definition",
            Self::Uses => "HIR use",
            Self::Bodies => "HIR body",
            Self::Types => "HIR type reference",
            Self::Sigs => "HIR signature",
        }
    }
}

/// A practical resource budget (`P2D-A7`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Budget {
    /// The per-module HIR node budget (`MAX_HIR_NODES`).
    HirNodes,
    /// The per-module definition budget (`MAX_DEFS`).
    Defs,
    /// The per-module use budget (`MAX_USES`).
    Uses,
}

impl Budget {
    /// Returns the human-readable budget label used in diagnostics.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HirNodes => "HIR node budget",
            Self::Defs => "HIR definition budget",
            Self::Uses => "HIR use budget",
        }
    }
}

/// Internal control-flow marker: an allocation failed and construction
/// must stop. Carries no data; the reason is recorded on the lowerer and
/// surfaced through [`StageFate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Aborted;

impl Aborted {
    /// The single abort signal.
    pub(crate) const SIGNAL: Self = Aborted;
}
