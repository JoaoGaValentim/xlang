# Pending Decisions

Questions that the specification does **not** answer yet. Implementers must
not invent answers (see `docs/governance.md` §10–11). Each entry states the
decision needed, why it matters, and its current status.

## PD-001 — Original specification document not in repository

- **Question:** Where is the original "xlang — Engineering Specification and
  Development Plan" v0.1 (2026-10-12) document, and when will it be
  committed to this repository?
- **Impact:** `docs/spec.md` currently records the binding summary from
  Engineering Directive 001 only. Phases 1–9 scope and several semantic
  areas cannot be transcribed without the original. Phase 1 must not begin
  until the document is available and reconciled.
- **Status:** UNRESOLVED — owner action required.
- **Owner action:** Commit the original document (suggested path
  `docs/spec-v0.1.md`) and confirm `docs/spec.md` against it.

## PD-002 — Operator grammar completeness

- **Question:** Which operators exist in XLANG, and what are their exact
  tokens and arities?
- **Impact:** Lexer and parser design in the next phases.
- **Status:** UNRESOLVED — not defined in available material.

## PD-003 — Assignment operator semantics

- **Question:** Is `=` an expression or a statement? Does XLANG have
  compound assignment (`+=`, …)? Can assignment be chained? Does
  assignment produce a value?
- **Impact:** Parser grammar and MIR semantics.
- **Status:** UNRESOLVED.

## PD-004 — Interpolated string representation

- **Question:** How are interpolated strings tokenized and represented in
  the AST (segments, embedded expressions, escapes)?
- **Impact:** Lexer/parser; diagnostics for malformed interpolation.
- **Status:** UNRESOLVED.

## PD-005 — Operator precedence

- **Question:** What is the exact precedence and associativity table?
- **Impact:** Parser; silently wrong parses are a correctness hazard.
- **Status:** UNRESOLVED.

## PD-006 — Import and visibility semantics

- **Question:** What are the module system, import forms, and visibility
  rules (`pub`, private-by-default, re-exports)?
- **Impact:** Name resolution, HIR.
- **Status:** UNRESOLVED.

## PD-007 — Source offset and column conventions

- **Question:** Does the specification agree with the Phase 0 choice of
  UTF-8 byte offsets (`u32`), 1-based lines, and 1-based columns counted in
  Unicode scalar values? Should display-width alignment (wide characters,
  tabs) matter for rendered diagnostics?
- **Impact:** Every span, every rendered diagnostic, tooling interfaces.
- **Status:** PROPOSED — implemented per
  `docs/decisions/adr-0002-source-position-conventions.md`; needs owner
  confirmation (Accept, or amend the ADR).

## PD-008 — License selection (mandatory Phase 0 decision)

- **Question:** Which license(s) will XLANG use?
- **Impact:** Without a license, default copyright applies (all rights
  reserved) and the project is not open for contributions under a known
  grant. Phase 0 cannot be fully approved with this unresolved.
- **Status:** UNRESOLVED — owner decision required. No license was chosen
  without authorization.
- **Owner action:** Select a license (for example MIT / Apache-2.0 /
  dual) and authorize adding the license file.

## PD-009 — Public API compatibility policy

- **Question:** What compatibility guarantees apply to `xl-span`,
  `xl-diag`, and future crates' public APIs between phases and releases?
- **Impact:** Review strictness; whether breaking API changes are routine
  during development; versioning of crates (`publish = false` today).
- **Status:** UNRESOLVED — proposal needed before external consumers exist.

## PD-010 — Diagnostics specification details

- **Question:** Should diagnostics carry stable error codes (e.g.
  `E0308`-style)? What exact rendered format is canonical for the project
  (color, terminal width handling)?
- **Impact:** Diagnostic stability for users and tests; may amend
  `xl-diag` rendering.
- **Status:** UNRESOLVED — current plain-text format is a Phase 0 choice,
  not a specification claim.
