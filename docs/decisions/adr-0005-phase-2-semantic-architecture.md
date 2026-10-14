# ADR-0005 — Ratified Phase 2 Semantic Architecture

- **Status:** Accepted — approved by the project owner via Engineering
  Directive 010-R3 (2026-10-14). Binding until superseded.
- **Date:** 2026-10-14

## Context

Phase 2 requires a precise semantic contract (AST → HIR lowering with
desugaring, name resolution and modules, the type system, traits and
coherence, pattern exhaustiveness, mutability, diagnostics, and resource
budgets) before any Phase 2 code can be written. Directive 010 produced the
design package under `docs/proposals/phase-2/`;

- Directive 010-R1 corrected contract gaps found in an independent review
  (executable module registry and cyclic-import detection, fail-closed
  import soundness, checked HIR identities and accurate AST→HIR origin,
  cross-module/package identity, stage-interface consistency, explicit
  resource budgets, and the Phase 2/Phase 5 boundary);
- Directive 010-R2 corrected the remaining contract inconsistencies
  (registry graph enumeration, synthesized-origin identity, the
  `DefKind::ModuleBinding` representation, and the dependency diagram);
- Directive 010-R3 ratified the individual decisions and applied one final
  integration amendment (A1/A5: `LowerOutput.import_edges` and a separate,
  caller-invoked module-graph validation stage).

This ADR records the ratification itself. It deliberately does not
duplicate the contracts: the binding content is the archived original
specification plus the accepted Phase 2 proposal documents.

## Decision

The project owner approved, via Engineering Directive 010-R3:

- **Category A (8):** `P2D-A1` (crate topology and stage interface, approved
  **with amendment**), `P2D-A2` (arenas, checked identities, origin
  mapping), `P2D-A3` (immutable HIR, two-stage resolution), `P2D-A4` (owned
  symbol names, no interning), `P2D-A5` (module registry and cyclic
  imports, approved **with amendment**), `P2D-A6` (`xc check` exit and
  diagnostic contract), `P2D-A7` (finite deterministic resource budgets),
  and `P2D-A8` (the interim diagnostic/API policy, formerly `P2D-B21`),
  which is approved **as interim policy only**.
- **Category B (21 individual decisions):** `P2D-B1`…`P2D-B20` and
  `P2D-B22`, each approved exactly as written in the decision packet.
  `P2D-B21` remains reclassified to `P2D-A8`.
- **Technical amendment (Directive 010-R3 §4):** the lowering result
  exposes its discovered import edges (`LowerOutput.import_edges`); the
  caller registers those edges after lowering and only then invokes the
  separate `xl-hir` module-graph validator. Lowering reads an immutable
  registry and performs no graph validation. This is architectural only:
  import syntax, visibility, namespace behavior, and other language
  semantics are unchanged.
- **Category C:** `C1`…`C11` remain **DEFERRED**.
- **Open decisions:** `PD-009` (public API compatibility) and `PD-010`
  (diagnostic format details) remain **OPEN**; `P2D-A8` records an interim
  policy and makes no claim that either is resolved.

The decision packet (`docs/proposals/phase-2/decision-packet.md` §13) holds
the full record: the eight Category A outcomes, the 21 Category B
approvals, the A1/A5 amendment, the Category C deferrals, and the verified
revision (`d2c874b4d36ca88e8aaf2c0f5deb64be605cdc8f`).

## Alternatives

- **Ratifying Category A wholesale and inferring Category B.** Rejected:
  Category B decisions change language semantics and require an explicit
  per-item owner decision (the packet's approval model).
- **Recording ratification as integration.** Rejected: ratification is the
  owner's decision; integration into `main` is a separate event (PR #7
  merge plus post-merge CI). Conflating them would misrepresent the
  repository state.
- **Folding the A1/A5 amendment into the original Phase 2 directive.**
  Rejected: the amendment is a distinct, dated owner correction and is
  recorded as such.

## Consequences

- The Phase 2 proposal documents are binding contracts: changes go through
  specification change control and a new owner decision.
- Status: **owner-ratified but pending integration** until PR #7 is merged
  and its post-merge integration CI passes. The design is not yet part of
  `main`.
- **No implementation is authorized.** `xl-hir`, `xl-types`, and
  `xc check` are not authorized by this ratification; the first work
  package (WP-A, HIR foundation) requires a separate owner directive
  issued only after PR #7 is merged, post-merge CI is green, and the owner
  explicitly authorizes it.
- Category C items and `PD-009`/`PD-010` remain open; deviations discovered
  during implementation must be resolved against the contracts, not
  silently.

## Specification impact

Implements the owner-approved semantic architecture for Phase 2. The
archived original (`docs/spec-v0.1-original.txt`) retains priority for any
conflict; the accepted packet's rules fill areas the original leaves open
and do not alter the original specification text.
