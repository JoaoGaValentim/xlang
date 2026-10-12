# Specification Consistency Audit

- **Directive:** Engineering Directive 002, §3 (2026-10-12)
- **Compared:** `docs/spec-v0.1-original.txt` (authoritative, v0.1,
  12/10/2026) against the repository's architectural records
  (`docs/spec.md`, `docs/architecture.md`, `docs/roadmap.md`,
  `docs/decisions/`, `docs/pending-decisions.md`) and the Phase 0 crates
  (`crates/xl-span`, `crates/xl-diag`).
- **Method:** every audited item is classified and given evidence. Items are
  not resolved by assumption; gaps remain tracked in
  `docs/pending-decisions.md`.

Classifications:

- **CONTRADICTION** — a repository decision conflicts with the original
  specification. Resolution: restore the original's wording and scope.
- **UNDERSPECIFIED** — the original leaves the behavior insufficiently
  defined; implementers must not invent it; tracked as a pending decision.
- **CONSISTENT** — the repository accurately implements or documents the
  original requirement.

## 1. Language surface (original §2, §3, §5)

| Item | Original reference | Repository record | Classification |
| --- | --- | --- | --- |
| `.x` source files; 1 file = 1 module | §2 (Modules), §3 | No contradicting statement; Phase 0 docs make no module claim | CONSISTENT (not yet implemented — Phase 1+) |
| Module and import syntax | §3 (`import std.io;`), §5 keywords | PD-006 records the partial information and open questions | UNDERSPECIFIED |
| Function definitions (`fn`, `pub`, `impl`, `self` methods) | §3 example, EBNF | No contradicting statement | CONSISTENT |
| Struct and enum declarations with payload | §3 | No contradicting statement | CONSISTENT |
| Pattern matching | §3; §7 exhaustiveness | No contradicting statement | CONSISTENT |
| `impl` and trait declarations | §3, §7 | No contradicting statement | CONSISTENT |
| Generic type syntax and bounds | §3 EBNF, §6 Phase 5, §7 | No contradicting statement | CONSISTENT |
| Mutable and immutable bindings | §3, §7 (`let mut`; assignment requires `mut`) | No contradicting statement | CONSISTENT |
| Expression-oriented blocks | §2 (Syntax), EBNF `block` | No contradicting statement | CONSISTENT |
| Numeric literals (`1_000`, `0xFF`, `0b1010`) and `as` casts | §3, §7 | No contradicting statement | CONSISTENT |
| Strings with escapes and `{expr}` interpolation | §3 | PD-004 records partial information and open representation questions | UNDERSPECIFIED |
| `Option` and `Result` types | §3 (type list), §6 Phase 5 (prelude) | No contradicting statement | CONSISTENT |
| Error propagation with `?` | §3 example, §6 Phase 2 (desugar `?` → `match`) | No contradicting statement | CONSISTENT |
| Operators and tokens | §5 (`TokenKind` sketch), §3 (Pratt chain) | PD-002 / PD-003 / PD-005 record partial information; open details preserved | UNDERSPECIFIED |

Note: the operator example `total += i as f64;` (§3) uses a compound
assignment token that does not appear in the §5 `TokenKind` sketch. This
internal partiality of the original is preserved as an open question
(PD-002 / PD-003), not resolved by the repository.

## 2. Compiler pipeline (original §4)

| Stage | Original reference | Repository record | Classification |
| --- | --- | --- | --- |
| Lexer | §4 pipeline | `docs/architecture.md` §1 states the identical pipeline; not implemented (Phase 0 scope) | CONSISTENT |
| Parser | §4, §5 (Pratt core sketch) | Same | CONSISTENT |
| AST | §4, §5 | Same | CONSISTENT |
| HIR (name resolution + desugaring) | §4, §6 Phase 2 | Same | CONSISTENT |
| Type checker | §4, §7 | Same | CONSISTENT |
| MIR (CFG) | §4, §6 Phase 3 | Same | CONSISTENT |
| Bytecode and VM | §4, §5, §8 | Same | CONSISTENT |
| Cranelift code generation | §4, §6 Phase 7 | Same | CONSISTENT |
| Driver / `xc` CLI | §4 (layout), §6 milestones | Same | CONSISTENT |

## 3. Engineering structure (original §4, §6, §9, §10)

