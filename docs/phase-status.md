# Phase Status

```text
CURRENT_PHASE: 1
PHASE_NAME:    Lexical and Syntactic Frontend
STATUS:        IN PROGRESS — contracts ratified; lexer IMPLEMENTED
               (PENDING OWNER REVIEW); parser/AST/CLI NOT AUTHORIZED
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
- **Phase 1 — CONTRACTS RATIFIED** (Directive 006): the proposal set under
  `docs/proposals/phase-1/` was merged via PR #2 (merge commit
  `db2437b660225b78d5b008bf64a8d68a0f2c7049`) and ratified by the owner
  (proposal status `ACCEPTED — OWNER DIRECTIVE 006`; ADR-0004; PD-002
  through PD-006 resolved). The `xl-lexer` implementation stage is
  authorized (Directive 006); `xl-ast`, `xl-parser`, and `xc parse` remain
  **NOT AUTHORIZED**.

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

## Phase 1 — contract and implementation record

| Item | Status | Evidence / pointer |
| --- | --- | --- |
| Grammar RFC (token inventory, grammar, precedence, structural gaps) | ACCEPTED — OWNER DIRECTIVE 006 | `docs/proposals/phase-1/grammar-rfc.md` |
| Lexer contract | ACCEPTED — OWNER DIRECTIVE 006 | `docs/proposals/phase-1/lexer-contract.md` (span invariant §5.5) |
| Parser and AST contract | ACCEPTED — OWNER DIRECTIVE 006 | `docs/proposals/phase-1/parser-ast-contract.md` |
| Conformance plan | ACCEPTED — OWNER DIRECTIVE 006 | `docs/proposals/phase-1/conformance-plan.md` |
| Decision packet (categories A/B/C: PD-002…PD-006, G-1…G-4, B-1…B-4) | ACCEPTED — OWNER DIRECTIVE 006 | `docs/proposals/phase-1/decision-packet.md` (ratification record) |
| Consistency review (Directive 005 corrections) | RECORDED — OWNER DIRECTIVE 006 RATIFICATION | `docs/proposals/phase-1/consistency-review.md` |
| PD-002 through PD-006 | RESOLVED (Directive 006) | `docs/pending-decisions.md` |
| ADR for the ratified syntax contract | ACCEPTED | `docs/decisions/adr-0004-phase-1-syntax-contract.md` |
| `xl-lexer` implementation | IMPLEMENTED — PENDING OWNER REVIEW (Directive 006) | `crates/xl-lexer`; 100 lexer tests (144 workspace-wide, 0 failed); CI run 38328242463 green on `d2972b8` |
| `xl-lexer` fuzz harness (preliminary evidence) | PASS — preliminary only | `fuzz/`; 1,848,945 runs in 61 s on the local nightly toolchain, zero crashes and zero artifacts. NOT the Phase 1 acceptance criterion: the specification's lexer/parser fuzzing gate cannot pass without the parser |
| `xl-ast`, `xl-parser`, `xc parse` | NOT AUTHORIZED / NOT STARTED | Directives 004 §3, 005, 006 |
| Phase 1 docs CI (proposal set) | PASS | PR #2 merged as `db2437b`; run 38323917744 all four jobs green |

## Notes

- Phase 0 is complete and integrated; the archived original specification
  and accepted ADRs are frozen except by owner decision.
- The Phase 1 contracts are ratified (Directive 006; ADR-0004). The lexer
  implementation is complete and awaiting owner review (PR #3); `xl-ast`,
  `xl-parser`, and `xc parse` remain unauthorized, so the Phase 1
  acceptance criteria (all specification examples parse; combined
  lexer/parser fuzzing) cannot yet be claimed.
- Known documentation erratum (disclosed; no rule changed): the
  illustrative row `"{ "abc }"` in the accepted `lexer-contract.md` §5.3
  disagrees with the contract's own scanning rules; the actual behavior is
  pinned by the test `inner_string_consumes_brace_until_its_own_close`.
  Correcting the example requires an owner decision.
- The `xl-lexer` fuzz run is preliminary evidence only; no fabricated or
  extrapolated fuzzing results are recorded.
