# XLANG Phase 2 — Traits, Coherence, and Match Exhaustiveness

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §9 (traits, coherence, orphan
  rule) and §10 (Maranget-style exhaustiveness proposal), amended by
  Engineering Directive 010-R1 §6 (package-scoped ownership/orphan rule)
  and §9 (Phase 2 impl scope vs Phase 5 blanket impls).
- **Companion documents:** `type-system-contract.md` (types, unification,
  budgets), `hir-contract.md` (patterns, impls, traits),
  `name-resolution-contract.md` (paths, ambiguity, suggestions),
  `diagnostics-contract.md` (messages), `conformance-plan.md` (corpus),
  `decision-packet.md` (`P2D-B5`, `P2D-B14`, `P2D-B15`, `P2D-B19`,
  `P2D-B22`, `P2D-A7`).
- **No implementation is authorized.** All rules are PROPOSED unless
  labeled SPECIFIED.

## Part I — Traits

## 1. Surface and evidence

SPECIFIED §7: “Traits: declaração, `impl Trait for Type`, bounds;
resolução por coerência (uma implementação por par trait/tipo; regra de
órfão); associated types na v1.1” — associated types are therefore
**excluded from Phase 2 by the original itself**.

SPECIFIED surface (accepted grammar, §3.1 of `grammar-rfc.md`):

```ebnf
trait_decl = [ "pub" ] "trait" IDENT [ generics ] "{" { trait_item } "}" ;
trait_item = "fn" IDENT "(" [ params ] ")" [ "->" type ] ";" ;   (* signatures only *)
impl_decl  = "impl" [ generics ] path_type [ "for" path_type ] "{" { impl_item } "}" ;
generic_param = IDENT [ ":" path_type ] ;                        (* at most ONE bound *)
```

Consequences honored by this design:

- Trait items are signature-only functions; no default bodies, no
  associated consts/types (UNRESOLVED; not introduced).
- A generic parameter carries **at most one bound** (the grammar has one
  optional `: path_type`); `T: A + B` is not expressible. No workaround
  is invented.
- `impl` may be inherent (`impl Point { … }`) or trait
  (`impl Show for Point { … }`); generics are allowed on both.
- The original's bound example is `fn f<T: Ord>(a: T)` (§6 Phase 5).

## 2. Trait model (P2D-B14)

### 2.1 Declarations

- A `trait` declaration introduces a `Def { kind: Trait }` with generic
  parameters and trait-item signatures. Trait items are **implicitly
  public** (no `pub` is syntactically permitted; DERIVED from trait
  semantics — recorded in the packet).
- A trait’s method signatures may use:
  - the receiver `self` (parameter-position syntax accepted by Phase 1);
    `self`’s type is the contextual `Self`;
  - **contextual `Self`** in any type position of the signature
    (`fn show(self) -> str`, `fn cmp(self, other: Self) -> i64`). `Self`
    is **not a keyword** (it is absent from the §3 keyword list): it is an
    identifier recognized contextually in trait declarations and impl
    bodies, resolving to the trait’s implicit `Self` parameter / the
    impl’s self type. This is new surface semantics — proposed only
    because without it additive traits are inexpressible — and requires
    explicit owner approval (`P2D-B14`).
  - the trait’s own generic parameters and the prelude types.
- A trait declaration’s “bound” for its implicit `Self` parameter is
  itself.

### 2.2 Implementations

- `impl Trait for Type` must:
  1. resolve `Trait` to a trait definition and `Type` to a nominal type
     (struct/enum) — a trait as the self type is not a Phase 2 concept
     (`dyn`/`impl Trait` do not exist);
  2. provide a method for **every** trait item, with matching arity and
     pairwise-unifiable parameter/return types after substituting
     `Self := Type` and the impl’s generics;
  3. contain no extra methods that are not in the trait (an extra method
     in a trait impl is an error — no evidence for trait-impl extension;
     recorded in the packet);
  4. satisfy the orphan rule (§2.5) and coherence (§2.4).
- Method bodies type-check under the impl’s generics; `Self` is the self
  type.
