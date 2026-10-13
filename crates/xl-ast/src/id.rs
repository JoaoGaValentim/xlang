//! Node identifiers for the XLANG syntax tree.

use std::fmt;

/// Identifier of an [`crate::Item`] node.
///
/// Allocated by the future parser; `xl-ast` never allocates or rewrites
/// identifiers. Identifiers are **instance-scoped**: they are unique within
/// one parsed file/AST instance only, and manually constructed nodes may
/// repeat them. They are not globally unique and must not be used as
/// cross-instance keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(u32);

/// Identifier of an [`crate::Expr`] node.
///
/// Same allocation and scoping contract as [`ItemId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(u32);

impl ItemId {
    /// Returns the raw index of this identifier.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Creates an `ItemId` from a raw index (manual construction/tests).
    #[must_use]
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }
}

impl ExprId {
    /// Returns the raw index of this identifier.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Creates an `ExprId` from a raw index (manual construction/tests).
    #[must_use]
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "item#{}", self.0)
    }
}

impl fmt::Display for ExprId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expr#{}", self.0)
    }
}
