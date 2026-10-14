# XLANG Phase 2 — Conformance and Test Plan

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §15 (conformance and test
  planning), §16 (safety and performance), §17 (quality gates for a
  design-only package).
- **Companion documents:** every contract in this directory;
  `decision-packet.md` (`P2D-A7`).
- **No test in this plan can run today:** no `xl-hir`, `xl-types`, or
  `xc check` exists. Every example below is a **future conformance
  fixture** — a documented expectation, not an observed result. No
  results are claimed.

## 1. Test architecture (future implementation)

| Layer | Location (future) | What it asserts |
| --- | --- | --- |
| Unit tests | inside `xl-hir`/`xl-types` | unification matrix, defaulting, scope structures, suggestion engine, pattern-matrix steps, budgets |
| Integration tests | `crates/xl-hir/tests/`, `crates/xl-types/tests/` | lowering snapshots, resolution tables, type tables, exhaustiveness witnesses |
| HIR/AST golden snapshots | `insta` (existing dev-dependency) | deterministic HIR dumps and normalized ASTs |
| End-to-end CLI tests | `crates/xl-cli/tests/` (pattern established in Phase 1) | the real `xc` binary invoking `xc check` on real `.x` files, exact stdout/stderr/exit |
| Compile-fail corpus | `tests/compile-fail/<case>.x` + `<case>.stderr` | the SPECIFIED §6 Phase 2 test form |
| Property tests | `proptest` (existing dev-dependency) | determinism, idempotence of repeated checks, resolution totality |
| Fuzzing | `fuzz/` (existing harness style) | no panics/stack overflow/resource exhaustion on arbitrary input through the full frontend + check |

Only dependencies already present in the workspace (`insta`, `proptest`,
`libfuzzer` harness under `fuzz/`) are assumed; none may be added without a
documented justification (governance §6).

## 2. Corpus conventions

### 2.1 Compile-fail fixtures (SPECIFIED test form)

```text
tests/compile-fail/<case>.x        # the source under check
tests/compile-fail/<case>.stderr   # the exact rendered diagnostics
```

- `<case>.stderr` contains the expected **complete** stderr text
  (all diagnostics, in order) with:
  - **LF line endings, canonical** (checked-in files are LF; the
    comparison normalizes CRLF in both sides, matching the accepted
    Phase 1 CLI conformance practice);
  - **no absolute paths**: fixtures are invoked with a bare file name in
    the fixture directory, and the CLI prints the argument as given;
  - no environment- or platform-dependent content (no I/O errors, no
    locale, no timing);
  - exit status pinned by the runner (`1` for source errors — §6.1).
- Cases whose behavior depends on **unavailable infrastructure**
  (multi-module registry: import cycles, cross-module visibility,
  orphan-rule cross-module cases) are marked **FUTURE** and are
  documentation-only until the registry phase; they are listed but not
  part of any Phase 2 acceptance run.
- Positive cases use `xc check` expecting exit `0` and empty stderr
  (section 3) plus integration assertions on resolution/type tables and
  HIR snapshots.

### 2.2 Naming and traceability

Fixture identifiers follow the contract areas: `nm-*` (names),
`ty-*` (types), `tr-*` (traits), `pl-*` (patterns/exhaustiveness),
`typing-*`/`flow-*`, `res-*` (resources). Each fixture’s expectation must
cite the contract section that fixes the message shape; the decision
packet rows are cited where behavior depends on a pending decision.

## 3. Positive corpus (documentation-only, future)

### 3.1 Name resolution

| ID | Source sketch | Expected |
| --- | --- | --- |
| nm-ok-1 | module items referenced before/after declaration (order-independence) | resolves; exit 0 |
| nm-ok-2 | nested blocks shadowing outer locals | inner use resolves to inner `LocalId`; outer use to outer |
| nm-ok-3 | parameters, generics, and locals coexisting (`fn f<T>(x: T) -> T { let y = x; y }`) | each resolves by identity; no aliasing |
| nm-ok-4 | prelude use: `let o: Option<i64> = Some(1);` | resolves through prelude; unit variant `None` as a value (`let n: Option<i64> = None;`) |
| nm-ok-5 | shadowing a prelude constructor with a user item | user definition wins in scope |
| nm-ok-6 | unit-variant pattern (prelude and user enum) resolves as constructor | pattern classified as variant, not binding |
| nm-ok-7 | `::` module-relative paths and `.` member access, synthetic registry (FUTURE for real modules) | both resolve to the same public item |

### 3.2 Type system

