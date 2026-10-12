# Phase Status

```text
CURRENT_PHASE: 1
PHASE_NAME:    Lexical and Syntactic Frontend
STATUS:        DESIGN GATE — proposals pending owner approval;
               implementation NOT AUTHORIZED
```

A phase may transition to `APPROVED` only after all required acceptance
criteria have been verified and the project owner has approved advancement
(`docs/governance.md` §1). Status vocabulary: **PASS**, **FAIL**,
**BLOCKED**, **NOT VERIFIED**, **PENDING OWNER APPROVAL**.

## Phase log (2026-10-12)

- **Phase 0 — ACCEPTED.** The project owner formally accepted the Phase 0
  deliverables via Engineering Directive 004 §2, conditional on successful
  integration of PR #1. The condition was verified: PR #1 is **merged**
  (merge commit `4d82106243df2e4944f495f121155f10f18bc9dd`, merged
  2026-10-12T21:59:52Z) and the `main` CI run for the merge commit is green
  (run [38314053579](https://github.com/JoaoGaValentim/xlang/actions/runs/38314053579),
  all four jobs). Acceptance is therefore in effect.
- **Phase 1 — DESIGN GATE ACTIVE** (Directive 004 §3): syntax contracts
  proposed under `docs/proposals/phase-1/`, pending owner review.
  **No Phase 1 implementation is authorized.**

## Phase 0 closure record (final — Directives 002–004, 2026-10-12)

| # | Record | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Implementation completion | PASS | Workspace, `xl-span`, `xl-diag`, CI workflow, governance and specification documents |
| 2 | Local test verification | PASS | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` — 44 tests, 0 failed; `cargo build --workspace --locked` |
| 3 | Remote CI verification | PASS | Run [38312318035](https://github.com/JoaoGaValentim/xlang/actions/runs/38312318035) on `ebb70dd`; final integration run [38314053579](https://github.com/JoaoGaValentim/xlang/actions/runs/38314053579) on merge commit `4d82106` — all four jobs green |
| 4 | Specification reconciliation | PASS | `docs/spec-v0.1-original.txt` archived byte-faithfully (SHA-256 `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770`, unchanged); `docs/spec.md`, `docs/roadmap.md`, `docs/spec-consistency-audit.md`; PD-001 resolved |
| 5 | License selection | PASS | PD-008 resolved: dual `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, SPDX metadata) |
| 6 | ADR approval (ADR-0002 / PD-007) | PASS | Owner approved ADR-0002 (Directive 003); status `Accepted`; PD-007 resolved |
| 7 | Pull Request merge | PASS | [PR #1](https://github.com/JoaoGaValentim/xlang/pull/1) merged; merge commit `4d82106243df2e4944f495f121155f10f18bc9dd` contains all accepted commits |
| 8 | Owner acceptance | PASS | Formal acceptance via Directive 004 §2; integration condition verified (records #7 and #3) |

## Acceptance tracking — Phase 0 (final)

| Requirement | Status | Evidence |
| --- | --- | --- |
| Cargo workspace valid (explicit members, pinned toolchain, committed lockfile) | PASS | `Cargo.toml`, `rust-toolchain.toml`, `Cargo.lock` |
| `xl-span` implemented and tested | PASS | `crates/xl-span`; 24 tests |
| `xl-diag` implemented and tested | PASS | `crates/xl-diag`; 19 tests |
| Sample diagnostic renders with location, source line, label, message | PASS | `tests/cross_crate.rs` |
| Specification reconciled with the original | PASS | archival + `docs/spec.md` + `docs/spec-consistency-audit.md` |
| Roadmap transcribes all ten phases | PASS | `docs/roadmap.md` |
| Governance and ADR records | PASS | `docs/governance.md`, `docs/decisions/` (ADR-0002 accepted) |
| Ambiguities recorded, none invented | PASS | `docs/pending-decisions.md` |
| Local quality gates | PASS | fmt, clippy, 44 tests, locked build |
| GitHub Actions CI | PASS | runs 38312318035 and 38314053579 |
| No out-of-scope compiler stages implemented | PASS | `crates/` contains only `xl-span`, `xl-diag` |
| Original specification archived unchanged | PASS | hash `c3c6bce5…` on `main` |
| License resolved | PASS | dual `MIT OR Apache-2.0` |
| Branch protection enforced on `main` | PASS | API-verified settings (`docs/governance.md` §9); PR #1 merged through it |
| Project owner approval | PASS | Directive 004 §2 acceptance (condition satisfied) |

## Phase 1 — design-gate record

| Item | Status | Evidence / pointer |
| --- | --- | --- |
| Grammar RFC (token inventory, grammar, precedence, structural gaps) | PROPOSED — OWNER REVIEW REQUIRED | `docs/proposals/phase-1/grammar-rfc.md` |
| Lexer contract | PROPOSED — OWNER REVIEW REQUIRED | `docs/proposals/phase-1/lexer-contract.md` |
| Parser and AST contract | PROPOSED — OWNER REVIEW REQUIRED | `docs/proposals/phase-1/parser-ast-contract.md` |
| Conformance plan | PROPOSED — OWNER REVIEW REQUIRED | `docs/proposals/phase-1/conformance-plan.md` |
| Decision packet (categories A/B/C: PD-002…PD-006, G-1…G-4, B-1…B-4) | PROPOSED — OWNER REVIEW REQUIRED | `docs/proposals/phase-1/decision-packet.md` |
| Consistency review (Directive 005 corrections) | PROPOSED — OWNER REVIEW REQUIRED | `docs/proposals/phase-1/consistency-review.md` |
| PD-002 through PD-006 | PROPOSALS PREPARED — NOT RESOLVED | `docs/pending-decisions.md`; packet rows |
| Phase 1 implementation (`xl-lexer`, `xl-ast`, `xl-parser`, `xc parse`) | NOT AUTHORIZED | Directives 004 §3 and 005; no new crates exist |
| Phase 1 docs CI | See PR #2 | GitHub Actions run on the PR head |

## Notes

- Phase 0 is complete and integrated; the archived original specification,
  accepted ADRs, and Phase 0 crates are frozen except by owner decision.
- The design gate ends when the owner approves (or amends) the decision
  packet. Even then, coding begins only under a separate implementation
  directive.
