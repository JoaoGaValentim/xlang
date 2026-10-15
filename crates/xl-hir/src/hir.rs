//! The arena-owned HIR data model.
//!
//! Every identifier is a checked newtype index into a flat arena; no node
//! owns another node recursively, so dropping a [`HirModule`] is
//! non-recursive and deep *hand-built* inputs cannot overflow the stack on
//! drop. The module is immutable after lowering returns (`P2D-A3`).
//!
//! The representation implements the ratified `hir-contract.md` §3–§5
//! (owner-ratified by Directive 010-R3; ADR-0005). Deviations from the
//! indicative sketches are documented at the affected item and preserve
//! every ratified invariant.

use std::fmt;

use xl_span::{FileId, Span};

pub use xl_ast::{AssignOp, BinOp, Radix, UnOp, Visibility};

// ---------------------------------------------------------------------------
// Identity newtypes
// ---------------------------------------------------------------------------

/// Identifier of an expression, statement, or pattern node.
///
/// `HirId` is a single identity space whose entries are kind-tagged
/// ([`HirKind`]); expression, statement, and pattern slots never alias.
/// Identifiers are allocated in the deterministic lowering order and are
/// stable within one [`HirModule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HirId(u32);

/// Identifier of a module or block item slot (index into
/// [`HirModule::items`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HirItemId(u32);

/// Identifier of a named definition: items, variants, fields, methods,
/// trait items, generic parameters, function parameters, receivers,
/// module bindings, and locals.
///
/// There is exactly one numbering: a [`LocalId`] is a typed view over the
/// same space, never a second identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefId(u32);

/// Typed view of a [`DefId`] that is known to denote a local binding
/// (`DefKind::Local`).
///
/// `LocalId` is a documented `DefId` specialization (Directive 010-R1 §5):
/// it shares the `DefId` numbering and is obtained through
/// [`DefTable::as_local`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(DefId);

/// Identifier of a recorded name use (a path segment, a member name, an
/// import path segment, or one of the desugaring's synthesized internal
/// references).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UseId(u32);

/// Identifier of a body: one per function or method body and one per block
/// scope owner (every lowered block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyId(u32);

/// Identifier of a syntactic type reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeRefId(u32);

/// Identifier of a signature-table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FnSigId(u32);

/// Stable identity of a module within one compilation.
///
/// The value is assigned by the caller's module registry (`P2D-A5`); it is
/// stable for the lifetime of that registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u32);

/// Identity of the package that owns a module (`P2D-A5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(u32);

/// Composite identity of a definition across modules (`P2D-A2`).
///
/// A bare [`DefId`] is only an index inside one module; any reference that
/// can cross a module boundary is stored as a `GlobalDefId` so definitions
/// from different modules cannot alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalDefId {
    /// The module that owns the definition.
    pub module: ModuleId,
    /// The definition inside that module.
    pub definition: DefId,
}

macro_rules! index_newtype {
    ($name:ident, $label:literal) => {
        impl $name {
            /// Returns the raw index of this identifier.
            #[must_use]
            pub const fn index(self) -> u32 {
                self.0
            }

            /// Creates an identifier from a raw index.
            ///
            /// This is a low-level escape hatch for tests and for callers
            /// reconstructing identifiers from serialized data.
            #[must_use]
            pub const fn from_index(index: u32) -> Self {
                Self(index)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", $label, self.0)
            }
        }
    };
}

index_newtype!(HirId, "hir#");
index_newtype!(HirItemId, "item#");
index_newtype!(DefId, "def#");
index_newtype!(UseId, "use#");
index_newtype!(BodyId, "body#");
index_newtype!(TypeRefId, "type#");
index_newtype!(FnSigId, "sig#");
index_newtype!(ModuleId, "module#");
index_newtype!(PackageId, "package#");

impl LocalId {
    /// Creates a local view from its underlying definition identifier.
    ///
    /// The caller is responsible for having checked the definition's kind
    /// (see [`DefTable::as_local`]); the view is a type-level marker only
    /// and does not re-validate.
    #[must_use]
    pub const fn from_def(def: DefId) -> Self {
        Self(def)
    }

    /// Returns the underlying definition identifier.
    #[must_use]
    pub const fn def(self) -> DefId {
        self.0
    }
}

impl fmt::Display for LocalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "local#{}", self.0.0)
    }
}

