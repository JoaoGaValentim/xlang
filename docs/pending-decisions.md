# Pending Decisions

Questions that the specification does **not** (fully) answer yet.
Implementers must not invent answers (see `docs/governance.md` §10–11).
Each entry states the decision needed, why it matters, and its current
status. Statuses: `RESOLVED`, `PARTIALLY DEFINED` (the original provides
some information, listed; the remaining question is preserved),
`UNRESOLVED`, `PROPOSED` (implemented as an explicitly labeled choice,
owner confirmation pending).

## PD-001 — Original specification document not in repository

- **Question:** Where is the original "xlang — Specification and
  Engineering Plan" v0.1 (2026-10-12) document, and when will it be
  committed to this repository?
- **Impact:** Phases 1–9 scope and several semantic areas could not be
  transcribed without the original; Phase 1 was blocked on it.
- **Status:** **RESOLVED (2026-10-12, Directive 002).** The owner supplied
  the original; it is archived byte-faithfully at
  `docs/spec-v0.1-original.txt` (SHA-256
  `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770`).
  `docs/spec.md` and `docs/roadmap.md` were reconciled against it, and the
  consistency audit is recorded in `docs/spec-consistency-audit.md`.

## PD-002 — Operator grammar completeness

- **Question:** Which operators exist in XLANG, and what are their exact
  tokens and arities?
- **Partial information from the original:** the §5 `TokenKind` sketch
  lists `Plus, Minus, Star, Slash, Percent, Eq, EqEq, NotEq, Lt, Le, Gt,
  Ge, AndAnd, OrOr, Bang, Arrow, FatArrow, Question, Dot, DotDot,
  ColonColon` and the delimiters; the §3 EBNF fixes a Pratt chain from
  assignment (lowest) through `||`, `&&`, comparison, `+ -`, `* / %`,
  unary, to postfix (highest). No bitwise operators appear.
- **Remaining questions:** the §3 example uses `+=` (`total += i as
  f64;`) although no compound-assignment token exists in the §5 sketch —
  the compound-assignment operator set must be confirmed; the unary
  operator set is not enumerated.
- **Impact:** Lexer and parser design in Phase 1.
- **Status:** PARTIALLY DEFINED — the remaining table must be confirmed by
  the owner before Phase 1 implements operators.

## PD-003 — Assignment operator semantics

- **Question:** Is `=` an expression or a statement? Does XLANG have
  compound assignment (`+=`, …)? Can assignment be chained? Does
  assignment produce a value?
- **Partial information from the original:** assignment appears in the §3
  Pratt chain at the lowest precedence (suggesting an expression form) and
  appears as a statement in the §3 example (`total += i as f64;`), which
  also demonstrates compound assignment.
- **Remaining questions:** all matters listed in the question above; the
  compound-assignment token set (see PD-002).
- **Impact:** Parser grammar and MIR semantics.
- **Status:** PARTIALLY DEFINED.

## PD-004 — Interpolated string representation

- **Question:** How are interpolated strings tokenized and represented in
  the AST (segments, embedded expressions, escapes)?
- **Partial information from the original:** strings support escapes and
  `{expr}` interpolation (§3 lexical note); the §5 token sketch carries
  `Str(Symbol)`; Phase 2 desugars interpolation to concatenation (§6
  Phase 2).
- **Remaining questions:** tokenization of interpolation (single interned
  symbol versus segment list), escaping of literal `{`/`}`, diagnostics
  for malformed interpolation, and the exact AST representation.
- **Impact:** Lexer/parser; diagnostics for malformed interpolation.
- **Status:** PARTIALLY DEFINED.

## PD-005 — Operator precedence

- **Question:** What is the exact precedence and associativity table?
- **Partial information from the original:** the §3 EBNF fixes the chain
  assignment < `||` < `&&` < comparison < `+ -` < `* / %` < unary <
  postfix, and `as` casts appear in the postfix production.
- **Remaining questions:** associativity of each level; precedence within
  the comparison level (`==`/`!=` versus `<`/`<=`/`>`/`>=`); the position
  of the range operator `..` (used in `for i in 0..10`); the exact unary
  operator set.
- **Impact:** Parser; silently wrong parses are a correctness hazard.
- **Status:** PARTIALLY DEFINED — must be completed before Phase 1 parses
  expressions.

## PD-006 — Import and visibility semantics

