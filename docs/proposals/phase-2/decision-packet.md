# XLANG Phase 2 — Decision Packet

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §14 (decision packet),
  §13 (sequencing), §18 (phase status).
- **Purpose:** the actionable approval list for the Phase 2 design
  package. Every row is **PENDING**; none of it is ratified, and none of
  it authorizes implementation.
- **Companion documents:** all files in `docs/proposals/phase-2/`; the
  ratified Phase 1 contracts (`docs/proposals/phase-1/`); the archived
  original (`docs/spec-v0.1-original.txt`).

## 0. How to read this packet

- **Category A — architecture and implementation recommendations:**
  internal design choices (crates, representations, staging, budgets,
  CLI policy). They do not change language semantics; they still require
  owner ratification as the binding Phase 2 contract before any
  implementation directive.
- **Category B — language-semantics decisions: each requires an explicit
  owner decision.** A wholesale Category A approval does not cover them.
  Most fill gaps where the original is silent; two close deferred
  Phase 1 items (`P2D-B11` for B-2, `P2D-B16` for Category C).
- **Category C — deferred items:** listed so the boundary is explicit;
  no action requested now.

Nothing in this packet may be treated as accepted until the owner records
a ratification (the Phase 1 packet is the format precedent). This package
does **not** approve itself (Directive 010 §18).

---

## CATEGORY A — Architecture and implementation recommendations

| ID | Decision | Recommendation | Alternatives considered | Rationale | Risk |
| --- | --- | --- | --- | --- | --- |
| **P2D-A1 — Crate topology** | Add `xl-hir` and `xl-types`; no `xl-driver` in Phase 2; `xl-cli` composes `parse → lower → check` | (i) add `xl-driver` now; (ii) one combined `xl-semantics` crate | The original §4 fixes `xl-hir`/`xl-types`; governance §2 forbids speculative infrastructure; one command needs no driver | Low: driver can be added in Phase 3 without moving logic if the composition stays thin |
| **P2D-A2 — HIR representation and identity** | Flat arenas + newtype ids (`HirId`, `DefId`, `LocalId`, `UseId`, `BodyId`, saturating counters); every node has a span; desugared nodes inherit the originating construct’s span with an internal `Origin` flag; non-recursive drop | (i) recursive `Box` trees like the AST; (ii) `Rc` sharing | SPECIFIED §4 “newtype IDs; data in arenas/Vec”; instance-scoped AST ids cannot be reused; flat storage removes the deep-drop caveat | Low–Medium: more plumbing; pinned by validator and snapshots |
| **P2D-A3 — Two-stage resolution** | Syntactic resolution in `xl-hir`; type-directed member/method resolution in `xl-types`, recorded in side tables (`Resolutions`), HIR immutable after lowering | (i) all resolution in one stage after typing; (ii) resolve members syntactically by name only | Method/field lookup provably needs types; keeping HIR immutable keeps checks pure and MIR-adjacent analysis simple; matches original stage split (resolution+desugar vs type checker) | Medium: the split must be enforced; invariant “every use resolved or explicitly unresolved with a diagnostic” is tested |
| **P2D-A4 — Symbol strategy** | Owned `String` keys in hashed scopes; **no interning** in Phase 2 | (i) `lasso` interner now; (ii) `Rc<str>` names | The accepted lexer/AST deliberately have no interner; interning is a performance optimization, not semantics, and can arrive later (Phase 9) without API-facing change | Low: performance only; keeps zero new dependencies |
| **P2D-A5 — Module registry seam** | `ModuleResolver` interface (path → `Known(ModuleId)`/`Unknown`); Phase 2 CLI supplies a single-module resolver | (i) no interface (hardcode single-file); (ii) file-system convention (`import x;` → `x.x`) | The original routes modules through packages (`xl.toml`, Phase 8); a seam now avoids redesign later without implementing package loading | Low: interface only; no behavior leaks |
| **P2D-A6 — `xc check` CLI contract** | `xc check <file.x>`: run the full pipeline; diagnostics on stderr; **nothing on stdout**; exit `0` no error-level diagnostics / `1` source errors (lexical, syntactic, or semantic) / `2` usage/IO/internal; no new flags | (i) print checksummary on stdout; (ii) separate exit code for semantic vs syntactic errors | Mirrors the accepted `xc parse` policy exactly; stdout stays reserved for future structured output decisions; minimal surface | Low: policy only; pinned by CLI tests |
| **P2D-A7 — Resource budgets and determinism** | Budgets: `MAX_TYPE_DEPTH 128`, `MAX_UNIFY_STEPS 100_000`/body, `MAX_INSTANTIATIONS 10_000`/body, `MAX_TRAIT_DEPTH 32`, `MAX_PATTERN_CELLS 250_000`/match, plus HIR validator depth; **all fail closed** with a specific error; deterministic step accounting; no hash-order dependence anywhere | (i) no caps (trust input size); (ii) warnings instead of errors on exhaustion; (iii) different values | Directive 010 §16 demands explicit bounds; a silently unverified exhaustiveness or skipped unification would be unsound; determinism (INV-007) requires counters, not timeouts | Low: values are trivially adjustable later; boundary tests pin behavior |