// ---------------------------------------------------------------------------
// The module
// ---------------------------------------------------------------------------

/// The lowered representation of one source file (one file = one module;
/// SPECIFIED §2).
///
/// The module owns every arena and the AST-origin table. It contains no
/// hash maps and no interior mutability, so debug rendering and iteration
/// are deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct HirModule {
    /// Stable identity of this module (`P2D-A5`).
    pub module: ModuleId,
    /// The package that owns this module (`P2D-A5`).
    pub package: PackageId,
    /// The file every span in this module refers to.
    pub file: FileId,
    /// Module and block item slots, in allocation order (module items in
    /// source order first, then nested items at their lowering position).
    pub items: Vec<HirItem>,
    /// Import records in source order (module items and block imports).
    pub imports: Vec<ImportRecord>,
    /// All definitions.
    pub defs: DefTable,
    /// All recorded name uses.
    pub uses: UseTable,
    /// All expression, statement, and pattern nodes.
    pub nodes: NodeArena,
    /// All bodies.
    pub bodies: BodyTable,
    /// All syntactic type references.
    pub types: TypeRefTable,
    /// All function signatures.
    pub sigs: SigTable,
    /// AST source origin of every HIR entity.
    pub origins: crate::origin::AstOriginTable,
}

impl HirModule {
    /// Returns the item slot with `id`, if it is in bounds.
    #[must_use]
    pub fn item(&self, id: HirItemId) -> Option<&HirItem> {
        self.items.get(id.0 as usize)
    }

    /// Returns the import record introduced by item `id`, if `id` is an
    /// import item.
    #[must_use]
    pub fn import_of_item(&self, id: HirItemId) -> Option<&ImportRecord> {
        self.imports.iter().find(|record| record.item == id)
    }

    /// Returns the definition with `id`, if it is in bounds.
    #[must_use]
    pub fn def(&self, id: DefId) -> Option<&Def> {
        self.defs.get(id)
    }

    /// Returns the use with `id`, if it is in bounds.
    #[must_use]
    pub fn use_(&self, id: UseId) -> Option<&Use> {
        self.uses.uses.get(id.0 as usize)
    }

    /// Returns the node with `id`, if it is in bounds.
    #[must_use]
    pub fn node(&self, id: HirId) -> Option<&HirNode> {
        self.nodes.get(id)
    }

    /// Returns the body with `id`, if it is in bounds.
    #[must_use]
    pub fn body(&self, id: BodyId) -> Option<&Body> {
        self.bodies.get(id)
    }

    /// Returns the type reference with `id`, if it is in bounds.
    #[must_use]
    pub fn type_ref(&self, id: TypeRefId) -> Option<&TypeRef> {
        self.types.get(id)
    }

    /// Returns the signature with `id`, if it is in bounds.
    #[must_use]
    pub fn sig(&self, id: FnSigId) -> Option<&FnSig> {
        self.sigs.get(id)
    }
}

/// A module or block item slot (`HirItemId` indexes
/// [`HirModule::items`]).
#[derive(Debug, Clone, PartialEq)]
pub struct HirItem {
    /// This slot's identity.
    pub id: HirItemId,
    /// The item's kind.
    pub kind: HirItemKind,
    /// The item's own definition (the `Fn`, `Struct`, `Enum`, `Trait`,
    /// `Impl`, or import binding `Def` it introduces).
    pub def: DefId,
    /// Span covering the whole item.
    pub span: Span,
}

/// The kind of an item slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirItemKind {
    /// A function item.
    Fn,
    /// A struct item.
    Struct,
    /// An enum item.
    Enum,
    /// A trait item.
    Trait,
    /// An inherent or trait `impl` block.
    Impl,
    /// An import declaration.
    Import,
}

/// One lowered `import` declaration.
///
/// The declaration's *binding* is [`ImportRecord::binding`], a `Def` of
/// kind `DefKind::ModuleBinding`. The record retains the path segments as
/// uses so import resolution (WP-B) does not re-traverse the AST. The
/// binding's target is left `ModuleBindingTarget::Unresolved` by structural
/// lowering; resolving it against the caller's registry is WP-B's job and
/// must complete before a successful check.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportRecord {
    /// The import's item slot.
    pub item: HirItemId,
    /// The module-binding definition introduced by this import.
    pub binding: DefId,
    /// One use per dotted path segment, in written order.
    pub path: Vec<UseId>,
    /// Span of the whole declaration.
    pub decl_span: Span,
}

