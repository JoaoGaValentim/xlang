# XLANG Phase 2 — HIR Contract

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §7 (HIR architecture), amended
  by Engineering Directive 010-R1 §5 (HIR identity and overflow safety)
  and §6 (cross-module and package identity).
- **Companion documents:** `architecture-rfc.md` (stage boundaries),
  `lowering-contract.md` (per-node lowering and desugaring),
  `name-resolution-contract.md` (symbol tables and resolution),
  `type-system-contract.md` (typing over this representation),
  `decision-packet.md` (decisions `P2D-A1`, `P2D-A2`, `P2D-A3`, `P2D-A4`,
  `P2D-A5`).
- **No implementation is authorized.** Sketches below are indicative: the
  exact Rust names and field layout are implementation details for a
  future authorized package, constrained only by the properties marked
  **Contract**.

Evidence labels: SPECIFIED / DERIVED / PROPOSED / UNRESOLVED / FUTURE (see
the preamble of `architecture-rfc.md`).

## 1. Purpose and position

`xl-hir` is the stage between the accepted syntax tree and the type
checker (SPECIFIED §4: *name resolution + desugaring → HIR → type
checker*). It owns:

1. **Lowering** every `xl_ast` item, statement, expression, pattern, and
   type into an identity-stable, arena-owned representation.
2. **Definition collection** — items, variants, fields, methods, trait
   items, parameters, generics, and local bindings become `DefId`/
   `LocalId` definitions with names, kinds, spans, and visibility.
3. **Syntactic name resolution** — every name use is recorded and resolved
   against scopes, items, imports, and the prelude
   (`name-resolution-contract.md`).
4. **Desugaring** — `for`, `?`, and string interpolation are eliminated
   (`lowering-contract.md` §4–6) so typed HIR contains no such nodes.

`xl-hir` performs **no type checking**. It records what the program says
and which definitions its names refer to; whether those references are
well-typed is `xl-types`' concern. The boundary cases (mutability,
member lookup) are assigned explicitly (§6).

## 2. Design principles

- **Identity, not pointers.** Follow the SPECIFIED §4 convention: newtype
  IDs (`HirId`, `DefId`, `LocalId`, `UseId`) into flat arenas — never
  `Rc`, never interior mutability, never global state.
- **Source fidelity.** Every HIR node carries its exact `Span`
  (ADR-0002 UTF-8 byte offsets, half-open). Desugared nodes carry the
  span of the originating construct (`P2D-A2`). A diagnostic must always
  be able to point at the source text the user wrote.
- **Total and pure.** `lower` never panics and never performs I/O; it
  handles any structurally plausible `xl_ast` input, including trees with
  placeholder/error nodes from parser recovery.
- **Deterministic.** All identifiers are allocated in a deterministic
  traversal order (items in source order; within an item, signature before
  body; within a body, lexical order). No hash-order-dependent behavior.
- **Flat storage.** Recursive `Box` trees are deliberately replaced with
  arenas: dropping a `HirModule` is non-recursive, so deep *hand-built*
  inputs cannot overflow the stack on drop (a limitation the accepted
  `xl-ast` documents for itself, `xl-ast` crate docs). Arena storage also
  directly implements the SPECIFIED “IDs in arenas/Vec” convention.

## 3. Module structure

```rust
// Indicative sketch — not an implementation.
pub struct HirModule {
    pub module: ModuleId,             // stable identity of this module
    pub package: PackageId,           // owning package (Phase 2: the synthetic package)
    pub file: xl_span::FileId,
    pub items: Vec<HirItemId>,        // module items, source order
    pub defs: DefTable,               // all definitions
    pub uses: UseTable,               // all name uses, recorded by lowering
    pub exprs: ExprArena,             // all expressions, all bodies
    pub stmts: StmtArena,
    pub pats: PatArena,
    pub bodies: BodyTable,            // per-function bodies
    pub ast_origin: AstOriginTable,   // HIR node -> its AST source origin
    pub resolutions: Resolutions,     // syntactic resolutions (side table)
}

pub enum HirItemKind { Fn, Struct, Enum, Trait, Impl, Import }

// Module and package identity (P2D-A5; Directive 010-R1 §6). `ModuleId` is
// stable within one compilation and carries (or is looked up alongside)
// its `PackageId`; a `GlobalDefId` names a definition across modules.
pub struct ModuleId(u32);
pub struct PackageId(u32);
pub struct GlobalDefId { pub module: ModuleId, pub definition: DefId }
```