- **Question:** What are the module system, import forms, and visibility
  rules (`pub`, private-by-default, re-exports)?
- **Partial information from the original:** 1 `.x` file = 1 module and
  packages are described by `xl.toml` (§2); `import std.io;` is the only
  import form shown (§3); `pub` is a keyword and appears on functions
  (§3, §5); Phase 2 detects duplicate/undefined names and cyclic imports
  (§6 Phase 2); Phase 5 includes “módulos e pub” (§6 Phase 5).
- **Remaining questions:** import grouping/aliases (`as`), re-exports,
  path syntax inside imports (the examples use `.` while expressions use
  `::`), and whether items are private by default.
- **Impact:** Name resolution, HIR.
- **Status:** PARTIALLY DEFINED.

## PD-007 — Source offset and column conventions

- **Question:** Does the specification agree with the Phase 0 choice of
  UTF-8 byte offsets (`u32`), 1-based lines, and 1-based columns counted in
  Unicode scalar values? Should display-width alignment (wide characters,
  tabs) matter for rendered diagnostics?
- **Original reference:** the original fixes only the `Span { file, lo: u32,
  hi: u32 }` shape (§5) and is silent on units, bases, and invalid-position
  behavior. The Phase 0 choice therefore remains a proposal.
- **Impact:** Every span, every rendered diagnostic, tooling interfaces.
- **Status:** PROPOSED — implemented per
  `docs/decisions/adr-0002-source-position-conventions.md`. The Directive
  002 engineering review found no defect and recommends acceptance (see the
  ADR's “Engineering review” section). Owner confirmation (Accept, or amend
  the ADR) is still required.

## PD-008 — License selection (mandatory Phase 0 decision)

- **Question:** Which license(s) will XLANG use?
- **Impact:** Without a license, default copyright applies (all rights
  reserved) and the project is not open for contributions under a known
  grant. Phase 0 cannot be fully approved with this unresolved.
- **Options prepared for the owner (Directive 002 §6):**
  - **MIT** — short and maximally permissive; no explicit patent grant;
    minimal obligations for downstream users.
  - **Apache-2.0** — permissive with an explicit patent grant and patent
    termination clause; longer text; NOTICE-file conventions; commonly
    preferred by companies.
  - **MIT OR Apache-2.0 (dual)** — Rust-ecosystem standard: recipients may
    choose either license, so it maximizes compatibility; contributions
    are licensed under both; slightly more documentation overhead.
  - Technical recommendation: **dual MIT OR Apache-2.0**, matching the
    practice of the Rust toolchain ecosystem; the owner decides.
- **Files that will require updates after selection:** a new license file
  (`LICENSE-MIT` / `LICENSE-APACHE`, or `LICENSE` for a single license);
  `[workspace.package] license = "…"` in `Cargo.toml` plus
  `license.workspace = true` in each crate manifest; the License sections of
  `README.md` and `CONTRIBUTING.md`; closure of this entry and the license
  row in `docs/phase-status.md`.
- **Status:** UNRESOLVED — owner decision required. No license was chosen
  and no license file was created without authorization.

## PD-009 — Public API compatibility policy

- **Question:** What compatibility guarantees apply to `xl-span`,
  `xl-diag`, and future crates' public APIs between phases and releases?
  Should public enums (e.g., `SpanError`, `Severity`) be `#[non_exhaustive]`
  at stabilization?
- **Impact:** Review strictness; whether breaking API changes are routine
  during development; versioning of crates (`publish = false` today).
- **Status:** UNRESOLVED — proposal needed before external consumers exist.

## PD-010 — Diagnostics specification details

- **Question:** Should diagnostics carry stable error codes (e.g.,
  `E0308`-style)? What exact rendered format is canonical for the project
  (color, terminal width handling)?
- **Partial information from the original:** high-quality diagnostics
  (spans, labels, suggestions) are an explicit objective (§1) and Phase 2
  acceptance requires a primary span, secondary labels, and a suggestion
  (§6 Phase 2). Color, error codes, and terminal-width handling are not
  mentioned.
- **Impact:** Diagnostic stability for users and tests; may amend
  `xl-diag` rendering. Related: PD-007 (display-width alignment).
- **Status:** PARTIALLY DEFINED — the current plain-text Phase 0 format is
  a repository choice, not a specification claim.
