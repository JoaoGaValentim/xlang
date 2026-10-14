# Phase Status

```text
CURRENT_PHASE: 2
PHASE_NAME:    Name Resolution and Type System
STATUS:        DESIGN GATE — IN PROGRESS
               Phase 1: ACCEPTED (Engineering Directive 010, 2026-10-14)
               Phase 2: design package proposed under docs/proposals/phase-2/;
               awaiting owner ratification. No Phase 2 implementation is
               authorized (no production code exists beyond Phase 1).
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
  through PD-006 resolved). The `xl-lexer` implementation stage was
  authorized (Directive 006); `xl-ast`, `xl-parser`, and `xc parse`
  remained unauthorized at that point.
- **Phase 1 — LEXER INTEGRATED** (PR #3, merge
  `9ed50ad1bfd52965fba1e007ee15dfe877c9ee58`, 2026-10-13): the lexical
  frontend is on `main` with green CI and preliminary fuzz evidence.
- **Phase 1 — AST IMPLEMENTED** (Directive 007): the syntax tree crate
  `xl-ast` (data model, canonical printer, structural validator) is
  complete and pending owner review via PR #4. `xl-parser` and `xc parse`
  remain **NOT AUTHORIZED / NOT STARTED**.
- **Phase 1 — AST INTEGRATED** (PR #4, merge
  `df9e859175b94ae19f5b3998a0160da230ba77a3`, 2026-10-13T13:48:01Z):
  integration CI run
  [38407939639](https://github.com/JoaoGaValentim/xlang/actions/runs/38407939639)
  green on all four jobs; `main` contains `xl-lexer` and `xl-ast`.
- **Phase 1 — PARSER IMPLEMENTED** (Directive 008, branch
  `phase/1-parser`): the syntactic parser crate `xl-parser` (recursive
  descent + Pratt expressions, ratified recovery sets, shared
  `MAX_PARSE_DEPTH = 64`, G-3 generic closing, interpolation integration)
  is implemented and pending owner review. `xc parse` is **NOT STARTED**.
- **Phase 1 — PARSER INTEGRATED** (PR #5, merge
  `b4a7416e6cdb19fd016a65bc32a983d18f7655fe`, 2026-10-13T21:17:43Z):
  integration CI run
  [38469504073](https://github.com/JoaoGaValentim/xlang/actions/runs/38469504073)
  green on all four jobs; `main` contains `xl-lexer`, `xl-ast`, and
  `xl-parser`.
- **Phase 1 — CLI IMPLEMENTED** (Directive 009, branch `phase/1-cli`):
  `xl-cli` provides the `xc` binary with `xc parse <file.x>` (canonical AST
  on stdout via the accepted printer, diagnostics on stderr via `xl-diag`,
  deterministic exit codes `0`/`1`/`2`), plus `xc --help`/`--version`.
  End-to-end tests invoke the real binary with real `.x` files and cover
  the archived specification example, the accepted feature corpus,
  determinism, round-trip printing, robustness, and every documented
  usage/file error. Pending owner review. CI run
  [38484468487](https://github.com/JoaoGaValentim/xlang/actions/runs/38484468487)
  on `63d2af3` is green on all four jobs (Format, ubuntu, windows, macos).
  Directive 009 did not modify `xl-lexer`, `xl-ast`, or `xl-parser` code,
  so the recorded combined lexer/parser fuzz evidence remains valid.
- **Phase 1 — EMPTY-FILE CONFORMANCE FIX** (Directive 009-R1): an
  independent review identified that the CLI rejected byte-empty source
  files with exit `2` although the accepted grammar defines
  `module = { item }`, which allows zero items and is already how
  whitespace-only and comment-only inputs are handled. The CLI now parses
  a zero-byte file as the valid empty module: exit `0`, no diagnostics, and
  empty canonical output (zero items), exactly like trivia-only input.
  Missing, unreadable, directory, invalid UTF-8, `u32`-limit, usage, and
  source-diagnostic behaviors are unchanged. Tests and documentation were
  corrected accordingly; `xl-lexer`, `xl-ast`, and `xl-parser` were not
  modified. CI run
  [38488945189](https://github.com/JoaoGaValentim/xlang/actions/runs/38488945189)
  on `a84e8c4` is green on all four jobs (Format, ubuntu, windows, macos;
  CI-observed test counts 484/484/482, Windows excluding the two documented
  `#[cfg(unix)]` reproducibility tests).
