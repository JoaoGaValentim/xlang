# XLANG Phase 2 — Architecture RFC

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 — Phase 1 Formal Closure and
  Phase 2 Architecture & Specification Gate (2026-10-14).
- **Scope:** architecture for name resolution, HIR construction, and static
  type checking (original specification §4 pipeline stage and §6 Phase 2
  deliverables). **No production implementation is authorized by this
  package.**
- **Authoritative sources:** `docs/spec-v0.1-original.txt` (v0.1, archived,
  SHA-256 `c3c6bce5…`) §3–§7; accepted ADRs (`docs/decisions/`); the
  ratified Phase 1 contracts (`docs/proposals/phase-1/`); the actual public
  APIs of the six existing crates.
- **Companion documents (this directory):** `hir-contract.md`,
  `lowering-contract.md`, `name-resolution-contract.md`,
  `type-system-contract.md`, `traits-and-exhaustiveness.md`,
  `diagnostics-contract.md`, `conformance-plan.md`, `decision-packet.md`,
  `consistency-review.md`.

Evidence labels used throughout this package (same vocabulary as Phase 1):

- **SPECIFIED** — stated or shown by the original specification.
- **DERIVED** — forced by a combination of SPECIFIED facts.
- **PROPOSED** — a concrete design choice filling a gap the original leaves
  open; requires owner approval; nothing here is approved.
- **UNRESOLVED** — the original is silent and no rule is proposed (or a
  proposal is deliberately withheld).
- **FUTURE** — a deferred obligation explicitly assigned to a later phase
  or a future owner directive.

Every decision identifier (`P2D-A…`, `P2D-B…`) is defined in
`decision-packet.md`; this package's decisions are **PENDING** until the
owner ratifies them.

## 1. Phase 2 goal and milestone

The original specification (§6, Phase 2) fixes the scope:

- AST → HIR lowering with desugaring (`for` → `loop` + iterator, `?` →
  `match`, string interpolation → concatenation).
- A symbol table with scopes and modules.
- Detection of duplicate and undefined names, and of cyclic imports.
- The type checker of original §7.
- Match exhaustiveness checking (Maranget's algorithm).
- Mutability checking (`let mut`).

Milestone (SPECIFIED §6): `xc check program.x` reports precise semantic
errors. **This milestone is not implemented and is not authorized by this
directive.** The design must not be read as an implementation.

Acceptance shape (SPECIFIED §6): error messages with a primary span,
secondary labels, and a suggestion (e.g. “você quis dizer foo?”). The
`compile-fail/` test corpus (`.x` + expected `.stderr`) is part of the
original's Phase 2 test plan.

## 2. Crate topology (P2D-A1)

The original §4 fixes the v1.x workspace layout with two crates for this
stage (SPECIFIED):

```text
├─ xl-hir/     # name resolution, desugar          (original §4)
├─ xl-types/   # inference, traits, exhaustiveness (original §4)
```

### 2.1 Proposal

Two new crates join the workspace in Phase 2 implementation directives:

| Crate | Responsibility | May depend on |
| --- | --- | --- |
| `xl-hir` | AST → HIR lowering; item/definition collection; symbol tables; syntactic name resolution; desugaring (`for`, `?`, interpolation); resolution diagnostics | `xl-ast`, `xl-span`, `xl-diag`, `xl-parser` is **not** a dependency (takes `ParseOutput` values via `xl-ast`) |
| `xl-types` | Type lowering from HIR type references; inference and unification; literal defaulting; trait checking and solving; mutability checking; pattern checking; exhaustiveness; type diagnostics; final resolutions (fields/methods) | `xl-hir`, `xl-ast` (for operator enums only if kept there), `xl-span`, `xl-diag` |

- `xl-hir → xl-ast → xl-span`; `xl-types → xl-hir`; no cycles; nothing
  depends on `xl-cli`.
- **No `xl-driver` crate in Phase 2.** The original layout lists
  `xl-driver` as the pipeline orchestrator (SPECIFIED §4, v1.x target).
  Phase 2 has exactly one command (`xc check`), and governance §2 forbids
  infrastructure whose only purpose is hypothetical future extensibility.
  `xl-cli` composes the three calls (`xl_parser::parse_file` →
  `xl_hir::lower` → `xl_types::check`) directly; when Phase 3 needs a
  second command sharing orchestration, `xl-driver` is introduced under
  that phase's directive. Recorded as decision **P2D-A1** (alternative:
  introduce `xl-driver` now).
