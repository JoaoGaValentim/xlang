# XLANG Phase 2 — Diagnostics Contract

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §11 (diagnostic contract),
  §12 (PD-009 review).
- **Authoritative sources:** SPECIFIED §1 (diagnostics quality is an
  objective) and §6 Phase 2 (acceptance: primary span, secondary labels,
  suggestion); the accepted `xl-diag` crate; the accepted Phase 1
  diagnostic texts; `docs/pending-decisions.md` PD-010 (open).
- **Companion documents:** `name-resolution-contract.md` (suggestion
  mechanics for names), `type-system-contract.md` (poisoning),
  `traits-and-exhaustiveness.md` (trait/pattern messages),
  `conformance-plan.md` (§5 compile-fail fixtures),
  `decision-packet.md` (`P2D-B21`, `P2D-A7`).
- **No implementation is authorized.** All messages below are PROPOSED
  shapes; exact final strings are pinned at implementation by snapshot
  tests. Nothing here claims that any message exists today.

## 1. Compatibility with the accepted `xl-diag`

The accepted Phase 0/1 model is binding input for this contract:

- `Severity { Error, Warning, Note }`; `Label { span, message }`;
  `Diagnostic` with severity, message, primary label, and secondary
  labels; `render(&Diagnostic, &SourceMap) -> String` — deterministic
  plain-text rendering with file location, source line, caret highlight,
  and labels.
- Diagnostics are values; no panics; unresolvable spans render an explicit
  placeholder (`architecture.md` §3, `xl-diag`).
- **No `xl-diag` API change is proposed by this package.** Phase 2 adds
  *construction sites* (messages, labels, severities), not new rendering
  machinery. Any API change would require an explicit proposal and owner
  authorization (this section is that statement, per Directive 010 §11).

## 2. Required diagnostic properties (SPECIFIED §6 Phase 2 acceptance)

Every Phase 2 (WP-B/WP-C) diagnostic must provide:

1. **A primary span** at the construct the user must change or inspect.
2. **Secondary labels** where they materially help: the declaration site
   of a conflicting name, the origin of an expected type, the trait bound
   declaration, the earlier arm covering a pattern, the import a
   cyclic-cycle edge closes on, etc.
3. **A useful explanation** — the message states the conflict in terms of
   source concepts (names, types, traits), not compiler internals.
4. **A corrective suggestion when appropriate** — phrased as a concrete
   action (`did you mean 'y'?`, `help: declare it with 'let mut'`,
   `help: add a pattern for 'Shape::Rect(_, _)'`). Suggestions are only
   emitted when the checker can justify them (see §4); no speculative
   advice.

## 3. Diagnostic ordering and ownership

- Exactly one stage owns each diagnostic (`architecture-rfc.md` §4):
  lexical/syntactic (Phase 1, unchanged) → lowering/resolution (WP-A/B) →
  typing/traits/patterns (WP-C).
- Final render order is pipeline order, and within a stage, deterministic
  traversal order (source order; see each contract §“deterministic”
  sections). No sorting by span is applied by the CLI in Phase 2 (the
  emission order is already source-ordered by construction).
- Cascade suppression: a construct poisons to `Error` after its first
  diagnostic; dependent checks are silent (`type-system-contract.md`
  §15). This is what makes the required “complete matrix” of errors
  *readable*: one root cause, one message.

## 4. Suggestion engine rules (shared)

Used for names, members, and patterns:

- Candidate sets come only from the relevant scope chain/namespace
  (never from other modules’ private items; never fabricated).
- Similarity: bounded edit distance; thresholds — distance ≤ 2 for names
  longer than 4 characters, ≤ 1 for shorter names (case-sensitive;
  deterministic). Ties are ordered by edit distance, then declaration
  order (source order), then lexicographic name as the final key.
- At most three candidates are listed (first one in the `did you mean`
  label; up to two more as a note when they tie).
- Suggestions never replace the error: the primary diagnostic stands
  alone in the compile-fail snapshot; the suggestion is part of its
  rendered text and therefore snapshot-pinned.

## 5. Representative diagnostics (SPECIFIED required set + core extras)

Shapes below fix: severity, primary span, secondary labels, message
pattern, suggestion. `<name>`, `<type>` etc. are placeholder
interpolations; final punctuation and quoting are pinned by snapshots at
implementation.

### 5.1 Undefined name (required)

```text
error: cannot find value 'cafe' in this scope
  --> test.x:3:13
   |
 3 | let x = cafe + 1;
   |         ^^^^ did you mean 'café'?
```