| ID | Source sketch | Expected |
| --- | --- | --- |
| ty-ok-1 | `let x = 1;` | `x: i64` (default) |
| ty-ok-2 | `let x: u8 = 200;` | `x: u8`; representability ok |
| ty-ok-3 | `let f = 1.0;` / `let g: f32 = 1.0;` | `f64` default / `f32` |
| ty-ok-4 | `fn add(a: i64, b: i64) -> i64 { a + b }` | operator tier; exit 0 |
| ty-ok-5 | `fn id<T>(x: T) -> T { x } let y = id(1);` | instantiation; `y: i64` after defaulting |
| ty-ok-6 | tuple destructuring `let (a, b) = (1, true);` | `a: i64`, `b: bool` |
| ty-ok-7 | array `let xs: [i64; 3] = [1, 2, 3];` and indexing `xs[0]` | types check |
| ty-ok-8 | casts `1 as f64`, `300 as u8` | accepted pairs (`P2D-B12`) |
| ty-ok-9 | `let n = -9223372036854775808;` | accepted as `i64` (B-2 boundary) |
| ty-ok-10 | `fn f() -> i64 { if true { 1 } else { 2 } }` | branch unification; tail return |
| ty-ok-11 | `let x = 1; let mut y = x; y = 2;` | mutability ok; reads fine |
| ty-ok-12 | `fn f(x: i64) -> Result<i64, str> { let y = g(x)?; Ok(y) }` | `?` typing (with a user/local `g` returning `Result<i64, str>`) |
| ty-ok-13 | `let s = "v = {1}";` | interpolation with primitive formatting |
| ty-ok-14 | `for i in 0..10 { let j = i; }` | desugared loop types; `i: i64` |

### 3.3 Traits

| ID | Source sketch | Expected |
| --- | --- | --- |
| tr-ok-1 | trait declaration with receiver and `Self`; impl for a struct; call through a bound | declaration/impl/bound all check; method selected |
| tr-ok-2 | inherent method preference over a same-named trait method | inherent selected (no ambiguity) |
| tr-ok-3 | generic bound satisfied by a concrete impl (`f<T: Show>` called on a type with the impl) | bound satisfied |

### 3.4 Patterns

| ID | Source sketch | Expected |
| --- | --- | --- |
| pl-ok-1 | enum match covering all variants (with payload sub-patterns) | exhaustive; exit 0 |
| pl-ok-2 | `match b { false => 0, true => 1 }` | exhaustive |
| pl-ok-3 | `match n { 1 => 0, _ => 0 }` | exhaustive (wildcard) |
| pl-ok-4 | nested product `(bool, bool)` all four combos | exhaustive |
| pl-ok-5 | `match e { }` with `e: Never`-typed expression (`match return; { }`-style unreachable) | accepted (no values exist) — boundary fixture |

## 4. Negative corpus (documentation-only, future)

Each entry lists: expected **first** diagnostic shape (from
`diagnostics-contract.md`), the recovery expectation, and the fixing
contract section. The `.stderr` fixture carries the full rendered output;
only the first message is described here for brevity.

### 4.1 Name resolution

| ID | Source sketch | Expected |
| --- | --- | --- |
| nm-err-1 | `let x = cafe + 1;` (no `café`) | `cannot find value 'cafe' in this scope` (+ suggestion when close) |
| nm-err-2 | `fn f(x: i64) -> i64 { x }` twice | `duplicate definition of 'f'` |
| nm-err-3 | `let x = 1; let x = 2;` same block | `the name 'x' is already defined in this scope` (`P2D-B2`; alternative shadowing becomes the same fixture with exit 0) |
| nm-err-4 | parameter redeclared: `fn f(a: i64, a: i64)` | duplicate parameter error |
| nm-err-5 | `Type::method()` associated-item attempt | `'Point' has no associated items`-class error (path depth rule) |
| nm-err-6 | enum variant path too deep: `Shape::Circle::Deep` | error at the third segment |
| nm-err-7 | unknown generic arg arity: `Result<i64>` | `type 'Result' expects 2 generic arguments, found 1` |
| nm-err-8 | opaque module use: `import std.io; io.println(x);` | `module 'std.io' is not part of this compilation` (`P2D-B7`) |
| nm-err-9 | private item through the synthetic registry (FUTURE) | `item 'f' is private` |
| nm-err-10 | cyclic imports (FUTURE, two- and three-module fixtures) | `cyclic module import` + cycle note (`P2D-B7`, §9.4 of the contract) |

### 4.2 Types, literals, casts

