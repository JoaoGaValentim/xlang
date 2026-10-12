# Contributing to XLANG

Thanks for contributing. XLANG is developed under strict engineering
governance; please read this document fully before opening a Pull Request.
The binding rules live in `docs/governance.md`.

## Prerequisites

- [rustup](https://rustup.rs) — the required toolchain (currently Rust
  1.99.0 with `rustfmt` and `clippy`) is installed automatically from
  `rust-toolchain.toml` the first time you run any cargo command in the
  repository.
- Editors should use the workspace rust-analyzer with the same toolchain.
  Do not configure a different toolchain for this repository.

## Local development workflow

1. Create a branch from `main`:

   ```bash
   git switch -c phase/<number>-<short-description>
   ```

   Policy: never commit directly to `main`, never force-push it, and never
   rewrite its history; all changes land through a Pull Request. GitHub-side
   enforcement is not yet configured — the current status and the
   recommended branch-protection settings are recorded in
   `docs/governance.md` §9.

2. Before every commit, run the three quality gates:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

3. Commit with focused, logically grouped changes using Conventional
   Commits, for example:

   - `chore: bootstrap cargo workspace`
   - `feat(span): implement source mapping`
   - `feat(diag): implement structured diagnostics`
   - `ci: add rust quality gates`
   - `docs: establish architecture governance`

4. Push your branch and open a Pull Request targeting `main`, filling in
   every field of the Pull Request template.

## What will be reviewed

- Scope: only work authorized by the current phase. Out-of-scope changes
  are rejected regardless of quality.
- Correctness: tests must fail before a fix and pass after; edge cases
  (empty input, boundaries, invalid UTF-8 positions, Unicode) are expected.
- No fabricated results: never claim a check passed unless it was run and
  observed (`docs/governance.md` §5).
- No panics on untrusted input; errors are values.
- Dependency discipline: a new external dependency requires a documented
  justification (INV-011).
- Documentation synchronization: behavior/architecture changes update the
  relevant files under `docs/` (INV-014).

## Test requirements

- New behavior ships with tests that run under `cargo test --workspace`.
- Rendered output (diagnostics, messages) is asserted exactly so format
  drift fails loudly.
- Do not weaken, skip, or delete tests to make CI green.

## Pull Requests and review

- Required CI checks must be green before merge; a failing check blocks
  merge (`docs/governance.md` §8).
- The project owner approves and merges. Automated agents and contributors
  never merge their own work.
- If work reveals a question the specification does not answer, add it to
  `docs/pending-decisions.md` instead of guessing, and mention it in the
  Pull Request.

## License

No license has been chosen for this project yet. By contributing you
acknowledge that the licensing decision is pending and is the project
owner's to make (`docs/pending-decisions.md` PD-008).