// ---------------------------------------------------------------------------
// Definitions
// ---------------------------------------------------------------------------

/// The definition table ([`DefId`] indexes [`DefTable::defs`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefTable {
    /// All definitions, indexed by [`DefId`].
    pub defs: Vec<Def>,
}

impl DefTable {
    /// Returns the definition with `id`, if it is in bounds.
    #[must_use]
    pub fn get(&self, id: DefId) -> Option<&Def> {
        self.defs.get(id.0 as usize)
    }

    /// Returns the typed [`LocalId`] view when `id` denotes a local
    /// binding.
    #[must_use]
    pub fn as_local(&self, id: DefId) -> Option<LocalId> {
        match self.get(id) {
            Some(Def {
                kind: DefKind::Local { .. },
                ..
            }) => Some(LocalId(id)),
            _ => None,
        }
    }

    /// Iterates over every definition in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = &Def> {
        self.defs.iter()
    }
}

/// One definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Def {
    /// This definition's identity.
    pub id: DefId,
    /// The definition's name (owned; no interning, `P2D-A4`).
    ///
    /// The name is the identifier text as written. An empty name marks a
    /// parser-recovered placeholder name (a documented intermediate state).
    /// Internal desugaring temporaries use their documented internal names
    /// (`__state`, `__pair`, `__item`, `__next`, `__v`, `__e`) and are
    /// never inserted into any source scope: they cannot be named,
    /// shadowed, or observed by user code.
    pub name: String,
    /// The definition's kind and payload.
    pub kind: DefKind,
    /// The name's span for named definitions; the whole-node span where
    /// noted.
    pub span: Span,
    /// Recorded visibility.
    pub visibility: Visibility,
    /// The enclosing definition: the enum for variants, the impl for
    /// methods, the item for parameters, generic parameters, and trait
    /// items. `None` for module items, module bindings, receivers, and
    /// locals.
    pub parent: Option<DefId>,
}

/// The kind and payload of a definition.
#[derive(Debug, Clone, PartialEq)]
pub enum DefKind {
    /// A function item.
    Fn {
        /// The function's generic parameters, in order.
        generics: Vec<DefId>,
        /// The function's signature.
        sig: FnSigId,
        /// The function's body.
        body: Option<BodyId>,
    },
    /// A struct item.
    Struct {
        /// The struct's generic parameters, in order.
        generics: Vec<DefId>,
        /// The struct's fields, in declaration order.
        fields: Vec<DefId>,
    },
    /// An enum item.
    Enum {
        /// The enum's generic parameters, in order.
        generics: Vec<DefId>,
        /// The enum's variants, in declaration order (exhaustiveness
        /// rendering order).
        variants: Vec<DefId>,
    },
    /// A trait item.
    Trait {
        /// The trait's generic parameters, in order.
        generics: Vec<DefId>,
        /// The trait's signature-only items, in declaration order.
        items: Vec<DefId>,
    },
    /// An inherent or trait `impl` block.
    Impl {
        /// The impl's generic parameters, in order.
        generics: Vec<DefId>,
        /// The implemented trait reference, when written.
        trait_ref: Option<TypeRefId>,
        /// The self type.
        self_ty: TypeRefId,
        /// `true` for inherent impls (`trait_ref` absent), derived from
        /// syntax.
        is_inherent: bool,
    },
    /// An enum variant (parent: the enum).
    Variant {
        /// Payload types for tuple variants; empty for unit variants.
        payload: Vec<TypeRefId>,
    },
    /// A struct field (parent: the struct).
    Field {
        /// The field's declared type.
        ty: TypeRefId,
    },
    /// A method inside an `impl` block (parent: the impl).
    Method {
        /// The method's generic parameters, in order.
        generics: Vec<DefId>,
        /// The method's signature.
        sig: FnSigId,
        /// The method's body.
        body: Option<BodyId>,
    },
    /// A signature-only item inside a trait (parent: the trait).
    TraitItem {
        /// The item's signature.
        sig: FnSigId,
    },
    /// A generic parameter (parent: the declaring item).
    GenericParam {
        /// The declared bound, when written.
        bound: Option<TypeRefId>,
    },
    /// A function parameter (parent: the declaring item or trait item).
    Param {
        /// The parameter's declared type.
        ty: TypeRefId,
        /// The parameter's index in the signature.
        index: u32,
    },
    /// The `self` receiver (parent: the declaring item or trait item).
    Receiver {
        /// The receiver's index in the signature.
        index: u32,
    },
    /// A local binding introduced by a `let` or a pattern.
    ///
    /// The defining body is the innermost body whose scopes contain the
    /// binding's declaration site (the enclosing block body; for match-arm
    /// pattern bindings, the enclosing body of the match).
    Local {
        /// `true` when the binding came from a `let mut` statement.
        mutable: bool,
        /// The `let` type annotation shared by every binding of the
        /// pattern, when written.
        ty: Option<TypeRefId>,
        /// The body the binding belongs to.
        body: BodyId,
    },
    /// An import binding (see [`ModuleBinding`]).
    ModuleBinding(ModuleBinding),
    /// A slot reserved by an interrupted allocation (id-space exhaustion
    /// or a resource-budget abort).
    ///
    /// This kind never appears in a `StageFate::Complete` module: it is
    /// written only where construction stopped, and its presence marks the
    /// result as incomplete. The structural validator reports it.
    Error,
}

