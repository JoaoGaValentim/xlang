# Development Roadmap

Restored transcription of the authoritative development plan (original
specification §6, archived at `docs/spec-v0.1-original.txt`). Ten sequential
phases, numbered 0 through 9. **No phase may be reordered, and no phase
begins before the previous one is green and the project owner issues a
directive authorizing it** (`docs/governance.md` §1). Estimates are from the
original and are not commitments.

Status legend: **ACCEPTED** · **IN PROGRESS** · **DESIGN GATE** ·
**NOT STARTED**.
Planned content below describes what a phase is authorized to build when it
begins; the existence of a roadmap item does not authorize its
implementation.

| Phase | Name | Original estimate | Milestone (original §6) | Status |
| --- | --- | --- | --- | --- |
| 0 | Foundation (*Fundação*) | 1–2 weeks | Workspace, CI, initial specification | ACCEPTED — integrated via PR #1 (merge `4d82106`); see `docs/phase-status.md` |
| 1 | Lexical and syntactic frontend (*Frontend léxico e sintático*) | 3–4 weeks | `xc parse arquivo.x` prints the AST | IN PROGRESS — contracts ratified (Directive 006); lexer integrated (PR #3, merge `9ed50ad`); AST implemented, pending owner review (Directive 007); parser/CLI not started |
| 2 | Name resolution and types (*Resolução de nomes e tipos*) | 6–8 weeks | `xc check` with precise type errors | NOT STARTED |
| 3 | MIR | 3–4 weeks | `xc mir` prints the CFG | NOT STARTED |
| 4 | Bytecode VM (*VM de bytecode*) | 4–6 weeks | `xc run hello.x` executes programs | NOT STARTED |
| 5 | Complete language (*Linguagem completa*) | 8–10 weeks | structs, enums, match, traits, generics | NOT STARTED |
| 6 | Runtime and stdlib (*Runtime e stdlib*) | 6–8 weeks | Standard library and I/O | NOT STARTED |
| 7 | Native backend (*Backend nativo*) | 8–12 weeks | `xc build` produces an executable | NOT STARTED |
| 8 | Tooling (*Ferramental*) | 6–8 weeks | `xc fmt`, LSP, `xc pkg` | NOT STARTED |
| 9 | Optimization and stabilization (*Otimização e estabilização*) | 6–8 weeks | xlang 1.0 | NOT STARTED |

Dependencies for every phase: the previous phase must be green (all CI
gates pass on the delivered revision) and the project owner must issue a
directive authorizing the phase.

## Phase 0 — Foundation — ACCEPTED

Original deliverables (original §6):

- Repository with the Cargo workspace.
- `xl-span` and `xl-diag` crates.
- Specification document (`docs/spec.md`).
- CI (GitHub Actions) with `cargo fmt --check`, `clippy -D warnings`,
  `cargo test`.
- License and contribution guide.
- ADRs (architecture decision records).

Original acceptance criteria (original §6):

- `cargo test --workspace` green.
- A sample diagnostic rendered with span and label.

Final state (2026-10-12, Directives 002–004):

- All deliverables implemented and verified; PR #1 merged as `4d82106`
  with green CI; the project owner formally accepted Phase 0 (Directive 004
  §2, integration condition satisfied). Evidence in `docs/phase-status.md`.
- `docs/spec.md`: reconciled with the archival original (PD-001 resolved).
- License: dual `MIT OR Apache-2.0` (PD-008 resolved, Directive 003).
- ADR-0002 (source position conventions): accepted (PD-007 resolved,
  Directive 003).

## Phase 1 — Lexical and syntactic frontend — IN PROGRESS

Contracts ratified (Directive 006; ADR-0004). The lexical frontend
(`xl-lexer`) is implemented and integrated (PR #3, merge `9ed50ad`); the
syntax tree with canonical printing and structural validation (`xl-ast`)
is implemented and pending owner review (Directive 007). The parser and
CLI remain **not authorized / not started**; the Phase 1 acceptance
criteria (all specification examples parse; combined lexer/parser
fuzzing) therefore cannot yet be claimed.

Scope and deliverables (original §6):

- Complete lexer: Unicode, literals, nested comments, interpolated strings.
- Recursive-descent + Pratt parser for every construction of the grammar
  (original §3 EBNF).
- Error recovery (synchronization at `;` and `}`) to report multiple errors
  per run.
- AST pretty-printer.
- `xc parse`.

Tests (original §6): snapshot tests (insta) of tokens and AST; fuzzing of
the lexer/parser (no panics); the property `parse(print(ast)) == ast`.

Acceptance criteria (original §6): 100% of the specification examples parse;
1 hour of fuzzing without failures.

Dependencies: Phase 0 accepted (satisfied). The design gate is active: the
proposed contracts in `docs/proposals/phase-1/` require owner approval
before implementation, and an implementation directive is still required.
The former open questions (PD-002…PD-006) are addressed by those proposals
and remain formally open until the owner approves the decision packet.

## Phase 2 — Name resolution and type system — NOT STARTED

Scope and deliverables (original §6):

- AST → HIR lowering with desugaring (`for` → `loop` + iterator, `?` →
  `match`, string interpolation → concatenation).
- Symbol table with scopes and modules.
- Detection of duplicate and undefined names, and of cyclic imports.
- Type checker (original §7).
- Match exhaustiveness checking (Maranget's algorithm).
- Mutability checking (`let mut`).

Tests (original §6): `compile-fail/` suite where each case has a `.x` file
plus expected `.stderr`.

Acceptance criteria (original §6): error messages with a primary span,
secondary labels, and a suggestion (e.g., “você quis dizer foo?”).

Dependencies: Phase 1 green; import/visibility semantics (PD-006) must be
resolved.

## Phase 3 — MIR — NOT STARTED

Scope and deliverables (original §6):

- Control-flow-graph IR: basic blocks, typed locals, terminators
  `goto`/`switch`/`call`/`return`.
- Lowering from typed HIR to MIR.
- Pattern matching compiled to decision trees.
- MIR verifier (structural sanity).
- Simple passes: constant propagation, dead code elimination, CFG
  simplification.

Acceptance criteria (original §6): the MIR verifier passes on all test
programs; `xc mir` prints a stable textual format.

Dependencies: Phase 2 green.

## Phase 4 — Bytecode VM (first execution) — NOT STARTED

Scope and deliverables (original §6):

- `xl-bytecode`: Chunk format, constant table, `.xbc` serialization.
- MIR → bytecode generator.
- Stack-based VM with call frames.
- Values: `Int`, `Float`, `Bool`, `Char`, `Str(Rc<str>)`, `Obj(Rc<…>)`.
- Panics with backtraces mapped to spans.
- `xc run`.

Tests (original §6): `run-pass/` with expected output (`.stdout`);
benchmarks (fib(35), numeric loops, sorting).

Acceptance criteria (original §6): `hello.x`, factorial, fibonacci, and
FizzBuzz execute correctly; fib(35) below 2 s.

Dependencies: Phase 3 green.

## Phase 5 — Complete language — NOT STARTED

Scope and deliverables (original §6):

- `struct`/`enum` with payload; `impl` and methods.
- Traits with static resolution and bounds (`fn f<T: Ord>(a: T)`).
- Generics with monomorphization.
- Closures with capture.
- Tuples and arrays.
- `Option`/`Result` in the prelude.
- The `?` operator.
- Inference with integer defaulting.
- Modules and `pub`.

Acceptance criteria (original §6): reference programs (expression
interpreter, linked list, binary tree, JSON parser) compile and run.

Dependencies: Phase 4 green. Syntax is frozen only after this phase;
changes require an ADR (original §10).

## Phase 6 — Runtime and standard library — NOT STARTED

Scope and deliverables (original §6):

- core (`Option`, `Result`, `Ordering`, `Iterator`).
- collections (`Vec<T>`, `Map<K,V>`, `Set<T>`).
- string; math; io (stdin/stdout/stderr); fs; time; env.
- Value formatting (`Display`, `Debug`).
- Panic handling and exit codes.
- Documented memory strategy (RC; cycle policy: explicit `Weak<T>`).

Acceptance criteria (original §6): stdlib 100% documented with executable
examples (doctests); test coverage ≥ 85%.

Dependencies: Phase 5 green.

## Phase 7 — Native backend — NOT STARTED

Scope and deliverables (original §6):

- `xl-codegen` with Cranelift: MIR → Cranelift IR.
- Own ABI and calling convention, documented.
- Native runtime in Rust (allocator, RC, panics) linked statically.
- Full monomorphization.
- Linking via cc/lld.
- `xc build` and `xc build --release`.
- Automatic differential test: VM × native (same output for the whole
  suite).

Acceptance criteria (original §6): the whole `run-pass/` suite produces
identical output on both backends; fib(35) native in under 100 ms.

Dependencies: Phase 6 green. `xl-vm` and `xl-codegen` are the only crates
permitted unsafe code (original §4), with review and Miri testing.

## Phase 8 — Tooling — NOT STARTED

Scope and deliverables (original §6):

- `xc fmt`: idempotent formatter, AST + trivia based.
- LSP (tower-lsp): diagnostics, go-to-definition, hover with types,
  autocomplete, rename.
- `xc test` (`#[test]` functions).
- `xc doc` (HTML from `///` comments).
- Package manager `xc pkg` with `xl.toml`, `xl.lock`, and a registry.
- REPL.
- TextMate/tree-sitter grammar and VS Code extension.

Acceptance criteria (original §6): `fmt(fmt(x)) == fmt(x)` across the whole
codebase; LSP responds in under 100 ms on 5-thousand-line files.

Dependencies: Phase 7 green.

## Phase 9 — Optimization and stabilization — NOT STARTED

Scope and deliverables (original §6):

- MIR optimizations (inlining, CSE, elimination of proven bounds checks,
  escape analysis to avoid RC).
- Incremental compilation and per-module cache.
- Parallelization of type checking per module.
- Specification freeze, semantic-versioning policy, editions.
- Complete documentation (tutorial, reference, book).
- Security audit with Miri and continuous fuzzing.

Acceptance criteria (original §6): the criteria of original §10 (v1.0
definition of done; see `docs/spec.md` §10).

Dependencies: Phase 8 green.

## Cross-phase constraints (from the original)

- The pipeline order is a binding contract (original §4): Source → Lexer →
  Parser → AST → name resolution + desugaring / HIR → type checker → typed
  HIR → MIR → optimization → Bytecode VM / Cranelift.
- The bytecode VM is the initial execution backend; Cranelift is the
  subsequent native backend (original §2, §4).
- Diagnostics quality is a requirement from Phase 2 onward, with `.stderr`
  tests (original §6, §9–§10).
- `#![forbid(unsafe_code)]` in all crates except `xl-vm` and `xl-codegen`
  (original §4).
- Syntax is frozen only after Phase 5; any later syntax change requires an
  ADR (original §10).
