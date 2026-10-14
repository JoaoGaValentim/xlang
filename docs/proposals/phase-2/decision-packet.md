# XLANG Phase 2 — Decision Packet

**STATUS: OWNER-RATIFIED (Directive 010-R3) — PENDING INTEGRATION (PR #7 not merged)**

- **Directive:** Engineering Directive 010 §14 (decision packet),
  §13 (sequencing), §18 (phase status); **ratified by:** Engineering
  Directive 010-R3 (Phase 2 contract ratification), recorded in §13 of
  this packet.
- **Purpose:** the actionable approval list for the Phase 2 design
  package. **The individual decisions are now owner-ratified** (Directive
  010-R3, §13): Category A A1–A8 and Category B B1–B20/B22. Category C
  remains deferred; PD-009 and PD-010 remain OPEN. **Ratification is not
  integration:** the contracts are binding but not yet merged into `main`
  until PR #7 is merged and its post-merge CI passes, and ratification
  does **not** authorize any implementation.
- **Companion documents:** all files in `docs/proposals/phase-2/`; the
  ratified Phase 1 contracts (`docs/proposals/phase-1/`); the archived
  original (`docs/spec-v0.1-original.txt`); `docs/decisions/adr-0005-phase-2-semantic-architecture.md`.

## 0. How to read this packet

- **Category A — architecture and implementation recommendations**
  (A1–A8): internal design choices (crates, representations, staging,
  interfaces, budgets, CLI policy, and the interim diagnostic/API
  policy). They do not change language semantics. **All eight are
  owner-ratified by Directive 010-R3** (A1 and A5 with the interface
  amendment recorded in §4 of that directive and in §13 below).
- **Category B — language-semantics decisions (B1–B20, B22): each
  requires an explicit owner decision.** A wholesale Category A approval
  does not cover them. **All 21 are individually owner-ratified by
  Directive 010-R3** (§13 below). Most fill gaps where the original is
  silent; two close deferred Phase 1 items (`P2D-B11` for B-2,
  `P2D-B16` for Category C). `P2D-B21` was reclassified to `P2D-A8` (it
  holds compatibility/diagnostic policy, not language semantics) and is
  not a Category B row; see the redirect note after the Category B table.
- **Category C — deferred items (C1–C11):** **remain DEFERRED** (no
  change); listed so the boundary is explicit.

The owner recorded the ratification via Engineering Directive 010-R3;
the record is §13. **Ratification is not self-approval and not
integration:** the contracts are binding but not merged into `main` until
PR #7 is merged with post-merge CI, and no implementation is authorized.

---

## CATEGORY A — Architecture and implementation recommendations

| ID | Decision | Recommendation | Alternatives considered | Rationale | Risk |
| --- | --- | --- | --- | --- | --- |
| **P2D-A1 — Crate topology and stage interface** *(owner: APPROVED WITH AMENDMENT, Directive 010-R3)* | Add `xl-hir` and `xl-types`; no `xl-driver` in Phase 2. **One internally consistent interface contract**: `xl_parser::parse_file(&SourceMap, FileId) -> Result<ParseOutput, SpanError>`; **lowering receives AST items, not `ParseOutput`** — `xl_hir::lower(&[Item], FileId, &ModuleRegistry) -> LowerOutput { module: HirModule, resolutions: Resolutions, import_edges: Vec<ImportEdge>, diagnostics: Vec<Diagnostic>, fate: StageFate }` (so `xl-hir` does **not** depend on `xl-parser`); the module graph is validated by a **separate** entry point `xl_hir::validate_module_graph(&ModuleRegistry) -> Vec<Diagnostic>` invoked by the caller **after** the returned edges are registered; `xl_types::check(&HirModule, &Resolutions) -> CheckOutput { diagnostics, resolutions, fate: StageFate }`. `xl-cli` owns the `SourceMap` and the mutable `ModuleRegistry`, registers modules before lowering, registers `import_edges` after lowering, then runs graph validation, then `check`. `lower` owns syntactic name resolution; `check` owns type-directed member/method resolution. | (i) add `xl-driver` now; (ii) one combined `xl-semantics` crate; (iii) `lower` takes `ParseOutput` (rejected: would force `xl-hir → xl-parser`); (iv) graph validation inside `lower` (rejected, Directive 010-R3 §4: the caller has not yet registered the discovered edges) | The original §4 fixes `xl-hir`/`xl-types`; governance §2 forbids speculative infrastructure; one command needs no driver; a single signature set removes the A1/A3 drift; graph validation must run after edge registration | Low: driver can be added in Phase 3 without moving logic if the composition stays thin |
| **P2D-A2 — HIR representation, identity, and source origin** | Flat arenas + newtype ids (`HirId`, `DefId`, `UseId`, `BodyId`, `TypeRefId`; `LocalId` is a documented `DefId` specialization, not a separate identity space) with **checked** allocation — ids never saturate, wrap, or alias; `u32` exhaustion is a controlled internal error that leaves the partial result **unusable for a successful check**. Every node has a span. Definitions from different modules cannot alias: composite `GlobalDefId { module: ModuleId, definition: DefId }` (`ModuleId` carries its `PackageId`). Desugared nodes inherit the originating construct’s span, carry an internal `Origin::Desugared` flag, and record an `AstOrigin`. **Source origin is accurate**: nodes that actually have an AST id use it (`ItemId`/`ExprId` — the accepted AST assigns ids only to items and expressions, verified in `xl-ast::id`); statements, patterns, and types (which have **no** AST id) use a deterministic **structural occurrence key** (anchor id + child index path); **synthesized nodes use `Synthesized { construct, expansion, occurrence }`**, a unique occurrence identity because one desugaring produces many nodes. `AstOrigin` is a *shared reference* (desugaring is one-to-many); `HirId`/`(AstOrigin, OccurrenceKey)` is the *unique* node identity. Spans are never used as identity keys (grouping is transparent; recovered placeholders share spans). Non-recursive drop. | (i) recursive `Box` trees like the AST; (ii) `Rc` sharing; (iii) saturating counters (rejected: could give two entities one identity); (iv) spans as keys (rejected: not unique); (v) `Synthesized { construct, span }` alone (rejected: aliases many nodes) | SPECIFIED §4 “newtype IDs; data in arenas/Vec”; instance-scoped AST ids cannot be reused and do not exist for every node kind; desugaring is inherently one-to-many; flat storage removes the deep-drop caveat | Low–Medium: more plumbing; pinned by validator, origin/synthesized-uniqueness tests, and overflow tests |
| **P2D-A3 — Two-stage resolution (immutable HIR)** | Syntactic resolution (locals, items, paths, variants, prelude, imports/module bindings) is part of `xl_hir::lower`. Type-directed member/method resolution runs in `xl_types::check` and returns its own `Resolutions` side table; `HirModule` is immutable throughout (no HIR rewrite, no interior mutability). `check` is a pure function of `(&HirModule, &Resolutions)`; lexical/syntactic diagnostics stay owned by Phase 1 and are only rendered first; spans travel in nodes and the CLI holds the `SourceMap` for rendering. Poisoned nodes (`Error`) propagate by typing (never by mutation) and suppress cascades. | (i) all resolution in one stage after typing; (ii) resolve members syntactically by name only; (iii) mutate HIR in place (rejected) | Method/field lookup provably needs types; immutable HIR keeps checks pure and MIR-adjacent analysis simple; matches original stage split (resolution+desugar vs type checker) | Medium: the split must be enforced; invariant “every use resolved or explicitly unresolved with a diagnostic” is tested |
| **P2D-A4 — Symbol strategy** | Owned `String` keys in hashed scopes; **no interning** in Phase 2 | (i) `lasso` interner now; (ii) `Rc<str>` names | The accepted lexer/AST deliberately have no interner; interning is a performance optimization, not semantics, and can arrive later (Phase 9) without API-facing change | Low: performance only; keeps zero new dependencies |
| **P2D-A5 — Module registry, module identity, and cyclic imports** *(owner: APPROVED WITH AMENDMENT, Directive 010-R3)* | A caller-provided, **in-memory** `ModuleRegistry` (owned by the CLI; never a mutable global): stable `ModuleId`s; explicit registration of \*multiple\* modules (dotted path + optional `FileId`); recorded **import edges with spans** (`ImportEdge { from, path, span, target }`, `ImportTarget::{Known, Unavailable, Missing}`); **lookup** (`resolve`/`surface`/`package_of`) **and deterministic graph enumeration** (`modules()`/`imports_of()`); **construction** (`register`/`record_import`). **Amendment:** `lower` receives the registry **immutably** and **returns** its discovered edges in `LowerOutput.import_edges`; the caller `record_import`s them **after** lowering and only then invokes `validate_module_graph` (`lowering-contract.md` §1.1). No edges discovered during lowering are omitted from cycle detection, and a shared registry is never mutated through an immutable reference. **Deterministic cycle detection with a reproducible witness path, primary/secondary spans, duplicate-cycle suppression, safe handling of missing/unavailable modules, and explicit resource limits** — fully executable in Phase 2 against synthetic registries (mg-1…mg-9 feasible with `register`/`record_import` alone). A `PackageId` groups modules; `GlobalDefId` names definitions across modules. No `xl.toml`, filesystem discovery, network, interior mutability, global state, or Phase 8 package loading. | (i) no interface (hardcode single-file); (ii) file-system convention (`import x;` → `x.x`, invented); (iii) postpone cycle detection to a later registry phase (rejected by Directive 010-R1 §3: the original requires it in Phase 2); (iv) lookup-only interface with no graph enumeration (rejected by Directive 010-R2 §3); (v) graph validation inside `lower` against a mutated shared registry (rejected by Directive 010-R3 §4) | The original requires cyclic-import detection and correct import resolution in Phase 2 (§6); a synthetic in-memory registry satisfies it without package loading; the original routes real modules through packages (`xl.toml`, Phase 8) | Low: interface + in-memory graph only; no behavior leaks outside `xl-hir` |
| **P2D-A6 — `xc check` CLI contract** | `xc check <file.x>`: run the full pipeline; diagnostics on stderr; **nothing on stdout**; exit `0` no error-level diagnostics and every stage `Complete` / `1` source errors (lexical, syntactic, or semantic, including a resource-budget failure or an unresolved import) / `2` usage/IO/internal (including id-space exhaustion / aborted stage); no new flags | (i) print check summary on stdout; (ii) separate exit code for semantic vs syntactic errors | Mirrors the accepted `xc parse` policy exactly; stdout stays reserved for future structured output decisions; minimal surface | Low: policy only; pinned by CLI tests |
| **P2D-A7 — Resource budgets and determinism** | Finite, explicit, fail-closed budgets (all deterministic counters; no wall-clock timeouts): **HIR** — `MAX_HIR_DEPTH = 256` (max nested block/expr/pat/type chain; accepted source depth ≤ `MAX_PARSE_DEPTH = 64`; each desugaring adds ≤ 4 levels, so 256 is finite and generous), `MAX_HIR_NODES = 1_000_000`/module (AST→HIR expansion), `MAX_DEFS = 100_000`/module, `MAX_USES = 1_000_000`/module; **module graph** — `MAX_MODULE_GRAPH_MODULES = 1_024`, `MAX_MODULE_GRAPH_EDGES = 16_384`, `MAX_MODULE_GRAPH_DEPTH = 256`; **types** — `MAX_TYPE_DEPTH = 128`, `MAX_UNIFY_STEPS = 100_000`/body, `MAX_INSTANTIATIONS = 10_000`/body, `MAX_TRAIT_DEPTH = 32`, `MAX_PATTERN_CELLS = 250_000`/match. Id space is `u32` with **checked** allocation (`P2D-A2`). Every budget failure is a controlled diagnostic (fail closed); exceeding a source-reachable budget is exit `1`; id-space exhaustion is a controlled internal error (exit `2`); neither ever yields a successful check; no hash-order dependence anywhere. | (i) no caps (trust input size); (ii) warnings instead of errors on exhaustion; (iii) wall-clock timeouts (rejected: non-deterministic); (iv) saturating ids (rejected) | Directive 010 §16 and Directive 010-R1 §8 demand explicit bounds and deterministic counters; a silently unverified exhaustiveness or skipped unification would be unsound; determinism (INV-007) requires counters, not timeouts | Low: values are trivially adjustable later; boundary tests pin behavior |
| **P2D-A8 — Diagnostic and API policy recommendations (PD-009/PD-010 remain OPEN)** | Interim Phase 2 policy: keep the accepted plain-text canonical `xl_diag::render` (no color, no terminal-width wrapping, no multi-line change); do **not** introduce stable error codes now (reserve the additive `with_code` shape for later); all crates stay internal (`publish = false`); `#[non_exhaustive]` deferred to a stabilization decision; CLI stdout/stderr are test-pinned but **not** external contracts. **Approving A8 ratifies only this interim policy; it does NOT resolve, close, or predetermine PD-009 or PD-010, which remain OPEN owner decisions.** | (i) introduce codes now (changes every snapshot before the owner chooses a taxonomy); (ii) promise external API stability now (premature) | Directive 010 §11/§12 and Directive 010-R1 §10 require reasoned recommendations without silently resolving open decisions; `xl-diag` compatibility is preserved by making nothing mandatory | Low: nothing becomes mandatory; PD-009/PD-010 stay OPEN |

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
| **P2D-B7 — Import resolution and import soundness in Phase 2** | The `ModuleRegistry` (`P2D-A5`) is **executable now**: modules are registered in memory and imports resolve against it. Four outcomes are distinguished: (a) **resolved imported symbol** — a `Known` module with its surface available and the symbol found; (b) **known module, symbol not public/absent** — member error; (c) **known module, surface unavailable in this compilation** (`Unavailable(ModuleId)`) — error for the import and every use; (d) **missing module** (`Missing`) — error. **Fail closed:** in strict `xc check`, a `Missing`/`Unavailable` import produces an error at the import site even when its binding is unused; `xc check` never certifies an unresolved dependency as valid. Rationale for the change from the earlier "silent unused opaque import" policy (Directive 010-R1 §4): a successful semantic check must not certify an unresolved dependency. The accepted `import std.io;` remains **syntactically** valid; the semantic error is reported until a separately approved external-module contract supplies the interface (recorded as UNRESOLVED: whether the standard library is later provided as a pre-registered module or as an approved external contract). A future **analysis-only partial mode** (exit `0` over known parts while reporting unresolved imports) is explicitly **outside the accepted `xc check` success contract** and requires independent owner approval; it must not be the default. | (i) silent unused-opaque-import (the earlier policy — rejected: certifies unresolved deps); (ii) hard error at every import even when a registry could resolve it (too coarse); (iii) implicit sibling-file loading (invented); (iv) full package loading (out of scope) | SPECIFIED §6 Phase 2 requires import resolution; Directive 010-R1 §4 requires fail-closed unresolved imports; invents no file-discovery convention; spec routes real modules through packages | Medium: stricter in single-file mode (a file with any import that cannot be resolved no longer checks clean); the multi-module fixtures and the cycle fixtures run against the synthetic registry |
| **P2D-B8 — `for` lowering and iteration protocol** | Desugar `for` to `loop` + internal `range_next` protocol over `Range<T>` (from `a..b`, integer `T` only); state advanced before the body (so `continue` is correct); no overflow in increment; user-implementable iteration deferred | (i) also iterate arrays now; (ii) expose a user `Iterator` trait now (impossible: no `mut self`, no associated types, no stdlib) | The original mandates `for → loop + iterator` but supplies no iterator surface; the evidenced only iterable is `0..10` | Medium: general iteration arrives Phase 5/6; the internal protocol is the seam |
| **P2D-B9 — `?` semantics** | Lower `e?` to `match e { Ok(v) => v, Err(x) => return Err(x) }` with **exact error-type equality** to the enclosing `Result<_, E>` return type; `Result` only (no `Option?`); no conversions; internal references cannot be shadowed | (i) `From`-style error conversion (invents a trait/stdlib surface); (ii) allow `?` in non-`Result` functions | SPECIFIED: `?` → match; errors as values; no implicit conversions | Medium: error conversion later must be an owner decision |
| **P2D-B10 — Interpolation formatting** | Interpolation lowers to internal concat; parts must be `str` or one of the primitive set (`i8`…`usize`, `f32`/`f64`, `bool`, `char`, `str`) formatted by a fixed rule reusing the accepted printer’s float round-trip rule; user types are errors until Display (Phase 6) | (i) `str`-only (rejects the spec example’s `{total}` intent); (ii) full `Display` now (undefined by the original) | The §3 example interpolates an `f64`; a bounded primitive contract invents the least and reuses an accepted formatting rule | Medium: primitives’ exact text is user-visible; frozen by snapshots once implemented |
| **P2D-B11 — Literal defaulting and representability (closes Phase 1 B-2)** | `IntVar` → integer primitive set, default `i64`; `FloatVar` → `{f32,f64}`, default `f64`; representability checked against the resolved type, with the `-i64::MIN` negation exception; `f32` overflow diagnosed, underflow accepted | (i) default only at `let` (narrower); (ii) no `f32` overflow check | SPECIFIED §7 (“quando ambíguos”); B-2 deferred signed/target checks to this phase | Low: boundaries pinned by fixtures ty-ok-9/ty-err-4 |
| **P2D-B12 — `as` conversions** | Numeric ↔ numeric only, with Rust’s documented `as` value semantics (int→int wrap, int→float round, float→int saturating truncate); `bool`/`char`/`str` casts not introduced | (i) value-preserving casts only (`300 as u8` errors); (ii) permit `bool`/`char` casts | Every permitted pair needs defined value semantics for Phase 4; Rust’s rules are the ecosystem baseline (no invention of *behavior*, only of the permitted table) | Medium: value semantics are user-visible at runtime; documented as a baseline decision |
| **P2D-B13 — Polymorphism restriction** | Function/method items are polymorphic over **declared** generics only (instantiated per use); `let` bindings **never generalize**; no signature inference; no higher-ranked/`dyn`/`impl Trait` | (i) HM-style let-generalization (contradicts “let-polimorfismo restrito”); (ii) infer generic signatures from use | SPECIFIED §7 restricts generalization to `fn`; closures do not exist, so let-generalization buys nothing | Low: relaxing later is additive |
| **P2D-B14 — Traits** | Declaration/impl checking with `Self` as a **contextual type name**; at most one bound per parameter (grammar); unification-based coherence/overlap check; orphan rule implemented and unit-tested against a **synthetic single-package registry** (`P2D-A5`); trait items implicitly public; impl must provide exactly the trait’s methods (no extras); inherent methods take precedence in selection; ambiguity is an error. **Phase 2 impl scope (Directive 010-R1 §9):** support generic impls whose self-type *head* is a **nominal** type (`impl<T> Show for Vec<T>`, `impl Show for Point`) — the needed Phase 2 semantic checking; **defer unconstrained blanket impls** such as `impl<T> Show for T` to Phase 5. Rationale: an unconstrained blanket impl makes coherence algorithmic and can silently capture method selection across the whole program (a Phase 5 trait-ecosystem concern), while nominal-head generic impls are what the specification’s bound checking and `impl … for …` surface require in Phase 2. All three alternatives remain in the packet: (i) allow blanket impls now with solver cycle detection + depth limits; (ii) reject blanket impls; (iii) nominal-head generic impls only (recommended, kept pending owner decision). | (i) allow blanket impls now (solver safeguards); (ii) reject all blanket impls; (iii) parameter-name matching in impls (noise); (iv) allow trait-impl extras (invents extension semantics) | SPECIFIED §7: coherence, orphan rule, no associated types; grammar fixes one bound; `Self` is needed for additive traits; Directive 010-R1 §9 asks Phase 2 to avoid premature full trait-ecosystem scope | Medium–High: the impl-scope choice is visible in what compiles; visible surface (Self) is new and owner-gated |
| **P2D-B15 — Operators and type parameters** | Operators are built-in on primitives only (arithmetic on same numeric type; comparisons on numeric/`char`; logical on `bool`); **no operator traits**; bounds are declared/checked/solvable but operators on `T` remain errors in Phase 2 with a specific diagnostic | (i) built-in operator traits now (`Ord` for `<` etc.) — invents trait-method semantics; (ii) silently allow and defer checking — unsound | The original defines no operator overloading; inventing trait desugaring would prejudge the Phase 5 traits design | Medium: generic bodies are restricted in Phase 2; clearly documented |
| **P2D-B16 — Control-flow typing (closes Phase 1 Category C)** | `return`/`break`/`continue`: internal `Never` (bottom, unnameable); `while`/`for`/`loop`: `()`; `if` without `else`: `()`; assignment (including compounds): `()`; `?`: payload type; no divergence/reachability analysis beyond match arms | (i) `loop` typed by break values (none exist); (ii) full divergence analysis | Phase 1 deferred exactly this to the type-system phase (ratified Category C); a `Never` bottom is the minimal standard mechanism | Low–Medium: `Never` is internal; no user syntax changes later |
| **P2D-B17 — Mutability model** | Mutability is per-binding (`let mut` covers all bindings of its pattern); parameters and the receiver are immutable; assignment requires the **root** to be a mutable local; fields/elements inherit; diagnostics with `let mut` help | (i) per-field mutability; (ii) `mut self` receivers now (not in grammar, deferred) | SPECIFIED §7: `let` immutable, assignment requires `mut`; grammar has no `mut self` | Low: root rule is restrictive but predictable |
| **P2D-B18 — Arrays and slices** | `[T; N]`: `N` must be a non-negative integer literal (no const evaluation); `[T]` nameable but no expression has slice type in Phase 2; indexing only on arrays with integer index (no static bounds check); no `str`/tuple indexing; array literals infer `[T; N]` | (i) full const-expression evaluation of `N`; (ii) array→slice coercion (implicit conversion, rejected); (iii) forbidding slices entirely | Original lists both array forms but defines no operations; minimal, explicit subset | Low: growth is additive; coercion ban is deliberate |
| **P2D-B19 — Match severity and fail-closed limits** | Non-exhaustive match: **error** (SPECIFIED); unreachable arm: **warning**; exhaustiveness resource exhaustion: **error** (fail closed); malformed-arm matches: analysis skipped silently | (i) unreachable arm as error; (ii) resource exhaustion as warning | Original fixes only “não exaustivo é erro”; a warning for reachability matches common practice; fail-closed protects soundness | Low: severity is visible in fixtures; owner may choose stricter |
| **P2D-B20 — Visibility semantics** | `pub` recorded per item/field/method; private-by-default; enforcement active at module boundaries (vacuous single-file; through the registry a member of an `Available` module is accessible only when `pub`, and a `Missing`/`Unavailable` module fails closed); trait items implicitly public; trait-impl methods public for dispatch | (i) enforce nothing until Phase 8; (ii) require `pub` on trait-impl methods (noise) | SPECIFIED §6 Phase 5 (“módulos e pub”) moves full module semantics late; recording + boundary rule now avoids a later retrofit | Low: single-file behavior effectively unchanged |
| **P2D-B22 — Refutability of `let`/`for` patterns** | `let` and `for` patterns must be **irrefutable** for the target type; refutable patterns are errors with `use 'match'` help; checked by the exhaustiveness machinery | (i) defer to runtime panics (no panic semantics exist for this); (ii) allow with warnings | Without this rule, `let Some(x) = opt;` would have undefined behavior; the cheapest correct rule reuses the mandated algorithm | Low: standard; pinned by fixtures |

**Reclassification note (`P2D-B21` → `P2D-A8`).** The former Category B row
`P2D-B21` contained **compatibility and diagnostic-policy recommendations**
(PD-009/PD-010 handling), not language semantics, so it has been
**reclassified** to the Category A row **`P2D-A8`** above (Directive
010-R1 §10). This is a category move, not a new decision. The identifier
`P2D-B21` is retained only as this redirect so existing references resolve;
it is **not** a Category B decision and **requires no per-item semantic
approval**. Crucially, `P2D-A8` (like `P2D-B21` before it) makes **no**
claim that PD-009 or PD-010 is resolved: both remain **OPEN** owner
decisions, and approving `P2D-A8` ratifies only the interim policy.

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

Status: **executed** by Directive 010-R3 (see §13).

- **Category A rows (A1–A8):** approved as proposed, except A1 and A5
  **approved with the interface amendment** (Directive 010-R3 §4).
  Category A covers internal architecture, interfaces, budgets, CLI
  policy, and the interim diagnostic/API policy (`P2D-A8`); it does
  **not** change language semantics.
- **Category B rows (B1–B20, B22):** individually approved; a wholesale
  Category A approval did not cover them. `P2D-B21` is not a Category B
  row (reclassified to `P2D-A8`).
- **Category C (C1–C11):** **no action — remains DEFERRED.**
- **Recorded outcome:** this packet gained the ratification record in §13
  (owner directive reference, date, revision); the contract documents
  changed status to **OWNER-RATIFIED — PENDING INTEGRATION**; the ADR is
  `docs/decisions/adr-0005-phase-2-semantic-architecture.md` (the Phase 1
  precedent: proposal set → owner directive → ADR-0004).
- **Implementation remains unauthorized** until a separate directive
  names a work package (`architecture-rfc.md` §6), its predecessor gate
  is satisfied, **and** PR #7 is merged with post-merge CI green.
- This packet does not resolve PD-009 or PD-010; `P2D-A8` (formerly
  `P2D-B21`) records recommendations and an interim policy only, and both
  remain OPEN.

## 13. Ratification record (Directive 010-R3)

- **Authority:** the project owner, via **Engineering Directive 010-R3**
  (Phase 2 Contract Ratification and Final Integration Gate, 2026-10-14).
- **Verified revision at ratification:** branch `phase/2-design`,
  starting head `d2c874b4d36ca88e8aaf2c0f5deb64be605cdc8f`; PR #7 open and
  unmerged; `docs/spec-v0.1-original.txt` unchanged
  (SHA-256 `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770`).
- **Category A outcomes (8):**

  | ID | Outcome |
  | --- | --- |
  | P2D-A1 | APPROVED WITH AMENDMENT — `LowerOutput` exposes `import_edges`; graph validation is a separate, caller-invoked stage |
  | P2D-A2 | APPROVED AS PROPOSED — arenas, checked identities, origin mapping |
  | P2D-A3 | APPROVED AS PROPOSED — immutable HIR and two-stage resolution |
  | P2D-A4 | APPROVED AS PROPOSED — owned symbol names without interning |
  | P2D-A5 | APPROVED WITH AMENDMENT — immutable registry input to lowering; explicit edge collection, then graph validation |
  | P2D-A6 | APPROVED AS PROPOSED — `xc check` exit and diagnostic contract |
  | P2D-A7 | APPROVED AS PROPOSED — finite deterministic resource budgets |
  | P2D-A8 | APPROVED AS INTERIM POLICY — PD-009 and PD-010 remain OPEN |

- **Category B outcomes (21, each an independent decision):** B1, B2, B3,
  B4, B5, B6, B7, B8, B9, B10, B11, B12, B13, B14, B15, B16, B17, B18,
  B19, B20, B22 — each **APPROVED** exactly as written in this packet.
  `P2D-B21` remains reclassified to `P2D-A8`.
- **Technical amendment (Directive 010-R3 §4):** `LowerOutput` carries
  `import_edges: Vec<ImportEdge>`; the caller registers the returned edges
  **after** lowering and only then invokes
  `xl_hir::validate_module_graph(&ModuleRegistry)`. Lowering reads an
  immutable registry and performs no graph validation
  (`lowering-contract.md` §1.1–§1.2, `architecture-rfc.md` §3–§4). Purely
  architectural: no import syntax, visibility, namespace, or other
  language semantics changed.
- **Category C:** C1–C11 remain **DEFERRED** (unchanged).
- **Open decisions:** PD-009 and PD-010 remain **OPEN**.
- **Ratification vs integration:** this record ratifies the decisions; it
  does **not** merge PR #7. The design is **owner-ratified but pending
  integration** until PR #7 is merged and post-merge integration CI
  passes. Phase 2 implementation is **not** active.
- **No implementation authorization:** no work package (WP-A…WP-D) is
  authorized by this ratification (Directive 010-R3 §8).

## Dependency/ordering note (Directive 010 §13)

The recommended first implementation unit after ratification is **WP-A
(HIR foundation)**, then WP-B (name resolution, including the executable
module registry and cyclic-import detection of `P2D-A5`/`P2D-B7`), then
WP-C (types), then WP-D (CLI). If the owner wishes to reduce risk before
the full semantic surface, the packet’s reviewable split points are:
(a) WP-A+WP-B alone (hir, resolution, registry/cycle detection,
diagnostics — no types), and (b) the internal WP-C milestones C1/C2/C3
(`architecture-rfc.md` §6). Both splits are compatible with the proposed
dependency order; neither is authorized by this package. The independent
review recommended **WP-A (HIR foundation)** as the first work package
(Directive 010-R1 §11/§14).

## Traceability

| Packet area | Sources |
| --- | --- |
| A1–A7 | SPECIFIED §4 conventions/layout; governance §2/§6; Directive 010 §12–§16; Directive 010-R1 §3–§9; Directive 010-R2 §3–§6 (A5); Directive 010-R3 §4 (A1/A5 amendment) |
| A8 | Directive 010 §11/§12; Directive 010-R1 §10; `docs/pending-decisions.md` PD-009/PD-010 (OPEN) |
| B1–B7 | SPECIFIED §2/§3/§6 Phase 2; ratified PD-006; Phase 1 corrections; Directive 010-R1 §3–§4 (B7) |
| B8–B10 | SPECIFIED §6 Phase 2 desugaring mandate; §3 example; ratified PD-004/B-3 |
| B11–B13 | SPECIFIED §7; ratified B-2 |
| B14–B15 | SPECIFIED §7; grammar-rfc §3.1; Directive 010-R1 §9 (B14) |
| B16–B18 | ratified Category C deferral; SPECIFIED §3 type list |
| B19–B22 | SPECIFIED §6–§7; Directive 010 §10–§11 |
| §13 ratification | Directive 010-R3 §2–§5; `docs/decisions/adr-0005-phase-2-semantic-architecture.md` |
| C1–C11 | SPECIFIED §6 phases 5–9; §1 non-goals; ratified Phase 1 Category C |
