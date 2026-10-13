# Compiler Architecture

## 1. Planned pipeline (contract)

The full XLANG compiler pipeline is fixed by the specification:

```text
Source → Lexer → Parser → AST → Name Resolution / HIR → Type Checking
       → Typed HIR → MIR → Optimization → Bytecode VM / Cranelift
```

Execution backends: a bytecode VM first, Cranelift-generated native code
as a subsequent backend.

## 2. Implemented subset (Phases 0–1)

Phase 0 implements the source-location and diagnostics foundation;
Phase 1 implements the lexical frontend (`xl-lexer`, integrated via PR #3)
and the syntax tree with canonical printing and structural validation
(`xl-ast`, Directive 007). **No parser, CLI, HIR, type checking, MIR,
bytecode, VM, native code generation, standard library, formatter, LSP, or
package management code exists in this repository.**

```text
┌──────────────────────────┐
│ xl-span                  │  source registration, FileId, Span,
│ (source location infra)  │  line/column mapping, UTF-8 validation
└────────────┬─────────────┘
             │ depends on
┌────────────▼─────────────┐
│ xl-diag                  │  Severity, Label, Diagnostic, deterministic
│ (diagnostics infra)      │  source-aware text rendering
└────────────┬─────────────┘
             │ depends on
┌────────────▼─────────────┐
│ xl-lexer                 │  Token/TokenKind, structured strings with
│ (lexical frontend)       │  interpolation, deterministic diagnostics
└──────────────────────────┘   (future xl-parser depends on this)

┌──────────────────────────┐
│ xl-ast                   │  Item/Stmt/Expr/Pat/Type model, canonical
│ (syntax tree + printer)  │  printer, structural validator
└──────────────────────────┘   (depends on xl-span only)
```

Dependency direction: `xl-lexer → {xl-span, xl-diag}` and
`xl-ast → xl-span`; the future `xl-parser` will depend on both. Foundation
crates must never depend on later-phase crates. All crates build with
`#![forbid(unsafe_code)]` and hold no global mutable state; every piece of
compiler state is owned by the caller and passed explicitly.

## 3. Public contracts

### `xl-span`

- `FileId` — identifier for a registered source file. **Map-scoped:** an
  identifier is only meaningful with the `SourceMap` instance that produced
  it. Raw indices can alias across distinct maps (index 0 of one map is a
  different file than index 0 of another), and in-range cross-map use
  cannot be detected by lookups; only out-of-range indices are rejected,
  as `SpanError::UnknownFile`. Spans must always be resolved against the
  map they were created with. Pinned by the regression test
  `file_ids_from_distinct_maps_alias_by_raw_index`.
- `Span { file, lo, hi }` — half-open UTF-8 byte range; fields public by
  specification contract.
- `SourceMap` — owns registered sources; `add_file`, `file`, `source`,
  `span_slice`, `line_col`, `line_col_range`, `line_text`.
- `LineCol` — 1-based line/column (columns in Unicode scalar values).
- `SpanError` — every invalid position is reported through `Result`, never
  a panic.

See `docs/decisions/adr-0002-source-position-conventions.md` for the offset
and column conventions.

### `xl-diag`

- `Severity` — `Error`, `Warning`, `Note`.
- `Label { span, message }` — a span with its display text.
- `Diagnostic` — severity, message, primary label, secondary labels;
  constructed through builders.
- `render(&Diagnostic, &SourceMap) -> String` — deterministic plain-text
  rendering; unresolvable spans produce an explicit placeholder instead of
  a panic.

### `xl-lexer`

- `lex(&SourceFile, FileId) -> LexOutput { tokens, diagnostics }` and the
  `lex_file(&SourceMap, FileId)` convenience — total, deterministic, never
  panics; exactly one final `Eof` token in the top-level stream.
- `Token { kind, span }` / `TokenKind` — identifiers (owned `String`; no
  global interner), integer literals (`IntLit { magnitude, radix }`, B-2),
  finite floats (B-3), characters, structured strings
  (`StrLit { parts }` with `Text`/`Interp` segments and absolute spans,
  §5.5 invariants), all keywords/operators/delimiters, `Underscore`,
  `Error` (recovery), `Eof`.
- Interpolation recursion is bounded by `MAX_INTERPOLATION_DEPTH = 64`
  (B-4); comments use iterative depth counting.
- Dependencies: `xl-span`, `xl-diag`, `unicode-ident`. No parser
  dependency.

### `xl-ast`

- `Item` (root: a parsed file is a sequence of items) and the full
  `StmtKind`/`ExprKind`/`PatKind`/`TypeKind` model of the accepted
  grammar; `Block { stmts, tail }` keeps the tail expression separate by
  construction.
- Node identifiers (`ItemId`, `ExprId`) are parser-assigned and
  **instance-scoped** (not globally unique); `xl-ast` performs no
  allocation.
- `print_item`/`print_items`/`print_expr`/`print_type`/`print_pattern`/
  `print_block` — deterministic canonical printer preserving tree
  structure under the accepted precedence and statement/tail rules;
  structured `PrintError` for malformed or over-deep trees
  (`MAX_PRINT_DEPTH = 128`); `inf`/`NaN` are never emitted.
- `validate_item`/`validate_expr`/`validate_type`/`validate_pattern` —
  structural `Violation` reports (span sanity/containment, file
  consistency, required names/paths/generics, tail ordering,
  interpolation ordering, `MAX_VALIDATE_DEPTH = 128`); no name resolution
  or typing.
- Dependencies: `xl-span` only (`xl-lexer` is a dev-only test dependency
  used to re-lex printed output).

## 4. Design principles

- **Determinism.** Identical inputs produce identical results; rendering is
  a pure function and no timestamps, environment data, or unordered
  iteration affect output. (INV-007)
- **Controlled failure.** Invalid user-derived positions surface as
  `SpanError` values. No `unwrap`/indexing on untrusted offsets.
  (INV-006)
- **Minimal dependencies.** Both crates currently have **zero external
  dependencies**; `xl-diag` depends only on `xl-span` within the workspace.
  A new dependency requires a documented technical justification.
  (INV-011)
- **No unapproved semantics.** Phase 0 makes no statement about language
  grammar or semantics; see `docs/pending-decisions.md`.

## 5. Testing strategy

- Unit tests for internal invariants (line index construction).
- Integration tests against the public API per crate
  (`crates/*/tests/`).
- One workspace-level cross-crate integration test in `tests/`, wired as an
  explicit test target (`cross_crate`) so it runs under
  `cargo test --workspace`. It renders a complete sample diagnostic
  (file location, source line, highlighted span, labels, message) and is
  the reference artifact requested for Phase 0.
- Property-based testing is deferred: the current invariants are covered by
  targeted edge-case tests, and Phase 0 keeps dependencies minimal.

## 6. Continuous integration

`.github/workflows/ci.yml` runs on pushes to `main`, pull requests
targeting `main`, and manual dispatch. Quality gates (exact commands):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The format gate runs on Ubuntu; the clippy and test gates run on a matrix
of Ubuntu, Windows, and macOS with `fail-fast: false`.

Security posture:

- Workflow-level `permissions: contents: read`; no other permissions.
- Third-party actions pinned to immutable commit SHAs:

  | Action | Pin | Upstream release |
  | --- | --- | --- |
  | `actions/checkout` | `3d3c42e5aac5ba805825da76410c181273ba90b1` | v7.0.1 (2026-07-20) |
  | `Swatinem/rust-cache` | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` | v2.9.2 (2026-08-06) |

- The Rust toolchain is installed by rustup itself (`rustup install`), which
  resolves the pinned version and components from `rust-toolchain.toml`.
  There is no third-party toolchain action, and CI therefore uses exactly
  the toolchain configuration used locally.
- No secrets are referenced, printed, or required; no release, deploy, or
  merge automation exists.
- Automated dependency updates are not enabled (not authorized).
