# XLANG Phase 2 — Consistency Review

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §14 (required documents) and
  §17 (quality gates: “Verify the design documents are internally
  consistent”; “Verify each proposed semantic rule is supported by the
  specification or marked as a proposal”; “Verify all new owner decisions
  are explicitly listed”), **amended by Engineering Directive 010-R1 §11**
  (required design regression audit of the corrected package).
- **Nature of this document:** an internal editorial audit of the Phase 2
  design package performed before delivery. It records what was checked,
  what was found, and how each finding was disposed of **within the
  proposed drafts**. It ratifies nothing and approves nothing; every
  decision remains PENDING (Directive 010 §18).

## 1. Scope

Audited surface: all ten documents in `docs/proposals/phase-2/` plus the
Phase 1 closure record entries touched by this delivery (`docs/phase-status.md`,
`docs/roadmap.md`, `README.md`, `docs/architecture.md`).

Audited properties:

1. Package completeness against Directive 010 §14.
2. Status uniformity: no document claims ratification, acceptance, or
   implementation.
3. Decision-ID integrity: every `P2D-*` reference resolves; every packet
   decision is used; no orphan or duplicated identifiers.
4. Cross-document references resolve to existing sections.
5. Shared constants, protocol names, and terminology agree where the
   documents describe the same object.
6. Fixture identifiers referenced outside `conformance-plan.md` are
   defined in it.
7. Factual claims about the Phase 1 surface match the accepted code at
   `210e1e0` (the design must start from the actual implementation, not
   hypothetical interfaces — Directive 010 §6).
8. Each semantic rule is traceable to SPECIFIED evidence or explicitly
   marked PROPOSED/UNRESOLVED/FUTURE.
9. The closure-record edits add new entries and do not rewrite recorded
   history.

Out of scope: re-deriving the specification interpretation (that is the
decision packet’s and owner’s territory); verifying any future Phase 2
implementation (none exists); verifying the illustrative source sketches
inside fixtures as compilable programs (they are documentation-only
conformance sketches).

## 2. Check results

| # | Check | Result |
| --- | --- | --- |
| 1 | All ten documents present (`architecture-rfc`, `hir-contract`, `lowering-contract`, `name-resolution-contract`, `type-system-contract`, `traits-and-exhaustiveness`, `diagnostics-contract`, `conformance-plan`, `decision-packet`, this review) | PASS |
| 2 | Every document carries `STATUS: PROPOSED — PENDING OWNER RATIFICATION`; the only occurrence of “ACCEPTED” in the package is the decision packet’s description of post-ratification mechanics | PASS |
| 3 | Decision IDs: 8 Category A (`P2D-A1`…`P2D-A8`) + 21 Category B (`P2D-B1`…`P2D-B20`, `P2D-B22`; `P2D-B21` reclassified to `P2D-A8`) defined in `decision-packet.md`; every reference resolves to a packet row (or the `P2D-B21`→`P2D-A8` redirect); every row is referenced by at least one companion document | PASS (after R-6) |
| 4 | Cross-document section references (`…md` §x) resolve to existing headings | PASS (after F-4, F-5, R-7) |
| 5 | Shared constants: resource budgets (`MAX_HIR_DEPTH = 256`, `MAX_HIR_NODES = 1_000_000`, `MAX_DEFS = 100_000`, `MAX_USES = 1_000_000`, `MAX_MODULE_GRAPH_MODULES = 1_024`, `MAX_MODULE_GRAPH_EDGES = 16_384`, `MAX_MODULE_GRAPH_DEPTH = 256`, `MAX_TYPE_DEPTH = 128`, `MAX_UNIFY_STEPS = 100_000`, `MAX_INSTANTIATIONS = 10_000`, `MAX_TRAIT_DEPTH = 32`, `MAX_PATTERN_CELLS = 250_000`), exit-code policy (`0`/`1`/`2`), prelude contents, `Range<T>` protocol, `BuiltinOp` set, `ModuleId`/`PackageId`/`GlobalDefId`, and stage signatures agree across the documents that state them | PASS (after R-3, R-4) |
| 6 | Fixture IDs referenced outside the conformance plan (`nm-err-3`, `nm-err-8/8b/8c/11`, `nm-ok-8`, `ty-ok-9`, `ty-err-4`, `pl-err-4`, `mg-1`…`mg-9`, …) are defined in `conformance-plan.md` §3–§5/§7.3 with the claimed expectations | PASS |
| 7 | Factual claims spot-checked against code at `210e1e0`: `xl_parser::parse_file` signature and `ParseOutput`; **AST instance-scoped `ItemId`/`ExprId` exist only on `Item`/`Expr`** (`crates/xl-ast/src/id.rs`, `ast.rs`); grouping transparency; `Block { stmts, tail }`; `ImportDecl { path, alias }`; `xl_diag::{Diagnostic, Label, Severity, render}` with no codes; parser saturating id counters; float shortest-round-trip printer rule (B-3); CLI exit codes | PASS |
| 8 | Traceability: every major rule states SPECIFIED/DERIVED evidence or is explicitly PROPOSED/FUTURE; gaps that fill the original are packet rows | PASS |
| 9 | PD-009/PD-010 are left OPEN with recorded interim policy (`P2D-A8`, formerly `P2D-B21`); no document silently resolves them | PASS |
| 10 | Phase 1 closure record: new entries appended; the Directive 009 audit table preserved verbatim with a dated status note; no historical row rewritten | PASS |
| 11 | Phase 2 status: `phase-status.md`, `roadmap.md`, `README.md`, and `architecture.md` all describe Phase 2 as a design gate with no implementation authorized | PASS |