- Primary: the unresolved use span. Secondary: none required.
- Suggestion: nearest in-scope candidate per §4; omitted when none is
  close (message alone: `cannot find value 'cafe' in this scope`).
- Variants: `cannot find type 'T' in this scope`, `cannot find trait
  'Sow' in this scope`.

### 5.2 Duplicate definition (required)

```text
error: duplicate definition of 'area'
  --> test.x:7:4
   |
 4 | fn area(s: Shape) -> f64 { 0.0 }
   |    ---- first defined here
 7 | fn area(x: i64) -> i64 { x }
   |    ^^^^ duplicate definition
```

- Primary: the second definition’s name span. Secondary: the first
  definition’s name span (`first defined here`).
- Scope variants: `the name 'x' is already defined in this scope`
  (locals, same scope), `duplicate method 'm' on type 'Point'`,
  `field 'x' is defined multiple times`.

### 5.3 Type mismatch (required)

```text
error: mismatched types: expected 'i64', found 'str'
  --> test.x:5:16
   |
 5 |     let n: i64 = "text";
   |            ---   ^^^^^^ expected 'i64', found 'str'
   |            |
   |            expected due to this annotation
```

- Primary: the expression span. Secondary: the span that produced the
  expectation (annotation, parameter, return type, other branch) with
  `expected due to this` / `expected because of this`.
- Variants (same shape): return mismatch (`expected 'i64', found '()'`
  at the block), argument mismatch, branch mismatch, operator operand
  mismatch, `?` mismatches, `if` condition not `bool`.

### 5.4 Invalid assignment to an immutable binding (required)

```text
error: cannot assign to immutable binding 'x'
  --> test.x:4:5
   |
 3 |     let x = 0;
   |         - help: declare it as 'let mut x'
 4 |     x = 1;
   |     ^ cannot assign to immutable binding 'x'
```

- Primary: the assignment target span. Secondary: the binding’s
  declaration (`help:` label). Variants: `cannot assign to parameter
  'o'`; `cannot assign to a call result` (defensive).

### 5.5 Missing trait implementation (required)

```text
error: the trait 'Show' is not implemented for 'Point'
  --> test.x:9:16
   |
 9 |     let s = show(p);
   |                - ^ required by this bound
   |                |
   |                help: implement 'Show' for 'Point'
```

- Primary: the use/argument span. Secondary: the bound declaration
  (`required by this bound`). Suggestion: `help: implement 'Show' for
  'Point'` (only when the orphan rule permits it in the current module —
  otherwise no suggestion is emitted).
- Variant: `the trait bound 'i64: Ord' is not satisfied`.

### 5.6 Cyclic import (required; future registry)

```text
error: cyclic module import
  --> a.x:1:1
   |
 1 | import c;
   | ^^^^^^^^^^ this import closes the cycle
   |
   = note: cycle: a → b → c → a
```

- Primary: the import that closes the cycle. Secondary (when module
  declaration sites are known): the module declaration spans. Note: the
  full cycle path, rendered from the DFS stack
  (`name-resolution-contract.md` §9.4).
- In Phase 2 single-file mode this diagnostic is unreachable; its shape
  is fixed now and its fixtures are edge-case documentation until the
  registry phase.

### 5.7 Non-exhaustive match (required)

```text
error: match is not exhaustive
  --> test.x:4:5
   |
 4 |     match s {
   |     ^^^^^ missing pattern: 'Shape::Rect(_, _)'
   |
   = help: add a pattern for 'Shape::Rect(_, _)'
```

- Primary: the `match` scrutinee/keyword span. Suggestion: witness
  pattern(s) per `traits-and-exhaustiveness.md` §6.1 (up to 4, with
  `(and N more)`).

### 5.8 Unreachable match arm (required)

```text
warning: unreachable pattern
  --> test.x:6:9
   |
 5 |         1 => "a",
   |         - this pattern covers all values it could match
 6 |         1 => "b",
   |         ^ unreachable pattern
```

- Severity warning (proposal; `P2D-B19`). Primary: the redundant arm’s
  pattern. Secondary: the covering arm (`this pattern covers …`).

### 5.9 Additional core shapes (not exhaustive)

