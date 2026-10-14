# XLANG

[![CI](https://github.com/JoaoGaValentim/xlang/actions/workflows/ci.yml/badge.svg)](https://github.com/JoaoGaValentim/xlang/actions/workflows/ci.yml)

XLANG is a statically typed, compiled, expression-oriented programming
language with its compiler implemented in Rust. The planned pipeline is:

```text
Source → Lexer → Parser → AST → Name Resolution / HIR → Type Checking
       → Typed HIR → MIR → Optimization → Bytecode VM / Cranelift
```

## Status

**Phase 1 — Lexical and syntactic frontend: ACCEPTED (Engineering
Directive 010, 2026-10-14).** Phase 0 is accepted and integrated. The
Phase 1 contracts are ratified and the frontend components are on `main`:
the lexer (`xl-lexer`), the syntax tree with canonical printing and
structural validation (`xl-ast`), the syntactic parser (`xl-parser`), and
the user-facing command `xc parse` (`xl-cli`). Phase 1 is syntax-only and
its closure record (merge SHA, post-merge CI, acceptance matrix, fuzzing
evidence, limitations) is in `docs/phase-status.md`.

**Phase 2 — Name resolution and type system: DESIGN GATE.** The design
package under `docs/proposals/phase-2/` is proposed and awaiting owner
ratification; no design item is ratified and no implementation is
authorized.

XLANG is not yet executable and no type checking exists: there is no name
resolution, type checker, runtime, standard library, or code generator, and
`xc` never runs a program. The printed AST is normalized XLANG syntax for
the parsed tree, not compiled code.

What exists today:

- A Cargo workspace with pinned toolchain and committed lockfile.
- **`xl-span`** — source location infrastructure: file registration,
  `FileId`, `Span`, UTF-8-aware line/column mapping, and explicit error
  handling for invalid positions.
- **`xl-diag`** — structured diagnostics: severities, labels, diagnostics,
  and deterministic source-aware text rendering.
- **`xl-lexer`** — lexical analysis: Unicode identifiers, numeric
  literals, nested comments, and structured interpolated strings.
- **`xl-ast`** — syntax tree data model, canonical printer, and structural
  validator.
- **`xl-parser`** — recursive-descent and Pratt parser with ratified error
  recovery and bounded recursion.
- **`xl-cli`** — the `xc` binary; `xc parse <file.x>` prints the canonical
  AST.
- Architecture, governance, and decision documentation under `docs/`,
  including the archived original specification v0.1
  (`docs/spec-v0.1-original.txt`) and its reconciled working record
  (`docs/spec.md`).
- Recorded Phase 1 acceptance evidence: the specification example parses
  and round-trips, and one hour of combined lexer/parser fuzzing completed
  with zero failures (`fuzz/fuzz_targets/lex_parse.rs`).
- GitHub Actions CI running formatting, clippy (`-D warnings`), and tests
  on Linux, Windows, and macOS.

Work proceeds strictly phase by phase; see `docs/roadmap.md` and
`docs/phase-status.md`.

## Using `xc parse`

Build the CLI and parse the specification example:

```bash
cargo build -p xl-cli
./target/debug/xc parse examples/program.x
```

Equivalently: `cargo run -p xl-cli --bin xc -- parse examples/program.x`.

Command contract:

- `xc parse <file.x>`: read the file, lex and parse it, print the
  canonical AST to stdout, and render diagnostics to stderr.
- `xc --help` and `xc --version`: print usage and version.
- Exit codes: `0` success (no error-level diagnostics), `1` lexical or
  syntactic source errors, `2` usage, I/O, or internal errors (CLI policy;
  see `docs/architecture.md` §3, `xl-cli`).
- Source errors never print a tree to stdout: a recovered or incomplete
  tree is never presented as valid source. Warnings and notes alone do not
  make a successful parse fail. The command never executes the program.

## Repository structure

```text
xlang/
├── .github/            CI workflow and pull request template
├── crates/
│   ├── xl-span/        source location infrastructure
│   ├── xl-diag/        structured diagnostics
│   ├── xl-lexer/       lexical analysis (source text to tokens)
│   ├── xl-ast/         syntax tree, canonical printer, validator
│   ├── xl-parser/      syntactic parser (tokens to AST)
│   └── xl-cli/         command-line interface (`xc` binary)
├── docs/               specification record, architecture, roadmap,
│                       governance, phase status, pending decisions,
│                       and architecture decision records
├── examples/           sample XLANG programs (specification example)
├── fuzz/               cargo-fuzz targets for the lexer/parser frontend
├── tests/              workspace-level cross-crate integration test
├── Cargo.toml          workspace manifest
├── rust-toolchain.toml pinned Rust toolchain (1.99.0)
├── README.md
└── CONTRIBUTING.md
```

## Building and testing

Requirements: [rustup](https://rustup.rs). The toolchain is pinned by
`rust-toolchain.toml` and is installed automatically on first use.

```bash
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Contributing

Development happens on `phase/<number>-<description>` branches; `main` is
the integration branch and receives changes only through reviewed Pull
Requests, enforced by GitHub branch protection (settings in
`docs/governance.md` §9). See `CONTRIBUTING.md` for the workflow and
`docs/governance.md` for the engineering rules that apply to everyone,
including automated agents.

## Specification

The authoritative document is "xlang — Especificação e Plano de
Engenharia" v0.1 (October 12, 2026). The original (Portuguese) document is
archived byte-faithfully at `docs/spec-v0.1-original.txt`; `docs/spec.md` is
the reconciled working record derived from it, and
`docs/spec-consistency-audit.md` records the consistency audit. Where they
differ, the archived original governs.

## License

XLANG is dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any additional
terms or conditions.