- One `HirModule` corresponds to one parsed file: **1 `.x` file = 1
  module** (SPECIFIED §2). In Phase 2 the CLI registers **one** `HirModule`
  into the module registry under a synthetic single `PackageId`; genuine
  multi-file/package compilation is FUTURE (`P2D-A5`/`P2D-B7`, Category
  C1), but the identity types above are fixed now so the registry, the
  import edges, and the cycle-detection harness are executable today.
- `Impl` items remain items (they declare methods, not a named type);
  their methods are definitions owned by the impl, associated with a
  *self type use* (resolved in WP-C; inherent vs trait impl recorded from
  syntax, which already distinguishes `ImplDecl.trait_`).
- `Import` items are lowered to import records (path segments, alias,
  span) feeding the module-binding step (`name-resolution-contract.md`
  §6).
- Struct/enum/trait declarations are lowered in full — including fields,
  variants, payload types, generic parameters and bounds, trait item
  signatures — even when unused.

## 4. Stable node identities and source references (P2D-A2)

```rust
pub struct HirId(u32);    // expression / statement / pattern nodes (one space, kind-tagged)
pub struct HirItemId(u32);// module/block item slots (index into HirModule.items)
pub struct DefId(u32);    // named definitions: items, variants, fields,
                          // methods, trait items, generics, params, locals
pub struct LocalId(u32);  // NOT a separate identity space: a documented
                          // `DefId` specialization. `DefKind::Local` values
                          // share the `DefId` space; `LocalId` is a typed
                          // view/newtype for call sites that want the
                          // guarantee, not a distinct arena.
pub struct UseId(u32);    // one per recorded name use (path, type name, pattern path...)
pub struct BodyId(u32);   // one per function body (and each block scope owner)
pub struct TypeRefId(u32);// one per syntactic type reference
```

`LocalId` vs `DefId`: **`LocalId` is a documented `DefId` specialization**
(Directive 010-R1 §5), not a separate identity type or arena. A local
binding is a `Def` with `DefKind::Local` in the single `DefId` space;
`LocalId` exists only as a typed view where a call site wants the stronger
guarantee that the id denotes a local. Consequently a `DefId` and a
`LocalId` can never refer to the same entity under two different numbers:
there is exactly one numbering.

Contract properties:

1. **Allocation order is deterministic** and specified by traversal:
   items in source order; for each item, the name/definition first, then
   generics, then signature types (left to right), then the body (lexical
   order); desugared items are allocated at the point of the originating
   construct, in expansion order (`lowering-contract.md` §3).
2. **Stability scope and cross-module identity.** Identifiers are stable
   within one `HirModule` instance. A bare `DefId` is **not** globally
   unique across modules, so an unqualified `DefId` must never be used to
   identify a declaration globally. Cross-module references use the
   composite `GlobalDefId { module: ModuleId, definition: DefId }`
   (Directive 010-R1 §6). Definitions from different modules therefore
   cannot alias accidentally. Phase 2 registers a single module, so every
   `GlobalDefId` has the same `ModuleId`, but the type and the registry
   seam are fixed now.
3. **Overflow: checked allocation, never saturation** (Directive 010-R1
   §5). Counters are **checked**, not saturating:
   - `allocate()` returns `Result<Id, IdSpaceExhausted>`.
   - On exhaustion **no id is reused, wrapped, or saturated**; in
     particular, two distinct entities can never receive the same id.
   - Exhaustion is a controlled internal error: construction stops safely
     and the `lower` result is marked `StageFate::Aborted` and is
     **unusable for a successful semantic validation** (the CLI maps it to
     a non-zero exit, never `0`). It is never a panic.
   - This is a correctness property, not a probability argument: the
     design does not rely on `u32::MAX` being unreachable.
