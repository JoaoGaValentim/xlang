# Phase Status

```text
CURRENT_PHASE: 0
PHASE_NAME:    Foundation
STATUS:        IN_PROGRESS
```

A phase may transition to `APPROVED` only after all required acceptance
criteria have been verified and the project owner has approved advancement.
See `docs/governance.md` §1.

## Acceptance tracking — Phase 0

| Requirement | Status | Evidence |
| --- | --- | --- |
| Cargo workspace valid (explicit members, pinned toolchain, committed lockfile) | PASS | `Cargo.toml`, `rust-toolchain.toml`, `Cargo.lock` |
| `xl-span` implemented and tested | PASS | `crates/xl-span`; 4 unit + 19 integration tests |
| `xl-diag` implemented and tested | PASS | `crates/xl-diag`; 2 unit + 17 integration tests |
| Sample diagnostic renders with location, source line, label, message | PASS | `tests/cross_crate.rs` (exact-output test) |
| Architecture and specification documentation exists | PASS | `docs/spec.md`, `docs/architecture.md`, `docs/roadmap.md` |
| Governance and decision records exist | PASS | `docs/governance.md`, `docs/decisions/` |
| Specification ambiguities recorded | PASS | `docs/pending-decisions.md` |
| Local format gate | PASS | `cargo fmt --all -- --check` |
| Local clippy gate (no warnings) | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| Workspace tests | PASS | `cargo test --workspace` — 43 tests, 0 failed |
| GitHub Actions CI run succeeds | PENDING | Recorded after the workflow run for this revision completes |
| No out-of-scope compiler stages implemented | PASS | Code review: Phase 0 crates only |
| Mandatory licensing decision resolved | BLOCKED | No license selected; owner decision required (PD-008) |
| Original specification document available in repository | BLOCKED | Not supplied; must be committed before Phase 1 (PD-001) |
| Project owner approval | PENDING | Owner provides after reviewing the Pull Request |

## Notes

- `STATUS` remains `IN_PROGRESS` until owner approval; technical gates
  alone do not complete the phase.
- CI evidence (workflow URL and revision SHA) is appended to this file
  after the run completes and is also recorded in the Pull Request.