/// The payload of a `DefKind::ModuleBinding` (P2D-B7; Directive 010-R2 §5).
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleBinding {
    /// The dotted import path as written, e.g. `["std", "io"]`.
    pub path: Vec<String>,
    /// The `as` alias, when written (`import std.io as io;`).
    pub alias: Option<String>,
    /// The import declaration's span.
    pub span: Span,
    /// The resolved target, or why it did not resolve.
    pub target: ModuleBindingTarget,
}

/// The resolution outcome of a module binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleBindingTarget {
    /// Resolved to a registered module: a usable binding.
    Resolved(ModuleId),
    /// A registered module whose interface this compilation cannot inspect
    /// (`surface = Unavailable`) — fail closed (`P2D-B7`).
    Unavailable(ModuleId),
    /// No such module registered — fail closed (`P2D-B7`).
    Missing,
    /// Not resolved yet: the pre-resolution state produced by structural
    /// lowering (WP-A).
    ///
    /// Import resolution (WP-B) replaces every occurrence with one of the
    /// three outcomes above before `LowerOutput` is returned; an
    /// `Unresolved` target is never a resolution result and never reaches
    /// a successful check.
    Unresolved,
}

// ---------------------------------------------------------------------------
// Uses
// ---------------------------------------------------------------------------

/// The use table ([`UseId`] indexes [`UseTable::uses`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UseTable {
    /// All recorded uses, indexed by [`UseId`].
    pub uses: Vec<Use>,
}

/// One recorded name use.
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    /// The written name. An empty name marks a parser-recovered
    /// placeholder occurrence (a documented intermediate state that
    /// resolution must not treat as a resolvable name).
    pub name: String,
    /// The name occurrence's exact span.
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Nodes
// ---------------------------------------------------------------------------

/// The node arena ([`HirId`] indexes [`NodeArena::nodes`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodeArena {
    /// All nodes, indexed by [`HirId`].
    pub nodes: Vec<HirNode>,
}

impl NodeArena {
    /// Returns the node with `id`, if it is in bounds.
    #[must_use]
    pub fn get(&self, id: HirId) -> Option<&HirNode> {
        self.nodes.get(id.0 as usize)
    }

    /// Returns the expression kind when `id` denotes an expression.
    #[must_use]
    pub fn expr(&self, id: HirId) -> Option<&ExprKind> {
        match self.get(id).map(|node| &node.kind) {
            Some(HirKind::Expr(kind)) => Some(kind),
            _ => None,
        }
    }

    /// Returns the statement kind when `id` denotes a statement.
    #[must_use]
    pub fn stmt(&self, id: HirId) -> Option<&StmtKind> {
        match self.get(id).map(|node| &node.kind) {
            Some(HirKind::Stmt(kind)) => Some(kind),
            _ => None,
        }
    }

    /// Returns the pattern kind when `id` denotes a pattern.
    #[must_use]
    pub fn pat(&self, id: HirId) -> Option<&PatKind> {
        match self.get(id).map(|node| &node.kind) {
            Some(HirKind::Pat(kind)) => Some(kind),
            _ => None,
        }
    }

    /// Iterates over every node in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = &HirNode> {
        self.nodes.iter()
    }
}