---

## CATEGORY B — Language-semantics decisions (explicit per-item approval)

| ID | Question and recommended rule | Alternatives considered | Evidence / rationale | Risk if wrong |
| --- | --- | --- | --- | --- |
| **P2D-B1 — Namespace model** | One flat **module item namespace** (items + import bindings), locals per scope, variants under their enum path, generics per item; lookup order locals → items → prelude (values) and generics → items → prelude (types) | (i) Rust-style split type/value namespaces; (ii) variants in the module namespace | Original silent on namespaces; single namespace is the simplest deterministic model and produces clear duplicate errors; no evidence requires a split | Medium: splitting later is a breaking change for programs that exploit collisions; none exist yet |
| **P2D-B2 — Redeclaration and shadowing** | Duplicates in the same scope are **errors** (items, imports, params, same-scope `let`, fields, variants, methods); inner scopes **shadow** outer scopes freely; locals are visible sequentially (not in their own initializer) | (i) allow same-scope `let` re-binding (Rust-style shadowing); (ii) forbid all shadowing | Sequential visibility is intuitive; re-`let` shadowing is a convenience with no evidence; deterministic either way | Low: fixture nm-err-3 flips to exit 0 under the alternative |
| **P2D-B3 — Nested items** | Block-level items are visible throughout their block (hoisted within it), shadow outer items; nested functions **do not capture** locals; nested `import` binds in the block | (i) treat as module-level; (ii) forbid nested items (contradicts the ratified grammar) | Grammar admits items in blocks (SPECIFIED EBNF; PD-006 §5), semantics were deferred to now; no closures exist in v1, so capture must be rejected | Medium: scoping surprises; pinned by fixtures |
| **P2D-B4 — Prelude contents** | Phase 2 prelude: primitive type names; `Option<T>`/`Some`/`None`; `Result<T,E>`/`Ok`/`Err` (constructors unqualified, matching the example); lowest precedence (shadowable); a path to a zero-payload variant is a value; internal `Range` is not nameable | (i) prelude only in Phase 5 (then `?`/`Result` are unusable in Phase 2); (ii) full Phase 5 prelude now | `?` must be checkable in Phase 2 (SPECIFIED §6 Phase 2) and the example uses `Ok`/`Err` unqualified; nothing else is evidenced | Low–Medium: prelude growth later is additive |
| **P2D-B5 — Pattern classification** | A lone identifier pattern resolves to a **unit-variant constructor** when such a name is in scope, otherwise it introduces a binding; `path(args)` / `path::…` patterns are always constructors | (i) lone identifiers always bind (then `None =>` silently binds — rejected); (ii) require qualification for all constructors (then `None` is unmatchable unqualified) | Follows resolution (Rust-like) and keeps prelude matching natural; limitation (no `@`-binding) documented | Medium: binds a gone-wrong hazard to a documented, testable rule |
| **P2D-B6 — Module path access** | `::` paths may traverse **module bindings** (public items) and enum paths; `.` after a module binding is member access; both forms denote the same item; deeper paths than enum→variant are errors | (i) only `.` (leaves imported types unnameable in type positions); (ii) only `::` (contradicts the accepted `io.println` example) | Required for type positions; `io.println` frozen by Phase 1 correction F-3 keeps `.` valid | Low: additive, both spellings already parse |
| **P2D-B7 — Import resolution in Phase 2** | Single-module mode: imports create bindings resolving to an **opaque module**; unused imports are silent; any *use* through an opaque module is an error (`module '…' is not part of this compilation`); registry + cycle-detection algorithm defined now, exercised when a registry exists | (i) hard error at every import; (ii) implicit sibling-file loading (invented); (iii) full package loading (out of scope) | Keeps checkable files checkable; invents no file-discovery convention; spec routes modules through packages | Medium: Phase 8 will need the registry; the seam is designed |
| **P2D-B8 — `for` lowering and iteration protocol** | Desugar `for` to `loop` + internal `range_next` protocol over `Range<T>` (from `a..b`, integer `T` only); state advanced before the body (so `continue` is correct); no overflow in increment; user-implementable iteration deferred | (i) also iterate arrays now; (ii) expose a user `Iterator` trait now (impossible: no `mut self`, no associated types, no stdlib) | The original mandates `for → loop + iterator` but supplies no iterator surface; the evidenced only iterable is `0..10` | Medium: general iteration arrives Phase 5/6; the internal protocol is the seam |
| **P2D-B9 — `?` semantics** | Lower `e?` to `match e { Ok(v) => v, Err(x) => return Err(x) }` with **exact error-type equality** to the enclosing `Result<_, E>` return type; `Result` only (no `Option?`); no conversions; internal references cannot be shadowed | (i) `From`-style error conversion (invents a trait/stdlib surface); (ii) allow `?` in non-`Result` functions | SPECIFIED: `?` → match; errors as values; no implicit conversions | Medium: error conversion later must be an owner decision |
| **P2D-B10 — Interpolation formatting** | Interpolation lowers to internal concat; parts must be `str` or one of the primitive set (`i8`…`usize`, `f32`/`f64`, `bool`, `char`, `str`) formatted by a fixed rule reusing the accepted printer’s float round-trip rule; user types are errors until Display (Phase 6) | (i) `str`-only (rejects the spec example’s `{total}` intent); (ii) full `Display` now (undefined by the original) | The §3 example interpolates an `f64`; a bounded primitive contract invents the least and reuses an accepted formatting rule | Medium: primitives’ exact text is user-visible; frozen by snapshots once implemented |
| **P2D-B11 — Literal defaulting and representability (closes Phase 1 B-2)** | `IntVar` → integer primitive set, default `i64`; `FloatVar` → `{f32,f64}`, default `f64`; representability checked against the resolved type, with the `-i64::MIN` negation exception; `f32` overflow diagnosed, underflow accepted | (i) default only at `let` (narrower); (ii) no `f32` overflow check | SPECIFIED §7 (“quando ambíguos”); B-2 deferred signed/target checks to this phase | Low: boundaries pinned by fixtures ty-ok-9/ty-err-4 |
| **P2D-B12 — `as` conversions** | Numeric ↔ numeric only, with Rust’s documented `as` value semantics (int→int wrap, int→float round, float→int saturating truncate); `bool`/`char`/`str` casts not introduced | (i) value-preserving casts only (`300 as u8` errors); (ii) permit `bool`/`char` casts | Every permitted pair needs defined value semantics for Phase 4; Rust’s rules are the ecosystem baseline (no invention of *behavior*, only of the permitted table) | Medium: value semantics are user-visible at runtime; documented as a baseline decision |
| **P2D-B13 — Polymorphism restriction** | Function/method items are polymorphic over **declared** generics only (instantiated per use); `let` bindings **never generalize**; no signature inference; no higher-ranked/`dyn`/`impl Trait` | (i) HM-style let-generalization (contradicts “let-polimorfismo restrito”); (ii) infer generic signatures from use | SPECIFIED §7 restricts generalization to `fn`; closures do not exist, so let-generalization buys nothing | Low: relaxing later is additive |
| **P2D-B14 — Traits** | Declaration/impl checking with `Self` as a **contextual type name**; at most one bound per parameter (grammar); unification-based coherence/overlap check; **blanket impls allowed** with solver cycle detection and depth limits; orphan rule implemented and unit-tested against a synthetic registry; trait items implicitly public; impl must provide exactly the trait’s methods (no extras); inherent methods take precedence in selection; ambiguity is an error | (i) reject blanket impls; (ii) parameter-name matching in impls (noise); (iii) allow trait-impl extras (invents extension semantics) | SPECIFIED §7: coherence, orphan rule, no associated types; grammar fixes one bound; `Self` is needed for additive traits | Medium–High: blanket impls stress the solver; visible surface (Self) is new and owner-gated |
| **P2D-B15 — Operators and type parameters** | Operators are built-in on primitives only (arithmetic on same numeric type; comparisons on numeric/`char`; logical on `bool`); **no operator traits**; bounds are declared/checked/solvable but operators on `T` remain errors in Phase 2 with a specific diagnostic | (i) built-in operator traits now (`Ord` for `<` etc.) — invents trait-method semantics; (ii) silently allow and defer checking — unsound | The original defines no operator overloading; inventing trait desugaring would prejudge the Phase 5 traits design | Medium: generic bodies are restricted in Phase 2; clearly documented |
| **P2D-B16 — Control-flow typing (closes Phase 1 Category C)** | `return`/`break`/`continue`: internal `Never` (bottom, unnameable); `while`/`for`/`loop`: `()`; `if` without `else`: `()`; assignment (including compounds): `()`; `?`: payload type; no divergence/reachability analysis beyond match arms | (i) `loop` typed by break values (none exist); (ii) full divergence analysis | Phase 1 deferred exactly this to the type-system phase (ratified Category C); a `Never` bottom is the minimal standard mechanism | Low–Medium: `Never` is internal; no user syntax changes later |
| **P2D-B17 — Mutability model** | Mutability is per-binding (`let mut` covers all bindings of its pattern); parameters and the receiver are immutable; assignment requires the **root** to be a mutable local; fields/elements inherit; diagnostics with `let mut` help | (i) per-field mutability; (ii) `mut self` receivers now (not in grammar, deferred) | SPECIFIED §7: `let` immutable, assignment requires `mut`; grammar has no `mut self` | Low: root rule is restrictive but predictable |
| **P2D-B18 — Arrays and slices** | `[T; N]`: `N` must be a non-negative integer literal (no const evaluation); `[T]` nameable but no expression has slice type in Phase 2; indexing only on arrays with integer index (no static bounds check); no `str`/tuple indexing; array literals infer `[T; N]` | (i) full const-expression evaluation of `N`; (ii) array→slice coercion (implicit conversion, rejected); (iii) forbidding slices entirely | Original lists both array forms but defines no operations; minimal, explicit subset | Low: growth is additive; coercion ban is deliberate |
| **P2D-B19 — Match severity and fail-closed limits** | Non-exhaustive match: **error** (SPECIFIED); unreachable arm: **warning**; exhaustiveness resource exhaustion: **error** (fail closed); malformed-arm matches: analysis skipped silently | (i) unreachable arm as error; (ii) resource exhaustion as warning | Original fixes only “não exaustivo é erro”; a warning for reachability matches common practice; fail-closed protects soundness | Low: severity is visible in fixtures; owner may choose stricter |
| **P2D-B20 — Visibility semantics** | `pub` recorded per item/field/method; private-by-default; enforcement active at module boundaries (vacuous single-file; opacity enforced through opaque modules; registry-ready); trait items implicitly public; trait-impl methods public for dispatch | (i) enforce nothing until Phase 8; (ii) require `pub` on trait-impl methods (noise) | SPECIFIED §6 Phase 5 (“módulos e pub”) moves full module semantics late; recording + boundary rule now avoids a later retrofit | Low: single-file behavior effectively unchanged |
| **P2D-B21 — PD-009/PD-010 policy recommendations** | PD-010: keep the plain-text canonical rendering; **defer** stable codes (reserve the shape; additive `with_code` later); no color in `render`; no wrapping; PD-010 stays OPEN with this recommendation recorded. PD-009: all crates internal (`publish = false`, no external stability promise) until an owner stabilization decision; `#[non_exhaustive]` recommended **at** stabilization; CLI output not an external contract | (i) introduce codes now (changes every snapshot before the owner chooses a taxonomy); (ii) promise external API stability now (premature) | Directive 010 §11/§12 require reasoned recommendations without silent resolution; `xl-diag` compatibility is preserved by making nothing mandatory now | Low: both remain open owner decisions |
| **P2D-B22 — Refutability of `let`/`for` patterns** | `let` and `for` patterns must be **irrefutable** for the target type; refutable patterns are errors with `use 'match'` help; checked by the exhaustiveness machinery | (i) defer to runtime panics (no panic semantics exist for this); (ii) allow with warnings | Without this rule, `let Some(x) = opt;` would have undefined behavior; the cheapest correct rule reuses the mandated algorithm | Low: standard; pinned by fixtures |