4. **Accurate source origin (`AstOrigin`), not spans-as-keys.** The
   accepted AST assigns an id **only** to `Item` (`ItemId`) and `Expr`
   (`ExprId`) — verified in `crates/xl-ast/src/id.rs` and `ast.rs`;
   statements, patterns, and types carry **no** AST id. Lowering therefore
   records, for each HIR node, an `AstOrigin`:

   ```rust
   pub enum AstOrigin {
       Item(xl_ast::ItemId),          // node lowered from an Item
       Expr(xl_ast::ExprId),          // node lowered from an Expr
       Structural {                   // nodes with no AST id (stmt/pat/type/...)
           anchor: AnchorId,          // nearest enclosing id-bearing AST node
           path: Vec<u32>,            // deterministic child-index path from anchor
       },
       Synthesized { construct: DesugarKind, span: Span }, // desugared
   }
   ```

   - The `Structural` key is deterministic: an anchor (the nearest
     enclosing `Item`/`Expr`, which does have an AST id) plus the
     child-index path taken by the single lowering traversal. Distinct
     syntactic nodes receive distinct keys even when they share a span.
   - **Spans are never used as identity keys.** Grouping parentheses are
     transparent (amendment B-1) and recovered placeholders are zero-width
     at a deterministic insertion point, so spans are not unique; using
     them as keys would conflate distinct nodes.
   - `Origin::Desugared { construct }` remains an internal flag on the HIR
     node for debug dumps and tests; the richer `AstOrigin::Synthesized`
     record additionally ties each synthesized node to the originating
     construct’s span for diagnostics.
5. **Every HIR node has a span.** For syntactic nodes, the AST span
   verbatim. For desugared/synthesized nodes, the originating construct’s
   span, flagged with `Origin::Desugared { construct: DesugarKind }`
   (`P2D-A2`); the flag is not user-visible.

**Completeness invariant (implementable).** The `AstOriginTable` is total:
every HIR node produced from exactly one AST node has an `AstOrigin`
entry, and every AST node reachable from the input items has a
corresponding entry (the reverse mapping used by completeness tests).
Because grouping is transparent and statements/patterns/types have no AST
id, completeness is stated over **lowering-traversal visits**, not over
AST ids: the validator walks the AST and the HIR together and asserts that
every visited AST node produced exactly one HIR node with a recorded
origin. Matching tests are listed in §11 and `conformance-plan.md` §7.

## 5. Item, statement, expression, pattern, and type representation

The following sketch is the **contract level**: node kinds and payloads
must be able to represent every derivation of the accepted grammar and
every desugared form; naming and exact layout are implementation details.

### 5.1 Definitions

```rust
pub struct Def {
    pub id: DefId,
    pub name: String,            // owned; no interning in Phase 2 (P2D-A4)
    pub kind: DefKind,
    pub span: Span,              // name span for items; whole-node spans where noted
    pub visibility: Visibility,  // Private | Public(span); recorded, enforced per P2D-B20
    pub parent: Option<DefId>,   // enum for variants; impl for methods; item for params
}

pub enum DefKind {
    Fn { generics: Vec<GenericParam>, sig: FnSigId, body: Option<BodyId> },
    Struct { generics: Vec<GenericParam>, fields: Vec<DefId> },
    Enum { generics: Vec<GenericParam>, variants: Vec<DefId> },
    Trait { generics: Vec<GenericParam>, items: Vec<DefId> },
    Impl { generics: Vec<GenericParam>, trait_ref: Option<TypeRefId>,
           self_ty: TypeRefId, is_inherent: bool },
    Variant { payload: Vec<TypeRefId> },   // parent = enum
    Field { ty: TypeRefId },               // parent = struct
    Method { generics: Vec<GenericParam>, sig: FnSigId, body: Option<BodyId> },
    TraitItem { sig: FnSigId },            // signature-only (accepted grammar)
    GenericParam { bound: Option<TypeRefId> },
    Param { ty: TypeRefId, index: u32 },   // function parameters
    Receiver { index: u32 },               // `self`; typed in xl-types from impl/trait context
    Local { mutable: bool, ty: Option<TypeRefId>, body: BodyId },  // let bindings + pattern bindings
}
```

