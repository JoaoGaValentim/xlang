//! Practical resource budgets (`P2D-A7`).
//!
//! Every budget is a deterministic counter; no wall-clock timeout exists.
//! The ratified values bound a single module's lowering. Budget checks are
//! independent of the checked `u32` identity allocation: budgets are
//! practical limits tested before allocation, while id exhaustion is a
//! correctness failure reported through [`crate::StageFate`].
//!
//! Exceeding a budget is fail closed: construction stops with
//! [`crate::AbortReason::BudgetExceeded`] and an error diagnostic, and the
//! partial result is never a successful check.

/// Maximum HIR nesting depth (max nested block/expression/pattern/type
/// chain), ratified by `P2D-A7`.
///
/// Accepted source is depth-bounded by the parser
/// (`xl_parser::MAX_PARSE_DEPTH = 64`); each desugaring adds a small,
/// fixed number of levels, so 256 is finite and generous. The limit also
/// bounds every non-recursive-dropping traversal in this crate; exceeding
/// it poisons the affected subtree with an error node and a fail-closed
/// diagnostic.
pub const MAX_HIR_DEPTH: u32 = 256;

/// Maximum HIR nodes (expressions, statements, patterns) per module,
/// ratified by `P2D-A7`.
pub const MAX_HIR_NODES: u32 = 1_000_000;

/// Maximum definitions per module, ratified by `P2D-A7`.
pub const MAX_DEFS: u32 = 100_000;

/// Maximum recorded name uses per module, ratified by `P2D-A7`.
pub const MAX_USES: u32 = 1_000_000;

/// The configurable form of the ratified budgets.
///
/// The public entry points use [`LowerLimits::RATIFIED`]; the
/// `with_limits` entry points accept smaller values so tests can exercise
/// each boundary without allocating enormous structures. Reducing a limit
/// only makes lowering fail earlier; it never changes which programs are
/// accepted at the ratified values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowerLimits {
    /// Maximum HIR nesting depth.
    pub max_hir_depth: u32,
    /// Maximum HIR nodes per module.
    pub max_hir_nodes: u32,
    /// Maximum definitions per module.
    pub max_defs: u32,
    /// Maximum name uses per module.
    pub max_uses: u32,
}

impl LowerLimits {
    /// The owner-ratified Phase 2 budgets (`P2D-A7`).
    pub const RATIFIED: Self = Self {
        max_hir_depth: MAX_HIR_DEPTH,
        max_hir_nodes: MAX_HIR_NODES,
        max_defs: MAX_DEFS,
        max_uses: MAX_USES,
    };

    /// Returns the ratified budgets.
    #[must_use]
    pub const fn ratified() -> Self {
        Self::RATIFIED
    }

    /// Test overrides may only tighten the owner-ratified budgets.
    pub(crate) fn capped(self) -> Self {
        Self {
            max_hir_depth: self.max_hir_depth.min(MAX_HIR_DEPTH),
            max_hir_nodes: self.max_hir_nodes.min(MAX_HIR_NODES),
            max_defs: self.max_defs.min(MAX_DEFS),
            max_uses: self.max_uses.min(MAX_USES),
        }
    }
}

impl Default for LowerLimits {
    fn default() -> Self {
        Self::RATIFIED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configurable_limits_can_tighten_but_never_expand_ratified_budgets() {
        let too_large = LowerLimits {
            max_hir_depth: u32::MAX,
            max_hir_nodes: u32::MAX,
            max_defs: u32::MAX,
            max_uses: u32::MAX,
        };
        assert_eq!(too_large.capped(), LowerLimits::RATIFIED);

        let tightened = LowerLimits {
            max_hir_depth: 32,
            max_hir_nodes: 100,
            max_defs: 10,
            max_uses: 50,
        };
        assert_eq!(tightened.capped(), tightened);
    }
}
