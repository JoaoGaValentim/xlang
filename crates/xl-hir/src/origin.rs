//! AST source origins for every HIR entity (`P2D-A2`).
//!
//! The accepted AST assigns identifiers only to items ([`xl_ast::ItemId`])
//! and expressions ([`xl_ast::ExprId`]); statements, patterns, types, and
//! idents carry no AST id. Lowering therefore records, for every HIR
//! entity, an [`AstOrigin`]:
//!
//! - nodes lowered from an `Item` or `Expr` record the existing
//!   `ItemId`/`ExprId`;
//! - statements, patterns, types, blocks, definition sites, and use sites
//!   record a deterministic [`AstOrigin::Structural`] key: the nearest
//!   enclosing id-bearing AST node (the *anchor*) plus the child-index path
//!   taken by the single lowering traversal;
//! - desugared entities record an [`AstOrigin::Synthesized`] key: the
//!   originating construct, the expansion, and the occurrence index.
//!
//! Spans are never identity keys (grouping parentheses are transparent and
//! recovered placeholders are zero-width), so two distinct constructs may
//! share a span while keeping distinct structural keys.
//!
//! # Child-index schema
//!
//! A structural path is a sequence of child indices from the anchor. The
//! schema below is the frozen traversal order; the lowering and the
//! origin-completeness tests both follow it. Slots are listed in source
//! order; absent optional children are simply not visited, and vector
//! children occupy one slot per element. Only slots that receive entities
//! (nodes, definitions, or uses) are listed.
//!
//! - `Item`, by kind:
//!   - `Fn`: `0` generics?, `1` signature, `2` body.
//!   - `Struct`: `0` generics?, fields at `1 + i`.
//!   - `Enum`: `0` generics?, variants at `1 + i`.
//!   - `Trait`: `0` generics?, items at `1 + i`.
//!   - `Impl`: `0` generics?, `1` trait path?, `2` self type,
//!     items at `3 + i`.
//!   - `Import`: path segments at `i`, alias at `n` (when written).
//! - `Generics`: parameters at `i`.
//! - `GenericParam`: `1` bound? (`0` is the name, which receives the
//!   parameter's definition).
//! - `FnSig`: parameters at `i`, return type at `n` (when written).
//!   (The signature's name receives the item's definition.)
//! - `Param::Named`: `0` type. `Param::Receiver`: no children.
//! - `Struct::Field`: `0` type.
//! - `Enum::Variant`: payload types at `i`.
//! - `Trait::TraitItem`: `0` signature.
//! - `Impl::ImplItem::Fn`: `0` generics?, `1` signature, `2` body.
//! - `Block`: statements at `i`, tail at `n` (when present).
//! - `Stmt::Let`: `0` pattern, `1` type annotation?, `2` initializer.
//! - `Stmt::Expr`: `0` expression. `Stmt::Item`: `0` item (a fresh
//!   anchor).
//! - `Expr` children, by kind:
//!   - `Lit(Str)`: string parts at `i` (text parts receive nothing).
//!   - `Path`: segments at `i`.
//!   - `Unary`/`Try`: `0` operand. `Binary`/`Range`: `0` left, `1` right.
//!   - `Assign`: `0` target, `1` value.
//!   - `Cast`: `0` expression, `1` type.
//!   - `Call`: `0` callee, arguments at `1 + i`.
//!   - `Field`: `0` base, `1` member name (a use).
//!   - `Index`: `0` base, `1` index.
//!   - `If`: `0` condition, `1` then block, `2` else expression?.
//!   - `Match`: `0` scrutinee, arms at `1 + i`.
//!   - `Arm`: `0` pattern, `1` body.
//!   - `Block`: `0` block. `Loop`: `0` block.
//!   - `While`: `0` condition, `1` block.
//!   - `For`: `0` pattern, `1` iterable, `2` block.
//!   - `Return`: `0` operand?.
//!   - `Tuple`/`Array`: elements at `i`.
//! - `Type`, by kind:
//!   - `Path`: segments at `i`, generic arguments at `n + i`.
//!   - `Tuple`: elements at `i`.
//!   - `Slice`: `0` element.
//!   - `Array`: `0` element, `1` length expression.
//!   - `Fn`: parameter types at `i`, return type at `n` (when written).
//! - `Pat`, by kind:
//!   - `Path`: segments at `i`, payload patterns at `n + i`.
//!   - `Tuple`: elements at `i`.
//!
//! Entering an `Item` or `Expr` node resets the anchor to that node's AST
//! id and clears the path (paths are always relative to the anchor).