- **Differences from the AST (justified):**
  - AST `FnDecl` (name/signature/body in one node) becomes a `Def` plus
    arena entries: identity must survive desugaring and be referencable by
    `UseId` resolutions.
  - AST `ImplItem` (a wrapper) disappears; methods become `Method` defs
    whose `parent` names the impl (needed for inherent/trait method
    selection in `xl-types`).
  - AST `Ident` (name + span) is split: the definition gets the name; the
    *use* gets a `UseId` + span. This is what prevents accidental aliasing
    of identical textual names in different scopes (`P2D-B1`).
  - Visibility is carried as recorded data (the accepted AST shape);
    enforcement semantics are `P2D-B20`.
- **Definition identity across modules (`P2D-A2`, Directive 010-R1 §6).**
  A `Def` stores an instance-scoped `DefId`; any reference that can cross a
  module boundary is stored as a `GlobalDefId { module, definition }`
  (`§3`) so that a definition of module A and one of module B never alias.
  Within the single module of Phase 2, `GlobalDefId { module: this, .. }`
  is the identity used by the registry and by the trait/orphan checks; the
  bare `DefId` remains an internal index.
- `GenericParam` bounds: at most one bound per parameter (the accepted
  grammar has one optional bound; SPECIFIED surface). Multiple bounds per
  parameter are not representable in source and therefore not in HIR.

### 5.2 Signatures and type references

```rust
pub struct FnSig { pub name: DefId, pub params: Vec<DefId>,
                   pub ret: Option<TypeRefId>, pub span: Span }

pub struct TypeRef { pub id: TypeRefId, pub kind: TypeRefKind, pub span: Span }
pub enum TypeRefKind {
    Path { segments: Vec<UseId>,    // each segment is a recorded use
           generics: Vec<TypeRefId> },
    Tuple(Vec<TypeRefId>),
    Slice(TypeRefId),
    Array { elem: TypeRefId, len: HirId },   // len is a lowered expression
    Fn { params: Vec<TypeRefId>, ret: Option<TypeRefId> },
    SelfTy,                          // contextual `Self` (P2D-B14)
}
```

- Type references are **syntactic** at this stage: `Path` segments are
  uses to be resolved (to structs, enums, traits, generic parameters, or
  prelude types). `xl-types` converts `TypeRef`s into `TyId`s once
  resolution has run (`type-system-contract.md` §3).
- `Array.len` stays an expression (accepted grammar: `[T; N]`), lowered
  like any expression; its acceptance criteria are `P2D-B18`.
- Grouping is already transparent in the accepted AST, so no `Grouped`
  node exists here either (consistent with amendment B-1).

### 5.3 Statements

```rust
pub struct Stmt { pub id: HirId, pub kind: StmtKind, pub span: Span }
pub enum StmtKind {
    Let { local: DefId, init: HirId },      // pattern already lowered into local(s)
    Expr(HirId),                            // expression statement (value discarded)
    Item(HirItemId),                        // nested items (accepted grammar)
}
```

- The accepted `Block { stmts, tail }` split is preserved: a body has
  `stmts: Vec<HirId>` and `tail: Option<HirId>`; the tail's value is the
  block's value (SPECIFIED). This is not re-derived by “last expression”
  heuristics.
- `Let` initializer type annotations (`: type`) lower to
  `Local.ty: Option<TypeRefId>`; the pattern is desugared into one `Local`
  definition per binding it introduces plus a lowered pattern used for
  destructuring checking (`lowering-contract.md` §3.2).

### 5.4 Expressions

```rust
pub enum ExprKind {
    // ---- direct lowerings of accepted syntax ----
    Lit(HirLit),                       // Int { magnitude, radix } | Float | Bool | Char | Str(all-literal)
    Path(UseId),                       // resolved name use: local, item, variant, prelude item
    Unary(UnOp, HirId),
    Binary(BinOp, HirId, HirId),
    Assign(AssignOp, HirId, HirId),
    Call(HirId, Vec<HirId>),           // callee expression + arguments
    Field(HirId, FieldSel),            // base + member: resolved in xl-types (P2D-A3)
    Index(HirId, HirId),
    Cast(HirId, TypeRefId),
    If(HirId, BodyId, Option<HirId>),  // else branch is a block or `if` expression
    Match(HirId, Vec<Arm>),
    Block(BodyId),
    While(HirId, BodyId),
    Loop(BodyId),
    Return(Option<HirId>),             // operand optional (accepted grammar)
    Break { target: HirId },           // target = enclosing loop node (lowering records it)
    Continue { target: HirId },
    Tuple(Vec<HirId>),                 // unit = empty tuple
    Array(Vec<HirId>),
    Error,                             // error node (see §7)
    // ---- desugared forms (never present as source syntax) ----
    BuiltinCall { op: BuiltinOp, args: Vec<HirId> },
}
```