## 3. Findings log

Findings were fixed by editing the **unratified proposal drafts** (normal
drafting procedure; no accepted contract or historical record was
modified). No finding remains open as an editorial defect.

| ID | Finding | Severity | Disposition |
| --- | --- | --- | --- |
| F-1 | `hir-contract.md` §5.4 sketch still carried placeholder `IterNew`/`TryDesugar` expression variants that contradicted the finalized internal-protocol design | Editorial (internal contradiction) | Fixed while drafting: replaced by the single `BuiltinCall { op: BuiltinOp, … }` form; `Try`/`For` recorded as not surviving lowering |
| F-2 | Unit-variant value/pattern wording was inconsistent between the HIR, lowering, and type-system drafts (a path to a zero-payload variant in value position) | Editorial | Fixed while drafting: the type-system contract states the value rule (a zero-payload variant path in value position has the enum type, uniform for prelude and user enums, `P2D-B4`/`P2D-B5`); the HIR and lowering contracts state the matching pattern-side classification (lone identifier = binding candidate; path with `::`/args = constructor candidate) |
| F-3 | Two cross-references in `diagnostics-contract.md` and one in `architecture-rfc.md` pointed at the wrong sections for diagnostic ordering | Editorial | Fixed while drafting: references updated to the correct sections |
| F-4 | `hir-contract.md` cited `architecture-rfc.md` §0 for the evidence-label vocabulary; the RFC has no §0 (labels live in its preamble) | Editorial | Fixed in this review pass: reference now says “the preamble of `architecture-rfc.md`” |
| F-5 | `traits-and-exhaustiveness.md` §7 cited `lowering-contract.md` §4.6 (no such subsection; §4 has 4.1–4.2) for the `for`-pattern refutability check | Editorial | Fixed in this review pass: cites §4.2, contract point 6 |
| F-6 | `OkNew`/`ErrNew` appeared in the HIR `BuiltinOp` list and `ok_new`/`err_new` in the lowering hygiene list, while `lowering-contract.md` §5.1 lowers `?` via the prelude `Ok`/`Err` constructors; `CallBuiltin(range_new, …)` also diverged from the `BuiltinCall { op: RangeNew, … }` node form | Semantic-adjacent (contradictory representation of the same construct) | Fixed in this review pass: the `?` desugaring is uniformly specified as using synthesized, pre-resolved uses of the prelude `Result` constructors; `BuiltinOp` is exactly `RangeNew`, `RangeNext`, `StrConcat`, `FormatPrim`; the lowering table uses the node form |

No finding changed a language-semantics proposal; F-6 aligned the
documents on the representation already chosen by `P2D-B9` and described
in §5.1 of the lowering contract.

## 3.1 Independent regression audit (Directive 010-R1 §11)

This audit is **separate from** the internal §1–§3 checks above. It was
performed after the independent review that produced Directive 010-R1, and
it addresses *that* review’s findings with evidence — not a re-statement
of the earlier internal audit. Each row maps to one of the fourteen
required audit points.