---

## CATEGORY C — Deferred (no action requested)

Listed so the boundary is explicit; each re-opening requires a future
owner decision (and, where it changes the language, a specification-level
event per `docs/governance.md` §10).

- **C1 — Package management / `xl.toml` / multi-file loading** (Phase 8;
  registry seam `P2D-A5` ready).
- **C2 — User-visible `Iterator`/`IntoIterator` and general `for`
  iteration** (Phase 5/6; the internal protocol `P2D-B8` is the seam).
- **C3 — `Display`/`Debug` and user-type formatting** (Phase 6;
  interpolation is primitive-only until then, `P2D-B10`).
- **C4 — `?` error conversion (`From`-style)** (needs stdlib + traits).
- **C5 — Associated types, associated consts, trait default bodies**
  (v1.1 per SPECIFIED §7; not in Phase 2).
- **C6 — Operator overloading / operator traits** (`P2D-B15`
  alternative).
- **C7 — Struct-literal construction (G-5)** (remains UNRESOLVED from
  Phase 1).
- **C8 — Receiver mutability (`mut self`), method `Self`-by-mut** (not in
  the accepted grammar).
- **C9 — Carried Phase 1 Category C surface:** `break` with a value,
  loop values, open-ended ranges, `..=`, bitwise/shift operators,
  attributes, turbofish, `pub` re-exports, match guards, or-patterns,
  negative literal patterns, struct-like enum variants, `dyn`/`impl
  Trait`, `Self` beyond contextual use, Unicode identifier normalization.