- `xl-cli` gains one command implementation and no new dependency. Its
  dependency set becomes `{xl-parser, xl-ast, xl-span, xl-diag} ∪
  {xl-hir, xl-types}` for the `check` path.
- All new crates carry `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`
  like the existing crates (SPECIFIED §4 convention; governance §4).

### 2.2 Dependency graph (proposed, complete)

```text
xl-span ──▶ xl-diag ──▶ xl-lexer ──▶ xl-parser
   │           │                         │
   ▼           ▼                         ▼
 xl-ast ◀──────┴─────────────── (ParseOutput)
   │
   ▼
 xl-hir ──▶ xl-types
   │           │
   └─────┬─────┘
         ▼
      xl-cli  (composes; `xc check`)

Edges are the complete permitted set. `xl-hir` must not depend on
`xl-parser` (it consumes `xl_ast::Item` trees, not token streams);
`xl-types` must not depend on `xl-lexer`/`xl-parser`.
```

## 3. Data flow (proposed)

```text
source file
  │  xl_parser::parse_file           (Phase 1, accepted)
  ▼
ParseOutput { items: Vec<Item>, diagnostics }
  │  xl_hir::lower(&SourceFile, FileId, &[Item])
  ▼
LowerOutput { module: HirModule, diagnostics }     ← resolution + desugaring
  │  xl_types::check(&HirModule, &SourceFile)      ← inference, traits, patterns
  ▼
CheckOutput { diagnostics, resolutions }           ← side tables (fields/methods)
  │  xl_cli::run_check
  ▼
stderr: rendered diagnostics (xl_diag::render) · stdout: empty · exit 0/1/2
```

Stage contracts (all PROPOSED, following the SPECIFIED §4 conventions —
phases are pure functions, newtype IDs in arenas, every error a
`Diagnostic` with a `Span`, no panics on user input):

1. `xl_hir::lower` is a pure function of `(ParseOutput, &SourceFile)`.
   It always returns a complete `HirModule` (partial HIR with error nodes
   on malformed input), plus diagnostics. It performs no I/O.
2. `xl_types::check` is a pure function of `(&HirModule, &SourceFile)`
   and returns diagnostics plus a resolution table. It never mutates the
   HIR (resolutions live in side tables; see `hir-contract.md` §6).
3. Both stages are deterministic: identical inputs produce identical
   diagnostics in identical order, with no environment, time, or hash-map
   iteration-order dependence (`architecture.md` §4 INV-007 extended).
4. Lexical and syntactic diagnostics from `ParseOutput` are rendered first
   (their existing order), then lowering/resolution diagnostics, then type
   diagnostics, each in deterministic traversal order
   (`diagnostics-contract.md` §3).

Input policy: `xl_hir::lower` and `xl_types::check` are only invoked on
pipeline-consistent input (`ParseOutput` produced from the same source).
A `ParseOutput` that already has error-level diagnostics is still lowered:
HIR must be able to represent recovered/placeholder trees
(`hir-contract.md` §7). The CLI invokes `check` regardless and renders
everything.

## 4. Stage boundaries (responsibility matrix)

| Concern | Owner | Notes |
| --- | --- | --- |
| Tokenization, syntax trees | `xl-lexer`, `xl-ast`, `xl-parser` (accepted) | Phase 1; spans per ADR-0002 |
| AST → HIR shape mapping | `xl-hir` lowering | `hir-contract.md`, `lowering-contract.md` |
| Definition collection (items, variants, fields, methods, params, locals) | `xl-hir` | `DefId`/`LocalId` allocation |
| Imports registration; module bindings | `xl-hir` | `P2D-B7` |
| Syntactic name resolution (locals, items, paths, variants, prelude) | `xl-hir` | `name-resolution-contract.md` |
| Desugaring (`for`, `?`, interpolation) | `xl-hir` | done before typing; `lowering-contract.md` §4–6 |
| Duplicate/undefined/ambiguous name diagnostics | `xl-hir` | "did you mean" suggestions |
| Cyclic import detection | `xl-hir` (registry-level) | algorithm defined; vacuous in single-file mode (`P2D-B7`) |
| Mutability checking (`let mut`, assignment targets) | `xl-types` | needs types; `type-system-contract.md` §9 |
| Type lowering (HIR `TypeRef` → `TyId`) | `xl-types` | `type-system-contract.md` §3 |
| Inference, unification, literal defaulting | `xl-types` | §4–5 |
| Generics instantiation; signature checking | `xl-types` | §6 |
| Trait declaration checking; coherence; bound solving; method selection | `xl-types` | `traits-and-exhaustiveness.md` §2 |
| Field/member/method resolution (type-directed) | `xl-types` | recorded in side tables; `P2D-A3` |
| Pattern checking; exhaustiveness; reachability | `xl-types` | `traits-and-exhaustiveness.md` §3–7 |
| Type/mutability/pattern diagnostics | `xl-types` | `diagnostics-contract.md` |
| Composition, file I/O, rendering, exit codes | `xl-cli` | `P2D-A6` |