/// One expression, statement, or pattern node.
#[derive(Debug, Clone, PartialEq)]
pub struct HirNode {
    /// This node's identity.
    pub id: HirId,
    /// The node's source span (the originating construct's span for
    /// synthesized nodes).
    pub span: Span,
    /// The node's kind.
    pub kind: HirKind,
}

/// The kind-tagged payload of a [`HirNode`].
#[derive(Debug, Clone, PartialEq)]
pub enum HirKind {
    /// An expression.
    Expr(ExprKind),
    /// A statement.
    Stmt(StmtKind),
    /// A pattern.
    Pat(PatKind),
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// The shape of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A literal.
    Lit(HirLit),
    /// A path reference: one use per segment, in written order.
    ///
    /// (Layout note: the contract sketch shows `Path(UseId)`; the
    /// ratified rule "one use per segment" is expressed exactly by the
    /// segment vector.)
    Path {
        /// One recorded use per segment.
        segments: Vec<UseId>,
    },
    /// A unary operation.
    Unary(UnOp, HirId),
    /// A binary operation.
    Binary(BinOp, HirId, HirId),
    /// An assignment (target, operator, value).
    Assign(AssignOp, HirId, HirId),
    /// A call: callee expression plus arguments.
    Call(HirId, Vec<HirId>),
    /// A member access: base plus member selection state.
    Field(HirId, FieldSel),
    /// Indexing (`base[index]`).
    Index(HirId, HirId),
    /// A type cast.
    Cast(HirId, TypeRefId),
    /// `if cond { .. } else { .. }`; the else branch is a lowered
    /// expression (a block or another `if`).
    If(HirId, BodyId, Option<HirId>),
    /// `match scrutinee { arms }`, arms in source order.
    Match(HirId, Vec<Arm>),
    /// A block expression.
    Block(BodyId),
    /// `while cond { .. }`.
    While(HirId, BodyId),
    /// `loop { .. }`.
    Loop(BodyId),
    /// `return`, with an optional operand.
    Return(Option<HirId>),
    /// `break`; the target is the nearest enclosing loop node in the same
    /// body, recorded structurally at lowering time.
    Break {
        /// The targeted loop node.
        target: HirId,
    },
    /// `continue`; see [`ExprKind::Break`].
    Continue {
        /// The targeted loop node.
        target: HirId,
    },
    /// A tuple/unit expression (`()` is the empty tuple).
    Tuple(Vec<HirId>),
    /// An array literal.
    Array(Vec<HirId>),
    /// An error node: a parser-recovered placeholder, or a construct whose
    /// lowering was poisoned.
    Error,
    /// A call of a compiler-owned internal operation.
    ///
    /// Internal operations are not nameable, callable, or shadowable from
    /// source (`lowering-contract.md` §2).
    BuiltinCall {
        /// The internal operation.
        op: BuiltinOp,
        /// The operation's arguments, in evaluation order.
        args: Vec<HirId>,
    },
}

/// A compiler-owned internal operation produced by lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinOp {
    /// `a..b` → an internal `Range<T>` value; evaluates `a` then `b`,
    /// each exactly once (`P2D-B8`).
    RangeNew,
    /// The `for`-loop iteration step
    /// `range_next(r: Range<T>) -> Option<(T, Range<T>)>` (`P2D-B8`).
    RangeNext,
    /// Internal n-ary string concatenation for interpolation
    /// (`P2D-B10`).
    StrConcat,
    /// Primitive formatting of one interpolation part (`P2D-B10`).
    FormatPrim,
}

/// The member-selection state of a `Field` expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSel {
    /// Not resolved yet: the member name use to resolve later.
    ///
    /// Structural lowering produces this form for every `Field`; WP-B
    /// resolves module members and WP-C completes field/method selections
    /// in its own `Resolutions` side table (`P2D-A3`).
    Unresolved {
        /// The member name use.
        name: UseId,
    },
    /// A struct field.
    Field(DefId),
    /// A method.
    Method(DefId),
    /// A public member of an imported module (a `GlobalDefId` in
    /// practice; the definition id is carried inside
    /// [`Def`] identity for the current module).
    ModuleMember(DefId),
    /// A compiler-owned built-in member.
    Builtin(BuiltinMember),
}

