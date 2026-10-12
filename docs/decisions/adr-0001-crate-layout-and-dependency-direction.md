# ADR-0001 — Crate Layout and Dependency Direction

- **Status:** Accepted — mandated by Engineering Directive 001 (§4).
- **Date:** 2026-10-12

## Context

Phase 0 must create the workspace foundation for a compiler that will grow
across ten phases with the pipeline Source → Lexer → Parser → AST → HIR →
Type Checking → MIR → Optimization → Bytecode VM / Cranelift. The
directive requires two foundation crates (`xl-span`, `xl-diag`), an
explicit dependency direction `xl-diag → xl-span`, `#![forbid(unsafe_code)]`
in both crates, no global mutable compiler state, and minimal dependencies.

## Decision

- A virtual Cargo workspace with explicit membership:
  `crates/xl-span` and `crates/xl-diag`.
- Dependency direction is fixed: `xl-diag` depends on `xl-span`;
  `xl-span` depends on nothing in the workspace and must never depend on
  `xl-diag` or any future compiler-phase crate.
- No external dependencies in either crate today. `xl-diag` depends only
  on `xl-span` (path dependency declared once in
  `[workspace.dependencies]`).
- Both crates carry `#![forbid(unsafe_code)]`; no global mutable state
  exists anywhere; `SourceMap` and diagnostics are values owned by callers
  and passed explicitly.
- Future compiler crates join the same workspace and may depend only
  *downward* along the pipeline.

## Alternatives

- **Single crate for everything.** Rejected: the directive mandates the
  two-crate structure, and boundary erosion would follow once parser/type
  checker arrive.
- **More crates now (parser skeleton, driver, CLI).** Rejected: outside
  Phase 0 scope; placeholders for future phases are prohibited.
- **`xl-span` depending on `xl-diag` (for error types).** Rejected: the
  direction inverts the pipeline and is explicitly forbidden.

## Consequences

- Layer boundaries are enforced by Cargo, not convention.
- Diagnostics cannot leak into source infrastructure; spans stay
  dependency-free and cheap.
- Adding a future crate requires an ADR or directive naming its position in
  the dependency graph.

## Specification impact

Implements the workspace requirement of the specification as restated in
Directive 001 §4. No language semantics are affected.
