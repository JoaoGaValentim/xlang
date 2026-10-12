# ADR-0002 — Source Position Conventions

- **Status:** Proposed — implemented in Phase 0; owner confirmation pending
  (tracked as PD-007).
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
  the alignment caveat is recorded in PD-007/PD-010.

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

Neutral until confirmed: the specification is silent on these conventions
in the material available. If the original document specifies different
conventions, this ADR is superseded by an owner-approved change and the
implementation is migrated in an authorized phase.
