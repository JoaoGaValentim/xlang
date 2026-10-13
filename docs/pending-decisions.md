# Pending Decisions

Questions that the specification does **not** (fully) answer yet.
Implementers must not invent answers (see `docs/governance.md` §10–11).
Each entry states the decision needed, why it matters, and its current
status. Statuses: `RESOLVED`, `PARTIALLY DEFINED` (the original provides
some information, listed; the remaining question is preserved), and
`UNRESOLVED`.

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
- **Phase 1 proposal (pending owner approval):** complete token inventory
  and gap resolutions — `Break`/`Continue` restoration, the compound
  assignment family, `Underscore`, and the recovery-only `Error` token — in
  `docs/proposals/phase-1/grammar-rfc.md` §2; decision-packet row PD-002.
  Numeric literal payloads (magnitude + radix) require the explicit
  amendment **B-2**; float overflow policy is **B-3**
  (`docs/proposals/phase-1/decision-packet.md`, category B).
- **Status:** **RESOLVED (2026-10-12, Directive 006).** The owner approved
  the token inventory in `docs/proposals/phase-1/grammar-rfc.md` §2:
  `Break`/`Continue` restoration, compound assignment family, `Underscore`,
  and the internal `Error` token; numeric literal payloads are approved as
  amendment B-2 and the float policy as B-3. No bitwise, shift, or
  inclusive-range operators are authorized. Ratification record in the
  decision packet.

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
- **Phase 1 proposal (pending owner approval):** assignment as a
  syntactically non-associative lowest-tier expression; place-expression
  LHS validation at parse time, with grouping parentheses transparent
  (explicit amendment **B-1**); `let` remains the sole declaration. Whether
  assignment produces a value (proposed: none — a `()` result) is a
  type-system question deferred with the rest of the type rules, not fixed
  by the frontend contract — `docs/proposals/phase-1/grammar-rfc.md` §5;
  decision-packet row PD-003.
- **Status:** **RESOLVED (2026-10-12, Directive 006).** The owner approved
  the assignment syntax: expression form, lowest precedence,
  non-associative unparenthesized chains, place-expression targets with
  grouping parentheses transparent (B-1), and `let` as declaration syntax.
  Assignment parsing and type checking remain future work. Ratification
  record in the decision packet.

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
- **Phase 1 proposal (pending owner approval):** structured `Str` token
  with lexer-decoded escapes and recursively lexed embedded expressions;
  `\{`/`\}` escaping; `StringLit { parts }` AST; full diagnostic set —
  `docs/proposals/phase-1/lexer-contract.md` §5 and
  `parser-ast-contract.md` §4; decision-packet row PD-004. Nesting and
  stack-safety limits require the   explicit amendment **B-4**.
- **Status:** **RESOLVED (2026-10-12, Directive 006).** The owner approved
  the structured string design: structured `Str` tokens, decoded literal
  segments, embedded expression token sequences, absolute UTF-8 spans,
  escaped literal braces, and deterministic lexical diagnostics; nesting
  and stack safety follow the clarified B-4 limits
  (`MAX_INTERPOLATION_DEPTH = 64`) and the span invariant in
  `lexer-contract.md` §5.5. Ratification record in the decision packet.

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
- **Phase 1 proposal (pending owner approval):** complete Pratt
  binding-power table; `..` between assignment and `||`;
  non-associative assignment/range/comparison with dedicated diagnostics;
  postfix `as`; concrete precedence examples —
  `docs/proposals/phase-1/grammar-rfc.md` §4;   decision-packet row PD-005.
- **Status:** **RESOLVED (2026-10-12, Directive 006).** The owner approved
  the precedence and associativity table in
  `docs/proposals/phase-1/grammar-rfc.md` §4 as a binding contract for the
  future parser. Ratification record in the decision packet.

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
- **Phase 1 proposal (pending owner approval):** `import module_path ["as"
  IDENT];` with dotted paths; `::` reserved for expression/type paths;
  `pub` placement fixed; unsupported forms (groups, wildcard, `::` in
  imports, re-exports) rejected with diagnostics; resolution semantics
  deferred to Phase 2 — `docs/proposals/phase-1/grammar-rfc.md` §6;
  decision-packet row PD-006.
- **Status:** **RESOLVED (2026-10-12, Directive 006).** The owner approved
  the syntactic rules: `import module_path ["as" IDENT];` with dotted
  paths, `::` expression/type paths, fixed `pub` placement. Name
  resolution and visibility enforcement remain outside Phase 1.
  Ratification record in the decision packet.

## PD-007 — Source offset and column conventions

- **Question:** Does the specification agree with the Phase 0 choice of
  UTF-8 byte offsets (`u32`), 1-based lines, and 1-based columns counted in
  Unicode scalar values? Should display-width alignment (wide characters,
  tabs) matter for rendered diagnostics?
- **Original reference:** the original fixes only the `Span { file, lo: u32,
  hi: u32 }` shape (§5) and is silent on units, bases, and invalid-position
  behavior.
- **Impact:** Every span, every rendered diagnostic, tooling interfaces.
- **Status:** **RESOLVED (2026-10-12, Directive 003).** The project owner
  approved ADR-0002 as written — UTF-8 byte offsets, half-open `[lo, hi)`
  spans, `u32` offsets, 1-based lines and 1-based Unicode scalar-value
  columns, explicit errors for invalid positions, and LSP UTF-16 conversion
  deferred to the protocol boundary. ADR-0002 is now `Accepted`; the public
  API is unchanged. The known terminal display-width limitations and
  map-scoped `FileId` behavior remain documented; display-width handling
  stays tracked under PD-010.

## PD-008 — License selection (mandatory Phase 0 decision)

- **Question:** Which license(s) will XLANG use?
- **Impact:** Without a license, default copyright applies (all rights
  reserved) and the project is not open for contributions under a known
  grant. Phase 0 could not be fully approved while this was unresolved.
- **Status:** **RESOLVED (2026-10-12, Directive 003).** The project owner
  approved dual licensing under `MIT OR Apache-2.0` — the option
  technically recommended in Directive 002 for Rust-ecosystem
  compatibility, with an explicit patent grant available through the
  Apache alternative. Implemented with the canonical license texts in
  `LICENSE-MIT` and `LICENSE-APACHE`, the SPDX expression
  `MIT OR Apache-2.0` declared in `[workspace.package]` and inherited by
  each crate manifest, and updated License sections in `README.md` and
  `CONTRIBUTING.md`. No CLA or additional legal requirements were
  introduced; copyright attribution uses the repository owner's verified
  identity (no invented legal identity).

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
  `xl-diag` rendering. Related: PD-007 (resolved — scalar-value columns are
  the accepted convention; display-width handling remains open here).
- **Status:** PARTIALLY DEFINED — the current plain-text Phase 0 format is
  a repository choice, not a specification claim.