- Inherent `impl Type { … }` blocks:
  - may contain `fn`s with receivers and associated (non-receiver)
    functions; bodies check normally;
  - **duplicate method names within the same type’s inherent impls are
    errors** (`duplicate method 'm' on type 'Point'`), across multiple
    impl blocks as well (coherence for inherent methods);
  - inherent methods take precedence over trait methods in selection
    (§2.6).

### 2.3 Bounds and their checking

- A bound `T: Trait` on a generic parameter introduces an obligation
  whenever `T` is instantiated:
  - **At the definition site:** the bound is recorded; inside the body,
    only what the bound guarantees may be relied on — in Phase 2 that is
    *nothing operational* because operator resolution through traits is
    deferred (`P2D-B15`); the bound is still *checked* at every use site;
  - **At use sites:** instantiating `f<T: Ord>` at a concrete type `C`
    requires an impl of `Ord` for `C` to exist in the program (solver
    §2.7). Missing: `the trait bound 'C: Ord' is not satisfied`
    (primary at the argument/use, secondary at the bound declaration).
- A bound naming an undeclared or non-trait path is an error at the
  generic parameter span (`cannot find trait 'Ord' in this scope` /
  `'Point' is not a trait`).
- Bound cycles through impls are handled by the solver budgets (§2.7),
  not by rejecting them up front.

### 2.4 Coherence

SPECIFIED §7: one implementation per trait/type pair; resolution by
coherence. Implementation plan:

- Collect all impls per trait. For each **pair of impls of the same
  trait**, check whether their self-type *heads* can unify under fresh
  variables for all impl generics (**unification-based overlap check** —
  the faithful reading of “one implementation per pair”):
  - `impl Show for Point` vs `impl Show for Point` → overlap;
  - `impl Show for Vec<i64>` vs `impl Show for Vec<i64>` → overlap;
  - `impl<T> Show for Vec<T>` vs `impl Show for Vec<i64>` → overlap
    (`T := i64` unifies);
  - `impl<T> Show for Vec<T>` vs `impl<T> Show for Option<T>` → no
    overlap (different heads).
  - `impl<T> Show for T` (blanket) overlaps with every other `Show` impl.
- **Phase 2 impl scope (`P2D-B14`, Directive 010-R1 §9).** The
  recommendation is to **support generic impls whose self-type head is a
  nominal type** (`impl<T> Show for Vec<T>`, `impl Show for Point`,
  `impl<T, U> Show for Pair<T, U>`) — these are the impls the
  specification’s `impl … for …` surface and bound checking require in
  Phase 2, and their coherence check is the ordinary unification-based
  overlap check above.
- **Unconstrained blanket impls** (`impl<T> Show for T`) are **deferred to
  Phase 5**. They are not required for Phase 2 semantic checking, they
  make coherence algorithmic (overlap with every impl), and they can
  silently capture method selection across the whole program — a
  trait-ecosystem concern (Directive 010-R1 §9). The packet keeps all
  alternatives: (i) allow blanket impls now with solver cycle detection
  and depth limits; (ii) reject blanket impls outright; (iii)
  nominal-head generic impls only (recommended, pending owner decision).
- The solver still ships cycle detection and depth limits (`§2.7`)
  because nominal-head generic impls can themselves be recursive
  (`impl<T: Show> Show for Vec<T>`).
- Diagnostics: `conflicting implementations of trait 'Show' for type
  'Point'` (primary at the later impl header, secondary at the first,
  with a note naming the unifying substitution when the conflict is
  generic).
- Inherent impls do not participate in trait coherence (they have no
  trait), but their method-name uniqueness is enforced (§2.2).

### 2.5 Orphan rule (ownership domain)

SPECIFIED §7. Statement of the rule for XLANG, with an explicit
ownership domain (Directive 010-R1 §6):

- **Definition ownership is modeled explicitly and includes a package
  identity.** Every definition carries its owning `ModuleId` and, through
  it, its `PackageId` (`hir-contract.md` §3), and every cross-module
  reference is a `GlobalDefId { module, definition }`.