| ID | Source sketch | Expected |
| --- | --- | --- |
| ty-err-1 | `let n: i64 = "text";` | `mismatched types: expected 'i64', found 'str'` |
| ty-err-2 | `fn f() -> i64 { }` | `expected 'i64', found '()'` at the body |
| ty-err-3 | `if 1 { }` | condition not `bool` |
| ty-err-4 | `300 as u8` value ok; `let x: u8 = 300;` | `literal '300' does not fit in 'u8'` (cast form is ty-ok-8: `as` casts truncate, so `let x: u8 = 300 as u8;` is accepted) |
| ty-err-5 | `let x = [];` | `cannot infer type; add a type annotation` |
| ty-err-6 | `let x = None;` (no context) | `cannot infer type` |
| ty-err-7 | `true as f64` | `cannot cast 'bool' to 'f64'` |
| ty-err-8 | `1 + true` | operand mismatch |
| ty-err-9 | `let a: [i64; 2] = [1, 2]; let b: [i64; 3] = a;` | array length mismatch |
| ty-err-10 | `let a: [i64; n] = [];` with non-literal `n` | `array length must be a non-negative integer literal` |
| ty-err-11 | `let s = "x" + "y";` | `+` not defined for `str` |
| ty-err-12 | `f::<i64>(1)` turbofish | parse error already (Phase 1); recorded as a non-Phase-2 fixture |
| ty-err-13 | operator on type parameter: `fn f<T>(a: T, b: T) -> bool { a < b }` | `operators on type parameters are not supported in this phase` (`P2D-B15`) |
| ty-err-14 | occurs-check shape (self-application via fn types) | `cyclic type` |

### 4.3 `?`, mutability, control flow

| ID | Source sketch | Expected |
| --- | --- | --- |
| res-err-1 | `let x = f()?;` where `f() : i64` | `the '?' operator requires a 'Result<T, E>' operand, found 'i64'` |
| res-err-2 | `fn f() -> i64 { g()? }` with `g() : Result<i64, str>` | `'?' requires the enclosing function to return 'Result<_, E>'` |
| res-err-3 | `fn f() -> Result<i64, i64> { let x: Result<i64, str> = …; x? }` | mismatched error types in `?` |
| flow-err-1 | `let x = 1; x = 2;` | `cannot assign to immutable binding 'x'` + `let mut` help |
| flow-err-2 | `fn f(x: i64) { x = 1; }` | `cannot assign to parameter 'x'` |
| flow-err-3 | `let mut p = (1, 2); p.0 = 3;` | parse error at `.0` (Phase 1, N21) — recorded boundary |
| flow-err-4 | compound assign to immutable: `let x = 1; x += 1;` | immutability error via `+=` |

### 4.4 Traits and patterns

| ID | Source sketch | Expected |
| --- | --- | --- |
| tr-err-1 | `impl Show for Point` missing a method | `missing method 'show' in implementation of trait 'Show'` |
| tr-err-2 | extra method in trait impl | `method 'extra' is not a member of trait 'Show'` |
| tr-err-3 | duplicate/overlapping impls (`Point`, and `impl<T> Show for Vec<T>` + `impl Show for Vec<i64>`) | `conflicting implementations of trait 'Show' for type …` |
| tr-err-4 | unsatisfied bound at instantiation | `the trait bound 'i64: Ord' is not satisfied` (with no impl declared) |
| tr-err-5 | undeclared trait in bound: `fn f<T: Ord>` with no `Ord` declared | `cannot find trait 'Ord' in this scope` |
| tr-err-6 | orphan rule via synthetic registry (FUTURE) | orphan diagnostic |
| tr-err-7 | ambiguous method (two traits with the same method implemented) | `multiple applicable methods in scope` |
| tr-err-8 | no method | `no method 'f' on type 'Point'` (+ suggestion) |
| tr-err-9 | field/method name collision | declaration-side error |
| pl-err-1 | non-exhaustive enum match (missing `Rect`) | `match is not exhaustive` + `add a pattern for 'Shape::Rect(_, _)'` |
| pl-err-2 | non-exhaustive `bool` (`true` only) | witness `false` |
| pl-err-3 | integer literals without wildcard | witness `_` |
| pl-err-4 | duplicate literal arm / wildcard-before-arm | `unreachable pattern` (warning; exit code unchanged by warnings) |
| pl-err-5 | refutable `let Some(x) = opt;` | `refutable pattern in let binding` + `use 'match'` |
| pl-err-6 | refutable `for Some(x) in 0..10 { }` | `refutable pattern in for binding` |
| pl-err-7 | empty match over a non-`Never` scrutinee | non-exhaustive with full constructor list (up to 4) |
| pl-err-8 | error-pattern arm (syntactically recovered) | no exhaustiveness cascade (parse error only) |

## 5. Diagnostic snapshot matrix (required set)

The directive’s required representative diagnostics are covered by
fixtures above and one dedicated snapshot each:

| Required diagnostic | Fixture | Contract |
| --- | --- | --- |
| Undefined name | nm-err-1 | diagnostics §5.1 |
| Duplicate definition | nm-err-2/3 | §5.2 |
| Type mismatch | ty-err-1 | §5.3 |
| Immutable assignment | flow-err-1 | §5.4 |
| Missing trait implementation | tr-err-4 | §5.5 |
| Cyclic import | nm-err-10 (FUTURE) | §5.6 |
| Non-exhaustive match | pl-err-1 | §5.7 |
| Unreachable arm | pl-err-4 | §5.8 |

## 6. Acceptance criteria mapping (future Phase 2 acceptance)

The original Phase 2 acceptance (SPECIFIED §6): *error messages with a
primary span, secondary labels, and a suggestion*; tests: a
`compile-fail/` suite with `.x` + `.stderr`. Proposed audit matrix for the
future implementation directive:

1. `xc check` exists and checks single files end-to-end (exit 0/1/2).
2. The §5 matrix’s eight required diagnostics all render with primary
   span, secondary label(s), and (where applicable) suggestion, pinned by
   snapshots.
3. The `compile-fail/` corpus (this plan’s §4, minus FUTURE entries)
   passes byte-for-byte, platform-independently.
4. Positive corpus (§3) checks clean; HIR snapshots stable.
5. Determinism: repeated runs byte-identical.
6. No panic/stack overflow on the fuzz obligations (§7).
7. Phase 1 suites unaffected.
8. Resource limits produce their fail-closed diagnostics at the exact
   boundaries.

## 7. Structural invariants, security, and performance obligations

### 7.1 Structural invariants (asserted by tests)

- Resolution totality after `check` (`hir-contract.md` §6/§11).
- No desugared syntax leakage into typed HIR.
- HIR validator acceptance for all lowering outputs.
- Type-table totality: every non-`Error` expression has a type.
- Deterministic arena/ID allocation orders.

### 7.2 Safety and performance (Directive 010 §16)

| Risk | Analysis | Bound / mitigation |
| --- | --- | --- |
| Recursive type expansion | Source types are depth-bounded (≤ 64 by parser limits); inference can grow types only by instantiations | `MAX_TYPE_DEPTH = 128`; `MAX_INSTANTIATIONS` per body |
| Occurs-check complexity | Linear in the size of the bound type; no exponential path | `MAX_UNIFY_STEPS`; occurs check before every bind |
| Trait-resolution recursion | Recursive impls (e.g. `impl<T: Show> Show for Vec<T>`) can chain | `MAX_TRAIT_DEPTH = 32` + goal-stack cycle detection; fail-closed |
| Cyclic import detection | DFS over modules, O(V+E), deterministic | registry-phase; single-file vacuous |
| Large symbol tables | O(definitions) maps; no quadratic scans | hashed scopes; interning deferred (`P2D-A4`) |
| Pattern-matrix explosion | Maranget is exponential in adversarial cases | `MAX_PATTERN_CELLS`; fail-closed error; boundary-fuzzed |
| Diagnostic cascades | Poisoning stops dependent checks | cascade tests; one-root-cause fixtures |
| Source-span preservation | Every node/use carries spans; desugared nodes inherit constructs | span-fidelity tests; validator |
| Malicious source input | Huge files, deep nesting, many literals, pathological matches | Phase 1 limits + Phase 2 budgets; no unbounded recursion anywhere; fuzz the full pipeline |

### 7.3 Fuzzing obligations (future)

- Extend the existing combined harness: a `check` target fuzzing
  `lex → parse → lower → check` over arbitrary bytes, asserting: no
  panic, no stack overflow, termination, deterministic diagnostics,
  budgets enforced, `Eof`/span invariants preserved.
- Documentation-level only in this package; no run is claimed.

## 8. What this package does **not** claim

- No fixture in this plan has been executed; none can be executed until
  an implementation directive authorizes WP-A…WP-D.
- No message string is final; shapes are contract, exact text is pinned
  at implementation.
- FUTURE fixtures (registry-dependent) are explicitly out of Phase 2
  acceptance scope.

## 9. Traceability

| Plan item | Source |
| --- | --- |
| `.x` + `.stderr` test form; diagnostics acceptance | SPECIFIED §6 Phase 2 |
| Snapshots (insta), properties (proptest), fuzzing | SPECIFIED §9; accepted Phase 1 practice |
| Positive/negative categories (names, types, traits, mutability, returns, `?`, exhaustiveness) | Directive 010 §15; SPECIFIED §6–§7 |
| Budgets and fail-closed behavior | Directive 010 §16; `P2D-A7` |
| Platform-independent rendering; LF fixtures | accepted Phase 1 CI practice (`architecture.md` §6) |