/// A compiler-owned built-in member.
///
/// No built-in members exist in Phase 2; the enum is part of the ratified
/// `FieldSel` shape and has no variants yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinMember {}

/// A lowered `match` arm.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    /// The arm pattern.
    pub pat: HirId,
    /// The arm body expression.
    pub body: HirId,
    /// Span covering pattern through body.
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// The shape of a statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// A `let` declaration.
    ///
    /// The pattern carries one `DefKind::Local` binding per name it
    /// introduces; the `let` annotation (when written) is shared by every
    /// such local. (Layout note: the contract sketch shows
    /// `Let { local: DefId, init }`; the ratified rule "one `Local`
    /// definition per binding the pattern introduces plus a lowered
    /// pattern" is expressed exactly by the pattern node.)
    Let {
        /// The lowered binding pattern.
        pattern: HirId,
        /// The initializer.
        init: HirId,
    },
    /// An expression statement (the value is discarded).
    Expr(HirId),
    /// A nested item statement.
    Item(HirItemId),
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

/// The shape of a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    /// `_`: binds nothing.
    Wildcard,
    /// A lone-identifier pattern: a binding candidate.
    ///
    /// Classification is semantic (`P2D-B5`): if [`PatKind::Binding::candidate`]
    /// resolves to a unit-variant constructor, the pattern is finalized as
    /// [`PatKind::Variant`] with empty `args` and the `local` is not in
    /// scope; otherwise the pattern introduces the binding.
    Binding {
        /// The local binding this pattern introduces when it is not a
        /// constructor.
        local: DefId,
        /// The recorded candidate use; its resolution decides the final
        /// classification.
        candidate: UseId,
    },
    /// A literal pattern.
    Lit(HirLit),
    /// A constructor pattern: a path with payload sub-patterns.
    ///
    /// (Layout note: the contract sketch shows `Variant { variant: UseId,
    /// args }`; one use per path segment is expressed by the path vector,
    /// matching [`ExprKind::Path`].)
    Variant {
        /// One recorded use per path segment.
        path: Vec<UseId>,
        /// Payload sub-patterns (empty for unit variants).
        args: Vec<HirId>,
    },
    /// A tuple pattern (`()` is the empty tuple).
    Tuple(Vec<HirId>),
    /// An error node: a parser-recovered placeholder, or a construct whose
    /// lowering was poisoned.
    Error,
}

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

/// A lowered literal.
#[derive(Debug, Clone, PartialEq)]
pub enum HirLit {
    /// An integer literal: unsigned magnitude plus the written radix
    /// (accepted amendment B-2; target-type range checks are WP-C's).
    Int {
        /// The literal's magnitude (`_` separators already removed).
        magnitude: u64,
        /// The radix the literal was written in.
        radix: Radix,
    },
    /// A finite floating-point literal (accepted amendment B-3).
    Float(f64),
    /// A boolean literal.
    Bool(bool),
    /// A character literal.
    Char(char),
    /// An all-literal decoded string.
    Str(StringLit),
}

/// A decoded, all-literal string payload.
#[derive(Debug, Clone, PartialEq)]
pub struct StringLit {
    /// The decoded text (escapes resolved).
    pub text: String,
    /// Span of the whole literal, including both quotes.
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Type references
// ---------------------------------------------------------------------------

/// The type-reference table ([`TypeRefId`] indexes
/// [`TypeRefTable::types`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeRefTable {
    /// All type references, indexed by [`TypeRefId`].
    pub types: Vec<TypeRef>,
}

impl TypeRefTable {
    /// Returns the type reference with `id`, if it is in bounds.
    #[must_use]
    pub fn get(&self, id: TypeRefId) -> Option<&TypeRef> {
        self.types.get(id.0 as usize)
    }
}

/// One syntactic type reference.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    /// This reference's identity.
    pub id: TypeRefId,
    /// The reference's shape.
    pub kind: TypeRefKind,
    /// Span covering the whole type.
    pub span: Span,
}