- **C10 — Closures; concurrency; borrow checker; unsafe** (SPECIFIED
  non-goals/Phase 9+).
- **C11 — Associated items on types (`Type::method`), module-level
  constants, statics** (no evidence).

---

## Approval mechanics

- **Category A rows:** approve as proposed / approve with amendment /
  reject (per row or wholesale).
- **Category B rows:** **explicit per-row decision required**; a
  wholesale Category A approval does not cover them.
- **Category C:** no action; recorded boundary only.
- On approval: this packet gains a ratification record (owner directive
  reference, date); the corresponding documents change status from
  PROPOSED to ACCEPTED; any resulting ADR is drafted in a subsequent
  owner-authorized step (the Phase 1 precedent: proposal set → owner
  directive → ADR-0004).
- **Implementation remains unauthorized** until a separate directive
  names a work package (`architecture-rfc.md` §6) and its predecessor
  gate is satisfied.
- This packet does not resolve PD-009 or PD-010; `P2D-B21` records
  recommendations only.

## Dependency/ordering note (Directive 010 §13)

The recommended first implementation unit after ratification is **WP-A
(HIR foundation)**, then WP-B (name resolution), then WP-C (types), then
WP-D (CLI). If the owner wishes to reduce risk before the full semantic
surface, the packet’s reviewable split points are: (a) WP-A+WP-B alone
(hir, resolution, diagnostics — no types), and (b) the internal WP-C
milestones C1/C2/C3 (`architecture-rfc.md` §6). Both splits are
compatible with the proposed dependency order; neither is authorized by
this package.

## Traceability

| Packet area | Sources |
| --- | --- |
| A1–A7 | SPECIFIED §4 conventions/layout; governance §2/§6; Directive 010 §12–§16 |
| B1–B7 | SPECIFIED §2/§3/§6 Phase 2; ratified PD-006; Phase 1 corrections |
| B8–B10 | SPECIFIED §6 Phase 2 desugaring mandate; §3 example; ratified PD-004/B-3 |
| B11–B13 | SPECIFIED §7; ratified B-2 |
| B14–B15 | SPECIFIED §7; grammar-rfc §3.1 |
| B16–B18 | ratified Category C deferral; SPECIFIED §3 type list |
| B19–B22 | SPECIFIED §6–§7; Directive 010 §10–§11 |
| C1–C11 | SPECIFIED §6 phases 5–9; §1 non-goals; ratified Phase 1 Category C |