| Situation | Message (shape) | Primary | Suggestion |
| --- | --- | --- | --- |
| `?` on non-`Result` | `the '?' operator requires a 'Result<T, E>' operand, found '<T>'` | the `?` construct | — |
| `?` in non-`Result` function | `the '?' operator requires the enclosing function to return 'Result<_, E>'` | the `?` construct (+ secondary: fn signature) | — |
| `?` error-type mismatch | `mismatched error types in '?': expected '<E1>', found '<E>'` | the `?` construct | — |
| unformattable interpolation | `cannot format value of type 'Point' in a string literal` | the interpolated expression | note: formatting for user types arrives with the standard library |
| for over non-iterable | `cannot iterate over value of type '<T>'` | the iterable expression | — |
| refutable `let` | `refutable pattern in let binding` | the pattern | `help: use 'match'` |
| refutable `for` pattern | `refutable pattern in for binding` | the pattern | `help: use 'match'` |
| array length invalid | `array length must be a non-negative integer literal` | the length expression | — |
| literal out of range | `literal '<300>' does not fit in 'i8'` | the literal | — |
| unsupported cast | `cannot cast '<bool>' to '<f64>'` | the cast expression | — |
| operator on type parameter | `operators on type parameters are not supported in this phase` | the operator | note: bounds are checked but operator traits are a later phase |
| no field | `no field 'z' on type 'Point'` | the member name | `did you mean 'x'?` |
| no method | `no method 'f' on type 'Point'` | the member name | `did you mean 'dist'?` |
| ambiguous method | `multiple applicable methods in scope` | the method name | note listing candidates in order |
| wrong arity | `this function takes 2 arguments but 1 was supplied` | the call/argument list | — |
| not callable | `cannot call value of type 'i64'` | the callee | — |
| not indexable | `cannot index into a value of type 'i64'` | the index expression | — |
| private item | `item 'f' is private` | the use | — (future registry) |
| conflicting impls | `conflicting implementations of trait 'Show' for type 'Point'` | the later impl | — |
| unresolved through opaque module | `module 'std.io' is not part of this compilation` | the member use | note: single-file checking only in this phase |
| cannot infer | `cannot infer type; add a type annotation` | the binding/expression | help: annotation position |
| cyclic type | `cyclic type` | the unifying expression | — |
| resource limit | `checking resource limit exceeded in '<stage>'` | the construct | — (fail closed) |

## 6. Where messages are pinned

- Every message above becomes an exact string at implementation time and
  is pinned by snapshot tests (`insta`) plus CLI `.stderr` fixtures.
- Phase 1 message texts are untouched: the accepted lexer/parser
  diagnostics keep byte-identical strings (regression).
- `xc check` renders diagnostics through `xl_diag::render` exactly like
  `xc parse`; no second renderer may exist.

## 7. PD-010 review — recommendation (P2D-B21)

PD-010 asks: should diagnostics carry stable error codes (e.g. `E0308`
style)? What exact rendered format is canonical (color, terminal width)?
Current status: PARTIALLY DEFINED — the Phase 0 plain-text format is a
repository choice, not a specification claim; **this package does not
silently resolve PD-010**.

### 7.1 Recommendation

- **Stable codes: adopt the *position*, defer the *format*.** Structured,
  stable identifiers are valuable for tests, documentation, and tooling;
  the natural shape is a `Code` newtype (e.g. `XL2-NM01` for
  name-resolution/undefined, `XL2-TY04` for type mismatch — category +
  stable ordinal), attached through an *additive* `Diagnostic` builder
  (`with_code`) so rendering can opt in later. However:
  - the choice of a code taxonomy and its stability promise is an owner
    decision (it becomes user-visible API);
  - introducing codes now would change every snapshot and every Phase 2
    message contract before the owner has chosen the scheme.
  - **Therefore:** Phase 2 implementation may precede codes; the
    diagnostic contract reserves the field in its data shapes
    (messages are constructed through builders), and PD-010 remains open
    with this recommendation recorded.
- **Rendering: keep the accepted plain-text format canonical.** No color
  in the canonical rendering (`render` output), because (a) Phase 1
  snapshots and cross-platform tests depend on it, (b) deterministic
  output is a stated invariant (INV-007), (c) color involves terminal
  behavior the specification never mentions. A future opt-in colorizer
  (CLI-level, never in `render`) would be a separate owner decision.
- **Terminal width: no wrapping.** Long lines print in full (accepted
  Phase 1 limitation). Wrapping heuristics are environment-dependent and
  would break snapshot determinism; if added later it must be an explicit
  CLI presentation option, not part of the canonical renderer.