- `BuiltinOp` enumerates the compiler-owned internal operations:
  `RangeNew` (`a..b`), `RangeNext` (`for` iteration step), `StrConcat`
  (interpolation concatenation), `FormatPrim` (primitive formatting of one
  interpolation part). Each op’s typing and semantics are fixed in
  `lowering-contract.md` §4–6; none of them is nameable, callable, or
  shadowable from user code (`lowering-contract.md` §2). The `?`
  desugaring needs no built-in: its synthesized patterns and uses (the
  `Some`/`None` patterns of `for`, the `Ok`/`Err` patterns of `?`, and
  the `Err` callee) are inserted into the resolution table
  **pre-resolved** to internal prelude definitions; the resolver never
  rewrites them (`P2D-B9`).

- `Try` and `For` from the accepted AST **do not survive** lowering: they
  are rewritten into `Match`/`Loop` structures (`lowering-contract.md`
  §4–5). `Range` does not survive either: `a..b` becomes
  `BuiltinCall { op: RangeNew, … }` producing the internal `Range` value
  (`P2D-B8`/`P2D-B4`).
- `Break`/`Continue` record their target loop at lowering time
  (structurally: nearest enclosing `Loop`/`While` in the same body). This
  removes any later need for label analysis and makes `break` inside a
  desugared `for` body target the desugared loop deterministically.
- `FieldSel` is `Unresolved { name: UseId } | Field(DefId) |
  Method(DefId) | ModuleMember(DefId) | Builtin(BuiltinMember)`; it starts
  unresolved and is completed by `xl-types` (`P2D-A3`).
  `ModuleMember` covers the accepted `io.println` shape (frozen by the
  Phase 1 correction F-3): the base resolves to a module binding and the
  member to a public item of that module.
- **String literals without interpolation** remain `Lit(Str)`; string
  literals **with** interpolation lower to `BuiltinCall { op: StrConcat,
  … }` (parts: all-literal parts kept as `Lit(Str)`; interpolated parts
  wrapped as `BuiltinCall { op: FormatPrim, … }` when not already `str`,
  per `P2D-B10`).
- Operator/precedence information is not needed in HIR: the accepted AST
  already encodes grouping; HIR mirrors the tree.

### 5.5 Patterns

```rust
pub struct Pat { pub id: HirId, pub kind: PatKind, pub span: Span }
pub enum PatKind {
    Wildcard,
    Binding { local: DefId },          // new binding (DefKind::Local)
    Lit(HirLit),
    Variant { variant: UseId,          // path/constructor use; resolved in xl-hir
              args: Vec<HirId> },      // payload sub-patterns
    Tuple(Vec<HirId>),
    Error,
}
```

- The accepted syntax-level distinction is preserved: a lone identifier
  is a *binding candidate*; a path with arguments or with `::` is a
  constructor candidate (Phase 1 §3(4)). The **final** classification of
  a bare identifier that resolves to a unit variant is a semantic rule
  (`P2D-B5`, `name-resolution-contract.md` §7.4).
- Patterns do not carry types; typing and exhaustiveness work on this
  structure in `xl-types`.

### 5.6 Literals

```rust
pub enum HirLit {
    Int { magnitude: u64, radix: Radix },   // exactly the accepted lexer payload (B-2)
    Float(f64),                              // always finite (B-3)
    Bool(bool),
    Char(char),
    Str(StringLit { text: String, span }),   // all-literal string (decoded)
}
```

Integer literals deliberately keep **magnitude + radix** (accepted
amendment B-2): the target-type range check is a Phase 2 obligation
(`P2D-B11`); no information is lost between lexer and checker.

## 6. Ownership, allocation, and resolution storage (P2D-A2, P2D-A3)

