# XLANG — Directive 011-R1: historical audit and HIR recovery

**Date:** 2026-10-14. **Baseline:** `5eaa6ab0b80c9ea343c183a685c632c0be4a9fb1`.
**Scope:** Phases 0–1 and Phase 2 WP-A. **Authority:** ADR-0005 and
`docs/proposals/phase-2/`; the archived v0.1 specification remains authoritative.
**Status:** WP-A changes proposed for owner review; no owner merge or WP-B
authorization is implied by this record.

## Recovery and independent baseline

The original worktree was `phase/2-hir-foundation`, pointing at the integrated
`main` SHA above, with no divergent local commits. It contained five modified
tracked files (`Cargo.toml`, `Cargo.lock`, `README.md`, `fuzz/Cargo.toml`,
`fuzz/Cargo.lock`) and 39 untracked files, including `crates/xl-hir/` and
`fuzz/fuzz_targets/lower_hir.rs`. No original changes were discarded.

Before source editing, a byte-checked external snapshot was created at
`~/.local/share/xlang-recovery/011-R1-20261014T233613Z-36176/`.
`VERIFIED.json` attests to 33,518 files, 10,686,165 bytes, 5 tracked
modifications, 39 untracked files, and 33,474 preserved ignored fuzz files.
The snapshot includes Git diffs, a Git bundle, test evidence and a SHA-256
manifest. `manifest.sha256.json` hashes to
`88103bf1edabec2ce77b021dc9fde5d68161e70011524bd32600cf38ec8c5e7d`.
Manifest verification, byte-size/hash rechecking and a separate restoration
exercise passed. The original archive `docs/spec-v0.1-original.txt` hashes to
`c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770`.
Additional copies of the post-snapshot validator edits and their checksums
are stored under `post-snapshot-validator-20261014/` in the snapshot.

A distinct clean worktree at `<snapshot>/baseline` was constructed from
integrated `main`. Its format, Clippy, workspace tests and locked build
all passed (`evidence/baseline-*.log` and `baseline-gates.json`). Separate
CLI probes returned the expected codes: 0 for empty/comment-only/valid
inputs, 1 for malformed source, 2 for invalid UTF-8, missing files and
usage errors. The archived specification example parsed through `xc parse`.
Historical CI counts (Ubuntu/macOS 484; Windows 482) are historical reference
points, not outcomes of this directive's pending PR CI.

## Historical reconstruction

| Phase and accepted deliverable | PR | Integrated merge SHA |
| --- | --- | --- |
| Initial repository | — | `2daa4b1` |
| 0: workspace, spans, diagnostics | #1 | `4d82106` |
| 1: lexical/syntactic contracts | #2 | `db2437b` |
| 1: lexer | #3 | `9ed50ad` |
| 1: AST | #4 | `df9e859` |
| 1: parser | #5 | `b4a7416` |
| 1: `xc parse` | #6 | `210e1e0` |
| 2: owner-ratified architecture, ADR-0005 | #7 | `5eaa6ab` |

The sequence is corroborated by `git log --all --graph`, merge commits,
ratification documents and existing CI records in `docs/phase-status.md`.
Acceptance through PR #7 is a historical governance fact; it does not prove
the absence of latent implementation defects.

## Historical component inspection

| Crate | Inspected behavior and test evidence | Result / limits of evidence |
| --- | --- | --- |
| `xl-span` | `FileId` map scope, checked `Span`/`SourceMap`, half-open byte offsets, UTF-8 boundary checks, CRLF and EOF mapping, overflow-conscious `u32` conversions; `span_api` and unit suites | Baseline tests pass. No new reproducible defect established. Cross-map aliasing is documented and intentional. |
| `xl-diag` | Ordered primary/secondary labels, invalid-span fallback, Unicode-scalar positioning, deterministic plain-text rendering; `diagnostics` tests | Baseline tests pass. Terminal display-width alignment remains an existing open policy decision. |
| `xl-lexer` | Unicode identifiers, numeric/string/escape forms, nested comments and interpolation depth, malformed-token diagnostics, resource tests and snapshots | Baseline tests pass. Historical fuzzing applies to its historical revision; no new confirmed regression. |
| `xl-ast` | `ItemId`/`ExprId`, structural validation, recursion guards, canonical precedence/associativity and cross-platform determinism/property tests | Baseline tests pass. No reproduced hash-order instability or invalid round trip in this audit. |
| `xl-parser` | Recursive descent/Pratt binding, types/patterns, synchronization and depth bound 64; seed-pinned property tests and malformed-input recovery | Baseline tests pass. Saturating AST-ID increments warrant future bound analysis for synthetic `parse_tokens` input, but no collision was demonstrated. |
| `xl-cli` | `xc parse`, source/usage error separation, exit 0/1/2, stdout/stderr, empty file, invalid UTF-8, missing file, Unicode and original example | Clean baseline CLI matrix passes. No verified functional CLI defect. |

