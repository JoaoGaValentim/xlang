# XLANG Language Specification (Working Record)

- **Document:** xlang — Engineering Specification and Development Plan
- **Version:** 0.1
- **Date:** October 12, 2026
- **Status:** working transcription — the original document is not yet available in this repository

> **Availability notice.** When Phase 0 was executed, the original
> specification document was not present in the repository or on the
> development machine (verified by filesystem and Spotlight search). This
> file records the binding summary of the specification as provided by the
> project owner in Engineering Directive 001 (2026-10-12). It adds no
> language semantics of its own. Before Phase 1 begins, the original
> document must be supplied and committed to this repository (for example as
> `docs/spec-v0.1.md`), and this file must be reconciled with it. Any
> difference between the original document and this record is a
> specification-change event governed by `docs/governance.md`.

## 1. Established requirements

The specification establishes XLANG and its compiler as follows:

- A statically typed, compiled, expression-oriented programming language.
- A compiler implemented in Rust.
- Bidirectional type inference and unification.
- Algebraic data types and pattern matching.
- Explicit error handling through `Result<T, E>`.
- A modular compiler architecture built as a Cargo workspace.
- A bytecode VM as the initial execution backend.
- Cranelift as the subsequent native backend.
- Structured diagnostics with accurate source spans.
- Deterministic, modular, testable compilation phases.
- Ten sequential development phases, numbered 0 through 9.

## 2. Compilation pipeline

The intended pipeline is a binding technical contract:

```text
Source → Lexer → Parser → AST → Name Resolution / HIR → Type Checking
       → Typed HIR → MIR → Optimization → Bytecode VM / Cranelift
```

## 3. Semantic areas not yet fully defined

The following areas are named by the specification summary but are not
defined in enough detail to implement. They must not be filled in from
assumption; every one is tracked as an open decision in
`docs/pending-decisions.md`:

- Operator grammar completeness.
- Assignment operator semantics.
- Interpolated-string representation.
- Operator precedence.
- Import and visibility semantics.

The detailed scope of development Phases 1 through 9 is also not yet
transcribed; see `docs/roadmap.md`.

## 4. Change control

The original specification is authoritative for all language semantics.
No semantic rule may be added, removed, or altered without an approved
specification change recorded according to `docs/governance.md`.
