# ADR-0003 — Toolchain Pinning and Rust Edition

- **Status:** Accepted — selection delegated by Engineering Directive 001
  (§4: "Select and document a compatible Rust toolchain and edition").
- **Date:** 2026-10-12

## Context

The compiler must be deterministic and reproducible (INV-007), CI must use
the same configuration as local development, and the lockfile is
version-controlled. Toolchain drift between machines is a common source of
"works locally, fails in CI", especially with formatting and lint gates
that run with `-D warnings`.

## Decision

- Pin the exact toolchain in `rust-toolchain.toml`:
  `channel = "1.99.0"`, `components = ["rustfmt", "clippy"]`,
  `profile = "minimal"`.
- Use **edition 2024** with workspace `resolver = "3"` and
  `rust-version = "1.99"`.
- CI installs the toolchain via `rustup install`, which resolves the
  version and components from `rust-toolchain.toml` itself — one source of
  truth for local and CI.
- Toolchain upgrades are intentional changes: they happen in a dedicated,
  reviewed commit that updates the pin and records the reason.

## Alternatives

- **Track `stable`.** Rejected: formatter and lint behavior changes on
  every release; CI could break without a code change, violating
  reproducibility.
- **An older LTS-like pin.** Rejected: no compatibility requirement exists
  that would justify forgoing edition 2024 and current lints.
- **Nightly.** Rejected: no unstable features are needed, and nightly
  undermines determinism.
- **A third-party toolchain action with an explicit version input.**
  Rejected: duplicates the version outside `rust-toolchain.toml` (two
  sources of truth) and adds an unnecessary third-party dependency to CI.
  The built-in `rustup install` resolves the pinned file directly.

## Consequences

- Identical compiler, formatter, and lints everywhere until the pin is
  deliberately changed.
- Editors without `rustup` integration may need the version configured
  manually; this is documented in `CONTRIBUTING.md`.
- The pin must be updated when a needed feature or fix requires it —
  a cost accepted for reproducibility.

## Specification impact

None: tooling choice only; no language semantics are affected.
