# XLANG Phase 2 — Lowering and Desugaring Contract

**STATUS: OWNER-RATIFIED (Directive 010-R3) — PENDING INTEGRATION (PR #7 not merged)**

- **Directive:** Engineering Directive 010 §7 (AST→HIR lowering and the
  required desugaring designs).
- **Companion documents:** `hir-contract.md` (representation),
  `name-resolution-contract.md` (scopes and def collection),
  `type-system-contract.md` (typing of lowered forms),
  `traits-and-exhaustiveness.md` (refutability reuse),
  `decision-packet.md` (`P2D-B5`, `P2D-B8`, `P2D-B9`, `P2D-B10`,
  `P2D-B18`, `P2D-B22`).
- **No implementation is authorized.** All rules below are PROPOSED
  unless labeled otherwise.

## 1. Lowering pipeline (order of operations)

Lowering a file (or, in the registry harness, each registered module)
proceeds in a fixed, deterministic order. **Lowering does not perform
graph validation** (Directive 010-R3 §4.3): it reads an **immutable**
registry and *returns* the import edges it discovered; the caller registers
them and then invokes the graph validator separately (see §1.1).

1. **Module scan** — collect module-level items in source order into
   definitions (two-pass: collect names first, then lower bodies, so
   module items are order-independent; `name-resolution-contract.md` §5).
2. **Import resolution and edge collection** — lower `ImportDecl`s, create
   module bindings, resolve each import target against the caller-provided
   `ModuleRegistry` (§2.3: `resolve`/`surface`), and **collect** the
   resulting `ImportEdge { from, path, span, target }` (with its span and
   `ImportTarget`) in the module’s declaration order (`P2D-A5`, `P2D-B7`).
   `lower` **returns** these edges in `LowerOutput.import_edges`; it does
   **not** register them and does **not** traverse the graph. The caller
   records them via `record_import` **after** lowering (§1.1), so the
   separate graph validator sees the complete graph. Missing/unavailable
   imports are resolved **fail closed**
   (`name-resolution-contract.md` §9.3).
3. **Signature lowering** — lower generics, parameters, and return types
   of every function/method/trait item into `TypeRef`s; lower struct
   fields and enum variants/payloads; lower impl headers.
4. **Body lowering** — for each function/method body (source order):
   create the body's scope chain, lower statements and the tail, resolving
   every name use against the scope tables as it goes (sequential
   visibility for locals; hoisted items per `P2D-B3`, §3.2), and apply the
   three desugarings (§4–6) at the point of the originating construct.
5. **Structural validation** — run the structural HIR validator on the
   result (`hir-contract.md` §9).

Lowering never reorders user code except where a desugaring specifies it;
diagnostics from steps 1–4 are emitted in this order.

`lower` receives `(&[Item], FileId, &ModuleRegistry)` — **by shared,
immutable reference** — and returns the result struct `LowerOutput`
(`architecture-rfc.md` §3, `P2D-A1`):

```rust
// Indicative layout; the observable contract is the field set.
pub struct LowerOutput {
    pub module: HirModule,
    pub resolutions: Resolutions,
    pub import_edges: Vec<ImportEdge>,   // discovered edges, caller records them
    pub diagnostics: Vec<Diagnostic>,
    pub fate: StageFate,
}
```

`lower` does **not** depend on `xl-parser`: it consumes `xl_ast::Item`
values. It never mutates the registry (no interior mutability, no global
state). If checked id allocation is exhausted, `fate =
StageFate::Aborted` and the result is unusable for a successful check
(`hir-contract.md` §4.3/§7.6).

### 1.1 Orchestration and the graph-validation stage (Directive 010-R3 §4)

Graph validation belongs to `xl-hir` but is a **separate, caller-invoked
entry point**, not a step inside `lower`. The caller (the CLI, or the
conformance harness) owns the **mutable** registry and sequences:

1. **Register first.** Register all known modules and their available
   interfaces (`register`, and any pre-known edges) before resolving
   imports, so `resolve`/`surface` can answer during lowering.
2. **Lower against an immutable snapshot.** Call
   `lower(&[Item], FileId, &registry)` — shared reference; the registry is
   observed immutably.
3. **Collect edges.** Read `LowerOutput.import_edges`.
4. **Register the new edges.** Call `registry.record_import(edge)` for each
   returned edge (mutably owned by the caller) — after lowering, never
   during it.
5. **Validate the graph.** Only now call the validator over the registry:
   `xl_hir::validate_module_graph(&registry) -> Vec<Diagnostic>` (see
   §1.2). Running it earlier would miss the just-discovered edges.
6. **Propagate diagnostics** deterministically: import-resolution
   diagnostics from lowering, then graph (cycle) diagnostics, in the
   documented order (`diagnostics-contract.md` §3).
7. **Continue to `check`** per the poisoned-node and `StageFate`
   contracts.
8. **Never report success if required import or graph validation failed:**
   any missing/unavailable import or detected cycle keeps `xc check` from
   exit `0` (`P2D-A6`).

For a **synthetic multi-module compilation**, the harness registers every
module and `record_import`s every module’s edges **before** the final
`validate_module_graph` call. For the **single-file CLI**, the same
ordering applies (one module, its edges registered, then validation —
vacuously acyclic). The registry is never mutated through an immutable
reference; there is no hidden global state, interior mutability, or
filesystem discovery.

### 1.2 Graph validator contract and stage ownership

```rust
// Indicative signature; a member of xl-hir, invoked by the caller.
// Reads the complete graph from the registry's enumeration methods only.
pub fn validate_module_graph(registry: &ModuleRegistry) -> Vec<Diagnostic>;
```

- **Ownership:** the validator lives in `xl-hir` (module semantics are
  `xl-hir`’s responsibility), but the **caller decides when** to invoke it
  — after all edges are registered.
- **Input:** `&ModuleRegistry` alone; nodes = `registry.modules()`, edges =
  `registry.imports_of(m)` (§2.3). No filesystem, no package loading, no
  globals.
- **Output:** cycle diagnostics (`diagnostics-contract.md` §5.6), with the
  deterministic witness path, primary/secondary spans, and duplicate-cycle
  suppression of `name-resolution-contract.md` §9.4.
- **Fail closed:** a graph that exceeds `MAX_MODULE_GRAPH_*` yields a
  fail-closed diagnostic and never a silent pass.

This keeps lowering pure and registry-immutable while still satisfying the
original’s cyclic-import requirement (SPECIFIED §6).

## 2. Identity and span rules for lowered nodes

- Every HIR node produced from exactly one AST node inherits its span
  verbatim and records an `AstOrigin` (`hir-contract.md` §4). Nodes lowered
  from an `Item` or `Expr` record the corresponding `ItemId`/`ExprId`;
  statements, patterns, types, and other nodes **without** an AST id record
  an `AstOrigin::Structural { anchor, path }` key. Spans are never used as
  identity keys (grouping is transparent; recovered placeholders are
  zero-width and can share a span).
- **Synthesized nodes** (desugaring temporaries, desugared control flow,
  formatting wrappers) carry the span of the originating construct and an
  `Origin::Desugared { kind }` flag plus an
  `AstOrigin::Synthesized { construct, expansion, occurrence }` record
  (`P2D-A2`; `hir-contract.md` §4). **Identity vs. reference:** a desugaring
  is inherently **one-to-many**, so many synthesized nodes legitimately
  share one originating `construct`; uniqueness comes from the
  `(expansion, occurrence)` pair, not from the construct. `occurrence` is
  the 0-based index of the node in the fixed expansion traversal
  (§4–§6); `expansion` distinguishes a nested desugaring under the same
  construct (e.g. a `?` inside a `for` body), incrementing in expansion
  order. Example: every node of the `for` expansion carries the `for`
  expression's span, so a type error inside the desugared loop still
  points at the whole `for` statement (and diagnostics inside `BODY` —
  which has real spans — point at user text, not the expansion).
- Internal temporaries (`__state`, `__item`, `__next`, `__v`, `__e`) are
  generated as **fresh local definitions** (a `DefId` with
  `DefKind::Local`, surfaced through the `LocalId` view; `hir-contract.md`
  §4), never as strings inserted into scopes. They are hygienic by
  construction: user code cannot name them, shadow them, or observe them
  (`P2D-A2`; this is why desugaring is specified in HIR, which has
  identity-based bindings, rather than in the AST).
- Internal operations (`RangeNew`, `RangeNext`, `StrConcat`,
  `FormatPrim` — `BuiltinCall` nodes) are **compiler-owned built-ins**,
  not resolved through user scopes. The desugared `?` lowering instead
  references the prelude `Result` constructors: its `Ok`/`Err` patterns
  and the `Err` callee are synthesized uses **pre-resolved** to the
  prelude definitions (`hir-contract.md` §5.4). A user item named `Err`
  or `Ok` (shadowing the prelude) cannot capture the desugared `?`
  lowering in either case, because nothing in the expansion is resolved
  by name (`P2D-B9`).

### 2.3 Module registry and import-graph construction

The registry is supplied by the caller (the CLI owns it; a test supplies a
synthetic one) — `P2D-A5`. It is an **explicit, caller-owned, in-memory
value** — never a mutable global, and never populated by filesystem
discovery. `lower` reads it (by shared reference); it never mutates it.
The **indicative** interface:

```rust
// Indicative interface — caller-owned, in-memory. No global state.
pub struct ModuleRegistry { /* ... */ }

impl ModuleRegistry {
    // ---- construction (explicit; the caller registers the graph input) ----
    /// Register a module with a stable identity and canonical dotted path.
    /// Returns a ModuleId in registration order (deterministic).
    pub fn register(&mut self, path: ModulePath, file: Option<FileId>) -> ModuleId;

    /// Record one import edge discovered for `from` (declaration order).
    pub fn record_import(&mut self, edge: ImportEdge);

    // ---- enumeration (deterministic; the detector's graph input) ----
    /// All registered modules in **registration order** (stable, total).
    pub fn modules(&self) -> &[ModuleId];
    /// The canonical dotted path of a module.
    pub fn path_of(&self, module: ModuleId) -> &ModulePath;
    /// The recorded import edges of a module, in **source (declaration) order**.
    pub fn imports_of(&self, module: ModuleId) -> &[ImportEdge];

    // ---- lookup (used by lowering and by the detector) ----
    /// Resolve a dotted module path to a known module id, or report it absent.
    pub fn resolve(&self, path: &[&str]) -> ModuleLookup;
    /// The public-item surface of a known module, when this compilation has it.
    pub fn surface(&self, module: ModuleId) -> ModuleSurface;   // Available(..) | Unavailable
    /// The package that owns a module (orphan-rule domain; P2D-B14).
    pub fn package_of(&self, module: ModuleId) -> PackageId;
}

/// One recorded import edge (the graph's edge type). It is produced by
/// lowering (real modules) or inserted directly (synthetic test modules).
pub struct ImportEdge {
    pub from: ModuleId,        // the importing module (stable identity)
    pub path: ModulePath,      // the dotted path as written
    pub span: Span,            // the import declaration's source span
    pub target: ImportTarget,  // resolution outcome (below)
}

pub enum ImportTarget {
    Known(ModuleId),           // resolves to a registered module (an edge in the graph)
    Unavailable(ModuleId),     // known module, interface not available in this compilation
    Missing,                   // no such module registered
}

pub enum ModuleLookup { Known(ModuleId), Missing }
pub enum ModuleSurface { Available(ModuleInterface), Unavailable }

// Indicative aliases (exact layout is an implementation detail):
//   ModulePath       = Vec<String>  // canonical dotted path segments
//   ModuleInterface  = the public-item surface of an Available module
// `ModuleBinding.path` in `hir-contract.md` §5.1 is the same `Vec<String>`.
```

**How the cycle detector obtains the complete graph.** The detector takes
**only** `&ModuleRegistry` and reads the graph from its enumeration
methods — no other input, no filesystem, no global:

1. **Modules:** `registry.modules()` returns every registered module in
   registration order (deterministic, stable).
2. **Edges:** for each module `m`, `registry.imports_of(m)` returns its
   recorded `ImportEdge`s in source-declaration order (deterministic).
3. **Participation:** only `ImportTarget::Known(_)` edges are graph edges;
   `Missing`/`Unavailable` edges are **failed edges** carrying their spans
   (reported by `name-resolution-contract.md` §9.3) and are **excluded**
   from cycle traversal, so no phantom cycle is ever reported through a
   module that is not part of the compilation.
4. **Duplicate edges:** repeated `import` of the same target from one
   module are deduplicated for traversal, but each occurrence keeps its
   own `span`.

**Who populates the registry.**

- **Synthetic conformance modules (WP-B tests):** the harness calls
  `register` for each module and `record_import` for each edge directly.
  This makes `mg-1`…`mg-9` (`conformance-plan.md` §7.3) executable with
  the specified interface alone, without lowering any source.
- **A real single-file check:** the CLI `register`s the checked module,
  `lower` discovers its imports and returns them in `LowerOutput`, and the
  caller `record_import`s them (or the module is registered with them up
  front). With one module the graph has no cycle, but the same detector
  runs unchanged.

This yields a finite, deterministic, in-memory graph that Phase 2 can
actually build and traverse — satisfying the original’s cyclic-import
requirement (SPECIFIED §6) without package loading, `xl.toml` parsing,
filesystem discovery, or network access.

## 3. The complete lowering table

### 3.1 Items

| AST | HIR result | Notes |
| --- | --- | --- |
| `ItemKind::Import(ImportDecl)` | import record + module binding `DefId` (kind `ModuleBinding`) + import edge | path segments as uses; alias or last segment binds; target resolved against the registry; fail-closed on `Missing`/`Unavailable` (`P2D-B6`, `P2D-B7`, §2.3) |
| `ItemKind::Fn(FnDecl)` | `Def { kind: Fn, generics, sig, body: Some }` | `pub` recorded |
| `ItemKind::Struct` | `Def { kind: Struct, generics }` + field `Def`s | field `pub` recorded; no constructor syntax exists (G-5 UNRESOLVED) |
| `ItemKind::Enum` | `Def { kind: Enum, generics }` + variant `Def`s | payload `TypeRef`s; variant order preserved (exhaustiveness rendering order) |
| `ItemKind::Trait` | `Def { kind: Trait, generics }` + trait-item `Def`s | signature-only items (accepted grammar) |
| `ItemKind::Impl` | `Def { kind: Impl, generics, trait_ref: Option, self_ty }` + method `Def`s | `is_inherent = trait_.is_none()` from syntax; method selection is WP-C |

### 3.2 Statements, blocks, patterns

| AST | HIR result | Notes |
| --- | --- | --- |
| `StmtKind::Let(LetStmt)` | `StmtKind::Let { local(s), init }` | annotation → `Local.ty`; `mutable` → `Local.mutable` for **every** binding the pattern introduces (`P2D-B17`); init lowered with the pattern's uses recorded |
| `StmtKind::Expr(e)` | `StmtKind::Expr(hir)` | value discarded (convention unchanged) |
| `StmtKind::Item(item)` | `StmtKind::Item` | nested items: hoisted inside their block (`P2D-B3`) |
| `Block { stmts, tail }` | `BodyId` with `stmts` + `tail` | tail/value split preserved by construction (SPECIFIED) |
| `PatKind::Wildcard` | `PatKind::Wildcard` | binds nothing |
| `PatKind::Binding(ident)` | `PatKind::Binding { local }` **or** `PatKind::Variant { variant }` for a unit-variant name | final classification is semantic (`P2D-B5`); lowering records a *binding candidate* use and resolution decides (`name-resolution-contract.md` §7.4) |
| `PatKind::Lit(lit)` | `PatKind::Lit` | literal payloads copied with magnitude/radix |
| `PatKind::Path { path, args }` | `PatKind::Variant { variant: use, args }` | always a constructor candidate (path with `::` and/or args) |
| `PatKind::Tuple(pats)` | `PatKind::Tuple` | unit pattern `()` = empty tuple |

### 3.3 Expressions (accepted syntax → HIR)

| AST `ExprKind` | HIR result | Notes |
| --- | --- | --- |
| `Lit(Int/Float/Bool/Char)` | `Lit` | magnitude/radix preserved (B-2); floats finite (B-3) |
| `Lit(Str)` all-literal | `Lit(Str)` | decoded text |
| `Lit(Str)` with interpolation | `BuiltinCall { op: StrConcat / FormatPrim, … }` expansion | §6 |
| `Path` | `Path(UseId)` | one use per segment; `Enum::Variant` resolved in WP-B |
| `Unary`, `Binary` | same kinds | grouping already transparent in the AST |
| `Call(callee, args)` | `Call(callee_hir, args_hir)` | arity/type checking in WP-C; built-in calls have no source syntax (see §2 hygiene rule) |
| `Field(base, name)` | `Field(base_hir, FieldSel::Unresolved { name })` | module member vs field vs method decided in WP-C (`P2D-A3`) |
| `Index(base, index)` | `Index` | typing restricted per `P2D-B18` |
| `Assign(op, lhs, rhs)` | `Assign` | target shape was validated at parse time (accepted B-1); legality (mutability) is WP-C |
| `Range(a, b)` | `BuiltinCall { op: RangeNew, args: [a, b] }` | internal built-in, `P2D-B8`; `Range` is not nameable |
| `Cast(e, ty)` | `Cast(e, TypeRef)` | conversions per `P2D-B12` |
| `Try(e)` | desugared match | §5 |
| `If(c, b, e)` | `If(c, BodyId, Option<HirId>)` | else-if nests naturally |
| `Match(s, arms)` | `Match` | arms in source order (exhaustiveness order) |
| `Block(b)` | `Block(BodyId)` | |
| `While(c, b)` | `While(c, BodyId)` | |
| `For(p, s, b)` | desugared loop | §4 |
| `Loop(b)` | `Loop(BodyId)` | |
| `Return(e)` | `Return(Option<HirId>)` | |
| `Break` / `Continue` | `Break/Continue { target }` | `target` resolved structurally to the nearest enclosing lowering-time loop (never across a body boundary, because no closures exist) |
| `Tuple` / `Array` | `Tuple` / `Array` | unit = empty tuple; empty array allowed syntactically (typing per `P2D-B18`) |
| recovered placeholder | `ExprKind::Error` | never diagnosable again (poison; `hir-contract.md` §7) |

### 3.4 Types

| AST `TypeKind` | HIR `TypeRefKind` | Notes |
| --- | --- | --- |
| `Path(PathType)` | `Path { segments: uses, generics }` | each segment a use; generics lowered recursively |
| `Tuple` | `Tuple` | `()` = empty tuple |
| `Slice` | `Slice` | `[T]` nameable; no Phase 2 expression has slice type (`P2D-B18`) |
| `Array(elem, len_expr)` | `Array { elem, len }` | `len` is a lowered expression, checked per `P2D-B18` |
| `Fn { params, ret }` | `Fn` | optional return (unit when absent) |
| name `Self` in trait/impl context | `SelfTy` | contextual, not a keyword (`P2D-B14`) |

## 4. Desugaring: `for` loops (P2D-B8)

Source (SPECIFIED surface, §3 example):

```xlang
for item in sequence {
    process(item);
}
```

### 4.1 Internal iteration protocol (Phase 2 scope)

The original mandates “`for` → `loop` + iterator” (SPECIFIED §6 Phase 2)
but the language defines no Iterator trait, no associated types, no
`mut self` receivers, and no standard library yet. Phase 2 therefore
defines a **compiler-owned, internal protocol**, not a user-visible
trait:

- Internal type `Range<T>` (not nameable in source; produced only by the
  `..` operator lowering). `T` is an integer primitive (`i8`…`i64`,
  `u8`…`u64`, `isize`, `usize`).
- `range_new(a: T, b: T) -> Range<T>` — built-in; evaluates `a` then `b`,
  each exactly once.
- `range_next(r: Range<T>) -> Option<(T, Range<T>)>` — built-in, internal
  semantics: `None` when `r.start >= r.end` (empty or exhausted);
  otherwise `Some((r.start, Range { start: start + 1, end }))`.
  **No overflow is possible:** this branch requires `start < end`, so
  `start + 1 <= end <= T::MAX` for any integer `T` (DERIVED invariant).

The protocol is *internal*: `range_new`/`range_next` cannot be named or
called by user code and are not subject to shadowing (§2). When Phase 5/6
introduce traits and the standard library, this internal protocol is the
seam to be replaced by (or exposed as) the real iteration traits under a
future owner decision (Category C).

**Phase 2 iterable set:** exactly `Range<T>` from `a..b` expressions.
Iterating arrays, structs, or user types is **FUTURE** (owner decision
`P2D-B8`; the alternative — adding arrays now — is recorded in the
packet).

### 4.2 Lowering

```xlang
for pat in seq { BODY }
```

lowers to the following structure (all synthesized nodes carry the `for`
expression's span unless they wrap user code):

```text
{
    let mut __state = ⟨seq⟩;                 // type Range<T>; seq evaluated exactly once
    loop {
        match range_next(__state) {
            Some(__pair) => {                // prelude Option::Some (internal reference)
                let (__item, __next) = __pair;
                let pat = __item;            // user pattern binds here (fresh scope per iteration)
                __state = __next;            // state advanced BEFORE BODY
                ⟨BODY⟩                       // body statements; tail value discarded
            }
            None => break,
        }
    }
}
```

Contract points:

1. **Evaluation order.** `seq` is evaluated exactly once, before the
   first `range_next` call. Each iteration evaluates `range_next(__state)`
   once. `pat` is bound before `BODY` runs. Nothing else is evaluated.
2. **Temporary bindings.** `__state` is a fresh mutable internal local
   (identity-based; §2). `__pair`, `__item`, `__next` are fresh internal
   locals; `__next` is assigned back into `__state` before `BODY`.
3. **Scope.** `pat`'s bindings are visible exactly inside `BODY` and are
   fresh for each iteration concept (no leakage after the loop, no
   cross-iteration state other than `__state`, which user code cannot
   name). The loop's expanded block introduces one scope; `BODY`'s own
   block scope nests inside it.
4. **`break`/`continue`/`return`.** Because the state update precedes
   `BODY`, `continue` is correct: the next iteration observes the advanced
   state. `break` exits the desugared `loop` (target recorded at lowering,
   §3.3). `return` exits the function. No value semantics are invented:
   `for` is an expression with type `()` (`type-system-contract.md` §10).
5. **Error behavior.** If `seq` is an error node, the whole `for` lowers
   to `Error` (poison). If `pat` is an error node, it lowers to
   `Wildcard` with poison marking; no additional diagnostics. If `range_next`
   cannot be typed (non-`Range` operand), exactly one diagnostic is
   emitted at the `for`/operand span by WP-C; the loop still type-checks
   to prevent cascades.
6. **Refutable patterns.** The `let pat = __item;` pattern must be
   irrefutable for the item type; a refutable pattern is an error
   (`P2D-B22`, using the exhaustiveness machinery of
   `traits-and-exhaustiveness.md`). Example: `for Some(x) in 0..10` is
   rejected because the item type `i64` cannot match `Some(x)`.
7. **Diagnostics inside `BODY`** point at real user spans; diagnostics
   about the iteration itself point at the `for` construct or `seq`.

## 5. Desugaring: error propagation `?` (P2D-B9)

Source (SPECIFIED surface, §3 example):

```xlang
let value = operation()?;
```

### 5.1 Lowering

```text
⟨expr⟩?   ⟹   match ⟨expr⟩ {
                  Ok(__v) => __v,
                  Err(__e) => return Err(__e),
              }
```

- `Ok`/`Err` here are **internal references** to the prelude `Result`
  constructors, not name lookups: shadowing `Ok`/`Err` in user code
  cannot capture the expansion (§2 hygiene).
- `⟨expr⟩` is evaluated exactly once. `__v`/`__e` are fresh internal
  locals.
- The `?` expression's type is the payload type of `Ok`.

### 5.2 Typing rules (checked in WP-C)

1. The operand type must unify with `Result<T, E>` for fresh `T`, `E`;
   otherwise: `the '?' operator requires a 'Result<T, E>' operand, found
   'X'` (a `?` on `Option` is **not** introduced — UNRESOLVED/FUTURE).
2. The `?` expression has type `T`.
3. The enclosing function's declared return type must unify with
   `Result<T1, E1>` with **`E ≡ E1` exactly** — no implicit conversions,
   no `From`-style conversion, no exception semantics (SPECIFIED §2:
   errors are values; §7: no implicit conversions). If the enclosing
   return type is not a `Result`, or `E` does not unify with `E1`, the
   diagnostic is emitted at the `?` construct with the enclosing
   function's return type as a secondary label.
4. `?` is never allowed outside a function body (the grammar admits
   `postfix ?` only in expression positions; module-level expressions do
   not exist, so this is unreachable — documented for completeness).
5. Error conversion (e.g. an explicit `map_err`-style mechanism) is
   **FUTURE**, contingent on the standard library and trait machinery
   (Category C). Phase 2 requires exact error-type equality.

### 5.3 Error behavior

- Error operand → whole `?` is `Error` (poison), no diagnostics.
- Mismatched error type → one diagnostic; the `?` expression still
  type-checks as `T` to prevent cascades.

## 6. Desugaring: string interpolation (P2D-B10)

Source (SPECIFIED surface, §3 example):

```xlang
let message = "result = {value}";
```

### 6.1 Semantic representation and lowering

The accepted AST carries `StringLit { parts: StringPart::Text |
StringPart::Expr }` (ratified PD-004). Lowering:

1. Consecutive `Text` parts (and the text around interpolations) are
   merged into single `Lit(Str)` nodes (a `Lit(Str)` with the *decoded*
   value; the accepted lexer already decoded escapes).
2. Each interpolated expression part lowers to
   `BuiltinCall { op: FormatPrim, args: [part] }` **unless** the whole
   string is all-literal text with no interpolations (then the string
   stays a single `Lit(Str)`).
3. All parts are combined with the internal n-ary `str_concat` built-in:
   `BuiltinCall { op: StrConcat, args: [part_0, …, part_n] }`; a string
   with zero parts lowers to `Lit(Str(""))`.
4. Evaluation order: parts are evaluated **left to right**, exactly once,
   in source order. (DERIVED from ordinary expression order and from the
   left-to-right structure of the accepted lexer segments.)

### 6.2 Typing rules (WP-C)

- `StrConcat`: every part must be `str`; result `str`. (Concatenation is
  internal: the `+` operator is **not** defined for `str`; no new operator
  semantics are introduced.)
- `FormatPrim` (one interpolation part):
  - `part: str` → type `str`; identity formatting (no runtime call needed
    after optimization; the HIR node is a marker).
  - `part` of one of the **primitive formatting set** → type `str`. The
    text produced for a value is the proposal below.
  - any other type → error: `cannot format value of type 'X' in a string
    literal`, with a note that formatting for user types arrives with the
    standard library (FUTURE Display machinery, Category C).

**Primitive formatting contract (PROPOSAL, `P2D-B10` — the original is
silent):** for the primitive types `i8`…`i64`, `u8`…`u64`, `isize`,
`usize`, `f32`, `f64`, `bool`, `char`, `str`:

| Type | Textual form |
| --- | --- |
| integer types | decimal digits with a leading `-` for negative values (value-based; literal radix is irrelevant) |
| `bool` | `true` / `false` |
| `char` | the character itself (no quotes, no escapes) |
| `str` | the string’s text unchanged |
| `f32`/`f64` | the shortest decimal representation that round-trips to the same value and is accepted by the Phase 1 float grammar (the same rule the accepted canonical printer uses for float literals, B-3); finite values only |

Rationale: the §3 example interpolates an `f64` (`"total = {total}"`)
and an `i64`-like accumulator, so the intent to format primitives is
evidenced; restricting the set to primitives avoids inventing `Display`
semantics for user types, and reusing the accepted printer’s float rule
avoids a second, conflicting numeric-formatting definition. The owner may
instead choose the conservative alternative (str-only interpolation) or a
different primitive contract — see `P2D-B10` in the decision packet.

### 6.3 Guaranteed Phase 2 behavior vs. future dependencies

- **Guaranteed (Phase 2):** lowering shape, evaluation order, typing of
  `str` parts and the primitive set, diagnostics for non-formatable
  types. Phase 2 checks; it does not run anything.
- **Future (dependencies):** runtime formatting and concatenation
  implementations (Phase 4 VM); `Display`/`Debug` traits and formatting
  of user-defined types (Phase 6); any user-visible `Iterator`/
  `IntoIterator` surface for `for` (Phase 5/6). None of these are assumed
  by the Phase 2 contract.

## 7. Desugaring hygiene and non-goals

- No other desugarings exist in Phase 2. Specifically: `while`/`loop`
  are **not** desugared (the original mandates only `for`), `if` is not
  desugared, operators are not desugared into trait calls (no operator
  traits exist; `P2D-B15`), `return`/`break`/`continue` remain nodes.
- No implicit conversions, temporaries with user-visible names, or
  exception/panic semantics are introduced anywhere in this contract
  (SPECIFIED §2, §7).
- The desugared forms are internal representation; the canonical printer
  and `xc parse` (Phase 1, accepted) are unaffected, and no new source
  syntax is introduced by any desugaring.

## 8. Test obligations (WP-A, for the future implementation)

1. **Golden lowering snapshots:** HIR dumps (deterministic debug format)
   for the accepted conformance corpus, including the §3 example.
2. **Desugaring tests:** for each of the three desugarings, a table of
   source fragments with expected HIR shapes (structure only), covering:
   `for` evaluation order (side-effecting iterator expression? — Phase 2
   has no side-effecting iterables beyond the expression itself; test
   single evaluation via a call expression operand structure), nested
   `for` (target resolution of `break`/`continue`), `?` in nested blocks
   and inside `for` bodies, interpolation with multiple and adjacent
   parts, empty strings, strings that are only interpolation (`"{x}"`).
3. **Hygiene tests:** user code declaring items/locals named `Ok`, `Err`,
   `Some`, `None`, `range_next`, `str_concat` cannot capture or break
   desugared expansions.
4. **Poison tests:** error nodes anywhere in the input produce complete
   HIR with `Error` nodes and no additional lowering diagnostics.
5. **Determinism tests:** lowering twice yields byte-identical HIR dumps.
6. **Origin-completeness tests:** every HIR node from an AST node has an
   `AstOrigin`; `Item`/`Expr` nodes record their AST id; statements,
   patterns, and types record `Structural` keys; `AstOrigin` may be shared
   across the many nodes of one expansion, but **no two distinct HIR nodes
   share an occurrence key** (`HirId` unique; `(AstOrigin, OccurrenceKey)`
   injective) (`hir-contract.md` §4/§11).
7. **Synthesized-origin uniqueness tests (deterministic, one per
   desugaring):** lowering a construct and asserting the
   `AstOrigin::Synthesized { construct, expansion, occurrence }` records:
   - **`for`** — a nested `for` and a `for` whose body contains a `?`:
     the inner expansion has a distinct `expansion`/`occurrence`; every
     generated node’s `occurrence` is unique within its expansion;
   - **`?`** — `a?` and `b?` in one body produce distinct constructs, and a
     `?` inside a desugared `for` body gets `expansion > 0`;
   - **interpolation** — `"{a}{b}{a}"` and a string with adjacent/empty
     parts: each `FormatPrim`/`StrConcat` node has a distinct
     `occurrence`, and identical interpolated text is not collapsed into
     one origin.
   Positive assertion: the occurrence-key multiset has no duplicates.
   Negative assertion: mutating two synthesized nodes to share an
   occurrence key fails the validator.
8. **Module-graph tests (executable in Phase 2, synthetic registry):**
   the nine module-graph cases enumerated in `conformance-plan.md` §7.3
   (two-module cycle, three-module cycle, acyclic graph, self-import,
   shared dependencies without cycles, duplicate import edges, missing
   imported modules, deterministic cycle diagnostics, graph-depth/size
   limits) run against a synthetic `ModuleRegistry` built with `register`
   and `record_import` alone.

## 9. Traceability

| Contract item | Source |
| --- | --- |
| `for` → `loop` + iterator; `?` → `match`; interpolation → concatenation | SPECIFIED §6 Phase 2 |
| No exceptions, no implicit conversions; errors as values | SPECIFIED §2, §7 |
| Result<T, E> model; `Ok`/`Err` surface | SPECIFIED §3 example, §6 Phase 5 |
| Range expression `0..10` and iteration in the example | SPECIFIED §3 example |
| String parts/decoding; no Phase 1 desugaring | ratified PD-004; lexer contract §5 |
| Grouping transparency; target validation at parse time | ratified B-1, PD-003 |
| Never panic; complete trees; placeholder discipline | SPECIFIED §4; accepted recovery contract |
| Printer float rule reused for formatting | ratified B-3; parser/AST contract §5(5) |
| Module registry, import edges, fail-closed imports, executable cycle detection | SPECIFIED §6 Phase 2; Directive 010-R1 §3–§4; `P2D-A5`, `P2D-B7` |
| Accurate AST→HIR origin; checked ids | Directive 010-R1 §5; `P2D-A2` |
| `lower(&[Item], FileId, &ModuleRegistry) -> LowerOutput`; `StageFate` | Directive 010-R1 §7; Directive 010-R3 §4; `P2D-A1` |
| Registry graph enumeration (`modules`/`imports_of`); complete detector input | Directive 010-R2 §3; `P2D-A5` |
| Synthesized occurrence identity (`expansion`, `occurrence`) | Directive 010-R2 §4; `P2D-A2` |
| `DefKind::ModuleBinding` import representation | Directive 010-R2 §5; `P2D-B7` |
| `LowerOutput.import_edges`; separate graph-validation stage; orchestration order | Directive 010-R3 §4; `P2D-A1`, `P2D-A5` |