No concern is shared: a diagnostic is emitted by exactly one stage. When a
stage cannot decide without the other (e.g. method lookup needs types),
the split is explicit: syntax-time resolution records *candidates and
uses*; type-time resolution records *selections* (`P2D-A3`).

## 5. Relationship to the accepted Phase 1 surface (facts used)

The design starts from the actual implementation (verified at
`210e1e0`), not from hypothetical interfaces:

- `xl_parser::parse_file(&SourceMap, FileId) -> Result<ParseOutput, SpanError>`;
  `ParseOutput { items, diagnostics }` with deterministic
  lexical-then-syntactic order.
- `xl_ast` nodes carry exact `Span`s and **instance-scoped** `ItemId`/
  `ExprId` (u32 newtypes, saturating allocation). HIR must therefore mint
  its own identities and record AST→HIR mapping itself; it cannot use AST
  ids as global keys (`hir-contract.md` §4).
- Grouping parentheses are not represented (transparency, amendment B-1);
  recovered nodes are zero-width placeholders; identifiers are plain
  `String`s (no interner, by design).
- `Block { stmts, tail }` separates the tail by construction; `For`, `Try`,
  `Assign(op, …)`, `Range`, `Cast`, `StringLit { parts }` are the AST
  shapes desugaring consumes.
- `ItemKind` includes `Import(ImportDecl { path, alias })` with dotted
  paths (`::` is the expression/type path separator).
- Diagnostics use `xl_diag::{Diagnostic, Label, Severity, render}` with
  primary + secondary labels; **no error codes exist** (PD-010 open).
- The accepted AST has no notion of: name resolution, mutability legality,
  exhaustiveness, or types. Nothing in Phase 1 constrains Phase 2 legality
  beyond syntax.

## 6. Implementation strategy (planning units only)

Work packages follow Directive 010 §13. **A package cannot start until the
previous integration gate is satisfied and the owner issues an explicit
directive** (Directive 010 §13; governance §1).

### WP-A — HIR foundation

- HIR representation, arenas, identities, source mapping.
- AST→HIR lowering for every item/statement/expression/pattern/type.
- The three desugarings.
- Structural HIR verification (a `validate` analogue of `xl-ast`'s
  validator) and lowering diagnostics.
- Gates: unit tests, desugaring snapshot tests, HIR validator, determinism.

### WP-B — Name resolution

- Symbol tables, scopes, module bindings, imports, prelude.
- Duplicate/undefined/ambiguous diagnostics with suggestions.
- Visibility recording (enforcement surfaces with the multi-module
  registry; `P2D-B20`).
- Import-cycle algorithm (obligation carried when a registry exists).
- Gates: resolution test matrix (positive/negative), suggestion snapshots.

### WP-C — Type checking

Assessed subdivisions (recommended; they are internal milestones inside
one owner-authorized package, each with its own integration checkpoint —
see the assessment note below):

- **C1** type representation, lowering, inference, unification, literal
  defaulting, `as`, function/return checking, mutability.
- **C2** traits: declaration checking, coherence/overlap, orphan rule,
  bound checking/solving, method selection, `Self`.
- **C3** patterns: pattern typing, exhaustiveness (Maranget), reachability,
  resource budgets.
- Gates: per-milestone suites plus the full conformance corpus.

### WP-D — CLI integration

- `xc check`, end-to-end tests against the real binary, compile-fail
  snapshot fixtures, Phase 2 acceptance audit.
- Gates: `xc check` end-to-end on the corpus; `.stderr` snapshots stable
  and platform-independent; acceptance report for owner review.

### Sequencing assessment (Directive 010 §13)

- A → B → C → D is correct as given and does not need reordering: C needs
  B's resolution results; D needs C; HIR shape must precede both.