- `HirModule` owns every arena. No node owns another node recursively;
  references are ids. `Clone` is not required; the module is moved by
  value. `HirModule` is **immutable after `lower` returns**; `xl_types`
  never mutates it (`P2D-A3`).
- **Resolutions are side tables**, not HIR rewrites, and they are
  *stage-owned*:
  `Resolutions { use_: Vec<Resolution>, field: Vec<FieldResolution>, ... }`
  indexed by `UseId`/`HirId`. `xl_hir::lower` returns the **syntactic**
  `Resolutions` in its `LowerOutput`; `xl_types::check(&HirModule,
  &Resolutions)` receives it and returns its **own** completed
  `Resolutions` (member/method/selection entries added or updated) in
  `CheckOutput`. This keeps `check` a pure function over immutable HIR
  (multiple checks with different — future — settings stay possible) and
  keeps HIR dumps stable across stages. There is no in-place mutation of a
  shared table across the stage boundary.
- `Resolution` is total: `Unresolved { reason }` is an explicit state that
  always has a corresponding diagnostic (WP-B or WP-C emitted it). An
  invariant test asserts: after `check`, every use is either resolved or
  marked unresolved with a diagnostic (`architecture-rfc.md` §8).

## 7. Error nodes and partial-HIR policy

Parser recovery already produces placeholder AST nodes; lowering must be
total over them:

1. **Error nodes exist at every level:** `ExprKind::Error`,
   `StmtKind::Expr(Error)`, `PatKind::Error`, missing `TypeRefId`s
   (represented as an `Error` type ref), and definitions with empty names
   are possible intermediate states.
2. **Lowering of an error node never emits a new diagnostic.** Lexical and
   syntactic errors were already reported by `xl-parser`; lowering adds
   diagnostics only for errors that are *new at this stage* (e.g. a
   structurally invalid tree that nevertheless parsed — defensive path).
3. **Poisoning:** any expression whose subtree contains an error node gets
   type `TyKind::Error` in `xl-types`; `Error` unifies with everything and
   produces no diagnostics. This is the cascade-suppression mechanism
   (`diagnostics-contract.md` §7).