For Phase 1, past fuzz results are recorded in the historical phase-status
document. They are not counted as fresh fuzzing of the recovered HIR.
Inspection and executed suites establish no newly verified Phase 0/1
functional defect; this is not a claim of exhaustive formal verification.

## ADR-0005 and WP boundary

The crate dependency graph keeps `xl-hir` independent of `xl-parser` in
production. `lower_structure` receives AST items, `FileId`, `ModuleId`, and
`PackageId`; it creates arena-backed, owned HIR and unresolved source uses.
This is the WP-A structural operation. The ratified final interface
`lower(&[Item], FileId, &ModuleRegistry) -> LowerOutput { module,
resolutions, import_edges, diagnostics, fate }` is a WP-B composition, with
caller-side `record_import` then separate module-graph validation. WP-A
intentionally does not create a registry, resolve imports, validate cycles,
type-check or implement `xc check`.

Checked arena indices use `u32` and deterministic allocation; supported
desugarings lower `for` into iterator/loop primitives, `?` into Result
matching, and interpolation into formatting/concatenation. Internal uses
carry established compiler-owned identities rather than source-visible
temporary names. Origin tables retain AST identities or deterministic
structural/synthesized keys. The ratified budgets are depth 256, nodes
1,000,000, definitions 100,000 and uses 1,000,000.

## Verified defect register

| ID / severity | Component and contract | Reproduction and root cause | Correction and regression | Verification / delivery |
| --- | --- | --- | --- | --- |
| HIR-001 / P2 | `xl-hir::validate`, import record consistency | Change an import-record declaration span to disagree with its item or module-binding span. Previous validator compared path/binding but omitted these equality checks. | Require `item.span == record.decl_span` and `binding.span == record.decl_span`; `mismatched_import_spans_are_rejected` and related import-negative tests. | Negative-validation suite passed; included in WP-A PR. |
| HIR-002 / P1 | `xl-hir::validate`, all arena references must be valid | Append an unreachable node containing `HirId(u32::MAX)` and a matching origin. **Red phase:** original validator returned `[]` and the test failed. Disconnected bodies and types likewise escaped full traversal. | Track all visited arena entries, report `UnreachableArenaEntry`, traverse orphaned nodes/bodies/types to detect malformed references; mark poison expression/statement and error-type entries visited. Tests `orphaned_arena_node_with_invalid_reference_is_rejected`, `orphaned_body_with_invalid_statement_is_rejected`, `orphaned_type_with_invalid_element_is_rejected`. | All three negative tests and full HIR suite pass; included in WP-A PR. |
| HIR-003 / P1 | `xl-hir::validate`, signature arena completeness | Append a cloned, unreferenced `FnSig` with an invalid parameter `DefId(u32::MAX)`. **Red phase:** `validate()` returned `[]`; targeted test failed. Signatures were checked for spans only; no reachability or contained-reference traversal occurred. | Track visited signatures, reject orphan signatures, traverse their referenced params/return types, reject unknown signature owners; `orphaned_signature_with_invalid_parameter_is_rejected`. | Red reproduction recorded; complete HIR suite passes after correction; included in WP-A PR. |
| DOC-001 / P3 | Repository status accuracy | `docs/phase-status.md`, `docs/roadmap.md` and `docs/architecture.md` retain an earlier statement that PR #7 is unmerged and HIR is not authorized, contrary to integrated `main` SHA `5eaa6ab` and Directive 011-R1. | Correct the current status to integrated architecture/authorized WP-A while retaining chronological historical acceptance records. | Documentation-only correction within the WP-A update, not a language-semantic change. |