use xl_ast::{ExprId, ItemId};

/// The nearest enclosing id-bearing AST node used as a structural anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorId {
    /// An item anchor.
    Item(ItemId),
    /// An expression anchor.
    Expr(ExprId),
}

impl AnchorId {
    /// Returns the anchor as an origin key.
    #[must_use]
    pub const fn key(self) -> AstOriginKey {
        match self {
            Self::Item(id) => AstOriginKey::Item(id),
            Self::Expr(id) => AstOriginKey::Expr(id),
        }
    }
}

/// The identity of a source construct: a real AST id when one exists, or
/// a structural key otherwise. Never a span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AstOriginKey {
    /// An item-identified construct.
    Item(ItemId),
    /// An expression-identified construct.
    Expr(ExprId),
    /// A construct without an AST id.
    Structural {
        /// The nearest enclosing id-bearing AST node.
        anchor: AnchorId,
        /// The deterministic child-index path from the anchor (never
        /// empty: structural constructs are strict descendants of their
        /// anchor).
        path: Vec<u32>,
    },
}

/// The kind of a desugaring expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesugarKind {
    /// The `for`-loop expansion (`P2D-B8`).
    For,
    /// The `?` error-propagation expansion (`P2D-B9`).
    Try,
    /// The string-interpolation expansion (`P2D-B10`).
    Interpolation,
}

/// Identity of one desugaring expansion.
///
/// The value is the nesting depth of the desugaring stack while the
/// expansion runs: an expansion with no enclosing expansion is `0`, and a
/// desugaring nested inside another expansion (for example a `?` inside a
/// desugared `for` body) has a strictly greater value
/// (`lowering-contract.md` §8.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpansionId(u32);

impl ExpansionId {
    /// Returns the raw value.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Creates an expansion id from a raw value.
    #[must_use]
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }
}

/// The identity of the source construct a desugaring expanded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SynthOrigin {
    /// The desugaring that produced the entities.
    pub kind: DesugarKind,
    /// The originating construct's source identity (never a span).
    pub source: AstOriginKey,
}

/// The recorded source origin of one HIR entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AstOrigin {
    /// Lowered from an item.
    Item(ItemId),
    /// Lowered from an expression.
    Expr(ExprId),
    /// Lowered from a construct without an AST id.
    Structural {
        /// The nearest enclosing id-bearing AST node.
        anchor: AnchorId,
        /// The deterministic child-index path from the anchor.
        path: Vec<u32>,
    },
    /// Produced by a desugaring expansion.
    ///
    /// `AstOrigin` is a *reference*, not a unique node identity: the
    /// entities of one expansion share the `construct` and are
    /// distinguished by the complete occurrence key
    /// `(construct.source, expansion, occurrence)`.
    Synthesized {
        /// The originating construct.
        construct: SynthOrigin,
        /// The expansion the entity belongs to.
        expansion: ExpansionId,
        /// The entity's 0-based index within its expansion (assigned by
        /// the fixed expansion traversal).
        occurrence: u32,
    },
}

/// Whether an entity was lowered from source syntax or produced by
/// desugaring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Lowered directly from source syntax.
    Source,
    /// Produced by a desugaring expansion (an internal flag; not
    /// user-visible).
    Desugared {
        /// The desugaring that produced the entity.
        construct: DesugarKind,
    },
}

