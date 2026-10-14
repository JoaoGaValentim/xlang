# XLANG Phase 2 — Name Resolution Contract

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §8 (complete name-resolution
  architecture).
- **Companion documents:** `hir-contract.md` (definitions and uses),
  `lowering-contract.md` (when resolution runs), `type-system-contract.md`
  (type-directed resolution complements this document),
  `traits-and-exhaustiveness.md` (method ambiguity),
  `diagnostics-contract.md` (message shapes), `decision-packet.md`
  (`P2D-A3`, `P2D-A4`, `P2D-A5`, `P2D-B1`…`P2D-B7`, `P2D-B20`).
- **No implementation is authorized.** All rules are PROPOSED unless
  labeled.

## 1. Scope and evidence

This contract defines how names written in source become references to
definitions. The accepted Phase 1 contracts deliberately fixed only
*syntax*; everything here is new semantics and therefore PROPOSED,
traceable to the original where possible:

| Original evidence | Used for |
| --- | --- |
| 1 `.x` file = 1 module; packages via `xl.toml` (§2) | module model |
| `import std.io;` and `io.println(…)` (§3 example) | import binding = last segment; module member access |
| `Shape::Circle`, `s.to_int()`, `self.x`, `Point`, `Result<i64, str>` (§3) | path kinds: enum variants, methods, fields, types |
| `pub` on functions, struct fields, methods (§3, ratified PD-006) | visibility recording |
| Phase 2 deliverables: “symbol table with scopes and modules; duplicate, undefined, and cyclic-import detection” (§6) | the required feature set |
| Phase 1 ratified PD-006: `import module_path ["as" IDENT];`; `::` for expression/type paths; resolution deferred to Phase 2 | import syntax (unchanged) and the delegation this document answers |

Where the original is silent (most namespace questions), the rule below is
labeled PROPOSED and appears in the decision packet.

## 2. Symbol identity (the anti-aliasing principle)

**Contract:** every *definition* gets a fresh `DefId`/`LocalId`; every
*use* gets a `UseId` and resolves to exactly one definition identity or to
an explicit `Unresolved` state. Text never acts as identity: two
definitions that happen to share a name (in different scopes, in different
items, in different modules) have distinct identities and can never be
confused by a later stage.

Consequences:

- Resolution tables map `UseId → Definition` (identity), never
  `String → String`.
- A shadowed binding remains a distinct `LocalId`; the outer one is not
  mutated, re-keyed, or forgotten (it may still be referenced by uses
  textually before the shadowing declaration).
- The same rule holds for modules (`P2D-B7`: a module binding resolves to
  a `ModuleId`, and a member use resolves to a `DefId` in that module —
  never to a name string).
- Type parameters are definitions too (`DefId { kind: GenericParam }`),
  so `fn f<T>(x: T)`’s `T` cannot alias a struct `T` declared in the
  module.

## 3. Namespace model (P2D-B1)

The original specifies no namespace rules. After examination of every
namespace-sensitive construct in the specification (§3–§7: `Point` as a
type, `area` as a value, `Shape::Circle` as a constructor path, `pub`
items, trait names in bounds), the original is **silent**, so this is an
explicit owner decision. The proposal:

### 3.1 Namespaces

| Namespace | Contents | Notes |
| --- | --- | --- |
| **Module item namespace** | one flat namespace containing: module-level items (`fn`, `struct`, `enum`, `trait` names), import bindings (module bindings), and `impl`-associated names via their items | a single namespace means an item name and an import binding cannot collide silently — duplicates are errors (§7.1) |
| **Enum variant space** | variants live *under their enum’s path* (`Shape::Circle`), not in the module namespace | evidenced by the qualified example form; prelude constructors are additionally bound unqualified (§4) |
| **Block/item-local value space** | per-scope: `let` bindings, pattern bindings, function parameters, the receiver `self` | nested items inside blocks bind in the module item namespace *restricted to that block* (§7.3) |
| **Generic parameter space** | per-item: type parameters are visible in the item’s signature and body type positions | a type parameter is not a value; using `T` in an expression is an unresolved-value error |