- **Phase 1 — LEXER/PARSER FUZZING ACCEPTED** (Directive 008): the
  combined `lex_parse` fuzz target completed the specification's one-hour
  criterion on `017962b` — 9,111,364 runs in 3601 s, zero crashes. The
  fuzzing found two real defects across earlier attempts (match-arm
  diagnostic cascade; printer non-idempotence after block-like statements)
  and one round-trip printer defect was found by property testing (cast
  tails before `<`); all three are fixed with regression tests
  (`1e427f6`, `94b11c7`, `017962b`). The remaining Phase 1 milestone,
  `xc parse`, is implemented and pending owner review (see the CLI entry
  above).
- **Phase 1 — FORMALLY ACCEPTED** (Engineering Directive 010, 2026-10-14):
  the project owner merged PR #6 and issued Directive 010, authorizing
  formal acceptance of Phase 1 subject to independent verification. All
  conditions were verified on 2026-10-14: PR #6 is **merged** (merge commit
  `210e1e04c55967050b6b3bba9e0d3a0bc95b7acc`, merged 2026-10-14T00:26:12Z);
  the integrated `main` head is the same SHA; the former PR head
  `1741530fc268bc923b2a4c632600e9a54136e487` is an ancestor and the two
  revisions have **identical trees**; the post-merge integration CI run
  [38490165826](https://github.com/JoaoGaValentim/xlang/actions/runs/38490165826)
  completed successfully with all four mandatory jobs green (Format,
  Check ubuntu-latest, Check macos-latest, Check windows-latest); the
  original specification archive hash is unchanged
  (`c3c6bce5…`); branch protection on `main` is active (verified through
  the GitHub API); the repository contains exactly the six Phase 0–1
  crates and no resolution, HIR, or type-checking code; and the working
  tree is clean. `PHASE 1 — ACCEPTED` is recorded in full below. Phase 2
  (Name Resolution and Type System) is a **DESIGN GATE**: the design
  package under `docs/proposals/phase-2/` is proposed and awaits owner
  ratification; no implementation is authorized.

## Phase 1 closure record (final — Directive 010, 2026-10-14)

```text
PHASE 1 — ACCEPTED
```

- **Owner acceptance authority:** Engineering Directive 010 (2026-10-14).
  The owner merged PR #6 and issued the acceptance directive; this record
  is the independent verification it requires. The acceptance directive
  did not modify historical records: this is a new closure entry appended
  to the phase log above.
- **PR #6:** merged into `main` 2026-10-14T00:26:12Z; merge commit SHA
  `210e1e04c55967050b6b3bba9e0d3a0bc95b7acc`
  ([PR #6](https://github.com/JoaoGaValentim/xlang/pull/6)).
- **Integrated `main` SHA at acceptance:**
  `210e1e04c55967050b6b3bba9e0d3a0bc95b7acc` (the merge commit is the
  head of `main`).
- **Post-merge integration CI:**
  [run 38490165826](https://github.com/JoaoGaValentim/xlang/actions/runs/38490165826)
  (`push` event on `main`, head SHA `210e1e0`, created
  2026-10-14T00:26:15Z): **completed / success**, all four mandatory jobs
  green — `Format`, `Check (ubuntu-latest)`, `Check (macos-latest)`,
  `Check (windows-latest)`. This run is the post-merge integration run,
  distinct from the PR-head run 38488945189.
- **Verified test counts:** CI-observed `484` (Ubuntu), `484` (macOS),
  `482` (Windows; the two documented `#[cfg(unix)]` reproducibility tests
  are excluded on Windows). Independent local re-verification on macOS at
  the accepted revision: `cargo test --workspace` — **484 passed, 0
  failed** across 52 test targets, plus `cargo fmt --all -- --check`
  clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
  and `cargo build --workspace --locked` successful.

### Integration verification (Directive 010 §2 preflight)

| # | Verification | Result | Evidence |
| --- | --- | --- | --- |
| 1 | PR #6 merged into `main` | PASS | GitHub API: `state: MERGED`, `mergedAt 2026-10-14T00:26:12Z` |
| 2 | Merge commit SHA recorded | PASS | `210e1e04c55967050b6b3bba9e0d3a0bc95b7acc` |
| 3 | Former PR head is an ancestor of integrated `main` | PASS | `git merge-base --is-ancestor 1741530… origin/main` holds; `git diff --stat 1741530… origin/main` is empty (identical trees) |
| 4 | Integrated `main` contains `xl-cli` and the `xc` executable | PASS | `crates/xl-cli/Cargo.toml` declares `[[bin]] name = "xc"`; CI builds and runs the real binary through end-to-end suites |
| 5 | Empty-module correction present | PASS | `fix(cli): accept empty source files as valid modules` (`a84e8c4`); tests `zero_byte_input_is_a_valid_empty_module`, `whitespace_only_input_is_a_valid_empty_module`, `comment_only_input_is_a_valid_empty_module` |
| 6 | Post-merge CI completed successfully | PASS | Run [38490165826](https://github.com/JoaoGaValentim/xlang/actions/runs/38490165826) `success` |
| 7 | All four mandatory jobs pass | PASS | `Format`, `Check (ubuntu-latest)`, `Check (macos-latest)`, `Check (windows-latest)` — all `success` |
| 8 | Original specification archive unchanged | PASS | `sha256(docs/spec-v0.1-original.txt)` = `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770` (matches the recorded hash) |
| 9 | Protected-branch policy verified | PASS | GitHub API: required checks (`Format`, three `Check` jobs), strict up-to-date requirement, administrators enforced, force pushes and deletions blocked |
| 10 | No unauthorized compiler phase implemented | PASS | `crates/` members are exactly `xl-span`, `xl-diag`, `xl-lexer`, `xl-ast`, `xl-parser`, `xl-cli`; `xc check` is explicitly rejected as an unknown command by the CLI argument tests; no HIR, resolution, or typing code exists |
| 11 | Working tree clean at verification | PASS | `git status` clean on the review branch before the design work began |
| 12 | No merge was performed by the agent | PASS | PR #6 was merged by the project owner before Directive 010 was issued |

### Acceptance matrix (revalidated 2026-10-14)

The 14 criteria of the Directive 009 audit remain satisfied; the
empty-module conformance correction is additionally verified. This matrix
supersedes the `PENDING OWNER APPROVAL` row of that audit; the audit table
itself is preserved above as the historical record at the time it was
written.

| # | Acceptance criterion | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Phase 0 was accepted and integrated | PASS | Directive 004 §2; PR #1 merge `4d82106`; CI run 38314053579 |
| 2 | Phase 1 syntax contracts are ratified | PASS | Directive 006; ADR-0004; PD-002…PD-006 resolved |
| 3 | Lexer implemented and integrated | PASS | `crates/xl-lexer`; PR #3 merge `9ed50ad` |
| 4 | AST implemented and integrated | PASS | `crates/xl-ast` (model, canonical printer, structural validator); PR #4 merge `df9e859` |
| 5 | Parser implemented and integrated | PASS | `crates/xl-parser`; PR #5 merge `b4a7416`; integration CI 38469504073 green |
| 6 | CLI implemented and integrated | PASS | `crates/xl-cli` (`xc parse`, `--help`, `--version`); PR #6 merge `210e1e0`; integration CI 38490165826 green |
| 7 | Original specification examples parse | PASS | the complete §3 example parses clean through the real `xc parse` (end-to-end test; canonical output pinned as a byte-identical fixture) |
| 8 | Canonical AST printing works | PASS | `xl-ast` printer suites; CLI fixture equality with the parser snapshot |
| 9 | Parser recovery and diagnostics work | PASS | `xl-parser` recovery suite; CLI multiple-independent-errors and exact-stderr tests |
| 10 | Source spans preserve UTF-8 conventions | PASS | lexer/parser span suites; ADR-0002 conventions (UTF-8 bytes, half-open, scalar-value columns); CLI exact-stderr diagnostics |
| 11 | Parser/AST round-trip tests pass | PASS | `roundtrip.rs`, `properties.rs`, CLI corpus round-trip |
| 12 | Lexer/parser fuzzing satisfies the one-hour criterion | PASS | `lex_parse` on `017962b`: 9,111,364 runs in 3601 s, zero crashes (scope below) |
| 13 | Full integration CI passes | PASS | Run [38490165826](https://github.com/JoaoGaValentim/xlang/actions/runs/38490165826) on the merge commit, all four jobs |
| 14 | Original specification archive unchanged | PASS | SHA-256 `c3c6bce5…` re-verified on the integrated `main` |
| 15 | Byte-empty files parse as valid empty modules | PASS | Directive 009-R1 fix (`a84e8c4`); empty, whitespace-only, and comment-only inputs all exit `0` with no output and no diagnostics |

### Fuzzing evidence and exact scope

- **Target:** the combined `fuzz/fuzz_targets/lex_parse.rs` (lexer + parser
  pipeline over arbitrary bytes), executed on revision `017962b`.
- **Result:** 9,111,364 runs in 3601 s (≈2,530 exec/s), zero crashes, zero
  sanitizer failures, no new artifacts. Two earlier attempts found real
  defects (match-arm diagnostic cascade; printer non-idempotence after
  block-like statements) and one printer defect was found by property
  testing (cast tails before `<`); all three are fixed with regression
  tests (`1e427f6`, `94b11c7`, `017962b`).
- **Scope limits (explicit):** the evidence covers the lexer/parser
  pipeline only; it does **not** cover the CLI I/O surface (file handling,
  argument parsing) or any later-phase component. Directives 009 and 009-R1
  modified no `xl-lexer`, `xl-ast`, or `xl-parser` code, so the evidence
  remains representative of the frontend behavior shipped with the CLI. The
  separate `lex_tokens` run (1,848,945 runs in 61 s) is preliminary evidence
  only.

### `xc parse` milestone confirmation

The original Phase 1 milestone (SPECIFIED §6: `xc parse arquivo.x` prints
the AST) is complete: `xl-cli` builds the `xc` binary; `xc parse <file.x>`
lexes and parses the file through the accepted `xl_parser::parse_file` API,
renders diagnostics on stderr via `xl_diag::render`, prints the canonical
AST via `xl_ast::print_items` on stdout, and exits `0`/`1`/`2` under the
documented exit-code contract. The end-to-end suites invoke the real
binary (`CARGO_BIN_EXE_xc`) with real `.x` files; the archived
specification example parses and round-trips.

### Known nonblocking limitations (Phase 1)

- Diagnostics are plain text: no colors, no stable error codes, no
  terminal-width wrapping; long source lines print in full; multi-line
  spans highlight only the first line; columns count Unicode scalar values
  (wide characters and tabs can misalign carets). Tracked under PD-010.
- I/O error messages embed operating-system text and are therefore
  platform-specific; all other CLI messages are stable byte-for-byte.
- Windows CI excludes the two documented `#[cfg(unix)]` reproducibility
  tests (file-permission simulation).
- `xl-ast` does not claim a global stack-safety guarantee for *hand-built*
  trees deeper than the native stack (parser-produced trees are bounded by
  the accepted `MAX_PARSE_DEPTH = 64`); drop recursion is not depth-bounded.
- Grouped definitions: G-5 (struct-literal construction syntax) remains
  UNRESOLVED; `Point { x: 1.0 }` is not valid Phase 1 syntax.
- No external Rust API stability is promised (PD-009 unresolved).

### Explicit scope acknowledgment

**Phase 1 is syntax-only.** Acceptance means the specified lexical and
syntactic frontend is complete: text → tokens → AST with canonical
printing, recovery diagnostics, and the `xc parse` command. `xc` does not
execute programs, does not resolve names, and does not check types;
undefined names are not Phase 1 errors; no desugaring, HIR, type checking,
MIR, bytecode, VM, or native code generation exists in this repository.
Nothing in this record claims any semantic behavior.

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
| `xl-lexer` implementation | IMPLEMENTED — INTEGRATED (Directive 006) | `crates/xl-lexer`; merged via PR #3 (merge `9ed50ad`); 104 lexer tests (148 workspace-wide at merge, 0 failed); CI runs 38328242463 (`d2972b8`) and 38328884949 (`d48a622`) green on all four jobs |
| `xl-ast` implementation | IMPLEMENTED — INTEGRATED (Directive 007) | `crates/xl-ast`; data model + canonical printer + structural validator; 94 AST tests (242 workspace-wide, 0 failed; CI-observed 242); 3 insta snapshots; merged via PR #4 (merge `df9e859`; integration CI run 38407939639, all four jobs green) |
| `xl-parser` implementation | IMPLEMENTED — INTEGRATED (Directive 008) | `crates/xl-parser`; recursive descent + Pratt expressions, ratified recovery sets, shared `MAX_PARSE_DEPTH = 64`, G-3 generic closing, interpolation integration; 187 parser tests; 4 insta snapshots; spec example parses clean and round-trips; merged via PR #5 (merge `b4a7416`; integration CI run [38469504073](https://github.com/JoaoGaValentim/xlang/actions/runs/38469504073) on the merge commit, all four jobs green) |
| `xl-cli` (`xc parse`) implementation | IMPLEMENTED — INTEGRATED (PR #6, merge `210e1e0`); PHASE 1 ACCEPTED (Directive 010) | `crates/xl-cli`; binary `xc`; hand-written argument parsing (no CLI dependency); file loading with recoverable I/O and UTF-8 decoding; `SourceMap` registration; accepted `xl-parser` API; diagnostics via `xl-diag` on stderr; canonical AST via `xl-ast::print_items` on stdout; exit codes `0`/`1`/`2` documented as CLI policy; 49 crate tests (15 unit + 34 end-to-end invoking the real binary); 484 workspace-wide tests, 0 failed locally; conformance fixture for the specification example byte-identical to the parser canonical snapshot; empty-module conformance corrected under Directive 009-R1 (byte-empty, whitespace-only, and comment-only inputs all parse as the valid empty module); CI run [38488945189](https://github.com/JoaoGaValentim/xlang/actions/runs/38488945189) on `a84e8c4` green on all four jobs (Format, ubuntu, windows, macos; CI-observed 484/484/482); post-merge integration CI run [38490165826](https://github.com/JoaoGaValentim/xlang/actions/runs/38490165826) on `210e1e0` green on all four jobs |
| `xl-lexer` fuzz harness (preliminary evidence) | PASS — preliminary only | `fuzz/fuzz_targets/lex_tokens.rs`; 1,848,945 runs in 61 s on the local nightly toolchain, zero crashes and zero artifacts. Superseded by the combined lexer/parser fuzzing row below for acceptance purposes |
| Combined lexer/parser fuzzing (Phase 1 acceptance criterion: 1 hour without failures) | **PASS** | `fuzz/fuzz_targets/lex_parse.rs` on revision `017962b`: **9,111,364 runs in 3601 s** (average 2,530 exec/s, peak RSS 770 MB, 22,355 corpus units added), zero crashes, zero sanitizer failures, no new artifacts. The two earlier attempts found and fixed real defects before this run: a match-arm diagnostic cascade (`crash-67a0b82e…`) and printer non-idempotence (`crash-206b3a0c…`), both with regression tests |
| `xl-parser`, `xc parse` | `xl-parser` INTEGRATED (PR #5); `xc parse` INTEGRATED (PR #6, merge `210e1e0`); PHASE 1 ACCEPTED (Directive 010) | Directive 008 (parser authorized); Directives 009 and 009-R1 (CLI authorized); Directive 010 (Phase 1 acceptance) |
| Phase 1 docs CI (proposal set) | PASS | PR #2 merged as `db2437b`; run 38323917744 all four jobs green |

## Phase 1 acceptance tracking (Directive 009 audit, 2026-10-13)

Classification vocabulary: **PASS**, **FAIL**, **NOT VERIFIED**,
**PENDING OWNER APPROVAL**.

| # | Acceptance criterion | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Phase 0 was accepted and integrated | PASS | Directive 004 §2; PR #1 merge `4d82106`; CI run 38314053579 |
| 2 | Lexical contracts are implemented | PASS | `crates/xl-lexer` (104 tests); PR #3 merge `9ed50ad` |
| 3 | Parser and AST contracts are implemented | PASS | `crates/xl-ast` (PR #4), `crates/xl-parser` (PR #5); 435-test baseline |
| 4 | All authoritative specification examples parse | PASS | the complete §3 example parses clean through the real `xc parse` and round-trips (`crates/xl-cli/tests/cli_success.rs`) |
| 5 | Source spans and diagnostics are correct | PASS | lexer/parser/diagnostic suites; CLI exact-stderr diagnostics tests; UTF-8 offset preservation |
| 6 | Error recovery is tested | PASS | `crates/xl-parser/tests/recovery.rs`; CLI multiple-independent-errors and determinism tests |
| 7 | Canonical AST printing is functional | PASS | `xl-ast` printer suites; CLI fixture equality with the parser snapshot |
| 8 | Parser/AST round-trip equivalence tests pass | PASS | `crates/xl-parser/tests/roundtrip.rs`, `properties.rs`; CLI corpus round-trip |
| 9 | One-hour lexer/parser fuzz evidence is valid | PASS | `lex_parse` on `017962b`: 9,111,364 runs / 3601 s, zero crashes; lexer/parser code unchanged by Directive 009 |
| 10 | The real `xc parse` command works | PASS | end-to-end tests invoke the built binary; 666-byte canonical output for the example; exit codes `0`/`1`/`2` verified |
| 11 | CI is green on the final CLI revision | PASS | run [38488945189](https://github.com/JoaoGaValentim/xlang/actions/runs/38488945189) on `a84e8c4` (the Directive 009-R1 fix revision; subsequent commits are documentation-only): Format and Check (ubuntu-latest, macos-latest, windows-latest) all green; CI-observed counts 484/484/482 tests (Windows omits the two documented `#[cfg(unix)]` reproducibility tests) |
| 12 | Original specification archive is unchanged | PASS | SHA-256 `c3c6bce5…` re-verified during the Directive 009 merge gate |
| 13 | No out-of-scope compiler functionality exists | PASS | `crates/` contains exactly `xl-span`, `xl-diag`, `xl-lexer`, `xl-ast`, `xl-parser`, `xl-cli` |
| 14 | All known limitations are documented | PASS | `docs/architecture.md` §3 (`xl-cli`; exit codes, empty-file policy, rendering limits); README status and limitations |
| — | Phase 1 formal acceptance | **PENDING OWNER APPROVAL** | requires the CLI Pull Request to be merged and an explicit owner decision; this audit is a closure recommendation, not self-approval |

**Status update (2026-10-14).** The `PENDING OWNER APPROVAL` row above is
the Directive 009 audit snapshot at the time it was written. The condition
was subsequently satisfied: PR #6 was merged by the project owner and
formal acceptance was granted via Engineering Directive 010; the
superseding record is the **Phase 1 closure record (final — Directive 010)**
at the top of this file. The audit table is preserved unchanged as
history.

## Notes

- Phase 0 and **Phase 1** are complete, accepted, and integrated; the
  archived original specification and accepted ADRs are frozen except by
  owner decision. The lexer, AST, parser, and CLI (PR #3, #4, #5, #6) are
  on `main`; `PHASE 1 — ACCEPTED` was recorded 2026-10-14 under Directive
  010 (closure record above).
- **Phase 2 is a DESIGN GATE.** The proposed design package under
  `docs/proposals/phase-2/` is documentation only and awaits owner
  ratification; no `xl-hir`, `xl-types`, resolution, or `xc check`
  implementation exists or is authorized. The end-user milestone remains
  `xc check program.x` (SPECIFIED §6 Phase 2).
- CLI exit codes are a documented command contract: `0` success, `1`
  lexical/syntactic source errors, `2` usage/I/O/internal errors (CLI
  policy; `docs/architecture.md` §3, `xl-cli`). A source file is one
  module: a byte-empty, whitespace-only, or trivia-only file is the valid
  empty module (`module = { item }`) and succeeds with no output and no
  diagnostics (Directive 009-R1 conformance fix).
- Directive 009 modified no `xl-lexer`, `xl-ast`, or `xl-parser` code; the
  combined fuzz evidence on `017962b` therefore remains representative of
  the frontend behavior shipped with the CLI. No new fuzz run was claimed.
- The lexer-contract example erratum was **corrected editorially** under
  Directive 007 §11: the `"{ "abc }"` row now describes the implemented,
  accepted behavior (pinned by the test
  `inner_string_consumes_brace_until_its_own_close`); lexical semantics
  were not changed.
- Parser-based `parse(print(ast)) ≡ ast` round-trip testing was the
  mandatory future integration gate recorded under Directive 007; the
  parser implementation closes it for the corpus and generated programs
  (`crates/xl-parser/tests/roundtrip.rs`, `properties.rs`), with spans and
  parser-assigned identifiers excluded per the ratified equivalence
  relation.
- The `xl-lexer` fuzz run is preliminary evidence only; no fabricated or
  extrapolated fuzzing results are recorded. The combined lexer/parser
  criterion was executed with real evidence (see the acceptance row
  above): 9,111,364 runs in 3601 s on `017962b`, zero crashes.
- The two fuzz-found defects and the property-test-found printer defect
  have documented reproductions: `fuzz/artifacts/lex_parse/`
  (`crash-67a0b82e…`, `crash-206b3a0c…`; gitignored) and the round-trip
  property counterexample `((a * a as i64) < a)`.