impl AstOrigin {
    /// Returns the origin's structural reference key.
    ///
    /// For [`AstOrigin::Synthesized`] entities the key is the
    /// `Synthesized` arm of [`OccurrenceKey`]; for the other arms it is
    /// the corresponding construct key.
    #[must_use]
    pub fn occurrence_key(&self) -> OccurrenceKey {
        match self {
            Self::Item(id) => OccurrenceKey::Item(*id),
            Self::Expr(id) => OccurrenceKey::Expr(*id),
            Self::Structural { anchor, path } => OccurrenceKey::Structural {
                anchor: *anchor,
                path: path.clone(),
            },
            Self::Synthesized {
                construct,
                expansion,
                occurrence,
            } => OccurrenceKey::Synthesized {
                source: construct.source.clone(),
                expansion: *expansion,
                occurrence: *occurrence,
            },
        }
    }

    /// Returns the source/desugared flag.
    #[must_use]
    pub fn origin(&self) -> Origin {
        match self {
            Self::Synthesized { construct, .. } => Origin::Desugared {
                construct: construct.kind,
            },
            _ => Origin::Source,
        }
    }

    /// Returns `true` for synthesized (desugared) entities.
    #[must_use]
    pub fn is_desugared(&self) -> bool {
        matches!(self, Self::Synthesized { .. })
    }
}

/// The occurrence key of one HIR entity.
///
/// Two distinct HIR nodes can never share an occurrence key: nodes with an
/// AST id use the id, structural nodes use the anchor-and-path key, and
/// synthesized entities use `(construct.source, expansion, occurrence)`
/// (`hir-contract.md` §4/§11).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OccurrenceKey {
    /// An item-identified occurrence.
    Item(ItemId),
    /// An expression-identified occurrence.
    Expr(ExprId),
    /// A structural occurrence.
    Structural {
        /// The nearest enclosing id-bearing AST node.
        anchor: AnchorId,
        /// The child-index path from the anchor.
        path: Vec<u32>,
    },
    /// A synthesized occurrence: origin construct, expansion, and
    /// occurrence index.
    Synthesized {
        /// The originating construct.
        source: AstOriginKey,
        /// The expansion.
        expansion: ExpansionId,
        /// The occurrence index within the expansion.
        occurrence: u32,
    },
}

/// The AST origin of every HIR entity, parallel to the module's arenas.
///
/// Every entity allocated by lowering pushes exactly one origin, so each
/// vector's length always equals the corresponding arena's length; the
/// structural validator checks this correspondence for hand-built values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AstOriginTable {
    /// Origins indexed by [`crate::hir::HirItemId`].
    pub items: Vec<AstOrigin>,
    /// Origins indexed by [`crate::hir::DefId`].
    pub defs: Vec<AstOrigin>,
    /// Origins indexed by [`crate::hir::UseId`].
    pub uses: Vec<AstOrigin>,
    /// Origins indexed by [`crate::hir::BodyId`].
    pub bodies: Vec<AstOrigin>,
    /// Origins indexed by [`crate::hir::HirId`].
    pub nodes: Vec<AstOrigin>,
    /// Origins indexed by [`crate::hir::TypeRefId`].
    pub types: Vec<AstOrigin>,
}

impl AstOriginTable {
    /// Returns the origin of item `id`, if recorded.
    #[must_use]
    pub fn item(&self, id: crate::hir::HirItemId) -> Option<&AstOrigin> {
        self.items.get(id.index() as usize)
    }

    /// Returns the origin of definition `id`, if recorded.
    #[must_use]
    pub fn def(&self, id: crate::hir::DefId) -> Option<&AstOrigin> {
        self.defs.get(id.index() as usize)
    }

    /// Returns the origin of use `id`, if recorded.
    #[must_use]
    pub fn use_(&self, id: crate::hir::UseId) -> Option<&AstOrigin> {
        self.uses.get(id.index() as usize)
    }

    /// Returns the origin of body `id`, if recorded.
    #[must_use]
    pub fn body(&self, id: crate::hir::BodyId) -> Option<&AstOrigin> {
        self.bodies.get(id.index() as usize)
    }

    /// Returns the origin of node `id`, if recorded.
    #[must_use]
    pub fn node(&self, id: crate::hir::HirId) -> Option<&AstOrigin> {
        self.nodes.get(id.index() as usize)
    }

    /// Returns the origin of type reference `id`, if recorded.
    #[must_use]
    pub fn type_ref(&self, id: crate::hir::TypeRefId) -> Option<&AstOrigin> {
        self.types.get(id.index() as usize)
    }
}