| Item | Original reference | Repository record | Classification |
| --- | --- | --- | --- |
| Cargo workspace | §4, §6 Phase 0, §11 | Implemented: workspace with `crates/xl-span`, `crates/xl-diag`; the full 15-crate layout is the v1.x target (original §4), which Phase 0 does not contradict | CONSISTENT |
| Planned crate boundaries | §4 | `docs/architecture.md` §2–3; ADR-0001 | CONSISTENT |
| Public contracts (`Span` struct, newtype IDs) | §5, §4 (conventions) | `xl-span` matches the `Span` struct exactly; newtype-ID convention followed | CONSISTENT |
| Dependency direction | Not specified in the original | ADR-0001 (Accepted — delegated by Directive 001) fixes `xl-diag → xl-span`; does not conflict with the original | CONSISTENT (repository decision where the original is silent; recorded in the ADR) |
| Source spans: offset unit, line/column bases, invalid positions | Implied by the `Span` sketch; not defined | ADR-0002 — `Accepted` (owner approval via Directive 003); engineering review completed under Directive 002; PD-007 resolved | CONSISTENT (repository decision where the original is silent; owner-approved) |
| Diagnostics (spans, labels, suggestions) | §1 (objective), §6 Phase 2 (acceptance) | `xl-diag` implements severity/labels/rendering (Phase 0 subset); suggestions are Phase 2 scope | CONSISTENT |
| Unsafe Rust restrictions | §4 (forbid except `xl-vm`, `xl-codegen`, with Miri) | Both Phase 0 crates carry `#![forbid(unsafe_code)]`; governance INV-005 mirrors the exception rule for future phases | CONSISTENT |
| Tests and CI | §6 Phase 0, §9 | `.github/workflows/ci.yml` runs exactly `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, on a Linux/Windows/macOS matrix | CONSISTENT |
| Quality process (snapshots, fuzzing, differential, coverage) | §9 | Not yet applicable (Phases 1+); no contradicting statement | CONSISTENT (planned) |
| Development phases (ten, sequential, milestones) | §6 | `docs/roadmap.md` now transcribes all ten phases with deliverables, tests, acceptance criteria, estimates, and dependencies | CONSISTENT (was incomplete before Directive 002; PD-001 resolved) |
| Phase 0 deliverables list | §6 Phase 0 | All Phase 0 deliverables implemented; license selection resolved (PD-008, Directive 003) | CONSISTENT |
| v1.0 definition of done | §10 | Indexed in `docs/spec.md` §10 | CONSISTENT |

## 4. Findings

### 4.1 Contradictions

**None.** No repository decision was found to conflict with the original
specification document. No original wording required restoration in code or
semantics.

### 4.2 Documentation-accuracy defects (repository-internal)

Corrected under Directive 002; all are documentation-level, none change
behavior or semantics:

1. **`xl-span` public docs overclaimed identifier validation.** The docs
   implied that a `FileId` produced by a different `SourceMap` is rejected
   by lookups. In fact, only out-of-range raw indices are rejected;
   in-range indices from another map alias silently to this map's file at
   the same index. Severity: documentation-only (low). Corrected in
   `crates/xl-span/src/lib.rs`; behavior pinned by the regression test
   `file_ids_from_distinct_maps_alias_by_raw_index`; the map-scoping
   constraint is now recorded in `docs/architecture.md`.
2. **`main` branch-protection claims described enforcement that is not
   configured.** `README.md`, `CONTRIBUTING.md`, and `docs/governance.md`
   described `main` as protected although GitHub branch protection and
   rulesets are not configured (verified via the GitHub API on 2026-10-12).
   Corrected: policy and enforcement are now distinguished, and the
   recommended protection settings are recorded as an owner action in
   `docs/governance.md` §9.
3. **Working specification and roadmap were pending the original.**
   `docs/spec.md` held a Directive 001 summary and `docs/roadmap.md`
   deliberately omitted Phases 1–9. Both are now reconciled with the
   archival original; PD-001 is resolved.
4. **`xl-diag` module docs did not state the multi-line span limitation.**
   Multi-line spans render on their first line only (behavior was already
   pinned by tests). One clarifying sentence added to the crate docs.

### 4.3 Remaining underspecified areas

Preserved, not invented:

- PD-002 operator table completeness (partial information from §5 tokens
  and the §3 Pratt chain).
- PD-003 assignment operator semantics (partial: appears in the Pratt chain
  and in the §3 example).
- PD-004 interpolated string representation (partial: `{expr}` syntax,
  escape support, desugaring to concatenation in Phase 2).
- PD-005 precedence and associativity details (partial: the §3 chain).
- PD-006 import and visibility semantics (partial: 1 file = 1 module,
  `xl.toml`, `import` form, `pub`, cyclic-import detection in Phase 2).
- PD-009 public API compatibility policy.
- PD-010 diagnostics format details (partial: primary span, secondary
  labels, and suggestions required from Phase 2).

PD-007 (source position conventions) and PD-008 (license) were
subsequently resolved by owner decisions under Directive 003: ADR-0002 is
now `Accepted`, and the project is dual-licensed under `MIT OR
Apache-2.0`.

### 4.4 Audit conclusion

The repository is specification-consistent. The remaining Phase 0 matters
are owner actions (merge and formal acceptance), not specification
contradictions or code defects; the owner resolved the license and
ADR-0002 decisions under Directive 003. This audit records no change to any
established language semantics.