- **Display width:** the accepted scalar-value column convention and its
  wide-character/tab caveat are unchanged (ADR-0002; PD-010 tracks the
  residual display-width question only).
- **Multi-line spans:** the accepted renderer highlights the first line
  only; Phase 2 diagnostics therefore keep primary spans on single-line
  constructs wherever reachable (all spans come from tokens/paths, which
  are single-line except block spans used as secondary labels on
  single-line boundaries in fixtures). Any change to multi-line rendering
  is an `xl-diag` change requiring authorization.

### 7.2 PD-010 status after this package

- **Still open** (no owner decision recorded). The package records a
  reasoned recommendation (§7.1) and the outstanding questions:
  (a) code taxonomy and stability promise; (b) opt-in color policy;
  (c) display-width handling; (d) any multi-line rendering change.
- **No `xl-diag` API change is needed for Phase 2** under this
  recommendation; if the owner chooses codes, the only change is an
  additive builder + render prefix, proposed separately.

## 8. PD-009 review — public API compatibility (P2D-B21)

PD-009 (public API compatibility policy) is UNRESOLVED. Recommendation
recorded here for the owner; **not resolved by this package**:

- **Internal by default.** All current crates are `publish = false`
  (workspace `[workspace.package]`), and `xl-hir`/`xl-types` will inherit
  that. Until XLANG 1.0, every crate is an **internal implementation
  detail**: no external compatibility promise, breaking changes allowed
  in any authorized phase.
- **Which APIs might eventually be published:** candidates are the
  tool-facing surfaces (`xl-span`, `xl-diag`, the AST, and the LSP),
  *if and only if* the owner later decides to support external consumers
  (e.g. editor tooling). Compiler-internal crates (`xl-hir`,
  `xl-types`, `xl-mir`, …) are expected to remain internal.
- **Compatibility expectations before 1.0:** none promised; changes
  coordinated per phase directive. Governance already implies this
  (`governance.md` §1, §7).
- **Versioning:** workspace version `0.1.0` for all crates; no semantic
  versioning promise until a stabilization decision.
- **`#[non_exhaustive]`:** recommended *at stabilization* for public
  enums that may grow (`SpanError`, `Severity`, diagnostic types,
  HIR/AST node enums if published); not applied now (it would not be
  meaningful while `publish = false` and would add noise to internal
  pattern matches). Recorded as an owner decision.
- **AST/HIR stability:** neither is a public contract today; the AST is
  documented as a Phase 1 artifact, HIR is internal by design.
  `xc parse` output is a debugging artifact (accepted Phase 1 statement);
  it is *grammar-stable* (it re-parses) but not promised as a
  byte-stable external format.
- **CLI output formats:** `xc parse`/`xc check` stdout/stderr are
  test-pinned inside the repository but **not** promised as stable
  external interfaces; any stabilization is a separate owner decision
  (aligned with PD-010 for diagnostics).

## 9. Test strategy for diagnostics (future)

1. **Snapshot tests** for every representative diagnostic (message +
   labels + suggestion) on synthetic inputs — exact strings.
2. **CLI compile-fail fixtures:** `tests/compile-fail/<case>.x` +
   `<case>.stderr`, compared byte-for-byte after line-ending
   normalization (LF canonical; CRLF tolerated per accepted Phase 1 CI
   practice), with no absolute paths in output (the CLI prints the file
   argument as given; fixtures use bare file names).
3. **Platform independence:** no locale, no environment, no terminal
   detection in messages; I/O error text (operational, not source
   diagnostics) remains platform-specific and is out of fixtures.
4. **Determinism tests:** repeated runs byte-identical; diagnostic order
   asserted exactly.
5. **Cascade tests:** one-root-cause cases assert exactly one error.
6. **Phase 1 regression:** existing lexer/parser/CLI snapshots unchanged.

## 10. Traceability

| Contract item | Source |
| --- | --- |
| Primary span + secondary labels + suggestion (acceptance) | SPECIFIED §6 Phase 2 |
| Diagnostics quality as a requirement from Phase 2 | SPECIFIED §9–§10 |
| `xl-diag` model and renderer compatibility | accepted Phase 0 implementation |
| One-diagnostic-per-cause; no panics | SPECIFIED §4; Phase 1 recovery discipline |
| PD-010 open; recommendation recorded | `docs/pending-decisions.md` (not resolved) |
| PD-009 open; recommendation recorded | `docs/pending-decisions.md` (not resolved) |
| Snapshot + `.stderr` test plan | SPECIFIED §6 Phase 2, §9 |