- **Ownership domain = the package.** For `impl Trait for Type`, the impl
  is legal when at least one of `Trait` or `Type` is **owned by the
  current package** (i.e. defined in a module whose `PackageId` equals the
  impl module’s package). This is the recommended policy: it preserves the
  ordinary ability to implement a local trait for a foreign type and a
  foreign trait for a local type *within one package*, and it does not
  silently establish a permanent module-only orphan rule that would later
  make normal cross-module implementations inside one package invalid.
  The alternative (module-scoped ownership) is recorded in the packet.
- **Phase 2.** In the single-module mode every trait and type is in the
  one synthetic package, so the rule is vacuously satisfied. It is still
  **implemented and unit-tested** against the synthetic registry harness
  (`P2D-A5`), including cross-module cases within one package (legal) and
  an external-package case (orphan), so the registry/package phase
  inherits a working check. Running it against real multi-file input is
  FUTURE (Category C).
- Diagnostics name the ownership domain consistently: `only traits defined
  in the current package can be implemented for external types` (and the
  mirror case). The earlier “current module” wording is superseded.
- The prelude’s `Option`/`Result` are **owned by the current package’s
  prelude module**: implementing a user trait for `Option<T>` is legal
  (the trait is local to the package); implementing a trait owned by an
  external package for any type is an orphan error.

### 2.6 Method selection (deterministic order)

For a call/member `base.name(args…)` where `base` has a nominal type:

1. **Inherent methods** of the type: if exactly one matches by name and
   arity → select it. If several (should be impossible after §2.2
   uniqueness, except by generics of the impl — see note) → ambiguity
   error listing candidates in declaration order.
   - Note: the same method name may exist in `impl Point` and
     `impl<T> Point<T>` — these are different impls; both unify against a
     concrete `base` type when `T` unifies. Genuine ambiguity is reported,
     never silently resolved.
2. **Trait methods**: all traits with a matching-name method and an impl
   for `base`’s type (after unification with impl generics). In Phase 2
   all declared traits are in the module and therefore “in scope”.
   - Zero candidates → `no method 'm' on type 'Point'` with suggestions.
   - One candidate → select.
   - Two or more → `multiple applicable methods in scope` listing
     `Trait::method` candidates in trait-declaration order (deterministic)
     with the primary span on the method name.
3. Field vs method: a `Field` use that is a call callee prefers a method;
   otherwise prefer a field. A name that is both a field and a method is
   an error in the declaration side (`field and method share the name
   'x'`) — methods live in impls and can collide with struct fields;
   this collision is rejected as part of coherence of the type’s
   interface.
4. Built-in members: **none** in Phase 2 (no stdlib; `.sqrt()`,
   `.to_int()` from the example are stdlib methods and are FUTURE). The
   internal `Range` functions are not user-callable.

### 2.7 Trait solver

The solver answers obligations of the form `Type: Trait` (bounds) during
instantiation and produces method evidence for selection:

- Deterministic search: for obligation `C: Trait`, scan impls of `Trait`
  in declaration order; for each, unify the impl head (with fresh
  variables for impl generics) against `C`; on success, recurse on the
  impl’s bounds with the substitution.
- **Cycle detection:** a goal stack of obligation keys; revisiting the
  same goal on the stack is a cycle → the goal fails (no matching
  evidence) and a diagnostic reports the cycle
  (`cannot satisfy 'T: Show': recursive requirement`).
- **Depth:** `MAX_TRAIT_DEPTH = 32`; exceeding it → fail-closed
  diagnostic (`trait resolution depth limit exceeded`), never a silent
  success.
- **Memoization:** solved goal → result cached per body for determinism
  and performance (cache keys are canonicalized types).
- Failure to satisfy a bound is always a diagnostic; there is no
  “default” impl, no specialization, and no backtracking beyond the
  deterministic scan.

### 2.8 Trait diagnostics (shapes)

Representative messages (exact strings pinned at implementation,
`diagnostics-contract.md` §5):

- `the trait bound 'i64: Ord' is not satisfied`
- `the trait 'Show' is not implemented for 'Point'`
- `conflicting implementations of trait 'Show' for type 'Point'`
- `only traits defined in the current package can be implemented for external types`
- `method 'show' is not a member of trait 'Show'` (extra method)
- `missing method 'show' in implementation of trait 'Show'`
- `multiple applicable methods in scope`

## Part II — Match Exhaustiveness and Reachability

## 3. Scope and evidence