### 3.2 Rationale for a single item namespace

- A split type/value namespace (two namespaces at module level, Rust
  style) is *possible* but unevidenced; the original never shows the same
  name used as both type and value, and the project’s governance forbids
  inventing distinctions that the specification does not require for
  expressiveness.
- The single namespace gives the clearest duplicate diagnostics and the
  simplest deterministic structure. If the owner prefers a split
  namespace, that choice changes only §7.1’s conflict surfacing, not the
  architecture; it is recorded as the packet alternative.

### 3.3 Namespaces are consulted in a fixed order

For a name in a **value** position: locals (innermost scope outward) →
item namespace (module scope; imports included) → prelude value names
(`P2D-B4`). For a **type** position: generic parameters (innermost item
outward; an impl’s generics for methods) → module item namespace → prelude
type names. For a **path with `::`**: see §6.

## 4. Prelude (P2D-B4)

The original puts `Option`/`Result` in the prelude in Phase 5 (§6 Phase 5)
but Phase 2 must check `Result<T, E>` propagation (`?`) and the example
uses `Ok(n)`/`Err("negativo")` unqualified. The proposal:

- The Phase 2 prelude contains exactly: the primitive type names
  (`i8`…`i64`, `u8`…`u64`, `isize`, `usize`, `f32`, `f64`, `bool`, `char`,
  `str`), `Option<T>` (variants `Some(T)`, `None`), `Result<T, E>`
  (variants `Ok(T)`, `Err(E)`), and the four constructor names bound
  **unqualified** (`Some`, `None`, `Ok`, `Err`), matching the §3 example.
- Prelude bindings live in a lowest-precedence layer: any user item,
  import binding, local, or type parameter with the same name shadows the
  prelude for that scope without error (DERIVED from the example: user
  code uses `Err`/`Ok` while also being able to declare its own items).
- Prelude constructors are also reachable qualified
  (`Option::Some`, `Result::Ok`); user enums remain reachable only
  qualified (`Shape::Circle`) — the example uses unqualified names only
  for the prelude constructors.
- The internal `Range` type and iteration built-ins are **not** in the
  prelude: they are not nameable at all (`P2D-B8`).

This adds the minimum prelude surface needed for Phase 2 to be meaningful
(`?` without `Result` in scope would be unusable). Whether the rest of the
Phase 5 prelude content (`Ordering` etc.) is pulled forward is **not**
proposed (Category C).

## 5. Scopes and resolution order (P2D-B2, P2D-B3)

### 5.1 Scope chain

```text
module scope
  └── (per item) generics scope
        └── (per function/method body) parameter scope
              └── block scopes, nesting as written
                    └── (per for-loop body) the desugaring's scopes
```

- **Module scope:** module-level items are order-independent (two-pass
  collection; `lowering-contract.md` §1). A function may call one declared
  later.
- **Item scope:** generic parameters are visible in the whole item
  (signature and body type positions); each impl contributes its generics
  to its methods’ scopes.
- **Parameter scope:** parameters are visible throughout the body
  (sequential order does not matter except that a parameter cannot be
  redeclared by a later parameter — duplicates are errors, §7.1).
- **Block scope:** a `let` binding is visible from its declaration
  (exclusive) to the end of its block, including the block tail and all
  nested blocks; inner scopes may shadow outer ones (§7.2).
- **Loop body scopes:** the desugared `for` introduces internal
  temporaries as fresh identities and a scope for `pat`’s bindings visible
  only inside the body (`lowering-contract.md` §4.3).
- **Match arms:** each arm body is checked with the arm pattern’s bindings
  in a fresh scope nested in the match expression’s scope.

### 5.2 Sequential visibility of locals

