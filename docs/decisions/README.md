# Architecture Decision Records

This directory holds XLANG's Architecture Decision Records (ADRs).

## Format

Every ADR contains, in order:

- **Identifier** — `ADR-NNNN`, zero-padded, never reused.
- **Title** — short imperative or descriptive title.
- **Status** — one of the states below.
- **Context** — the forces at play and why a decision is needed.
- **Decision** — what was decided.
- **Alternatives** — options considered and why they were not chosen.
- **Consequences** — what follows from the decision, including costs.
- **Specification impact** — how the decision relates to the authoritative
  specification (implements it, is silent under it, or would require a
  specification change).

## States

| State | Meaning |
| --- | --- |
| `Proposed` | Written but not approved; may be implemented only as an explicit, labeled experiment inside an authorized phase. |
| `Accepted` | Approved by the project owner, or mandated/delegated by an owner directive. Binding until superseded. |
| `Superseded` | Replaced by a later ADR, which is referenced. Kept for history. |
| `Rejected` | Considered and declined; kept so it is not silently retried. |

An ADR is never marked `Accepted` merely because implementation was easier
(see `docs/governance.md` §7).

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](adr-0001-crate-layout-and-dependency-direction.md) | Crate layout and dependency direction | Accepted — mandated by Directive 001 |
| [0002](adr-0002-source-position-conventions.md) | Source position conventions | Proposed — owner confirmation pending (PD-007) |
| [0003](adr-0003-toolchain-and-edition.md) | Toolchain pinning and Rust edition | Accepted — delegated by Directive 001 |