4. **Completeness invariant:** lowering always produces a `HirModule` that
   traverses without panics; there is no "failed lowering" return value for
   *source* errors. Partiality from source errors is represented by error
   nodes, never by absence of structure. (DERIVED from the never-panic
   convention and from the accepted parser's "callers always observe a
   complete tree".)
5. **Placeholders are never presented as valid:** like the accepted
   tree/printer policy, tests assert that error nodes are flagged and
   cannot silently satisfy resolution or typing.
6. **The one non-source partial result:** an `IdSpaceExhausted` internal
   error (§4.3) is *not* a source error and *not* an error node — it is an
   unrecoverable resource failure. Lowering then returns `StageFate::Aborted`
   and a `HirModule` that must **not** be used for a successful semantic
   validation (`architecture-rfc.md` §3; `xc check` exits non-zero). This
   is the only case in which the "no failed lowering return value" rule is
   qualified.

## 8. Boundaries: lowering vs. resolution vs. typing

The contract-level split (detailed in companion documents):

| Question | Answered by | When |
| --- | --- | --- |
| What definitions exist? | `xl-hir` def collection | before body lowering |
| Which definition does this local/item/variant/type name refer to? | `xl-hir` resolver | during body lowering |
| Does this module path exist; is this import binding legal? | `xl-hir` (registry seam) | before bodies (`P2D-B7`) |
| What does `expr.name` mean (field? method? module member?) | `xl-types` member resolution | WP-C (`P2D-A3`) |
| What is the type of anything? | `xl-types` | WP-C |
| Is this assignment legal (mutability)? | `xl-types` (needs root binding + type) | WP-C |
| Is this match exhaustive? | `xl-types` (needs pattern types + enum data) | WP-C |

`xl-hir` must not depend on type information; where it records a member
use it records *candidates* already known syntactically (e.g. module
bindings) and defers the rest.

## 9. Structural validation of HIR

Mirroring `xl-ast`'s validator (accepted Phase 1 design), `xl-hir` ships a
**structural validator** used by tests and by the CLI's defensive path
(never by normal execution):

- arena index ranges; parent/child references point inside the module;
- spans are within the file and non-inverted (checked against
  `SourceMap` in tests only; the validator checks local sanity);
- every `DefId` has a name and a kind consistent with its parent;
- block tail ordering is well-formed; `Break`/`Continue` targets enclose
  the node (structurally);
- resolution table sizes match use/expr counts;
- `HirModule.module` and `HirModule.package` are set and consistent with
  the registry entry that produced the module (`P2D-A5`);
- the `AstOriginTable` is total over lowering-traversal visits, and every
  `AstOrigin::Structural` key is well-formed (a valid anchor and a
  well-ordered child-index path) — no two HIR nodes share an origin key;
- traversal depth bounded by `MAX_HIR_DEPTH`, node count by
  `MAX_HIR_NODES`, defs by `MAX_DEFS`, uses by `MAX_USES` (`P2D-A7`).

## 10. Complexity and resource notes

- Memory is O(AST size): one arena slot per lowered node plus side tables.
  Roughly 3–5× the AST footprint (ids + resolutions); acceptable for a
  frontend.
- No recursive drop; no recursion in traversal beyond explicit depth
  budgets.
- Lowering is single-pass per body except def collection (two passes:
  collect, then lower) — necessary for order-independent module items and
  for prelude/import bindings (`name-resolution-contract.md` §5).

## 11. Invariants (testable)

1. Determinism: lowering the same items twice yields identical arenas,
   ids, origins, and diagnostics.
2. **Origin completeness (implementable):** every HIR node produced from
   an AST node has an `AstOrigin`; nodes with an AST id (`Item`/`Expr`)
   record it; statements/patterns/types record a unique `Structural` key;
   no two HIR nodes share an origin key. Tests walk AST and HIR together
   and assert the traversal-visit bijection (§4).
3. Span fidelity: every HIR node's span equals its AST origin, except
   synthesized nodes (flagged; span = originating construct).
4. No desugared-syntax leakage: typed HIR contains no `For`, `Try`,
   `Range`, or interpolated string nodes — only their lowered forms.
5. Resolution totality after `check` (see §6).
6. Validator acceptance: every module produced by `lower` passes
   `validate`; every module produced by tests that mutates arenas
   illegally fails it (negative tests).
7. **Checked-allocation safety:** constructing more entities than the
   `u32` id space allows yields `IdSpaceExhausted` (no reuse, no
   saturation, no panic) and a `StageFate::Aborted` result that the CLI
   never reports as a successful check (`P2D-A2`; Directive 010-R1 §5).

## 12. Traceability

| Contract item | Source |
| --- | --- |
| Stage position (resolution + desugar → HIR) | SPECIFIED §4 |
| Deliverables (symbol table, desugaring set, duplicate/undefined/cyclic detection) | SPECIFIED §6 Phase 2 |
| Newtype ids + arenas; pure phases; never panic | SPECIFIED §4 conventions |
| 1 file = 1 module | SPECIFIED §2 |
| AST input shapes (`Block.tail`, `AssignOp`, `StringLit`, `Param::Receiver`, instance-scoped ids) | accepted `xl-ast` implementation + Phase 1 contracts |
| AST ids exist only on `Item`/`Expr`; stmt/pat/type origin keys | accepted `xl-ast` (`id.rs`, `ast.rs`) + Directive 010-R1 §5 |
| Checked id allocation; no saturation/aliasing; `IdSpaceExhausted` | Directive 010-R1 §5 |
| Cross-module `GlobalDefId`/`ModuleId`/`PackageId` | Directive 010-R1 §6; `P2D-A5` |
| Module registry, import edges, executable cyclic detection | SPECIFIED §6 Phase 2; Directive 010-R1 §3; `P2D-A5`/`P2D-B7` |
| `LocalId` = `DefId` specialization | Directive 010-R1 §5 |
| Desugaring set (`for`, `?`, interpolation) | SPECIFIED §6 Phase 2 |
| B-2 literal payload preservation | accepted amendment B-2 |
| Error-tolerant, complete-tree behavior | accepted Phase 1 recovery contract (extended here) |
| `MAX_HIR_DEPTH`/`MAX_HIR_NODES`/`MAX_DEFS`/`MAX_USES` | `P2D-A7`; Directive 010-R1 §8 |