Locals are resolved **sequentially**: a `let` name is not visible in its
own initializer (`let x = x;` resolves the right `x` in the enclosing
scope — usually undefined if none exists), nor before its statement.
This matches ordinary scoping intuition and avoids inventing
declaration-hoisting semantics for values (items are the exception,
§5.1).

### 5.3 The four name kinds at a glance

1. **Type names** (`x: T`, `-> T`, generic args, impl headers): resolved
   in type positions in the order of §3.3.
2. **Value names** (expressions, patterns): resolved in value positions.
3. **Path names** (`A::B::C`): see §6.
4. **Member names** (`.`-access): **type-directed**; `xl-hir` records the
   use and any syntactically known candidate (module bindings); everything
   else is completed by `xl-types` (`P2D-A3`, §8).

## 6. Paths and module access (P2D-B6)

### 6.1 The three path shapes

- **`a::b::c` (`Path` / `PathType`)** — segment lists. Resolution proceeds
  segment by segment: the first segment resolves in the current scope
  order (§3.3). If it resolves to a **type parameter** or **generic
  context** the path must end there (generic parameters have no members);
  if it resolves to an **enum**, the next segment must be one of its
  variants (exactly one more segment; deeper paths are an error: no
  enum-in-enum namespaces exist); if it resolves to a **module binding**
  (`P2D-B7`), the next segment resolves in that module’s *public* item
  namespace and further segments repeat the rule; otherwise the path is
  an item reference and must end.
- **`a.b` (`Field`)** — `.` after a *value* is a member access
  (field/method, type-directed, §8). `.` after a **module binding** is
  module member access (a public item of the imported module); this is the
  accepted `io.println` shape (frozen by Phase 1 correction F-3).
- **Import paths (`import a.b.c;`)** — dots only; the last segment (or
  the alias) becomes the module binding name (ratified PD-006; syntax
  unchanged).

### 6.2 Both `::` and `.` reach module members

`io::println` and `io.println` both denote the public item `println` of
module `io` when `io` is a module binding. The AST shapes differ
(`Path` vs `Field`), the resolution result is the same; this is required
for *type* positions, where only `::` exists (`import std.io;` then
`io::Reader` is the only way to name an imported type). Recorded as
`P2D-B6` with the alternatives (reject `::` module paths — leaves imported
types unnameable; reject `.` member access — contradicts the example).

### 6.3 What `::` cannot express (checked, with diagnostics)

- No nested enum variants (`Shape::Circle::Deep` → error at the third
  segment).
- No associated items on types (`Type::method`, `Type::CONST`) — no
  evidence, UNRESOLVED/FUTURE.
- No turbofish (`f::<T>(…)`) — no expression-level generic arguments
  (ratified PD-006 §3).

## 7. Duplicates, shadowing, unresolved, ambiguous (P2D-B2, P2D-B3)

### 7.1 Duplicate declarations (errors)

- Two module-level items with the same name in the module item namespace:
  `duplicate definition of 'f'` (primary at the second, secondary at the
  first).
- An import binding conflicting with a module-level item or another import
  binding: same diagnostic, with the import span.
- Two parameters with the same name, or a parameter redeclaring a generic
  parameter name in the same item: error.
- Two `let`s introducing the same name **in the same scope**: error
  (`the name 'x' is already defined in this scope`), even if the first is
  never used. Sequential re-`let` in the same block is *not* shadowing in
  this proposal (alternative — Rust-style re-binding shadowing — is in the
  packet).
- Enum variants, struct fields, and trait items with duplicate names
  inside their container: error.
- `self` is not redeclarable (receiver parameter plus a named parameter
  `self` is an error).

### 7.2 Shadowing (allowed, deterministic)

- An inner block’s `let` may shadow an outer binding; a pattern binding
  may shadow a parameter; an item declared in a block may shadow a
  module-level item within that block (`P2D-B3`); locals and parameters
  shadow module items for value positions (locals-first order §3.3).