| # | Required check | Result | Evidence (post-correction) |
| --- | --- | --- | --- |
| 1 | All decision references resolve | PASS | `decision-packet.md` A1–A8, B1–B20, B22; `P2D-B21` is a documented redirect to `P2D-A8`; no dangling `P2D-*` reference |
| 2 | No accepted specification changed | PASS | `docs/spec-v0.1-original.txt` SHA-256 unchanged (`c3c6bce5…`, verified on branch); §2/§3/§6/§7 are cited, not edited |
| 3 | Cyclic-import requirement now satisfied by the planned Phase 2 implementation | PASS | `P2D-A5`/`P2D-B7`: executable in-memory `ModuleRegistry`; `name-resolution-contract.md` §9.4; `lowering-contract.md` §2.3; conformance §7.3 (mg-1…mg-9) |
| 4 | Unknown imports cannot silently certify successful checking | PASS | `P2D-B7` fail-closed policy; `name-resolution-contract.md` §9.3; `diagnostics-contract.md` §5.10; conformance §4.1 nm-err-8/8b/8c and acceptance §6.5/§6.6 |
| 5 | HIR identifiers unambiguous and overflow-safe | PASS | `P2D-A2`: checked allocation, `IdSpaceExhausted`, `StageFate::Aborted`, never saturating/aliasing (`hir-contract.md` §4.3/§7.6/§11.7) |
| 6 | Source mappings refer only to identifiers that actually exist | PASS | `P2D-A2`/`hir-contract.md` §4.4: `AstOrigin` uses `ItemId`/`ExprId` only where the AST assigns them; `Structural { anchor, path }` otherwise; spans never keys; verified against `crates/xl-ast/src/id.rs` |
| 7 | Module and package identities consistent | PASS | `ModuleId`/`PackageId`/`GlobalDefId` in `hir-contract.md` §3, `name-resolution-contract.md` §9.2/§9.5, `type-system-contract.md` §3 (`Nominal { def: GlobalDefId, .. }`), `traits-and-exhaustiveness.md` §2.5 |
| 8 | Orphan-rule terminology consistent across documents | PASS | “package”-scoped ownership everywhere: `P2D-B14`, `traits-and-exhaustiveness.md` §2.5/§2.8, `diagnostics-contract.md` §5.9; the old “current module” wording is removed |
| 9 | API signatures and dependency direction agree | PASS | `P2D-A1`: `lower(&[Item], FileId, &ModuleRegistry)`, `check(&HirModule, &Resolutions)`; dependency graph vs data flow separated (`architecture-rfc.md` §2.2/§3); `xl-hir` does not depend on `xl-parser` |
| 10 | Resource limits finite and explicit | PASS | `P2D-A7` canonical budget list (HIR, module graph, types, traits, patterns); `MAX_HIR_DEPTH` defined; mirrored in `type-system-contract.md` §16 and conformance §7.2 |
| 11 | Phase 2 vs Phase 5 responsibilities clearly separated | PASS | `architecture-rfc.md` §1; `P2D-B14` impl scope; `lowering-contract.md` §4.1/§6.3; `conformance-plan.md` §8 |
| 12 | Conformance fixtures match the updated decisions | PASS | `conformance-plan.md`: nm-ok-8, nm-err-8/8b/8c/11, §7.3 mg-1…mg-9, acceptance §6 |
| 13 | All semantic proposals remain explicitly pending | PASS | every document carries `STATUS: PROPOSED — PENDING OWNER RATIFICATION`; packet approval mechanics unchanged; §5 below |
| 14 | No compiler implementation added | PASS | diff is documentation-only; `crates/` unchanged (`git diff --stat` limited to `docs/`, `README.md`) |

### 3.2 Regression findings and dispositions

