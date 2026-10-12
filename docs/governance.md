# Engineering Governance

This document defines the permanent engineering rules for XLANG. It applies
to every phase and every contributor, including autonomous agents.

## 1. Phase authorization process

- Development is organized into ten sequential phases (0–9).
- Only the project owner authorizes a phase to begin, by issuing a
  directive that names the phase and its authorized scope.
- Work outside the currently authorized phase is prohibited, even when it
  appears harmless or convenient.
- `docs/phase-status.md` records the current phase and its verified status.
- A phase transitions to `APPROVED` only after every acceptance criterion
  has been verified with evidence **and** the project owner has approved
  advancement. Green checks alone never advance a phase.

## 2. Scope boundaries

- The authorized scope of the current phase is the exclusive work list;
  anything else is out of scope by default.
- No placeholder implementations of future phases may be created.
- Infrastructure whose only purpose is hypothetical future extensibility is
  prohibited.
- When scope is ambiguous, stop and record the ambiguity under
  `docs/pending-decisions.md` rather than guessing.

## 3. Acceptance gates

A phase is acceptable only when, at minimum:

- All required deliverables for the phase exist and are documented.
- The CI gates in §8 pass on the delivered revision.
- Required tests exist and pass; results are observed, never assumed.
- Specification ambiguities are recorded; nothing was invented.
- Documentation is synchronized with the delivered state.
- The project owner has approved completion.

## 4. Rust coding standards

- Toolchain and edition are pinned by `rust-toolchain.toml` and the
  workspace manifest (see
  `docs/decisions/adr-0003-toolchain-and-edition.md`). Local and CI builds
  use the same configuration.
- `#![forbid(unsafe_code)]` is required in every crate unless a future
  component is explicitly permitted unsafe code by the specification and
  separately reviewed. (INV-005)
- No global mutable compiler state. State is owned and passed explicitly.
- Public items carry doc comments; `missing_docs` warnings are enabled.
- Malformed input must not cause uncontrolled panics; recoverable problems
  are reported through `Result`. (INV-006)
- `cargo fmt` formatting is mandatory; `cargo clippy -- -D warnings` must
  be clean.
- Prefer focused, reviewable changes and small commits. (INV-012)

## 5. Testing requirements

- Every phase ships automated tests covering its behavioral requirements,
  including edge cases and error paths.
- Tests must run under `cargo test --workspace`.
- Expected outputs (renderings, message formats) are asserted exactly, so
  accidental format drift fails tests.
- Determinism is testable and must be tested where output is generated.
- Fabricated results, silent fallbacks that hide missing behavior, and
  placeholder tests are prohibited. (INV-008, INV-009)
- Property-based testing may be added only when justified and minimal; it
  must not expand dependencies without review.

## 6. Dependency-management rules

- No external crate enters the workspace without a documented technical
  justification. (INV-011)
- Dependency direction flows only along the pipeline (higher-level crates
  depend on lower-level ones; never the reverse, never circular).
- `Cargo.lock` is version-controlled.
- Dependencies are exact and reproducible; CI uses the committed lockfile.
- Automated dependency updates are not enabled until the owner authorizes
  them.
- Every CI third-party action is pinned to an immutable commit SHA with the
  upstream release recorded in `docs/architecture.md`.

## 7. Architectural change control

- Significant decisions are recorded as ADRs in `docs/decisions/` using the
  documented format (identifier, title, status, context, decision,
  alternatives, consequences, specification impact).
- ADR statuses: `Proposed`, `Accepted`, `Superseded`, `Rejected`.
- An ADR is never marked `Accepted` because implementation was easier; it
  is accepted only by project-owner approval or by explicit delegation in
  an owner directive.
- Proposed decisions may be implemented as experiments only within an
  authorized phase, and the implementing change must say so.
- Reversing an accepted ADR requires a new ADR that supersedes it.

## 8. CI failure policy

- The mandatory gates are exactly:

  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```

- A failing gate blocks merge. Skipping, disabling, `#[allow]`-ing away, or
  reordering gates to hide a failure is prohibited.
- Failures are fixed forward with new commits; history is not rewritten to
  hide red runs.
- Warnings are errors in CI (`-D warnings`).
- If CI cannot run (infrastructure failure, missing permissions), the phase
  status must say `CI NOT VERIFIED`; local passes are never reported as CI
  success.

## 9. Pull Request approval policy

- `main` is the integration branch. **Policy:** all changes land through a
  Pull Request targeting `main`; direct commits to `main` are not
  authorized. **Enforcement status (verified 2026-10-12):** branch
  protection is **active** on `main` — the owner authorized and applied the
  settings recorded at the end of this section (Directive 003).
- Phase branches follow `phase/<number>-<description>`.
- The PR template must be filled in completely, including CI evidence.
- Required checks must be green; a Pull Request must not be merged while
  any required check is failing.
- Agents and automated tooling never merge their own work; the project
  owner provides final authorization. (INV-017)
- Force pushes and history rewrites on `main` are prohibited. (INV-013)

### Branch protection for `main` (active)

Applied on 2026-10-12 with owner authorization (Directive 003) and verified
through the GitHub API. Effective settings:

- Require a pull request before merging, with **zero mandatory independent
  approvals** (single-maintainer project: GitHub does not allow Pull
  Request authors to approve their own Pull Requests, and no other eligible
  reviewer exists).
- Require these status checks (actual CI job names from
  `.github/workflows/ci.yml`), with “require branches to be up to date
  before merging” enabled:
  - `Format`
  - `Check (ubuntu-latest)`
  - `Check (windows-latest)`
  - `Check (macos-latest)`
- Force pushes blocked; branch deletion blocked.
- Administrators are subject to these protections, so required checks
  cannot be bypassed.

Changes to this configuration are owner actions.

## 10. Specification change control

- The authoritative specification defines all language semantics; it
  outranks any implementation, ADR, or discussion until formally changed.
- Semantic changes require an explicit, owner-approved specification
  change; the change is recorded (spec file and/or ADR) before or together
  with the implementing code.
- Implementations must not contradict the specification (INV-001); no
  unapproved semantic changes (INV-010).
- When the specification is silent, the question is recorded in
  `docs/pending-decisions.md`; implementers do not invent semantics.

## 11. Resolving technical ambiguity

1. Search the specification and existing ADRs first.
2. If the answer exists, follow it; do not relitigate it silently.
3. If it does not exist, record the question in
   `docs/pending-decisions.md` with its impact, and choose only the
   smallest reversible path that does not create semantics.
4. If work is blocked without an answer, stop that work and escalate to the
   project owner with the exact question and options; continue only with
   unaffected, authorized work.
5. Once decided, encode the outcome as an ADR or specification update and
   remove the pending entry.
