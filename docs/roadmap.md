# Development Roadmap

The project follows ten sequential development phases, numbered 0 through 9.
The original roadmap content for Phases 1–9 is defined by the authoritative
specification document; that document was not available when this file was
created, so Phases 1–9 are deliberately **not** transcribed here. Do not
plan, schedule, or implement against a placeholder. See
`docs/pending-decisions.md` (PD-001).

## Phase 0 — Foundation

Status: in progress (see `docs/phase-status.md`).

Scope, as authorized:

- Cargo workspace foundation.
- `xl-span` (source location infrastructure).
- `xl-diag` (structured diagnostics).
- Initial language specification documentation.
- Compiler architecture documentation.
- Architecture Decision Records.
- Repository governance.
- GitHub Actions CI.
- Licensing and contribution-policy assessment.
- Foundation-level automated tests.

Explicitly out of scope for Phase 0: lexer, parser, AST, HIR, type
inference, MIR, bytecode generation, virtual machine, native code
generation, standard library, formatter, LSP, and package management.

## Phase 1

Phase 1 is the next planned phase. It must not begin until:

1. Phase 0 has passed all of its acceptance criteria, and
2. the project owner has issued the directive authorizing Phase 1.

Its detailed scope will be taken from the authoritative specification. No
Phase 1 work of any kind was performed during Phase 0.

## Phases 2–9

Content pending transcription of the authoritative specification. The
known constraints that already bind later phases:

- Phases are sequential; a phase starts only after owner authorization.
- The pipeline order (Lexer → Parser → AST → HIR → Type Checking → Typed
  HIR → MIR → Optimization → Bytecode VM / Cranelift) is a binding
  contract.
- The bytecode VM is the initial execution backend; Cranelift is the
  subsequent native backend.
