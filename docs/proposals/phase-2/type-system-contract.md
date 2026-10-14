# XLANG Phase 2 — Type System Contract

**STATUS: OWNER-RATIFIED (Directive 010-R3) — PENDING INTEGRATION (PR #7 not merged)**

- **Directive:** Engineering Directive 010 §9 (static type system design),
  §16 (safety and performance analysis), amended by Engineering Directive
  010-R1 §5–§9 (identity, budgets, phase scope).
- **Authoritative source:** original specification §7 (SPECIFIED type
  rules), §3 (surface), §6 Phase 2 (deliverables and acceptance).
- **Companion documents:** `hir-contract.md` (input representation),
  `lowering-contract.md` (desugared forms typed here),
  `traits-and-exhaustiveness.md` (trait solving, method selection,
  exhaustiveness), `diagnostics-contract.md` (messages),
  `conformance-plan.md` (corpus), `decision-packet.md` (`P2D-B4`,
  `P2D-B9`, `P2D-B11`…`P2D-B19`, `P2D-A7`).
- **No implementation is authorized.** All rules are PROPOSED unless
  labeled SPECIFIED.

## 1. Scope and evidence

SPECIFIED type-system requirements (original §7):

- Inference: fresh type variables, unification with occurs check,
  **generalization only on `fn` (restricted let-polymorphism)**,
  bidirectional checking (infer and check modes) for better messages.
- Traits: declarations, `impl Trait for Type`, bounds; resolution by
  coherence (one implementation per trait/type pair; orphan rule);
  associated types in v1.1 (**excluded from Phase 2**).
- Generics: monomorphized in the backend (Phase 7); depth limit to avoid
  infinite expansion.
- Conversions: **no implicit coercion** between numerics; explicit `as`;
  integer literals default to `i64`, floats to `f64`, when ambiguous.
- Exhaustiveness and reachability of patterns; non-exhaustive `match` is
  an error.
- Mutability: `let` immutable by default; assignment requires `mut`.

Explicit boundaries (Directive 010 §9, honored throughout): no borrow
checker, no implicit numeric coercions, no exceptions, no concurrency, no
associated types, no global (unrestricted Hindley–Milner) inference.

## 2. Responsibility split

| Responsibility | Owner | Input → Output |
| --- | --- | --- |
| Type lowering (`TypeRef` → `TyId`) | `xl-types` | resolved HIR type refs → type arena nodes; diagnostics for malformed/unresolved types |
| Syntactic name resolution | `xl-hir` (`lower`, done before typing) | uses → definitions; returned in `LowerOutput.resolutions` |
| Type-directed resolution (fields/methods/members) | `xl-types` | `FieldSel::Unresolved` + types → `FieldSel` finals + diagnostics; returned in `CheckOutput.resolutions` |
| Bidirectional checking & inference | `xl-types` | bodies → per-node types (side table `HirId → TyId`) |
| Unification / constraint solving | `xl-types` (unification is the constraint solver; no separate constraint language) | `TyId` pairs → substitutions; one diagnostic per failed unification |
| Trait solving (bound satisfaction, method selection, coherence) | `xl-types` | obligations → evidence or ambiguity/not-found diagnostics |
| Literal defaulting, representability | `xl-types` | literal types/variables → final primitive types or diagnostics |
| Mutability checking | `xl-types` | assignment/`let` sites + resolution → diagnostics |
| Pattern checking; exhaustiveness; reachability | `xl-types` | patterns + scrutinee types → coverage results, diagnostics |
| Diagnostic emission | each stage owns its messages (shapes in `diagnostics-contract.md`) | — |

There is no separate constraint language: inference is unification over
union-find type variables, executed eagerly (bidirectional walk), which
matches the SPECIFIED “unification with occurs check” and keeps
deterministic, explainable errors.

## 3. Type representation

```rust
// Indicative sketch.
pub struct TyId(u32);        // arena index into the TypeStore
pub struct VarId(u32);       // inference variable

pub enum TyKind {
    Error,                   // poison: unifies with everything, silent
    Never,                   // internal bottom: `return`/`break`/`continue` (P2D-B16)
    Prim(PrimTy),            // i8..i64, u8..u64, isize, usize, f32, f64, bool, char, str
    Unit,                    // `()` (distinct from an empty tuple? — see note)
    Tuple(Vec<TyId>),
    Fn { params: Vec<TyId>, ret: TyId },
    Slice(TyId),             // `[T]`  (nameable; no Phase 2 expression produces it — P2D-B18)
    Array { elem: TyId, len: u64 },   // `[T; N]`, N resolved per P2D-B18
    Nominal { def: GlobalDefId, args: Vec<TyId> },  // struct/enum/trait application (P2D-A2)
    Var(VarId),              // unresolved inference variable
    IntVar(VarId),           // integer-literal variable (defaults to i64)
    FloatVar(VarId),         // float-literal variable (defaults to f64)
    Range(TyId),             // internal: type of `a..b`; not nameable (P2D-B8)
}
```

`Nominal` names a **nominal definition** by its cross-module identity
`GlobalDefId { module, definition }` (`hir-contract.md` §3), not a bare
`DefId`, so two structs/enums with the same instance-scoped `DefId` in
different modules are distinct types (Directive 010-R1 §6). Unification of
`Nominal` requires equal `GlobalDefId` (and equal argument arity/values).
Phase 2 registers one module, so all `GlobalDefId`s share one `ModuleId`,
but the identity space is correct now.

Notes:

- **Unit** is represented once; `()` in type position lowers to the same
  single type as the empty tuple *value*’s type. (The accepted AST
  distinguishes `Tuple([])` by construction; the type system may treat
  `Tuple([]) ≙ Unit` — an implementation detail, but the equivalence is
  contract: `fn f() -> ()` and a unit-typed tail unify.)
- `Nominal` covers structs, enums, and abstract trait types (traits are
  used as bounds, not as types in Phase 2 — `dyn`/`impl Trait` do not
  exist; UNRESOLVED/FUTURE).
- `Never` is internal and unnameable; it exists only to type divergent
  expressions (`return`/`break`/`continue`), and it unifies with every
  type (the usual bottom rule). Nameability would require new syntax and
  is not proposed.
- Type variables are **always** created by the checker; none come from
  source (there is no `_` type syntax — `Underscore` is not a type token).

### 3.1 Invariants

1. Every `TyId` in the arena is well-formed: nominal arguments match the
   definition’s declared generic arity (traits included); `Array.len` is
   materialized (checked during lowering).
2. After `check`, no `Var`/`IntVar`/`FloatVar` remains reachable from any
   *successfully typed* expression except where `Error`/silence made it
   irrelevant (see §8.3 for the defaulting rules and the “cannot infer”
   diagnostics).
3. `Error` never causes a diagnostic; `Error` is not nameable.
4. Type arena size is O(HIR size × small constant): every source type is
   lowered once; inference creates fresh variables per instantiation.

## 4. Type lowering

`TypeRef` (HIR) → `TyId`:

- `Path` → resolved definitions: generic parameter → a rigid parameter
  type (see §7.2); struct/enum → `Nominal` with lowered generic arguments;
  prelude primitives → `Prim`; `Option`/`Result` → the prelude enum’s
  `Nominal` type.
- `Slice`/`Array`/`Tuple`/`Fn` → the corresponding kinds; `Array.len`
  follows `P2D-B18` (non-negative integer literal; value must fit `usize`;
  `u64`-range check with a clear diagnostic otherwise).
- `SelfTy` → the impl/trait self type (`P2D-B14`): in an impl, the impl’s
  self type with its generics; in a trait declaration, the trait’s
  implicit `Self` parameter.
- Unresolved path, wrong generic arity, or malformed ref → `Error` plus
  exactly one diagnostic (from resolution or lowering), never a cascade.
- Structural bounds: lowering a type ref deeper than `MAX_TYPE_DEPTH`
  (P2D-A7) yields `Error` with the resource diagnostic; accepted source
  types cannot exceed `MAX_PARSE_DEPTH = 64`, so this is a defensive path.

## 5. Bidirectional checking and inference

Two modes (SPECIFIED §7 “checagem bidirecional (modo infer e check)”):

- **`infer(expr) -> TyId`** — synthesize a type: literals (with literal
  variables), names (via resolution), calls, field access, operators,
  blocks with a typed tail, etc.
- **`check(expr, expected: TyId)`** — propagate expectations down:
  `let` annotations, function returns, call arguments, annotation on
  parameters, `if`/`match` arms against the expected type, array elements
  against an expected array, etc.

Rules (standard bidirectional architecture, stated for determinism):

1. When `check(e, T)` is called and the expected type is the *only* way
   to type `e` (e.g. empty array literal `[]`, `None`), `e` adopts `T`.
2. Otherwise `check(e, T)` = `infer(e)`, then unify the inferred type with
   `T`, reporting a mismatch at the best span if it fails.
3. `infer` of a `Block` uses the tail: `None` tail → `Unit`; tail `None`
   with error statements → `Error`.
4. `if` without `else` has type `Unit` and its condition is checked as
   `bool`; with `else`, the branch types unify (§11).
5. `match` arms unify with each other and with the expected type; the
   scrutinee is inferred (or checked against an expected pattern-derived
   type when the expected type disambiguates constructors — a refinement
   used only for better messages; correctness never depends on it).

Every expression gets exactly one type recorded in the side table
`expr_types: Vec<TyId>` indexed by `HirId` — this is the artifact MIR
lowering (Phase 3) will consume.

## 6. Unification and occurs check

- Union-find over `VarId`s with path compression; deterministic variable
  numbering in creation order.
- `unify(a, b)`:
  - `Error` with anything (either side) → success, no binding (poison).
  - `Never` with anything → success (bottom).
  - `Var(v)` with `T` → occurs check (below), then bind; `IntVar`/
    `FloatVar` bind only to the permitted primitive sets (or to another
    literal variable of the same kind — merged).
  - `IntVar`/`FloatVar` with concrete primitives: allowed only inside
    their sets (integer types / float types); otherwise mismatch.
  - `Prim`/`Unit`/`Slice`/`Array`/`Tuple`/`Fn`/`Nominal`/`Range`:
    structural unification (same kind, pairwise children; `Nominal`
    requires the same `DefId` and equal generic arity; `Array` requires
    equal lengths — `[i64; 2]` and `[i64; 3]` do not unify).
- **Occurs check:** before binding `Var(v) := T`, reject if `v` occurs in
  `T`. Failure is a diagnostic (`cyclic type`), and the binding is not
  made. Complexity: O(size of `T`) with a depth budget
  (`MAX_UNIFY_STEPS`, P2D-A7); malicious inputs cannot force unbounded
  work — source types are depth-bounded by Phase 1 limits, and every
  type node created by the checker is created from a bounded source
  construct or a fresh variable.
- **Failure diagnostics:** one diagnostic per failed unification, with
  the expected type, found type, and spans; after failure the expression
  is poisoned (`Error`) so a single mistake yields a single diagnostic.
- Determinism: unification order follows the deterministic traversal
  (left-to-right, source order); the same input always yields the same
  bindings and the same errors.

## 7. Generics and the polymorphism restriction

### 7.1 Where generalization happens (P2D-B13)

SPECIFIED §7: “generalização apenas em `fn` (let-polimorfismo restrito)”.
Interpretation adopted by this package:

- **Function/method items** are polymorphic over *their declared generic
  parameters*; each use instantiates the scheme with fresh `Var`s.
  Generics must be **declared**: no signature inference (no implicit
  generics, no inferred `∀`).
- **`let` bindings do not generalize.** `let id = area;` binds a
  *monomorphic* type; there is no let-polymorphism in Phase 2. (The
  alternative reading — generalize at `let` when the RHS is a function
  item — is recorded in `P2D-B13`; it buys little because closures do not
  exist and complicates the story.)
- Trait method calls resolve through implemented methods (§Traits doc);
  no higher-ranked types, no `impl Trait`, no `dyn`.

### 7.2 Rigid vs. fresh parameters

- Inside a generic item’s body, the item’s parameters are **rigid**
  (can only be related through declared bounds — see the operator
  restriction, `P2D-B15`); they are not unifiable with concrete types.
- At a use site, instantiation creates fresh variables for each parameter;
  bounds become trait obligations checked by the solver.
- Arity checking: a path with wrong generic arity is a lowering
  diagnostic (`type 'Result' expects 2 generic arguments, found 1`).

### 7.3 Depth limits

SPECIFIED §7: “limite de profundidade para evitar expansão infinita”.
Enforced at: type lowering (`MAX_TYPE_DEPTH`), unification
(`MAX_UNIFY_STEPS`), instantiation chains (a fresh-variable creation
budget per body, `MAX_INSTANTIATIONS`), and trait solving
(`MAX_TRAIT_DEPTH`, traits doc). All limits fail closed with a specific
diagnostic (`P2D-A7`), never with a panic and never silently.

## 8. Literals, defaulting, and representability (P2D-B11)

### 8.1 Integer literals

- An `Int` literal has type `IntVar` constrained to the integer primitive
  set `{i8…i64, u8…u64, isize, usize}` (SPECIFIED: literals default to
  `i64` when ambiguous; the set is DERIVED from the SPECIFIED primitive
  list — the checker must be able to give `let x: u8 = 200;` the type
  `u8`).
- **Representability (B-2 closure):** when an `IntVar` resolves (or is
  checked against) a concrete integer type `T`, the magnitude must fit
  `T`:
  - unsigned `T`: `0 ≤ magnitude ≤ T::MAX`;
  - signed `T`: `0 ≤ magnitude ≤ T::MAX` — **except** when the literal is
    the direct operand of unary `-` (the accepted AST shape
    `Unary(Neg, Lit)`), where `magnitude ≤ T::MAX + 1` is allowed
    (permitting `-9223372036854775808` = `i64::MIN`, the exact
    case amendment B-2 preserved the magnitude for).
  - Violation: `literal '300' does not fit in 'i8'` at the literal span,
    naming the target type; the expression is poisoned.
- A literal that never resolves to a concrete type at end of body:
  defaults to `i64` (SPECIFIED “assume i64 … quando ambíguos”), then the
  representability rule is applied against `i64` (a magnitude above
  `i64::MAX` with no negation context is an error — B-2).
- Mixed literal arithmetic: `1 + x` with `x: u8` — the literal’s `IntVar`
  unifies with `u8` (checked against the other operand’s type); result
  `u8`. `1 + 1` with no context → both default to `i64` → `i64`.

### 8.2 Float literals

- A `Float` literal has type `FloatVar` constrained to `{f32, f64}`;
  defaults to `f64` when ambiguous (SPECIFIED). Values are finite by
  construction (accepted B-3).
- `f32` representability: a finite literal that overflows `f32` when
  checked against it is a diagnostic (`literal '1e300' does not fit in
  'f32'`); underflow-to-zero keeps the accepted policy (round-trips;
  accepted B-3).

### 8.3 Residual variables

At the end of a function body, every remaining `IntVar`/`FloatVar`
resolves by defaulting (§8.1/§8.2). Any remaining plain `Var` reachable
from a typed expression that was **not** poisoned is an error at the
instrumented construct: `cannot infer type; add a type annotation` (with
a note pointing at the annotation position when one exists). This covers
`let x = [];` and `let v = None;` deterministically.

## 9. Explicit conversions `as` (P2D-B12)

- SPECIFIED: no implicit coercion between numerics; `as` explicit. The
  original does not enumerate the allowed cast pairs or their value
  semantics; the proposal (owner decision) is:

| From → To | Allowed | Value semantics (proposal) |
| --- | --- | --- |
| integer → integer | yes | truncate to the target width, two’s-complement wrap (e.g. `300 as u8 == 44`) |
| integer → `f32`/`f64` | yes | round to nearest (IEEE-754 default) |
| `f32`/`f64` → integer | yes | saturating truncate toward zero (out-of-range → target MIN/MAX; NaN → 0) — Rust’s defined `as` semantics |
| `f32` ↔ `f64` | yes | IEEE-754 conversion (round to nearest) |
| `bool`/`char`/`str` → numeric, or numeric → `bool`/`char`/`str` | **no** | not introduced (no evidence; UNRESOLVED) |
| any other pair | **no** | `cannot cast 'X' to 'Y'` |

- Rationale for adopting Rust’s documented numeric `as` semantics:
  every permitted pair needs *some* defined value behavior to be
  implementable in the VM (Phase 4); Rust’s rules are the obvious
  ecosystem baseline and are already audited. The alternative
  (only value-preserving casts) is recorded in the packet.
- Casting is checked in WP-C: both sides are concrete types (no casts to
  or from unresolved variables except final defaulting, no casts
  involving `Never`/`Error` beyond poisoning).

## 10. Functions, calls, returns, and control flow (P2D-B16)

- **Calls:** callee must have a `Fn` type (or be a resolvable function
  item / method / prelude constructor / enum variant constructor via
  `DefId`); arguments are checked left to right against declared
  parameters; arity mismatch: `this function takes 2 arguments but 1 was
  supplied` (primary at the argument list). Return type is the instantiated
  signature’s return (`Unit` when absent).
- **Enum variant construction:** `Shape::Circle(r)` has type `Shape`
  (payload checked against the variant’s payload types); prelude
  `Ok(1)`, `Err("x")`, `Some(v)` accordingly.
- **Zero-payload variants as values:** a path expression that resolves to
  a zero-payload variant **is a value of the enum type** — this is how
  `None` is written (`let n: Option<i64> = None;`). The rule is uniform
  for prelude and user enums (DERIVED: the prelude is not usable without
  it, and asymmetry would be surprising); recorded with `P2D-B4`/`P2D-B5`.
  A bare path in a constructor position (`Shape::Circle` without
  arguments, for a payload-carrying variant) is a type error: `variant
  'Circle' expects 1 argument`.
- **Returns:** a `return` operand is checked against the declared return
  type; `return;` (no operand) requires `Unit`. A missing tail in a
  non-unit function is a mismatch (`expected 'i64', found '()'`),
  reported at the closing brace span (the accepted AST has the block
  span).
- **Divergence typing (`Never`):**
  - `return`, `break`, `continue` have type `Never`; `Never` unifies with
    everything (bottom) but is never nameable.
  - `while`, `for`, `loop` expressions have type `Unit` (a `break`
    completes the loop; no break-with-value exists — UNRESOLVED;
    divergence analysis is not performed in Phase 2).
  - `if` without `else`: `Unit`; the then-branch is checked against
    `Unit` (its value, if any, must unify with `Unit` — i.e. be unit or
    `Never`).
  - `!` operator: `bool → bool`; `-` unary: integer→same, float→same;
    binary arithmetic/comparison per §13; `&&`/`||`: `bool → bool`.
- These rules resolve the Category C item “control-flow type behavior”
  that Phase 1 deferred to the type-system phase; they are a `P2D-B16`
  owner decision (the original does not fix them).

## 11. Mutability checking (P2D-B17)

SPECIFIED: `let` immutable by default; assignment requires `mut`
(§7; §6 Phase 2 “verificação de mutabilidade”).

- Mutability is a property of **bindings** (`Local`): `let mut` marks
  every binding the pattern introduces mutable (the grammar has one `mut`
  per `let`); parameters and the receiver are **immutable** (no `mut` in
  the accepted param grammar — assignment to them is an error).
- **Root-binding rule:** an assignment target `path`, `o.f`, `v[i]`,
  possibly nested (`o.a.b`), is legal only if its *root* is a mutable
  local. Fields and elements do not have individual mutability in
  Phase 2. `self.x = 1` inside a method is an error (receiver
  immutable) unless the owner later decides receiver mutability
  (UNRESOLVED: `mut self` is not in the grammar; Category C).
- Assignment type rule: LHS and RHS unify; assignment has type `Unit`
  (Phase 1 deferred this: ratified PD-003 §5 proposal, decided here —
  recorded in `P2D-B16`).
- Compound assignment `+= -= *= /= %=`: LHS must be a mutable target; the
  underlying binary operator must apply (numeric LHS/RHS of the same
  primitive type after defaulting); result `Unit`.
- Diagnostics: `cannot assign to immutable binding 'x'` with
  `help: declare it with 'let mut'`; `cannot assign to parameter 'o'`;
  `cannot assign to immutable value` for non-local roots (e.g. a call
  result — the parser already rejects such shapes as invalid targets, so
  this is a defensive path).
- Shadowing: assignments before a shadowing declaration refer to the
  outer binding (identity-based; `name-resolution-contract.md` §2).

## 12. Composite types

- **Tuples:** `(T1, …, Tn)`; element access does not exist in Phase 2
  (`x.0` is a parse error — accepted N21); tuples are built and
  destructured by patterns only. Tuple type arity is part of identity
  (`(i64, bool)` ≠ `(bool, i64)`).
- **Arrays and slices (P2D-B18):** `[T; N]` with `N` a non-negative
  integer literal (per the `[T; N]` grammar, `N` is an expression; only
  literals are accepted; `array length must be a non-negative integer
  literal`); a literal above `usize::MAX` is a diagnostic; `[T]` (slice)
  is nameable and self-unifiable but **no Phase 2 expression has slice
  type** (array literals are `[T; N]`; there are no slice operations).
  Indexing `a[i]`: `a: [T; N]`, `i` an integer literal variable resolved
  to any integer primitive → `T` (index bounds are a runtime concern for
  Phase 4, not Phase 2 — no static bounds checking is promised).
  `str` indexing, tuple indexing, and indexing non-arrays: not introduced
  (`cannot index into a value of type 'X'`).
- **Function types (`fn(A) -> B`):** unify structurally
  (contravariance/covariance is irrelevant: type variables are only
  unified; no subtyping exists). Function items have fn types; a path
  referring to a function may be used wherever its fn type is expected.
- **Unit/empty tuple:** equivalent as types (§3).
- No subtyping, no variance, no coercions: unification is symmetric
  equality except the bottom/poison rules.

## 13. Operators (P2D-B15)

- Built-in operators are defined for primitive types only:

| Operator(s) | Operand / result rule |
| --- | --- |
| `+ - * / %` | both operands the **same** primitive numeric type (after literal defaulting); result that type. `str` has no `+` (concatenation is internal; `P2D-B10`). `%` on floats: allowed (value semantics is a Phase 3/4 implementation question; recorded as not-yet-specified). |
| `== !=` | both operands the same type, including tuples/nominals/fn types (structural equality is intended; implementation semantics deferred to Phase 3/4 for composite values; recorded) |
| `< <= > >=` | both operands the same primitive numeric type, `char`, or `str` (lexicographic?) — proposal: numeric and `char` only; `str` comparison NOT introduced (no evidence) |
| `&& \|\|` | both `bool`; result `bool` |
| `!` (unary) | `bool` → `bool` |
| `-` (unary) | numeric (integer/float) → same type |

- **Operators on type parameters or user types are not supported**
  (no operator traits exist, and none are invented — the original never
  defines operator overloading). A bound such as `T: Ord` is *declared,
  checked, and solvable* (a user may implement `Ord`), but `<` on a value
  of parameter type `T` remains an error in Phase 2 with a specific
  diagnostic (`operators on type parameters are not supported in this
  phase`). Rationale and alternative in `P2D-B15`; this is the single
  most visible Phase 2 limitation and is deliberately explicit.
- `Never` operands poison silently (`Error`-like unification).

## 14. Patterns, exhaustiveness, and `Result` propagation

- Pattern typing, constructor coverage, spliced matching, missing-pattern
  witnesses, reachability, and resource limits are specified in
  `traits-and-exhaustiveness.md` §3–§7.
- `?` typing (exact `Result`/error-type rules) is specified in
  `lowering-contract.md` §5.2 and enforced here; `?` on non-`Result` is a
  type error; error-type conversion is FUTURE (Category C).
- Irrefutability of `let`/`for` patterns reuses the exhaustiveness
  machinery (`P2D-B22`).

## 15. Diagnostics and cascade suppression

- One diagnostic per root cause: after a failed unification or an
  unresolved use, the expression/pattern is poisoned (`Error`), and all
  further checks involving `Error` are silent. `Never` propagates as
  bottom, not as poison.
- Message shapes and spans: `diagnostics-contract.md`; every message
  carries the best available span (user spans preferred; desugared
  constructs report the originating construct).
- Deterministic ordering (source order) and no duplicates: a construct is
  reported by exactly one stage (§2).

## 16. Complexity, limits, and adversarial inputs (P2D-A7)

All budgets are **finite, explicit, deterministic counters** (never
wall-clock timeouts) and all **fail closed** (Directive 010-R1 §8):

| Concern | Bound |
| --- | --- |
| HIR nesting depth | `MAX_HIR_DEPTH = 256` (max nested block/expr/pat/type chain; source depth ≤ `MAX_PARSE_DEPTH = 64`; each desugaring adds ≤ 4 levels) |
| HIR size (AST→HIR expansion) | `MAX_HIR_NODES = 1_000_000` per module |
| Definitions recorded | `MAX_DEFS = 100_000` per module |
| Uses recorded | `MAX_USES = 1_000_000` per module |
| Module graph size | `MAX_MODULE_GRAPH_MODULES = 1_024`; `MAX_MODULE_GRAPH_EDGES = 16_384` |
| Module graph traversal | `MAX_MODULE_GRAPH_DEPTH = 256` |
| Type lowering depth | `MAX_TYPE_DEPTH = 128` (accepted source types are ≤ 64 by parser limits; defensive margin) |
| Unification work | `MAX_UNIFY_STEPS` per body (proposal: 100_000 node-visits); occurs check O(|T|), bounded by the same budget |
| Fresh variables per body | `MAX_INSTANTIATIONS` (proposal: 10_000) |
| Trait solving | `MAX_TRAIT_DEPTH = 32`, cycle detection on solver goals (`traits` doc) |
| Exhaustiveness | `MAX_PATTERN_CELLS` (proposal: 250_000 cell-visits per match), deterministic fail-closed diagnostic |
| Diagnostic count | no hard cap; cascade suppression makes counts linear in root causes; the CLI never truncates |

These values agree with the canonical list in `P2D-A7`
(`decision-packet.md`), which is the single source of truth for the
numbers.

- All budgets fail **closed**: exceeding one produces a specific error
  diagnostic (`checking resource limit exceeded in <stage>`) — never a
  silent pass and never a panic. Rationale: an unverified exhaustiveness
  claim or a skipped unification would be unsound; failing the compile is
  honest.
- No uncontrolled native recursion: every recursive walk is bounded by
  the HIR depth budgets (accepted source depth ≤ 64) or the explicit
  limits above; the arena representation makes traversal iterative where
  needed.
- Malicious source inputs (very large literal counts, deeply nested
  types/patterns/matches, pathological unification chains) are covered by
  the future fuzz/test obligations in `conformance-plan.md` §7.

## 17. Representative tests (for the future implementation)

Positive/negative examples are enumerated in `conformance-plan.md` §3–§5
(type inference, mismatches, defaults, casts, generics, traits,
mutability, returns, `?`, exhaustiveness). Unit-level obligations:

1. Unification matrix: every `TyKind` pair, including `Error`/`Never`.
2. Occurs check: `fn f(x: fn(fn(...)) ...)`-style self-application via
   inference produces `cyclic type` without unbounded work.
3. Defaulting: `let x = 1;` → `i64`; `let f = 1.0;` → `f64`;
   `let x: u8 = 1;` → `u8`; `let x = 256; let y: u8 = x;` → mismatch.
4. B-2 boundaries: `-9223372036854775808` accepted as `i64`;
   `9223372036854775808` alone rejected; `u64` max accepted for `u64`.
5. Cast table: accepted/rejected pairs; value semantics examples pinned
   for the future VM (documentation-level in Phase 2).
6. Mutability: all combinations of local/param/field/index roots.
7. Control-flow typing: `if` without else as statement; `loop`/`while`
   unit; `return` in branches; `Never` unification examples.
8. Determinism: repeated checks produce byte-identical diagnostics.

## 18. Traceability

| Contract item | Source |
| --- | --- |
| Bidirectional inference; unification + occurs check; generalization restricted to `fn` | SPECIFIED §7 |
| Literal defaults `i64`/`f64`; no implicit numeric coercion; explicit `as` | SPECIFIED §7 |
| Generics depth limits; monomorphization is Phase 7 | SPECIFIED §7, §6 Phase 5/7 |
| Traits: declaration, `impl … for …`, bounds, coherence, orphan rule; no associated types | SPECIFIED §7 |
| Mutability: `let` immutable; assignment requires `mut` | SPECIFIED §7, §6 Phase 2 |
| Non-exhaustive match is an error; reachability | SPECIFIED §7 |
| Primitive/composite types (`i8`…`str`, `()`, `[T]`, `[T; N]`, tuples, `fn`) | SPECIFIED §3 |
| `Result<T, E>` + `?` model | SPECIFIED §2, §3, §6 Phase 2 |
| B-2/B-3 literal payload and finiteness | accepted amendments B-2, B-3 |
| Control-flow typing (loops unit, `Never`) | Category C (“deferred to the type-system phase”); decided here by `P2D-B16` |
| `if` without else is unit; missing-else value is a type issue | ratified grammar-rfc §8.12 |
| Poison/cascade suppression | never-panic + single-diagnostic-per-cause conventions (SPECIFIED §4; Phase 1 recovery) |