SPECIFIED §6 Phase 2: pattern exhaustiveness checking (Maranget's
algorithm); SPECIFIED §7: exhaustiveness and reachability of patterns;
non-exhaustive `match` is an error. The original names the algorithm, so
the design follows Maranget's usefulness-based method.

Phase 2 pattern surface (accepted grammar, ratified):

```ebnf
pattern = "_" | IDENT | literal | path ["(" patterns ")"] | "(" patterns ")" ;
```

No or-patterns, no guards, no ranges, no negative literals, no `ref`/`mut`
bindings, no struct patterns (all UNRESOLVED; not introduced).

## 4. Pattern typing (input to the analysis)

- **Wildcard** `_`: matches anything; binds nothing; may repeat.
- **Binding** `x` (after classification, `P2D-B5`): matches anything;
  binds a fresh local of the scrutinee type.
- **Literal** patterns: check literal type against the scrutinee type
  with the same defaulting rules as expressions (`P2D-B11`):
  - integer literal patterns respect `IntVar` defaulting — `match x { 1
    => … }` with `x: u8` checks `1` as `u8` (must fit);
  - float literal patterns: allowed (grammar); equality is IEEE
    equality; `NaN` cannot be written (no literal) — a note in the
    packet records that float patterns are of dubious practical use but
    are accepted syntax (UNRESOLVED semantics beyond equality);
  - `char`, `str`, `bool` literals compare by value;
  - literal patterns never introduce bindings.
- **Constructor (variant) patterns**: the path resolves to a variant of
  an enum `E`; the pattern’s type is `E` (scrutinee unified); `args`
  arity and types check against the variant payload; nested sub-patterns
  check recursively. Prelude `Some`/`None`/`Ok`/`Err` are constructor
  patterns by the same rule.
- **Tuple patterns**: scrutinee must be a tuple of the same arity;
  sub-patterns check against elements. `()` matches `Unit`.
- **Malformed/error patterns** (`PatKind::Error` from recovery): typed as
  `Error` (poison); analysis treats the containing arm as *unknown*
  coverage (§6.6).

## 5. Pattern matrix and usefulness (Maranget)

The analysis operates on a **pattern matrix**: rows = arms (in source
order), columns = scrutinee components (one for `match e`, more for
tuple/constructor specialization). Each matrix cell is a simplified
pattern:

```text
Wildcard
Constructor(c, [sub-patterns])   // enum variant, tuple, unit, bool(true/false), or a literal class
Literal(value)                   // integers, chars, strings, floats (a "constructor" of an infinite signature)
```

Algorithm (standard, deterministic):

