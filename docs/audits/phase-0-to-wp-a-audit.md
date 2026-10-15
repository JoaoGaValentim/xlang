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
| HIR-004 / P1 | `xl-hir`, portable native-stack safety at ratified HIR depth | The original `deep_hand_built_expressions_are_poisoned_at_the_limit` test crashed on the Windows runner with `STATUS_STACK_OVERFLOW` (PR #8 initial CI run `38659704990`, Windows job `116177794199`). Reproduced on macOS with `RUST_MIN_STACK=1048576`, exiting with `SIGABRT`. Recursive lowering exhausted a 1-MiB stack before reaching the permitted 256-depth poison boundary. | Guard the bounded lowering and structural-validation traversals with conditional stack growth; run both 240-array valid lowering/validation and 300-array depth-poison cases inside an explicitly 1-MiB test thread on every platform. | PASS: targeted 1-MiB test locally and complete Windows/Ubuntu/macOS test jobs on final code SHA `6f64d08` (CI `38664341782`). |
| HIR-005 / P1 | `xl-hir`, ratified budgets must never be enlarged through public test overrides | Before correction, `lower_structure_with_limits` and `validate_with_limits` accepted caller-supplied values of `u32::MAX`; a 300-array AST bypassed the depth cutoff and overflowed the test stack. | Cap all four caller-provided limits to their ratified maxima at both public entry points. `explicit_limits_cannot_relax_the_ratified_depth_budget` reproduces the failure with an enlarged depth budget; `configurable_limits_can_tighten_but_never_expand_ratified_budgets` checks all four caps without allocating large arenas. | PASS: targeted depth regression locally and four-cap unit regression in cross-platform workspace suite (CI `38664341782`). |
| HIR-006 / P2 | Deterministic snapshots require identical source bytes on all CI platforms | Windows run `38663676913` passed deep resource tests but failed `snapshot_specification_example`: CRLF checkout changed fixture offsets (`item#1 span=16..47` on LF versus `18..49` on CRLF) without altering HIR structure. | Add `.gitattributes` to pin `crates/xl-hir/tests/fixtures/spec_example.x` to LF and assert the fixture has no CR before lowering. Preserve production source handling and the original specification archive. | PASS: Windows snapshot and separate Format job on final code SHA `6f64d08` (CI `38664341782`). |
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
Following HIR-004's stack-safety correction, the harness was compiled again
and ran **25,114 additional executions in 21 seconds** without recorded
crashes (`evidence/r1-windows-fix-hir-fuzz.log`). Earlier runs used the
pre-HIR-004 code; only the final run exercises its stack-growth guard.

## Directive 011-R1 local verification

On `phase/2-hir-foundation`, these checks completed successfully on macOS
before and after HIR-004. The table reports the later post-fix execution;
logs are retained in the external recovery snapshot.

| Gate | Result / evidence |
| --- | --- |
| `cargo fmt --all -- --check` | PASS — `evidence/r1-post-winfix-fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — `evidence/r1-post-winfix-clippy.log` |
| `cargo test --workspace` | PASS — **613 passed, 0 failed**, in 65 test/doc-test result groups; `evidence/r1-post-winfix-test.log` |
| `cargo build --workspace --locked` | PASS — `evidence/r1-post-winfix-build.log` |
| Archived specification SHA-256 | PASS — `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770` |
| `lower_hir` post-repair fuzzing | 39,396 executions in 31 seconds, no recorded crash; `evidence/r1-final-hir-fuzz.log` |
| `lower_hir` after HIR-004 | 25,114 executions in 21 seconds, no recorded crash; `evidence/r1-windows-fix-hir-fuzz.log` |

Cross-platform GitHub CI on the proposed PR remains a separate required
gate. Local macOS checks are not evidence of Linux or Windows success.

## Directive 011-R2 — HIR-004 follow-up and final validation gate

The local R2 working tree began at `fb0e76eff22c2f6024af7d4d50764886e6464424`
on `phase/2-hir-foundation` with 10 modified tracked files and no untracked
files. A new external snapshot, `~/.local/share/xlang-recovery/011-R2-final-20261015T005532Z`,
preserved 42 source artifacts and 10 Git index versions. Its `manifest.json`
SHA-256 is `b9008de33864c5f2284f00889c3d7fba75681447e2f6d4e85a39f1994bf6bd17`;
an independent temporary restoration verified byte counts and hashes. The
original 011-R1 and first 011-R2 archives remain present.

The 1-MiB reproducer on the pre-fix revision reached the lowering call and
overflowed before that call returned. It did not produce a complete native
stack trace, so the exact recursive frame cannot be assigned conclusively.
Lowering contains recursive expression traversal; validation also recursively
walks HIR arena references. Both public entry points now use
`stacker::maybe_grow` with a 1,536-KiB remaining-stack threshold and an
8-MiB segment for the bounded traversal. The 256-depth poisoning check stays
independent of stack growth. AST destruction happens inside the regression's
1-MiB thread; the resulting HIR arena does not recursively own its nodes.

The targeted `deep_hand_built_expressions_are_poisoned_at_the_limit` test
passed on macOS with `RUST_MIN_STACK=1048576`: 64, 100, 128, 240, and 255
wrapping arrays lower and validate; 256, 257, 300, and 1024 arrays each
produce the one ratified depth-limit diagnostic and structurally valid
poisoned HIR. Each array wrapper adds one HIR expression level around the
literal, so 255 wrappers reach a counted depth of 256. The same test checks
cyclic and out-of-range references within the deeply built arena. The
`explicit_limits_cannot_relax_the_ratified_depth_budget` targeted regression
also passed after capping the caller-specified limits.

**Additional Windows CI finding:** runs `38663130532` and `38663294062`
confirmed that the expanded `tests/errors.rs` test passed on Windows, but
the two deep `tests/resource.rs` cases
(`at_most_one_depth_diagnostic_is_reported` and
`explicit_limits_cannot_relax_the_ratified_depth_budget`) still overflowed
their default test-harness worker stacks. These failures are distinct from
the originally failing `tests/errors.rs` regression. The resource cases now
execute construction, lowering, validation and drop in explicitly sized
1-MiB worker threads, as the passing error regression already did. The
complete resource suite passed locally after that change (7/7); the final
Windows CI run also passed. The exact overflowing frame in the
original default-worker test was not captured by the CI logs, so whether
it was lowering or destruction on that path remains unproven.

The same updated Windows run `38663676913` subsequently passed the deep
resource suite but exposed HIR-006, a cross-platform snapshot fixture
normalization issue. A bounded `lower_hir` fuzz run on `0a1d20b` used the
installed `nightly-2026-10-02` compiler and completed **16,282 executions in
13 seconds**, no reported crash (601 MiB peak RSS). The initial fuzz attempt
using pinned stable 1.99 failed before any execution because its `-Z`
sanitizer flag requires nightly; those failed runs are not counted.

CI run `38664048209` passed all three Check jobs, including Windows
snapshot, 1-MiB resource tests and the new locked workspace build. Its
separate Format job rejected the layout of one assertion in
`tests/snapshots.rs`; this was corrected using `cargo fmt --all` and verified
by the subsequent successful CI run `38664341782`.

**Dependency assessment.** `stacker` is pinned to exactly `0.1.25`; the
lockfiles resolve `psm 0.1.32`. Its production dependency tree includes
`cfg-if 1.0.5`, `libc 0.2.190`, and `psm 0.1.32`; native build dependencies
include `cc 1.8.0`, `ar_archive_writer 0.5.3`, and their locked transitive
dependencies. Rust is pinned to 1.99.0. This solution keeps the recovered
HIR traversal and limits intact while protecting public entry points on
supported targets; an iterative rewrite would touch a substantially wider
surface. XLANG's `#![forbid(unsafe_code)]` remains in force; third-party
stack assembly is outside that crate-level guarantee. Growth requires a
platform implementation capable of measuring remaining stack. Unsupported
targets, unusual toolchains, and native-stack resource exhaustion remain
portability limitations; CI validates only Linux, macOS, and Windows.

**Pending verification.** The post-HIR-005 `cargo fmt --all -- --check`
initially reported an expected formatting difference in `tests/resource.rs`;
`cargo fmt --all` then completed. A subsequent attempt to run the four
mandatory workspace gates as a single logged process was blocked by the
Codex tool safety-status check before a subprocess result was returned.
Those four results remain **UNVERIFIED as a complete local run on the final
revision**. The R2 CI workflow added `cargo build --workspace --locked` to
all three OS jobs. GitHub Actions run `38664341782` subsequently executed
all four mandatory gates successfully against `6f64d0841ff0a84b51ac66264669a0494b54ad1b`.
The previously recorded 613-test local run and 25,114 fuzz executions apply
to earlier local revisions and are not counted as final-revision evidence.

### Final cross-platform evidence — Directive 011-R2

The completed [CI run 38664341782](https://github.com/JoaoGaValentim/xlang/actions/runs/38664341782)
on code SHA `6f64d0841ff0a84b51ac66264669a0494b54ad1b`
reported **success** on 2026-10-15 UTC. Its four jobs finished successfully:

| Job | Result | Actual test summaries |
| --- | --- | --- |
| Format | PASS — `cargo fmt --all -- --check` | N/A |
| Check (ubuntu-latest) | PASS — Clippy, workspace tests, locked build | 615 passed; 0 failed; 0 ignored (65 groups) |
| Check (macos-latest) | PASS — Clippy, workspace tests, locked build | 615 passed; 0 failed; 0 ignored (65 groups) |
| Check (windows-latest) | PASS — Clippy, workspace tests, locked build | 613 passed; 0 failed; 0 ignored (65 groups) |

All check jobs completed `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, and `cargo build --workspace --locked` successfully.
The full workspace suite includes the 1-MiB HIR depth/poison regression,
configurable-limit regression, desugaring cases, structural HIR validation,
and specification-example snapshot. Windows' total differs from Unix by two
platform-specific tests, as in the historical accepted baseline.

PR #8 was open and mergeable with a clean published branch at the code SHA
above. **WP-A is ready for manual project-owner review and integration**
based on the final code/CI evidence, subject to a fresh successful CI check
for any later commits, including this documentation-only audit update.
No merge, owner acceptance, or WP-B start is authorized here. The PR
description still contains earlier R1 test counts; updating its text was
denied by GitHub HTTP 403 in the previous attempt.

Unproven questions, not classified as confirmed defects: possible aliasing
of a HIR node through multiple parent paths could make visitation-sensitive
depth/loop checks incomplete; synthetic parser token streams may require a
stronger mathematical uniqueness argument for saturating AST IDs. These
deserve dedicated counterexample tests before any functional changes.

Only the project owner may merge the WP-A PR. WP-B requires its integration,
passing Format/Ubuntu/macOS/Windows CI, and a separate owner authorization.
The pre-WP-B gate includes confirmation that no name resolution, graph
validation, type checking or `xc check` has been smuggled into WP-A.

## Directive 011-R3 — final structural-validator soundness gate

**Starting HEAD:** `a46065d6714c8ab671d059a48abc348f8e51b89f`.
**Corrective commit:** `d5c8306` (`fix(hir): reject orphaned definitions and context-skipped ownership paths`).
The existing PR #8 remains the sole integration proposal; owner merge is prohibited at this gate.

### V-1: definition reachability and ownership

**Confirmed defect.** Previously `walk_roots` treated every unvisited
definition as a new root without issuing a violation. Consequently complete,
well-indexed, but disconnected top-level items, locals, parameters, generics,
and struct fields passed validation. The four first negative regressions
(`v1_orphaned_top_level_item_is_rejected`, `v1_orphaned_local_is_rejected`,
`v1_orphaned_parameter_and_generic_are_rejected`, and
`v1_valid_parent_without_owner_edge_is_rejected`) all failed with an empty
violation list on the original validator. They pass with the correction.

`walk_roots` now reports `UnreachableArenaEntry` for every definition not
reached through an owning item, definition edge, or binding pattern. It still
walks disconnected definitions to diagnose malformed references. `walk_def`
follows pattern-local ownership; `walk_owned_def` checks exact parent identity
for fields, variants, trait items, generics and signature parameters. The
signature checker also rejects a non-parameter in the parameter list. A
pattern binding must reference a local definition. Additional passing negative
tests cover a field whose `parent` denotes a different valid struct and a
receiver missing from its signature.

**Accepted representation exceptions:** an `impl` has no method-ID list,
so `Method.parent = impl DefId` is the documented direct owner link;
methods with a reachable impl and a matching signature are traversed as valid.
Local definitions use `Local.body` metadata and a binding pattern as their
declaration link. Legitimate desugaring-generated locals and the accepted
module/block item slot representation remain valid.

### V-2: multiple structural paths and validation context

**Confirmed defect.** A globally visited node/body could silently bypass its
second ownership path, including a loop target that is valid in only the
first path. The original validator failed five cause-specific regressions:
shared expression siblings, shared body, a node under two loops, a break
reused outside its enclosing loop, and a signature referenced by a second
definition. The signature and break cases already produced an incidental
unreachable-entry diagnostic, but lacked the direct ownership violation.
All regressions now pass, including the additional `continue` case.

For **nodes, bodies, and signatures**, second owning traversal edges report
`MultipleStructuralOwners`; active recursive references continue to report
`CyclicReference`. A nested item may appear in both `HirModule.items` and a
`StmtKind::Item` for its one declaration; this valid alias is handled once,
while two item slots introducing the same definition are rejected. Repeated
definition ownership edges also report `MultipleStructuralOwners`.

**Type references can be deliberately shared** between bindings lowered from
one annotated destructuring pattern, per the existing `Local.ty` layout.
Thus type references use context-aware validation: record the greatest
effective depth checked for each type ID and traverse it again only when
reached at a deeper depth. An isolated negative test verifies that a child
under a shared type cannot escape `DepthExceeded` through an earlier shallow
visit. The ratified maximum of 256 bounds per-type rechecks; active flags
still reject cycles. No HIR ownership or origin contract changed.

### R3 verification and residual risk

- Red phase: 4/4 V-1 negative tests failed (`[]`); 5/5 initial V-2
  ownership-negative tests failed to produce the expected direct violation.
- Green phase: `validate_negative` **41/41**; `coverage_items` **10/10**;
  complete `cargo test --workspace` **629 passed, 0 failed, 0 ignored**
  across 65 groups on macOS.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo build --workspace --locked` all **PASS**.
- Explicit 1-MiB thread stack regression, ratified-budget suite (7/7),
  and `spec_example_end_to_end_conformance` all **PASS**.
- Bounded `lower_hir` fuzzing on installed nightly with address sanitizer:
  **19,347 executions in 16 s**, 0 crashes, 89 new units.
- Previous R2 cross-platform baseline: CI run `38664341782` passed Format,
  Ubuntu (615), macOS (615), Windows (613) at its then-published SHA.
  The R3 final SHA requires its **own four successful jobs** before merge.

Unproven, nonblocking risks: extreme synthetic Phase 1 token-ID collision
behavior lacks a reproducible defect; native stack growth remains
platform-dependent outside the tested runner set; fuzzing is bounded and is
not an exhaustive proof. No parser, lowering, desugaring, ADR-0005, or original
specification was changed. WP-B, `xl-types`, and `xc check` remain excluded.