- Shadowing never produces a warning in Phase 2 (no unused-variable
  analysis exists; inventing a warning is out of scope). Diagnostics that
  reference shadowed definitions always carry spans, so no confusion is
  possible.

### 7.3 Nested items (P2D-B3)

The accepted grammar allows `import`, `fn`, `struct`, `enum`, `impl`, and
`trait` inside blocks (SPECIFIED §3 EBNF via `stmt = … | item`; semantics
were deferred). Proposal:

- Nested items are **visible throughout their enclosing block** (hoisted
  within that block, like module items are hoisted within the module) and
  shadow outer items of the same name from the block’s start.
- Nested `fn` items do **not capture** locals: a function body resolves
  only its own parameters, generics, its own nested items, and items
  visible at its declaration site (there are no closures in XLANG v1 —
  SPECIFIED non-goals; and functions must remain monomorphizable in
  Phase 7). A body referencing an enclosing local is an ordinary
  unresolved-name error.
- Nested `import` items create block-scoped module bindings.
- Nested `impl`/`trait`/`struct`/`enum` are items like any other; their
  visibility and coherence follow the module rules (they are still part of
  the same single module — no new module is created).
- The alternative — treating all nested items as if declared at module
  level — is rejected in the packet discussion because it would make the
  block structure meaningless and surprise the reader.

### 7.4 Pattern names (P2D-B5)

- A lone identifier pattern is first tested as a **unit-variant
  constructor** (resolves in the type/path space to an enum variant with
  no payload — user enums via the enum’s scope, prelude via unqualified
  names). If it resolves, it is a constructor pattern; otherwise it
  **introduces a fresh binding**.
- A path pattern with `::` and/or arguments is always a constructor
  pattern; unresolved paths are errors with suggestions.
- Consequence (documented, deterministic): `match x { None => … }` matches
  the prelude `None`; to bind a value that happens to be named like a unit
  variant, no binding form exists in Phase 2 (there is no `@`-pattern — an
  UNRESOLVED/future surface). This limitation is recorded in `P2D-B5`
  together with the alternative (bare identifiers always bind; then
  `None` would silently bind and never match — rejected as a silent class
  of bugs).
- `_` binds nothing and may repeat freely.

### 7.5 Unresolved symbols (errors with suggestions)

- Value position: `cannot find value 'x' in this scope`; type position:
  `cannot find type 'T' in this scope`; trait position: `cannot find trait
  'Ord' in this scope`.
- **Suggestions:** when at least one candidate in the relevant scope/order
  chain is within edit distance ≤ 2 (per candidate name, scaled for short
  names: distance 1 for names ≤ 4 chars), emit `did you mean 'y'?` as a
  secondary label or note. Candidate ordering is deterministic: ascending
  edit distance, then declaration order. At most three candidates are
  listed. Suggestion mechanics are shared with member lookup
  (`type-system-contract.md` §12) and pinned by snapshot tests.
- Unresolved in a pattern: same message shapes, at the pattern span.

### 7.6 Ambiguous references

Structural ambiguity is designed out wherever possible (no glob imports,
no re-exports, single namespace). What remains:

- **Trait method calls** (`x.m()` with several applicable trait impls):
  resolved in WP-C; ambiguity is an error listing the candidates
  deterministically (`traits-and-exhaustiveness.md` §2.6) — never a
  silent pick.
- Any future glob-like surface would need a new decision (none exists).

## 8. Member and method lookup boundaries (P2D-A3)

Resolution is split explicitly into two stages:

1. **WP-B (syntactic):** module bindings and their members; enum variant
   paths; prelude names; everything above. `Field(base, name)` where
   `base` is a module binding is resolved here to `ModuleMember(DefId)`.
   All other `Field` uses stay `FieldSel::Unresolved` with the name use
   recorded.