No unrelated Phase 0 or Phase 1 functional correction was needed, so no
speculative historical code patch is bundled into the HIR implementation.
Defects HIR-001/002/003 were found in unmerged recovered WP-A code and
therefore were never accepted production regressions.

## Recovery classification

| Disposition | Recovered files / explanation |
| --- | --- |
| KEEP | `crates/xl-hir/src/{hir,origin,limits,fate,lib,dump,messages}.rs`, `src/lower/{mod,alloc,expr,item,desugar}.rs`, the crate manifest, coverage/desugaring/error/identity/origin/resource/snapshot/specification tests and fixtures; recovered workspace/fuzz manifest integration; production-api fuzz target. Verified by HIR integration tests. |
| REPAIR | `crates/xl-hir/src/validate.rs` and `tests/validate_negative.rs`: HIR-001, HIR-002, HIR-003; recovered README status and test-count claims. |
| GENERATED | Ignored fuzz corpus/artifacts/build products: retained in the external snapshot, excluded from review commits. |
| DEFER | Registry, syntactic resolution, import graph, typing, CLI `xc check`: these are WP-B/C/D features, not unfinished WP-A changes. |
| UNRESOLVED | The author/provenance of the original uncommitted HIR files is not inferable from file timestamps. No attributable off-scope source modification was established. |

There was no technical evidence that replacing the existing HIR code would
be beneficial. Existing lowering, tests, arena layout, desugaring and fuzz
harness were retained, with localized validator repair.

## Coverage, robustness and follow-up

WP-A suites exercise all supported AST variant families, import records,
definitions/uses, signatures, type references, patterns, lexical recovery,
origin identities, three desugarings, ordering and capture avoidance,
determinism, poison states, resource-limit boundaries and negative HIR
validation. Test assertions inspect actual HIR structures and resolution
tables. The original specification example has separate parse/lower/dump
snapshot and determinism tests.

The recovered `lower_hir` fuzz harness invokes the production `xl_parser`
pipeline on lossily decoded arbitrary bytes, structurally lowers the AST,
validates completed modules and their resolution tables, checks controlled
aborts and compares repeated lowering for determinism. Invalid UTF-8 is
separately rejected by `xc parse` and is deliberately normalized for this
in-process fuzz target. An earlier 21-second run made 28,209 executions
without recorded crashes; it predates the final validator changes. A later
31-second run on the corrected validator executed 39,396 units without
recorded crashes (`evidence/r1-final-hir-fuzz.log` in the external recovery
snapshot). This limited run is adversarial evidence, not a proof of safety.

## Directive 011-R1 local verification

On `phase/2-hir-foundation`, these checks completed with exit code zero on
macOS. Evidence logs are retained in the external recovery snapshot.

| Gate | Result / evidence |
| --- | --- |
| `cargo fmt --all -- --check` | PASS — `evidence/r1-final-fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — `evidence/r1-final-clippy.log` |
| `cargo test --workspace` | PASS — **613 passed, 0 failed**, in 65 test/doc-test result groups; `evidence/r1-final-test.log` |
| `cargo build --workspace --locked` | PASS — `evidence/r1-final-build.log` |
| Archived specification SHA-256 | PASS — `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770` |
| `lower_hir` post-repair fuzzing | 39,396 executions in 31 seconds, no recorded crash; `evidence/r1-final-hir-fuzz.log` |

Cross-platform GitHub CI on the proposed PR remains a separate required
gate. Local macOS checks are not evidence of Linux or Windows success.

Unproven questions, not classified as confirmed defects: possible aliasing
of a HIR node through multiple parent paths could make visitation-sensitive
depth/loop checks incomplete; synthetic parser token streams may require a
stronger mathematical uniqueness argument for saturating AST IDs. These
deserve dedicated counterexample tests before any functional changes.

Only the project owner may merge the WP-A PR. WP-B requires its integration,
passing Format/Ubuntu/macOS/Windows CI, and a separate owner authorization.
The pre-WP-B gate includes confirmation that no name resolution, graph
validation, type checking or `xc check` has been smuggled into WP-A.