| ID | Finding (from the independent review) | Disposition |
| --- | --- | --- |
| R-1 | Cyclic imports were specified but postponed to a future registry phase | Corrected: registry and cycle detection are now Phase 2 deliverables (`P2D-A5`, `P2D-B7`, `name-resolution-contract.md` §9.4, `lowering-contract.md` §2.3, conformance §7.3) |
| R-2 | Unresolved opaque imports could be silent when unused | Corrected to a fail-closed policy with four explicit outcomes (`P2D-B7`, `name-resolution-contract.md` §9.3, `diagnostics-contract.md` §5.10) |
| R-3 | HIR id counters saturated, risking aliasing | Corrected to checked allocation with `IdSpaceExhausted`/`StageFate::Aborted` (`P2D-A2`, `hir-contract.md` §4.3) |
| R-4 | Claim that every AST node maps through `ExprId`/`ItemId` was inaccurate (stmt/pat/type have no AST id) | Corrected with `AstOrigin` (existing id where one exists, deterministic `Structural` key otherwise; spans never keys) (`hir-contract.md` §4.4) |
| R-5 | `LocalId` identity not clarified | Documented as a `DefId` specialization, not a separate identity space (`hir-contract.md` §4) |
| R-6 | `DefId` could not name a definition across modules; no package identity | Added `GlobalDefId`/`ModuleId`/`PackageId`; `Nominal` uses `GlobalDefId` (`hir-contract.md` §3/§5.1, `type-system-contract.md` §3) |
| R-7 | Orphan rule stated at module boundary; risk of a permanent module-only rule | Corrected to package-scoped ownership (`P2D-B14`, `traits-and-exhaustiveness.md` §2.5) |
| R-8 | Lowering/checking signatures differed across documents; dependency graph conflated with data flow | Corrected to one interface contract (`P2D-A1`) and separate diagrams (`architecture-rfc.md` §2.2/§3) |
| R-9 | `MAX_HIR_DEPTH` referenced without a definition; budgets incomplete | Corrected: exact meaning and bound (§9.4 of `hir-contract.md` §9; `P2D-A7`; `type-system-contract.md` §16) plus HIR/module-graph budgets |
| R-10 | `P2D-B21` held policy, not language semantics; classification could imply PD-009/PD-010 resolution | Reclassified to Category A row `P2D-A8` with a redirect; PD-009/PD-010 remain OPEN (`decision-packet.md`) |
| R-11 | Unconstrained blanket impls possibly premature for Phase 2 | Phase 2 scope narrowed to nominal-head generic impls; blanket impls deferred to Phase 5, all alternatives retained (`P2D-B14`, `traits-and-exhaustiveness.md` §2.4) |

No regression finding changed a language-semantics proposal beyond the
documented scope change in `P2D-B14` (which the owner still decides); all
corrections remain **PENDING** owner ratification.

## 4. Review notes for the owner

- **Highest-judgment decisions.** The items most likely to deserve close
  owner attention are `P2D-B1` (namespace model), `P2D-B5` (pattern/binding
  classification), `P2D-B7` (import resolution and fail-closed soundness),
  `P2D-B10` (primitive interpolation formatting), `P2D-B12` (`as` value
  semantics), and `P2D-B14` (trait impl scope + `Self` + package orphan
  rule). Each records its alternatives and the cost of reversal in the
  decision packet.
- **Intersections checked by this review.** Prelude ↔ pattern
  classification ↔ exhaustiveness; `Result` ↔ `?` ↔ interpolation
  formatting set; mutability ↔ pattern binding; visibility ↔ import
  resolution; module identity ↔ `GlobalDefId` ↔ `Nominal`; orphan rule ↔
  package identity; budgets ↔ fail-closed severities; stage signatures ↔
  dependency direction. These cross-cutting rules were read together, not
  per document, and are consistent.
- **Illustrative sketches.** Rust sketches in the HIR and type contracts
  are marked indicative; the binding properties are the **Contract**
  markers. Fixture sources in the conformance plan are documentation-only
  and not claimed to be polished programs.
- **What this review cannot do.** It does not verify that the proposals
  match the original specification’s intent beyond the cited evidence;
  that judgment belongs to the owner. It does not test any executable
  behavior (no Phase 2 code exists).

## 5. Status statement

At the time of this review — after the Engineering Directive 010-R1
independent audit corrections recorded in §3.1/§3.2 — the Phase 2 design
package is internally consistent, complete against the Directive 010 §14
file list, free of unresolved editorial defects, satisfies the
Directive 010-R1 §11 regression points, and contains no
self-ratification. It remains a proposal: all decisions (`P2D-A1`…`A8`,
`P2D-B1`…`B20`, `P2D-B22`, `P2D-C1`…`C11`) are PENDING, PD-009 and PD-010
remain OPEN, no contract is accepted, and no implementation is authorized.