2. **WP-C (type-directed):** for `Field(base, name)` with a value `base`:
   - compute `base`’s type;
   - if it is a struct: look up the field (fields are public/private per
     `P2D-B20`; private field from outside the module is an error
     surfaced when the registry exists — within one module, all fields are
     accessible but visibility is still recorded and checked);
   - otherwise, or when no field matches and the expression is a call
     callee `Call(Field(base, name), args)`: **method selection** —
     inherent methods first (impl blocks without a trait), then trait
     methods from impls whose trait is in scope, then prelude/builtin
     methods if any exist in Phase 2 (none are defined by this package:
     `s.to_int()` and `.sqrt()` from the §3 example are stdlib methods,
     FUTURE — their absence produces ordinary unresolved-member errors in
     Phase 2).
   - failure produces `no field 'z' on type 'Point'` / `no method 'm' on
     type 'Point'` with suggestions drawn from the type’s fields and
     methods.
- Recursion/limits: member selection is bounded (`P2D-A7`); trait method
  search respects the trait-solver budgets.

## 9. Imports, modules, and the package model (P2D-A5, P2D-B7)

### 9.1 Accepted import syntax (unchanged)

`import module_path ["as" IDENT];` with dotted paths, no groups, no
wildcards, no `::`, no `pub import` (ratified PD-006). This contract does
not change the syntax.

### 9.2 Module identity and the registry seam

- A module is identified by a `ModuleId`; in Phase 2 (single file), the
  compilation unit registers exactly one module: the checked file.
- `xl-hir` consumes a **module resolver** supplied by the caller
  (`P2D-A5`):

  ```rust
  // Indicative interface
  pub trait ModuleResolver {
      fn resolve_module(&self, path: &[&str]) -> ModuleLookup;
  }
  pub enum ModuleLookup {
      Known(ModuleId),      // module exists; its public items are available
      Unknown,              // no such module in this compilation
  }
  ```

  The CLI’s Phase 2 resolver answers `Unknown` for every import
  (single-file mode). Multi-file and package loading (file discovery,
  `xl.toml`, `xl.lock`) are **explicitly out of scope** (Phase 8,
  Category C) — but the interface is fixed now so the future package
  model plugs in without redesign.

### 9.3 Phase 2 policy for imports (P2D-B7)

**Opaque-module policy (proposed):**

- Every import binding is created and resolves to `ModuleId::Opaque(path)`.
- An imported binding that is **never used** produces no diagnostic
  (no unused-import analysis exists in Phase 2).
- Any **use through** an opaque module — `io.println`, `io::Reader` in
  type position — resolves to an `Unresolved` use with the specific
  diagnostic: `module 'std.io' is not part of this compilation; only
  single-file checking is available in this phase` (primary at the use,
  secondary at the import). This keeps files without imports fully
  checkable and never fabricates a file-discovery convention the
  specification does not define.
- Alternatives (packet): (a) hard error at every import site — harsher,
  rejects checkable parts of real files; (b) implicit sibling-file loading
  (`import util;` → `util.x`) — invented mapping, rejected (the
  specification routes module resolution through packages/`xl.toml`);
  (c) full package loading — explicitly out of scope.

### 9.4 Cyclic imports (algorithm fixed now; enforcement when a registry exists)

The Phase 2 deliverable requires cyclic-import detection (SPECIFIED §6).
With one file, no cycle is possible, so the algorithm is defined and the
*obligation* for the registry phase is recorded:

- Build the import graph over `Known` modules only. Deterministic DFS in
  source order of import declarations; states White/Grey/Black; an edge to
  a Grey module reports `cyclic module import` with the full cycle path
  (`a.b → c.d → a.b`) rendered from the DFS stack, primary span at the
  import that closed the cycle, secondary at the module’s declaration
  site.
- Cycles are reported once per cycle, at the closing edge; the modules in
  the cycle are still checked (no cascade; imports remain bindings).
- **FUTURE fixtures** (documentation-only in `conformance-plan.md`) cover
  two- and three-module cycles; they cannot run until the registry phase.

