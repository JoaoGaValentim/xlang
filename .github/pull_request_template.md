# Pull Request

## Phase

Phase number and name (for example, `Phase 0 — Foundation`).

## Scope of changes

What this change does, and what it explicitly does not do.

## Specification references

Sections of `docs/spec.md`, directive identifiers, or ADRs that authorize
this change.

## Architectural decisions

New or amended ADRs in `docs/decisions/`, or `None`.

## Tests executed

Exact commands run and their results (for example,
`cargo test --workspace` — all tests pass).

## CI evidence

Link to the GitHub Actions run for this revision and its conclusion.

## Known risks

Risks introduced by this change, or `None known`.

## Unresolved decisions

New or remaining entries in `docs/pending-decisions.md`, or `None`.

## Acceptance checklist

- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] Required CI checks are green.
- [ ] Documentation is synchronized with the change.
- [ ] No work outside the authorized phase.
- [ ] Project-owner approval obtained before merge.
