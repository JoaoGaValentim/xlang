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
Phase 1 implements the lexical frontend (`xl-lexer`, integrated via PR #3),
the syntax tree with canonical printing and structural validation
(`xl-ast`, integrated via PR #4), the syntactic parser (`xl-parser`,
integrated via PR #5), and the command-line interface (`xl-cli`,
Directive 009, pending owner review). **No HIR, name resolution, type
checking, MIR, bytecode, VM, native code generation, standard library,
formatter, LSP, or package management code exists in this repository.**

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
└────────────┬─────────────┘
             │ depends on
┌────────────▼─────────────┐
│ xl-parser                │  tokens → xl-ast items + diagnostics;
│ (syntactic frontend)     │  Pratt expressions, recovery, depth limits
└────────────┬─────────────┘
             │ depends on
┌────────────▼─────────────┐
│ xl-cli                   │  `xc` binary: `xc parse <file.x>` renders
│ (command-line interface) │  diagnostics (stderr) and canonical AST
└──────────────────────────┘   (stdout); exit codes 0/1/2

┌──────────────────────────┐
│ xl-ast                   │  Item/Stmt/Expr/Pat/Type model, canonical
│ (syntax tree + printer)  │  printer, structural validator
└──────────────────────────┘   (depends on xl-span only)
```

Dependency direction: `xl-lexer → {xl-span, xl-diag}`,
`xl-ast → xl-span`, `xl-parser → {xl-lexer, xl-ast, xl-span, xl-diag}`,
and `xl-cli → {xl-parser, xl-ast, xl-span, xl-diag}`. Foundation crates
must never depend on later-phase crates. All crates build with
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

### `xl-parser`

- `parse(&SourceFile, FileId) -> ParseOutput { items, diagnostics }` runs
  the full pipeline (lex + parse); `parse_file(&SourceMap, FileId)` is the
  lookup convenience; `parse_tokens(&[Token], FileId)` parses an existing
  stream (lexical diagnostics then remain the caller's responsibility).
  Diagnostic order is deterministic: lexical (scan order) then syntactic
  (parse order); `has_errors`/`error_count` summarize severity.
- The parser is a recursive-descent item/statement/block parser with a
  Pratt expression loop using the ratified binding powers; non-associative
  tiers (assignment, range, comparison) are enforced explicitly with the
  accepted diagnostics and recovery shapes (right-nested assignment,
  left-nested range/comparison). Grouping is transparent in the AST but
  not to chain detection.
- All recursion (expressions, types, patterns, blocks, items, generic
  arguments, interpolation interiors) shares one budget,
  `MAX_PARSE_DEPTH = 64` (B-4); exceeding it produces
  `expression nesting limit (64) exceeded`, skips to the expression
  synchronization set, and continues.
- Recovery uses the ratified synchronization sets; recovered constructs
  are zero-width placeholder nodes at deterministic insertion points and
  are never presented as successful parses (empty identifiers make
  canonical printing report `InvalidStructure`).
- Node identifiers are allocated in construction order per AST instance
  (saturating counters; never global). Generic closing implements the G-3
  rule: `>=` is split into `>` + `=` only in type-closing context, with
  byte-exact residual spans (no token dropped or duplicated).
- Dependencies: `xl-lexer`, `xl-ast`, `xl-span`, `xl-diag`. No CLI or
  later-stage dependency; no name resolution, typing, or desugaring.

### `xl-cli`

- Binary `xc` (package `xl-cli`); Phase 1 authorizes exactly one command:
  `xc parse <file.x>`, plus `xc --help`/`-h` and `xc --version`/`-V`.
  There is no argument-parsing dependency; the surface is parsed by a
  small hand-written, I/O-free, panic-free function. No other subcommands,
  flags, or output formats exist.
- `xc parse` reads the file with recoverable I/O (`std::fs::read`, no
  panics and no `unsafe`), decodes it as UTF-8, registers it in an
  `xl-span::SourceMap`, parses it through the accepted
  `xl_parser::parse_file` API, renders every diagnostic with
  `xl_diag::render` on stderr (file location, line, column, highlighted
  span, labels), and prints `xl_ast::print_items` output — the accepted
  canonical printer, never a CLI-specific printer — plus one trailing
  newline on stdout. An empty module (trivia-only source) prints nothing.
- The command is syntax-only: it never resolves names, checks types,
  compiles, or executes the program. Undefined names are not Phase 1
  errors. The canonical output is normalized XLANG syntax for the parsed
  tree, not compiled code or runtime output.
- **Exit-code policy (CLI contract, not language semantics):**
  `0` = parse produced no error-level diagnostics (warnings and notes alone
  do not fail); `1` = lexical or syntactic errors — diagnostics on stderr
  and nothing on stdout, so a recovered tree is never presented as valid
  source; `2` = usage errors (missing/unknown command, missing or
  excessive arguments, unknown flags), file handling errors (unreadable,
  missing, invalid UTF-8, byte-empty input, source beyond the `u32`
  span-addressing limit, which is rejected from file metadata before it is
  read), or unexpected internal failures (defensive paths; not reachable
  for parser-produced trees).
- **CLI policy notes:** a byte-empty input file is rejected as an
  operational error (exit `2`) because there is nothing to parse; a file
  containing only trivia is a valid empty module (exit `0`, no output).
  Any positional argument after `parse` that begins with `-` is treated as
  a flag and rejected (`--help` is accepted only at the top level; use
  `./-name.x` to parse a file whose name begins with `-`). File names are
  not required to end in `.x`. I/O error messages embed the operating
  system's error text and are therefore platform-specific; all other
  messages are stable byte-for-byte.
- **Known diagnostic-rendering limitations** (inherited from Phase 0
  `xl-diag` and tracked in `docs/pending-decisions.md`): rendering is
  plain text with no colors and no stable error codes; no terminal-width
  wrapping (long source lines are printed in full); multi-line spans
  highlight only the first line; columns count Unicode scalar values, so
  alignment assumes one display column per scalar value (wide characters
  and tabs can misalign the caret).
- Dependencies: `xl-parser`, `xl-ast`, `xl-span`, `xl-diag`. It uses
  `xl-lexer` only indirectly through the parser. No network access, no
  source execution, and no filesystem access beyond reading the requested
  file.

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

- Unit tests for internal invariants (line index construction, argument
  parsing, severity-to-exit-status policy).
- Integration tests against the public API per crate
  (`crates/*/tests/`).
- One workspace-level cross-crate integration test in `tests/`, wired as an
  explicit test target (`cross_crate`) so it runs under
  `cargo test --workspace`. It renders a complete sample diagnostic
  (file location, source line, highlighted span, labels, message) and is
  the reference artifact requested for Phase 0.
- Lexer, parser, and AST suites include exact-output assertions, `insta`
  snapshots, property tests (`proptest`), span/identifier invariants, and
  the `parse(print(ast)) ≡ ast` round-trip relation; the combined
  lexer/parser fuzz target lives in `fuzz/`.
- `xl-cli` ships end-to-end tests (`crates/xl-cli/tests/`) that invoke the
  real `xc` binary built by Cargo (`CARGO_BIN_EXE_xc`) — never an in-process
  shortcut — with real `.x` files in per-test temporary directories. They
  assert exit status, stdout, and stderr exactly, cover help/version and
  every documented usage error, assert that the archived specification
  example parses end-to-end and round-trips (its canonical output is pinned
  byte-for-byte as a fixture, identical to the parser suite snapshot), and
  exercise determinism across repeated runs, robustness (empty, invalid
  UTF-8, unreadable, missing, directory, deeply nested, and large inputs),
  and the accepted feature corpus. No new test dependency was introduced;
  temporary directories and process spawning use the standard library.
- Property-based testing may be added only when justified and minimal; it
  must not expand dependencies without review.

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
