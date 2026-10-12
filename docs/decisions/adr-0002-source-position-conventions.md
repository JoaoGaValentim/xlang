# ADR-0002 — Source Position Conventions

- **Status:** Accepted — approved by the project owner via Engineering
  Directive 003 (2026-10-12); implemented in Phase 0; PD-007 closed.
- **Date:** 2026-10-12

## Context

The specification's `Span` contract fixes the representation:

```rust
pub struct Span {
    pub file: FileId,
    pub lo: u32,
    pub hi: u32,
}
```

It does not define the unit of the offsets, the basis of line/column
numbers, newline handling, or behavior at invalid positions. Those choices
propagate into every later phase (lexer positions, diagnostic rendering,
tooling), so they must be explicit. The original specification document was
not available at decision time; this ADR records the chosen defaults with
justification so the owner can confirm or amend it.

## Decision

- **Offsets are UTF-8 byte positions**, stored as `u32`; spans are
  half-open ranges `[lo, hi)`. Byte offsets make span slicing O(1) and
  align with Rust `str`, which the compiler uses internally.
- **Lines are 1-based** in user-facing output; internally, each file stores
  the byte offset where each line starts. Lines split on `\n`; a file
  always has at least one line, and a file ending with `\n` has a final
  empty line.
- **Columns are 1-based and count Unicode scalar values** (`char`s) from
  the line start, so a multi-byte character occupies one column.
- **Display text** of a line strips the trailing `\n` and a single trailing
  `\r` (CRLF), so Windows-style files render cleanly. Columns still count
  raw content; a span that includes a stripped `\r` has its rendered
  highlight clamped to the visible line.
- **Invalid positions never panic.** Inverted spans, unknown files,
  out-of-bounds offsets, and offsets inside a multi-byte character are
  reported as `SpanError` values. At the absolute representable limits,
  lengths and columns saturate rather than overflow.
- **Registrations are bounded:** a source longer than `u32::MAX` bytes (or
  producing more lines than `u32` can address) is rejected at registration
  with `SpanError::SourceTooLarge`.

## Alternatives

- **UTF-16 offsets.** Rejected: Rust `str` is UTF-8; conversion on every
  lookup would add cost and error surface for no benefit in this project.
- **Character-index offsets.** Rejected: slicing and boundary checks
  become O(n), and the specification's `u32` fields suggest flat offsets.
- **0-based columns internally, 1-based only for display.** Equivalent
  complexity; the crate keeps 1-based `LineCol` as the single user-facing
  convention and document conventions of zero-based arrays are a pending
  question for later phases.
- **Display-width columns (wide characters, tabs).** Deferred: requires
  Unicode width data and grapheme handling, which would add dependencies;
  the alignment caveat is recorded in PD-010.

## Consequences

- Span arithmetic is constant-time and exact for any UTF-8 source.
- Diagnostic underlines align per scalar value; wide characters (CJK,
  emoji) may occupy two terminal columns while counting as one, so
  rendered output for such text can appear visually offset. Rendering
  remains deterministic.
- Tooling that exposes positions (LSP in a later phase) will need to
  translate between UTF-8 columns and UTF-16 code units at the boundary;
  that translation is isolated there.

## Specification impact

Confirmed by the project owner (Directive 003, 2026-10-12): the original
document (v0.1, archived at `docs/spec-v0.1-original.txt`) fixes only the
`Span { file, lo: u32, hi: u32 }` shape and is silent on units, bases, and
invalid-position behavior; the owner accepts this ADR as the project's
binding convention. If a future specification revision specifies different
conventions, this ADR is superseded by an owner-approved change and the
implementation is migrated in an authorized phase.

## Engineering review — Directive 002 (2026-10-12)

Reviewed by the Directive 002 engineering pass against the current
implementation of `xl-span` and `xl-diag`:

1. **Rust `str` indexing.** UTF-8 byte offsets make slicing O(1) and match
   `str` semantics exactly; every public slicing/counting path validates
   character boundaries first. Sound.
2. **Source-map efficiency.** One `Vec<u32>` of line starts per file plus
   binary search (`partition_point`) locates the line in O(log n). The
   column is then obtained by counting Unicode scalar values between the
   line start and the requested byte offset — a linear scan of the line
   prefix (O(k) in the prefix length, worst case linear in the file size) —
   so the combined mapping cost is O(log n + k), with O(lines) memory and
   no per-query allocation. Sound for compiler workloads; stated precisely
   as such.
3. **UTF-8 boundary validation.** Enforced at every public entry point
   (`span_slice`, `line_col`, `line_col_range`); violations return
   `SpanError::NotCharBoundary`. Sound.
4. **Scalar values versus grapheme clusters.** Columns count scalar values,
   so a grapheme with combining marks spans several columns. Compiler
   diagnostics are unaffected (spans come from lexer/parser token
   boundaries); editor-level grapheme semantics belong to the future LSP
   layer. Accepted, documented limitation.
5. **CJK and emoji terminal width.** Underline alignment assumes one column
   per scalar; wide characters can visually offset highlights. Deliberately
   deferred (Unicode width data would add dependencies); tracked in PD-010.
   A rendering-layer fix can be decided later without changing these
   conventions.
6. **Tabs and control characters.** Tabs count as one scalar column and
   render literally; terminal tab stops can misalign highlights. Same
   deferral as (5), at the rendering layer. Output stays deterministic and
   is pinned by exact-string tests.
7. **Future LSP UTF-16 conversion.** Byte offsets plus line starts make
   conversion to LSP positions (UTF-16 code units) a linear walk over one
   line, isolated at the protocol boundary. No Phase 0 change needed.
8. **Cross-platform consistency.** `\n`-based splitting, CRLF display
   stripping, and rootless offsets behave identically on Linux, Windows, and
   macOS; CI runs the full suite on all three. Sound.
9. **Diagnostic correctness.** Invalid positions never panic; rendering
   falls back deterministically; multi-line spans render on their first
   line only (pinned by tests; now stated in the `xl-diag` docs). Sound.
10. **Long-term API stability.** `Span` fields match the specification's
    fixed struct; `FileId` is map-scoped (aliasing is documented and cannot
    be detected in range); `LineCol` is 1-based. Whether public error enums
    should be `#[non_exhaustive]` at stabilization is part of PD-009. No
    Phase 0 change recommended.

**Associated contract clarification (severity: documentation-only, low,
no behavior change).** The `xl-span` public docs previously implied that
lookups reject identifiers produced by a different `SourceMap`; only
out-of-range raw indices are rejected, and in-range indices alias silently
by design. The documentation was corrected and the behavior pinned by the
regression test `file_ids_from_distinct_maps_alias_by_raw_index` under
Directive 002. No defect was found in the position conventions themselves.

**Recommendation:** keep the design and implementation as written.
Recommendation to the owner: **accept ADR-0002 as written** — accepted by
the project owner via Directive 003 (2026-10-12); the status is now
`Accepted` and PD-007 is closed. The implementation was not modified by
this approval.