- `is_useful(matrix, row)`: Maranget's `U` procedure, with these XLANG
  specializations:
  - **Constructors with finite signatures** — enums (variants in
    declaration order), `bool` (`false` then `true`), unit `()` (single
    constructor), tuples (single product constructor). A complete
    signature is enumerable and drives specialization.
  - **Infinite signatures** — integers, `char`, `str`, floats: the
    signature is never complete; a wildcard row is required for
    exhaustiveness; literal patterns specialize only against equal
    literals (the “default matrix” path).
  - **Witness construction**: when a row is *not* useful (redundant) or
    when a wildcard row is not useful against a matrix (exhaustive),
    witnesses are produced by the same recursion (Maranget's I).
- **Exhaustiveness:** `match e { arms }` is exhaustive iff
  `is_useful(matrix(arms), [Wildcard])` is **false**.
- **Reachability:** arm *i* is reachable iff `is_useful(matrix(arms[0..i]),
  row_i)` is **true**; otherwise it is unreachable (covered by earlier
  arms).
- Termination and complexity: matrix rows are finite; specialization
  strictly reduces the pattern size on the finite-constructor branches;
  the default-matrix branch handles infinite signatures without
  enumeration. Worst-case behavior is bounded by the resource limits of
  §6.7 (the algorithm is exponential in adversarial cases; the limit is
  the safety net).

## 6. Analysis contract

### 6.1 Missing-pattern diagnostics (deterministic)

When a match is not exhaustive, the checker computes one or more witness
patterns and renders them deterministically:

- The witness is computed by the *first* failing path of the usefulness
  recursion (a fixed traversal order — columns left-to-right,
  constructors in declaration order; this fixes the witness among many
  valid ones).
- Rendering rules:
  - enum witnesses render as `Enum::Variant(_, …)` with wildcards for
    unspecified payload columns; a variant whose payload is fully
    constrained by the remaining columns renders concrete nested
    witnesses;
  - infinite-signature witnesses render `_`;
  - tuples render `(w1, w2, …)`.
- Message: `match is not exhaustive` (primary at the match scrutinee or
  the `match` keyword) with `help: add a pattern for 'Shape::Rect(_, _)'`
  (the witness), or a list of up to 4 witnesses when the missing set is a
  small finite enumeration (all missing variants, declaration order);
  beyond 4, the first witness plus `(and N more)`.

### 6.2 Exhaustiveness over combinations

The analysis composes correctly across nested constructors and tuples
(e.g. `(bool, bool)` requires all four combinations, or wildcards that
cover them) because specialization is recursive. Missing combination
witnesses render as concrete nested patterns (e.g. `(false, true)`).

### 6.3 `bool`, unit, and enums

- `bool`: `{ false, true }` (fixed order); `match b { false => …, true =>
  … }` is exhaustive; a single-arm `true`-only match reports
  `add a pattern for 'false'`.
- Unit: single constructor; `match () { () => … }` is exhaustive (and the
  unit pattern is the only one).
- Enums: constructor set = variants in declaration order; a match listing
  every variant (with arbitrary payload sub-patterns) is exhaustive only
  if every variant row is itself exhaustive within its payload columns
  (nested usefullness recursion).

### 6.4 Literals and infinite types

- `match n { 1 => …, _ => … }` — exhaustive (wildcard).
- `match n { 1 => …, 2 => … }` — not exhaustive; witness `_` (an integer
  other than 1, 2 cannot be named with this pattern surface).
- Duplicate literal arms: the second is unreachable
  (`unreachable pattern` with the duplicate value and the covering arm).
- `str`/float patterns behave as infinite-signature literals (equality
  classes only).

### 6.5 Empty matches

`match e { }` (the grammar permits an empty arms list; accepted Phase 1
note N16 defers to Phase 2): this is **not exhaustive for any non-`Never`
scrutinee**; for a `Never` scrutinee it is trivially exhaustive (no value
exists) — the analysis recognizes `Never` and accepts it. Primary
diagnostic at the match, witness `_` (or the complete constructor set for
finite types: `add a pattern for 'Shape::Circle(_, …)'` etc.).

### 6.6 Error recovery when patterns are malformed

- An arm with a `PatKind::Error` pattern (from parser recovery) makes the
  matrix’s coverage **unknown** for that arm: the checker must not claim
  exhaustion and must not report missing patterns that may be covered by
  the malformed arm.
- Policy: if any arm in the match is error-typed, the exhaustiveness and
  reachability analyses are **skipped silently** for that match (the
  parse error was already reported; no cascade). Deterministic and
  conservative.
- Malformed *sub*-patterns inside an otherwise valid pattern poison only
  that arm’s row (treated as error) with the same skip policy.

### 6.7 Resource limits for pathological matrices

- `MAX_PATTERN_CELLS` (proposal: 250_000 cell-visits per match,
  `P2D-A7`): a deterministic budget on specialization steps.
- On exhaustion: the match is *not verified*; the checker emits
  `match exhaustiveness could not be verified (resource limit; simplify
  the match)` — **fail closed** (an error), never a silent pass, because
  an unverified exhaustiveness claim could be wrong at runtime (the VM’s
  tag checks would trap).
- Alternative (packet: warning severity) is recorded; the recommendation
  is the fail-closed error.
- Determinism: budget accounting is a deterministic step counter, so the
  same input always hits the same limit at the same point.

## 7. Refutability in `let` and `for` (P2D-B22)

- `let pat = expr;` requires `pat` to be **irrefutable** for the
  expression’s type: `is_useful(matrix([pat]), [Wildcard])` must be
  false (equivalently: no witness exists). Examples:
  - `let (a, b) = pair;` — irrefutable for a tuple type;
  - `let x = v;` — irrefutable;
  - `let Some(v) = opt;` — refutable → error `refutable pattern in let
    binding` with `help: use 'match'`.
- `for pat in seq { … }` uses the same check against the item type
  (`lowering-contract.md` §4.2, contract point 6).
- This reuses the machinery of §5 (no second algorithm).

## 8. Severity summary (P2D-B19)

| Situation | Severity | Source |
| --- | --- | --- |
| Non-exhaustive `match` | **error** | SPECIFIED §7 |
| Unreachable match arm | **warning** (PROPOSED) | SPECIFIED §7 lists reachability; severity not fixed → packet |
| Refutable `let`/`for` pattern | **error** | needed for correctness (no runtime mismatch semantics exist); packet |
| Exhaustiveness resource limit | **error** (fail closed) | `P2D-A7`/§6.7 |
| Pattern type mismatch | error | §4 |

## 9. Determinism and invariants

1. Arm order is source order; constructor order is declaration order;
   `bool` is `false, true`; witnesses choose the first failing path of a
   fixed traversal. Two runs produce identical diagnostics.
2. Reachability warnings are computed against the *prefix* of arms only
   (an arm never makes itself unreachable).
3. Exhaustiveness is computed against all non-error arms; error arms skip
   the analysis (§6.6).
4. No analysis depends on hash iteration order; matrices are `Vec`s.

## 10. Representative tests (future implementation)

Trait side:

1. Declaration/impl signature matching: arity, parameter types, return
   type, `Self` substitution; missing/extra methods.
2. Bounds: satisfaction at instantiation (positive/negative), one-bound
   limit, undeclared bound path.
3. Coherence: exact duplicates, generic overlaps (`Vec<T>` vs
   `Vec<i64>`), non-overlaps, and (Phase 2 scope) nominal-head generic
   impls; `impl<T> Show for T` is exercised as a deferred-Phase-5 case
   (recorded, not accepted in Phase 2 under the recommended scope).
4. Orphan rule via the synthetic registry harness: same-package
   cross-module impl (legal) and external-package impl (orphan).
5. Method selection: inherent precedence, trait ordering, ambiguity,
   no-method suggestions, field/method collision.
6. Trait solving determinism: identical goals produce identical
   evidence/diagnostics; budgets and cycle detection are exercised.

Exhaustiveness side:

1. Exhaustive/non-exhaustive for enums (all variants, subsets, nested
   payload columns), `bool`, unit, tuples (products), literals
   (integer/char/str/float), combinations.
2. Unreachable arms: duplicate literals, wildcard-before-specific,
   variant-before-nested.
3. Witness rendering determinism (fixed outputs).
4. Refutability in `let`/`for`.
5. Error-arm skip behavior (no cascade).
6. Resource limits: a matrix constructed to exceed `MAX_PATTERN_CELLS`
   yields exactly the fail-closed diagnostic; boundary tests at the
   limit.
7. Empty match for `Never` vs normal types.

The corpus identifiers and `.x`/`.stderr` fixtures are enumerated in
`conformance-plan.md` §4–§5.

## 11. Traceability

| Contract item | Source |
| --- | --- |
| Trait declarations, `impl Trait for Type`, bounds | SPECIFIED §7; grammar §3.1 |
| Coherence: one impl per trait/type pair | SPECIFIED §7 |
| Orphan rule (ownership domain = package) | SPECIFIED §7; Directive 010-R1 §6; `P2D-A5`/`P2D-B14` |
| Phase 2 impl scope (nominal-head generic impls; blanket deferred) | Directive 010-R1 §9; `P2D-B14` |
| Associated types excluded (v1.1) | SPECIFIED §7 |
| Maranget exhaustiveness; reachability; non-exhaustive match is an error | SPECIFIED §6 Phase 2, §7 |
| One bound per generic parameter (grammar) | ratified grammar-rfc §3.1 |
| `Self` contextual (proposed addition) | packet `P2D-B14` (no original evidence) |
| Pattern surface restrictions (no or-patterns/guards) | ratified grammar-rfc §3.5 |
| Empty match deferred to Phase 2 | accepted Phase 1 note N16 |
| Fail-closed budgets; determinism | `P2D-A7`; INV-007 |