/// The shape of a syntactic type reference.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeRefKind {
    /// A (possibly generic) named type: one use per segment plus generic
    /// arguments.
    Path {
        /// One recorded use per segment, in written order.
        segments: Vec<UseId>,
        /// Generic arguments, in written order.
        generics: Vec<TypeRefId>,
    },
    /// A tuple type (`()` is the unit type).
    Tuple(Vec<TypeRefId>),
    /// A slice type.
    Slice(TypeRefId),
    /// An array type `[T; N]`; the length is a lowered expression.
    Array {
        /// The element type.
        elem: TypeRefId,
        /// The length expression.
        len: HirId,
    },
    /// A function type.
    Fn {
        /// Parameter types.
        params: Vec<TypeRefId>,
        /// The return type, when written (unit when absent).
        ret: Option<TypeRefId>,
    },
    /// The contextual type name `Self` (`P2D-B14`).
    SelfTy,
    /// An error type reference: a parser-recovered placeholder, or a
    /// construct whose lowering was poisoned.
    Error,
}

// ---------------------------------------------------------------------------
// Bodies and signatures
// ---------------------------------------------------------------------------

/// The body table ([`BodyId`] indexes [`BodyTable::bodies`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BodyTable {
    /// All bodies, indexed by [`BodyId`].
    pub bodies: Vec<Body>,
}

impl BodyTable {
    /// Returns the body with `id`, if it is in bounds.
    #[must_use]
    pub fn get(&self, id: BodyId) -> Option<&Body> {
        self.bodies.get(id.0 as usize)
    }
}

/// One body: a function/method body or a block scope owner.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Body {
    /// Statement nodes in source order.
    pub stmts: Vec<HirId>,
    /// The tail (value) expression, when present.
    pub tail: Option<HirId>,
}

/// The signature table ([`FnSigId`] indexes [`SigTable::sigs`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SigTable {
    /// All signatures, indexed by [`FnSigId`].
    pub sigs: Vec<FnSig>,
}

impl SigTable {
    /// Returns the signature with `id`, if it is in bounds.
    #[must_use]
    pub fn get(&self, id: FnSigId) -> Option<&FnSig> {
        self.sigs.get(id.0 as usize)
    }
}

/// A lowered function signature.
#[derive(Debug, Clone, PartialEq)]
pub struct FnSig {
    /// The defining item's own definition (function, method, or trait
    /// item).
    pub name: DefId,
    /// Parameter (and receiver) definitions in order.
    pub params: Vec<DefId>,
    /// The declared return type, when written.
    pub ret: Option<TypeRefId>,
    /// Span covering the signature (name through return type).
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Resolutions (side table)
// ---------------------------------------------------------------------------

/// The syntactic resolution side table returned by lowering.
///
/// `Resolutions` is stage-owned data, never part of [`HirModule`]
/// (`P2D-A3`): structural lowering returns every source use as
/// [`Resolution::Unresolved`] and every synthesized internal reference as
/// an established [`Resolution`]. WP-B replaces the `Pending` entries with
/// resolved definitions and returns the completed table; WP-C returns its
/// own completed table. The HIR itself is never rewritten outside
/// lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resolutions {
    /// One resolution per [`UseId`], in use-allocation order.
    pub uses: Vec<Resolution>,
}

impl Resolutions {
    /// Returns the resolution of `id`, if it is in bounds.
    #[must_use]
    pub fn get(&self, id: UseId) -> Option<&Resolution> {
        self.uses.get(id.0 as usize)
    }
}

/// The resolution state of one use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Resolved to a definition in this module.
    Def(DefId),
    /// Resolved to a definition in another module.
    Global(GlobalDefId),
    /// A compiler-owned internal reference established by desugaring
    /// (never a name lookup).
    Internal(InternalDef),
    /// Not resolved; see [`UnresolvedReason`].
    Unresolved(UnresolvedReason),
}

/// Why a use is unresolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedReason {
    /// Structural lowering recorded the use; name resolution (WP-B) has
    /// not run.
    ///
    /// This is the only state produced by WP-A's structural output for
    /// source uses. It is not a resolution and must never be treated as
    /// one.
    Pending,
}

/// A compiler-owned internal definition referenced by a desugaring.
///
/// These references carry fixed identities and are established at
/// expansion time; they are never resolved through user scopes, so user
/// items with the same surface names cannot capture them
/// (`lowering-contract.md` §2, `P2D-B9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InternalDef {
    /// The prelude `Option::Some` constructor (used by `for`).
    OptionSome,
    /// The prelude `Option::None` constructor (used by `for`).
    OptionNone,
    /// The prelude `Result::Ok` constructor (used by `?`).
    ResultOk,
    /// The prelude `Result::Err` constructor (used by `?`).
    ResultErr,
}
