# XLANG

[![CI](https://github.com/JoaoGaValentim/xlang/actions/workflows/ci.yml/badge.svg)](https://github.com/JoaoGaValentim/xlang/actions/workflows/ci.yml)

XLANG is a statically typed, compiled, expression-oriented programming
language with its compiler implemented in Rust. The planned pipeline is:

```text
Source → Lexer → Parser → AST → Name Resolution / HIR → Type Checking
       → Typed HIR → MIR → Optimization → Bytecode VM / Cranelift
```

## Status

**Phase 0 — Foundation: in progress.** XLANG is not yet a usable language;
there is no lexer, parser, type checker, or execution backend in this
repository at this time.

What exists today:

- A Cargo workspace with pinned toolchain and committed lockfile.
- **`xl-span`** — source location infrastructure: file registration,
  `FileId`, `Span`, UTF-8-aware line/column mapping, and explicit error
  handling for invalid positions.
- **`xl-diag`** — structured diagnostics: severities, labels, diagnostics,
  and deterministic source-aware text rendering.
- Architecture, governance, and decision documentation under `docs/`,
  including the archived original specification v0.1
  (`docs/spec-v0.1-original.txt`) and its reconciled working record
  (`docs/spec.md`).
- GitHub Actions CI running formatting, clippy (`-D warnings`), and tests
  on Linux, Windows, and macOS.

Nothing else is implemented. Work proceeds strictly phase by phase; see
`docs/roadmap.md` and `docs/phase-status.md`.

## Repository structure

```text
xlang/
├── .github/            CI workflow and pull request template
├── crates/
│   ├── xl-span/        source location infrastructure
│   └── xl-diag/        structured diagnostics
├── docs/               specification record, architecture, roadmap,
│                       governance, phase status, pending decisions,
│                       and architecture decision records
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
Requests (repository policy — current enforcement status in
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

No license has been selected yet. Until the project owner chooses one, the
repository carries no license grant (default copyright applies); this is a
mandatory unresolved decision recorded in `docs/pending-decisions.md`
(PD-008).
