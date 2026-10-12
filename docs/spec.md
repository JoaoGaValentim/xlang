# XLANG Language Specification — Working Record

- **Document:** xlang — Engineering Specification and Development Plan
  (original Portuguese title: *xlang — Especificação e Plano de Engenharia*)
- **Version:** 0.1
- **Date:** October 12, 2026
- **Original language:** Portuguese
- **Archival original:** `docs/spec-v0.1-original.txt` — byte-faithful copy of
  the owner-supplied document (SHA-256
  `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770`; the
  original has no trailing newline and the copy preserves that). Compare the
  archive directly with the owner-provided attachment using this hash.
- **Status:** reconciled working record — derived from the archival original
  on 2026-10-12 under Engineering Directive 002.

> **What this file is.** A derived index and overlay over the archival
> original. It adds no language semantics of its own: every statement is
> either a restatement of the original (labeled SPECIFIED, with the
> original's section referenced), an explicitly labeled implementation
> choice (PROPOSED), or an open question (UNRESOLVED, tracked in
> `docs/pending-decisions.md`). Where this record and the original differ,
> the original wins until an owner-approved specification change says
> otherwise (`docs/governance.md` §10). The archival copy is unmodified and
> untranslated.

## 0. Specification authority hierarchy

From highest authority to lowest:

1. **`docs/spec-v0.1-original.txt`** — the owner-supplied original document
   v0.1 (12/10/2026). Authoritative for all language semantics, grammar,
   architecture, development phases, and acceptance criteria.
2. **Owner engineering directives** (Directive 001, Directive 002, …) —
   authoritative for process, scope authorization, and any point a directive
   explicitly decides. A directive does not change language semantics
   silently; where a directive interprets the original, the interpretation
   is recorded here.
3. **This working record (`docs/spec.md`)** — an index with no independent
   authority. Documentation defects here are corrected against the original.
4. **Accepted ADRs (`docs/decisions/`)** — decisions made under the
   specification; they must not contradict it. `Proposed` ADRs are not
   binding.
5. **Implementation and all other documentation** — lowest; code never
   defines semantics (INV-001, `docs/governance.md` §10).

## 1. Overview, objectives, and non-goals (original §1) — SPECIFIED

xlang is a compiled, statically typed, expression-oriented language with
type inference, algebraic data types, pattern matching, and errors as
values. The compiler (`xc`) is written in Rust and organized as a workspace
of small crates, each with a clear, testable contract. Execution starts on a
bytecode VM and evolves to native code via Cranelift.

Objectives (original §1):

- Small, unambiguous syntax, parseable by recursive descent + Pratt.
- Safety by default: no null, no undefined behavior in safe code.
- High-quality diagnostics (spans, labels, suggestions).
- Modular, deterministic, incremental-friendly pipeline.
- First-class tooling: CLI, formatter, LSP, package manager.

Non-goals for v1.0 (original §1): concurrency, procedural macros, a full
borrow checker, and FFI beyond basic C.

## 2. Design decisions (original §2) — SPECIFIED

| Theme | Decision | Rationale |
| --- | --- | --- |
| Typing | Static; bidirectional inference + unification (local Hindley–Milner style) | Ergonomics without losing predictability |
| Memory | Reference counting (Rc) in the VM; simple regions/ownership in the native phase | Simplicity first, optimization later |
| Errors | `Result<T, E>` and the `?` operator; panic only for bugs | No exceptions; explicit flow |
| Polymorphism | Monomorphized generics + traits | Performance in the native backend |
| Data | `struct`, `enum` with payload, tuples, arrays | Expressive modeling |
| Modules | 1 `.x` file = 1 module; packages described by `xl.toml` | Simple mental model |
| Syntax | `{}` blocks, statements terminated by `;`, the last expression of a block is its value | Familiar and unambiguous |
| Backends | Bytecode VM → Cranelift (native) | Fast feedback, then performance |

## 3. Language surface (original §3) — SPECIFIED, with tracked gaps

The original contains this example program (binding as an illustration of
surface syntax; quoted verbatim from the archival copy):

```xlang
import std.io;

struct Point { x: f64, y: f64 }

impl Point {
    pub fn dist(self, o: Point) -> f64 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape { Circle(f64), Rect(f64, f64) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rect(w, h) => w * h,
    }
}

fn parse(s: str) -> Result<i64, str> {
    let n = s.to_int()?;
    if n < 0 { Err("negativo") } else { Ok(n) }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 { total += i as f64; }
    io.println("total = {total}");
}
```

Established surface facts (traceable index):

- **Files and modules (original §2).** Source files use the `.x` extension;
  one file is one module; packages are described by `xl.toml`.
- **Lexical (original §3).** Unicode identifiers (XID); integers with `_`
  separators and `0x`/`0b` prefixes (`1_000`, `0xFF`, `0b1010`); floats;
  strings with escapes and `{expr}` interpolation; `char`; line comments
  `//` and nested block comments `/* */`.
- **Keywords (original §3).** `fn let mut if else match while for in loop
  break continue return struct enum impl trait import pub as true false`.
- **Primitive types (original §3).** `i8..i64`, `u8..u64`, `isize`, `usize`,
  `f32`, `f64`, `bool`, `char`, `str`, `()`. Compound types: `[T]`,
  `[T; N]`, `(A, B)`, `Option<T>`, `Result<T, E>`, `fn(A) -> B`.
- **Grammar sketch (original §3, EBNF).** module / item / fn_decl / block /
  stmt / pattern definitions; the expression grammar is specified as a
  Pratt parser with `postfix = primary { "(" args ")" | "." IDENT |
  "[" expr "]" | "?" | "as" type }`. The Pratt note fixes the precedence
  chain assignment < `||` < `&&` < comparison < `+ -` < `* / %` < unary <
  postfix.
- **Functions, structs, enums, impls, traits, generics.** `fn` declarations
  with optional `pub` and optional generics (`fn f<T: Ord>(a: T)` appears in
  Phase 5 scope, original §6); `struct`/`enum` declarations with payloads;
  `impl Type { … }` blocks with methods taking `self`; `trait` declarations;
  pattern matching over enum payloads.
- **Bindings and blocks.** `let` / `let mut`; blocks `{ … }` whose last
  expression is the block's value; `if`/`else`, `match`, `while`,
  `for … in …`, `loop`, `break`, `continue`, `return`.
- **Errors as values.** `Result<T, E>` in the prelude (original §6 Phase 5);
  `?` propagation; `Err(…)` / `Ok(…)` construction; panic reserved for bugs
  (original §2, §8).
- **Numeric behavior and casts (original §7).** No implicit coercion between
  numerics; explicit `as`; integer literals default to `i64` and float
  literals to `f64` when ambiguous.

Language-surface areas the original does **not** fully define are tracked as
pending decisions and are not resolved here: operator table completeness
(PD-002), assignment semantics (PD-003), interpolation representation
(PD-004), exact precedence and associativity (PD-005), module and visibility
semantics (PD-006).

## 4. Compiler architecture (original §4) — SPECIFIED

Pipeline (binding contract, original §4):

```text
Source .x → Lexer → tokens → Parser → AST → name resolution + desugaring →
HIR → type checker → typed HIR → lowering → MIR (control-flow graph) →
optimizations → Bytecode/VM or Cranelift/native
```

Workspace layout target (original §4; this is the v1.x layout — Phase 0
creates only the two foundation crates, consistent with the original's
Phase 0 deliverables):

```text
xlang/
├─ Cargo.toml                # [workspace]
├─ crates/
│  ├─ xl-span/               # FileId, Span, SourceMap
│  ├─ xl-diag/               # Diagnostic, Label, rendering
│  ├─ xl-lexer/              # text → tokens
│  ├─ xl-ast/                # AST nodes + visitor
│  ├─ xl-parser/             # tokens → AST (error recovery)
│  ├─ xl-hir/                # name resolution, desugar
│  ├─ xl-types/              # inference, traits, exhaustiveness
│  ├─ xl-mir/                # CFG, lowering, optimizations
│  ├─ xl-bytecode/           # Op, Chunk, serialization
│  ├─ xl-vm/                 # interpreter + runtime
│  ├─ xl-codegen/            # Cranelift backend
│  ├─ xl-driver/             # orchestrates the pipeline
│  ├─ xl-fmt/                # formatter
│  ├─ xl-lsp/                # LSP server
│  └─ xl-cli/                # `xc` binary
├─ stdlib/                   # standard library written in .x
├─ tests/{ui,run-pass,compile-fail}/
└─ docs/
```

Implementation conventions (original §4) — binding:

- Newtype IDs (`ExprId`, `DefId`, `TyId`) instead of pointers; data in
  arenas/Vec.
- Every error becomes a `Diagnostic` with a `Span`; the compiler never
  panics on user input.
- Phases are pure functions: `fn(&Input) -> (Output, Vec<Diagnostic>)`.
- `#![forbid(unsafe_code)]` in all crates except `xl-vm` and `xl-codegen`
  (which require review and Miri-tested code).

Recommended Rust libraries (original §4) — guidance for later phases, not
Phase 0 requirements: clap, lasso, ariadne or codespan-reporting, thiserror,
bumpalo, cranelift-*, tower-lsp, insta, proptest, cargo-fuzz, criterion.

## 5. Key code skeletons (original §5) — SPECIFIED contracts

The original fixes the `Span` representation (binding):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span { pub file: FileId, pub lo: u32, pub hi: u32 }
```

It also sketches, for later phases only, `TokenKind`/`Token`, `Expr` /
`ExprKind`, the Pratt core loop, the `Op` bytecode enum, and the VM main
loop. These sketches guide Phases 1–4; they do not authorize implementation
in Phase 0. The unit of `lo`/`hi`, line/column bases, and invalid-position
behavior are not defined by the original; the Phase 0 choices are recorded
in `docs/decisions/adr-0002-source-position-conventions.md`, accepted by
the project owner via Directive 003 (PD-007 resolved).

## 6. Development phases (original §6) — SPECIFIED

Ten sequential phases, 0 through 9. Each phase ends with an executable
milestone and CI-verifiable acceptance criteria; no phase starts before the
previous one is green. Full transcription with deliverables, tests,
acceptance criteria, estimates, and dependencies:
`docs/roadmap.md`. Currently implemented: **Phase 0 only** — see
`docs/phase-status.md`.

## 7. Type system (original §7) — SPECIFIED

- Inference: fresh type variables, unification with occurs check,
  generalization only on `fn` (restricted let-polymorphism), bidirectional
  checking (infer and check modes) for better messages.
- Traits: declarations, `impl Trait for Type`, bounds; resolution by
  coherence (one implementation per trait/type pair; orphan rule);
  associated types in v1.1.
- Generics: monomorphized in the backend; depth limit to avoid infinite
  expansion.
- Conversions: no implicit coercion between numerics; explicit `as`;
  integer literals default to `i64`, float literals to `f64`, when
  ambiguous.
- Exhaustiveness and reachability of patterns; non-exhaustive `match` is an
  error.
- Mutability: `let` immutable by default; assignment requires `mut`.

## 8. Runtime and memory model (original §8) — SPECIFIED

- VM: tagged values; objects in `Rc<Obj>`; strings immutable, interned when
  small.
- Native: C-like struct layout; enums as tagged unions; `Rc` with a
  non-atomic counter (no concurrency in v1); replaceable allocator.
- Failures: panic prints message + span + backtrace and exits with code
  101; integer overflow checked in debug and wrapping in release
  (`checked_*` / `wrapping_*` methods available).

## 9. Quality and process (original §9) — SPECIFIED

- Unit tests per crate; coverage measured with cargo-llvm-cov.
- Snapshots (insta) for tokens, AST, HIR, MIR, bytecode, and diagnostics.
- Integration: `tests/run-pass` (expected stdout) and `tests/compile-fail`
  (expected stderr).
- Fuzzing with cargo-fuzz on lexer, parser, and bytecode verifier.
- Properties with proptest: parse/print round-trip, idempotent `fmt`.
- Differential testing: VM × native over the whole suite.
- Performance: criterion + regression monitoring in CI.
- Security: Miri on unsafe code, cargo-audit, cargo-deny.
- Process: small PRs, ADRs, changelog, semantic releases.

## 10. Risks and v1.0 definition of done (original §10) — SPECIFIED

Risks (original §10): type-system scope (mitigate with a minimal subset in
Phase 5 and coherence tests); native backend complexity (keep the VM as
reference and differential-test); diagnostics quality (a requirement from
Phase 2, with `.stderr` tests); design debt (freeze syntax only after
Phase 5; ADR mandatory for syntax changes).

Definition of done (v1.0):

1. Specification published and consistent with the implementation.
2. Full test suite green on Linux, macOS, and Windows.
3. VM and native backend with identical behavior.
4. `xc` with `build`, `run`, `check`, `test`, `fmt`, `doc`, and `pkg`
   stable.
5. Functional LSP and a published VS Code extension.
6. Standard library documented, with doctests.
7. No compiler panic in 24 h of continuous fuzzing.
8. Reference programs (JSON parser, interpreter, simple TCP server)
   working.

## 11. Bootstrap recipe (original §11) — historical

The original ends with the concrete bootstrap recipe (`cargo new --lib`
plus one `cargo new` per crate, `cargo new crates/xl-cli --name xc`). The
repository has since been bootstrapped under phase authorization; the
recipe describes the v1.x target layout of §4, not Phase 0 scope. Phase 0
created the workspace plus `xl-span` and `xl-diag` only.

## 12. Specified vs proposed vs unresolved — summary

- **SPECIFIED by the original:** everything indexed in §1–§10 above,
  including the `Span` struct, the pipeline, the workspace target layout,
  implementation conventions, phase structure, type-system rules, runtime
  model, quality process, and the v1.0 definition of done.
- **PROPOSED (Phase 0 implementation choices, silent in the original):**
  offset unit and half-open spans, 1-based line numbers and 1-based
  scalar-counted columns, CRLF display handling, `SpanError`-based
  invalid-position handling, `xl-span` / `xl-diag` public API shapes, the
  plain-text diagnostic rendering format, toolchain pinning and edition,
  dependency direction, and `FileId` map-scoping. Recorded in ADR-0002
  (`Accepted` via Directive 003; PD-007 resolved) and ADR-0003 (Accepted
  by delegation of Directive 001).
- **UNRESOLVED (original silent or partial):** PD-002 through PD-010, with
  PD-002…PD-006 now partially informed by the original. See
  `docs/pending-decisions.md`; nothing in this list may be filled in from
  assumption.

## 13. Consistency audit

The traceable comparison between the archival original and the repository's
architectural records (Directive 002 §3) is maintained in
`docs/spec-consistency-audit.md`, with CONTRADICTION / UNDERSPECIFIED /
CONSISTENT classifications and evidence.

## 14. Change control

No semantic rule may be added, removed, or altered without an approved
specification change recorded according to `docs/governance.md` §10–11. Any
difference between the original document and this record is a
specification-change event.
