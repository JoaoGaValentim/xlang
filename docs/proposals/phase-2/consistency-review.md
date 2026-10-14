# XLANG Phase 2 — Consistency Review

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §14 (required documents) and
  §17 (quality gates: “Verify the design documents are internally
  consistent”; “Verify each proposed semantic rule is supported by the
  specification or marked as a proposal”; “Verify all new owner decisions
  are explicitly listed”).
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
| 3 | Decision IDs: 7 Category A + 22 Category B defined in `decision-packet.md`; 71 distinct reference sites across the package; every reference resolves to a packet row; every row is referenced by at least one companion document | PASS (after F-6) |
| 4 | Cross-document section references (`…md` §x) resolve to existing headings | PASS (after F-4, F-5) |
| 5 | Shared constants: resource budgets (`MAX_TYPE_DEPTH = 128`, `MAX_UNIFY_STEPS = 100_000`, `MAX_INSTANTIATIONS = 10_000`, `MAX_TRAIT_DEPTH = 32`, `MAX_PATTERN_CELLS = 250_000`), exit-code policy (`0`/`1`/`2`), prelude contents, `Range<T>` protocol, and `BuiltinOp` set agree across the documents that state them | PASS (after F-6) |
| 6 | Fixture IDs referenced outside the conformance plan (`nm-err-3`, `ty-ok-9`, `ty-err-4`, `pl-err-4`, …) are defined in `conformance-plan.md` §3–§5 with the claimed expectations | PASS |
| 7 | Factual claims spot-checked against code at `210e1e0`: `xl_parser::parse_file` signature and `ParseOutput`; AST instance-scoped ids and no interner; grouping transparency; `Block { stmts, tail }`; `ImportDecl { path, alias }`; `xl_diag::{Diagnostic, Label, Severity, render}` with no codes; float shortest-round-trip printer rule (B-3); CLI exit codes | PASS |
| 8 | Traceability: every major rule states SPECIFIED/DERIVED evidence or is explicitly PROPOSED/FUTURE; gaps that fill the original are packet rows | PASS |
| 9 | PD-009/PD-010 are left OPEN with recorded recommendations (`P2D-B21`); no document silently resolves them | PASS |
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

## 4. Review notes for the owner

- **Highest-judgment decisions.** The items most likely to deserve close
  owner attention are `P2D-B1` (namespace model), `P2D-B5` (pattern/binding
  classification), `P2D-B7` (imports as opaque modules in single-file
  mode), `P2D-B10` (primitive interpolation formatting), `P2D-B12` (`as`
  value semantics), and `P2D-B14` (blanket impls + `Self`). Each records
  its alternatives and the cost of reversal in the decision packet.
- **Intersections checked by this review.** Prelude ↔ pattern
  classification ↔ exhaustiveness; `Result` ↔ `?` ↔ interpolation
  formatting set; mutability ↔ pattern binding; visibility ↔ opaque
  modules; budgets ↔ fail-closed severities. These cross-cutting rules
  were read together, not per document, and are consistent.
- **Illustrative sketches.** Rust sketches in the HIR and type contracts
  are marked indicative; the binding properties are the **Contract**
  markers. Fixture sources in the conformance plan are documentation-only
  and not claimed to be polished programs.
- **What this review cannot do.** It does not verify that the proposals
  match the original specification’s intent beyond the cited evidence;
  that judgment belongs to the owner. It does not test any executable
  behavior (no Phase 2 code exists).

## 5. Status statement

At the time of this review, the Phase 2 design package is internally
consistent, complete against the Directive 010 §14 file list, free of
unresolved editorial defects, and contains no self-ratification. It
remains a proposal: all decisions are PENDING, no contract is accepted,
and no implementation is authorized.