### 9.5 One file = one module and the future package model

- The mapping “module path ↔ source file” is a package-model concern
  (Phase 8: `xl.toml`); nothing in Phase 2 guesses it. When it arrives,
  the resolver answers `Known` for registered modules, visibility
  (`P2D-B20`) becomes enforceable across module boundaries with the same
  diagnostics, and the cycle algorithm above activates unchanged.
- A module’s own items are always visible inside it regardless of `pub`
  (standard visibility rule; SPECIFIED §6 Phase 5 lists modules and `pub`
  as language features to complete — Phase 2 records visibility and
  enforces the boundary as far as the registry allows).

## 10. Visibility (P2D-B20)

- `pub` placement is syntax (ratified): before `fn`/`struct`/`enum`/
  `trait`, on struct fields, and on methods inside `impl` blocks.
- Recorded per definition (`Def.visibility`). Enforcement rule: an item
  (or field) is accessible outside its module only when `pub`. In Phase 2
  single-module mode the boundary is vacuous for local items; the rule is
  enforced for opaque-module uses by construction (nothing is accessible
  through an opaque module) and is ready for the registry phase.
- Trait items: signature-only in the accepted grammar (no `pub` allowed
  syntactically); they are **implicitly public** — a trait’s methods are
  its contract (DERIVED from trait purpose; recorded in the packet).
- Impl methods: `pub`/private recorded; inherent methods reachable
  according to visibility; trait-impl methods are part of the trait’s
  contract and are treated as public for dispatch (visibility does not
  gate trait-method dispatch).
- Access errors: `item 'f' is private` (primary at the use, secondary at
  the definition), with the module boundary mentioned when the registry
  provides one.

## 11. Deterministic resolution order and diagnostic behavior

- Resolution never depends on hash-map iteration order: scopes are
  ordered vectors; within a scope, lookups are by name and results do not
  depend on insertion order except for duplicate detection (which reports
  by source order).
- Diagnostics are emitted in traversal order: items in source order;
  within an item, signature then body; within a body, lexical order.
  All diagnostics for one construct are emitted before moving on.
- Every diagnostic uses the accepted `xl-diag` model (primary span,
  secondary labels) and the message shapes of
  `diagnostics-contract.md`.

## 12. Performance and resource notes

- Symbol tables: per-scope `HashMap<String, DefId/LocalId>` plus ordered
  scope stacks; memory O(definitions). Owned `String` keys in Phase 2
  (`P2D-A4`); interning is a later optimization, not a semantic change.
- Lookups are O(scope depth) with hash lookups per scope; module-scope
  lookup is one hash lookup after the local chain misses.
- Path resolution is O(segments); module traversal per registry lookup is
  O(1) hash lookup in the future package phase.
- No unbounded recursion: path/type/pattern walks respect the HIR depth
  budgets (`P2D-A7`).

## 13. Traceability

| Contract item | Source |
| --- | --- |
| Symbol table with scopes and modules; duplicate/undefined/cyclic detection | SPECIFIED §6 Phase 2 |
| Import syntax, aliases, dotted paths, no groups/wildcards | ratified PD-006 (unchanged) |
| `io.println` member access shape; `::` expression/type paths | SPECIFIED §3 example; Phase 1 correction F-3 |
| 1 file = 1 module; packages via `xl.toml` (deferred) | SPECIFIED §2; Phase 8 (FUTURE) |
| `pub` placement; private-by-default semantics | ratified PD-006 (placement); SPECIFIED §6 Phase 5 (semantics deferred to this design) |
| `Shape::Circle`, `Point`, `self.x`, `s.to_int()` | SPECIFIED §3 example |
| `Ok`/`Err` unqualified | SPECIFIED §3 example (`parse`) |
| Suggestions (“você quis dizer foo?”) | SPECIFIED §6 Phase 2 acceptance |
| Never panic; deterministic output | SPECIFIED §4; INV-007 |