- One dependency subtlety: **field/method resolution is type-directed**
  (`P2D-A3`). It cannot complete in WP-B. The design resolves this by
  splitting: WP-B resolves everything syntactic and records *unresolved
  member lookups*; WP-C performs member resolution and overwrites those
  entries in side tables. This is a design constraint, not a reordering.
- Another: exhaustiveness needs pattern types and enum declaration data.
  Enum declarations are available after WP-B; pattern *types* need C1. C3
  therefore trails C1/C2 inside WP-C, as proposed.
- No additional work packages are required; no package merges are
  recommended.

## 7. Alternatives considered (architecture level)

1. **Do resolution on the AST (no HIR).** Rejected: the original pipeline
   is binding (SPECIFIED §4: AST → name resolution + desugaring → HIR →
   type checker); typing desugared forms without HIR would either
   duplicate desugaring inside the checker or leak desugaring into the
   CLI; and no stable identity space would exist for resolutions.
2. **Mechanically mirror the AST into HIR with no differences.**
   Rejected: the original mandates desugaring inside this stage; `for`,
   `?`, and interpolation must not survive into typed HIR. Differences are
   enumerated and justified in `hir-contract.md` §5.
3. **Type-check the AST directly and build HIR only for MIR.** Rejected:
   contradicts the SPECIFIED pipeline; resolution results would have no
   canonical home; MIR would inherit desugaring work.
4. **A full query/incremental architecture (salsa-style) now.** Rejected:
   speculative; incremental compilation is a Phase 9 deliverable
   (SPECIFIED §6 Phase 9); pure-function stages keep it possible.
5. **Single `xl-semantics` crate.** Rejected: the original fixes two
   crates (`xl-hir` for resolution/desugar, `xl-types` for
   inference/traits/exhaustiveness, SPECIFIED §4); boundary erosion risk.
6. **Introduce `xl-driver` now.** Rejected for Phase 2 scope (see §2.1);
   kept as the alternative for `P2D-A1`.

## 8. Risks and mitigations (architecture level)

| Risk | Mitigation |
| --- | --- |
| HIR duplicates AST shape and drifts | HIR differences are enumerated and tested by lowering snapshots; validator enforces structural invariants |
| Member resolution split across stages becomes inconsistent | Single `Resolutions` table updated in place; final-state invariant checks (every use resolved or explicitly `Unresolved` with a diagnostic) |
| Desugaring loses source quality of diagnostics | Synthesized nodes carry originating spans (`P2D-A2`); diagnostics for desugared failures report the construct, not the expansion |
| Scope growth (semantics invented to fill gaps) | Every semantic rule is a `P2D-B` decision requiring owner approval; Category C lists deferrals |
| Deep/recursive structures cause stack issues | Arena (flat) storage for HIR and types; explicit depth budgets (`P2D-A7`); no unbounded native recursion |
| Diagnostic cascades | Poisoning rules and cascade suppression (`diagnostics-contract.md` §7) |
| Import semantics blocked on the package model | `ModuleResolver` seam (`P2D-A5`, `P2D-B7`); single-module policy defined and tested; multi-module obligations documented as FUTURE |
| Public API drift | PD-009 policy proposal (`P2D-B21`): all crates internal until owner decides otherwise |

## 9. Validation obligations for this package (design-time)

Because this directive is design-only:

- No dependencies, manifests, or workspace files change.
- No `xl-hir`/`xl-types` crate is created.
- Existing behavior is untouched; the repository's fmt/clippy/test/locked
  gates remain green at the branch base.
- Every rule in this package is traceable to the SPECIFIED sources or
  labeled PROPOSED and listed in `decision-packet.md`.
- `consistency-review.md` records the internal cross-document audit.

## 10. Traceability summary

| Package item | Original reference |
| --- | --- |
| HIR + desugaring | §4 pipeline; §6 Phase 2 deliverables |
| Symbol tables, scopes, modules | §6 Phase 2 |
| Duplicate/undefined/cyclic detection | §6 Phase 2 |
| Type checker | §7 |
| Exhaustiveness (Maranget) | §6 Phase 2; §7 |
| Mutability | §7; §6 Phase 2 |
| `xc check` milestone | §6 Phase 2 |
| Diagnostics quality; `.stderr` tests | §1; §6 Phase 2; §9 |
| Newtype IDs in arenas; pure phases; no panic | §4 conventions |
| Crate names `xl-hir`, `xl-types` | §4 layout |
