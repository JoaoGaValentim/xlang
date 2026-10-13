# XLANG Phase 1 — Decision Packet

**STATUS: ACCEPTED — OWNER DIRECTIVE 006**

- **Directive:** Engineering Directive 004 (created), corrected and
  restructured under Directive 005 — Syntax Contract Finalization
  (2026-10-12).
- **Purpose:** the actionable approval list for the Phase 1 syntax
  contracts, organized in three categories per Directive 005 §16. Each row
  carries one technically justified recommendation, precise enough to
  become an implementable contract after approval.
- **Related:** `grammar-rfc.md`, `lexer-contract.md`,
  `parser-ast-contract.md`, `conformance-plan.md`,
  `consistency-review.md` (this directory).

## Ratification record (Directive 006, 2026-10-12)

The project owner ratified this packet via Engineering Directive 006:

- **Category A — APPROVED as proposed:** PD-002, PD-003, PD-004, PD-005,
  PD-006, G-1 (syntax only), G-2 (corrected order), G-3, G-4.
- **Category B — APPROVED:** B-1 as corrected (parentheses transparent);
  B-2 as recommended (magnitude + radix; no signed restrictions in the
  lexer; no valid-looking zero substitution); B-3 as recommended
  (finite-only floats; overflow diagnosed; underflow accepted; parseable
  round-trip printing); B-4 approved **with clarifications** — initial
  limits `MAX_INTERPOLATION_DEPTH = 64` and future `MAX_PARSE_DEPTH = 64`;
  the future parser guard must cover *all* recursive syntax paths (types,
  patterns, blocks, nested items, generics) and recursive AST printing must
  also be stack-safe; nested comments use an iterative depth counter;
  limits must produce controlled diagnostics, never stack overflow; bounded
  recovery must guarantee forward progress and termination; adversarial
  inputs must be tested.
- **Category C — unchanged:** deferred; G-5 (struct literals) remains
  UNRESOLVED.
- **Additional clarification recorded:** the structured-interpolation span
  invariant (`lexer-contract.md` §5.7) — embedded tokens exclude the
  interpolation's delimiter braces, delimiter spans are accounted
  separately, and EOF handling is explicit per independently tokenized
  stream.

Approval authorizes the *contracts* and the `xl-lexer` implementation stage
only (Directive 006); the parser, AST, and CLI remain unauthorized.

---

## CATEGORY A — Proposed rules ready for approval

Consistent grammar, AST, lexer, diagnostics, and conformance examples;
no known conflict remains.

| Decision | Recommended rule | Alternatives considered | Rationale | Compatibility risk | Owner approval |
| --- | --- | --- | --- | --- | --- |
| **PD-002 — Token inventory** | Adopt `grammar-rfc.md` §2: the §5 sketch plus (a) restore `Break`/`Continue` (in the §3 keyword list, absent from the sketch); (b) add `+=` (required by the §3 example) and the family `-= *= /= %=`; (c) add `Underscore`; (d) add a recovery-only `Error` token. No bitwise/shift/`..=`/attribute tokens. Numeric literal *payloads* are covered separately by B-2. | (i) Only `+=`, not the family; (ii) add bitwise/shift; (iii) add `..=`; (iv) keep the sketch literally. | (a) restores SPECIFIED content, no new semantics; (b) one evidenced operator plus completing its family avoids a reserved-word graveyard; (c) `_` cannot be spelled otherwise; (d) internal recovery token. (ii)–(iii) have no evidence; (iv) contradicts §3. | Low: purely additive tokens. | **Required** |
| **PD-003 — Assignment semantics (core)** | `grammar-rfc.md` §5: assignment (`=` + compounds) is the lowest-precision expression tier, **syntactically non-associative** (chain = diagnostic, parenthesize to nest); LHS restricted at parse time to Path/Field/Index; `let` remains the sole declaration. The parenthesized-target question is B-1. | (i) Statement-only assignment; (ii) right-associative chains (Rust-style parse, type-rejected). | (i) contradicts the SPECIFIED Pratt chain ("atribuição" is an expression level); (ii) defers a classic trap to the type checker; explicit parentheses are clearer and need no semantics. | Low: original shows no chains; parenthesized nesting remains expressible. | **Required** |
| **PD-004 — Interpolated strings (core)** | `lexer-contract.md` §5: one structured `Str` token; lexer-decoded escapes; recursively lexed embedded expressions with absolute spans; `{expr}`; `\{`/`\}` escapes (lone `}` literal); single-line strings; empty/unterminated forms diagnosed. AST `StringLit{parts}`; Phase 2 desugars to concatenation. Resource limits are B-4. | (i) raw `Str(Symbol)` + parser re-lex; (ii) flat `StrStart/…/StrEnd` mode tokens; (iii) `{{`/`}}` escaping. | (i) duplicates logic across crates and blurs error ownership; (ii) complicates parser state, recovery, snapshots; (iii) two escape systems is more to learn. The recommended design keeps parser simple, spans exact. | Low: additive surface beyond the `{expr}` example; escape style is user-visible and revisitable before implementation. | **Required** |
| **PD-005 — Precedence & associativity** | `grammar-rfc.md` §4: original chain preserved; `..` between assignment and `\|\|`; assignment/range/comparison non-associative (diagnostics, not type errors); `\|\|`, `&&`, `+ -`, `* / %` left; unary `- !` above multiplicative; postfix `() . [] ? as` one left-to-right chain; concrete `lbp/rbp` table. | (i) different `..` slot; (ii) left-assoc comparisons; (iii) Rust-style `as` below unary. | (i) `0..n + 1` must mean `0..(n+1)`; chosen slot preserves every SPECIFIED relation; (ii) chained comparison is a bug pattern — fail early; (iii) the §3 postfix production literally contains `"as" type`. | Low–Medium: associativity and the `..` slot become visible contracts; alternatives expressible via parentheses. | **Required** |
| **PD-006 — Imports, paths, visibility** | `grammar-rfc.md` §6: `import module_path ["as" IDENT];` (dots only; no groups/wildcard/`::`/re-exports); `::` for expression/type paths; `pub` placement fixed (fn/struct/enum/trait/fields/methods); default-private is semantic (Phase 2); items may appear in blocks (SPECIFIED EBNF). | (i) no aliases; (ii) brace groups; (iii) `::` imports. | (i) aliases are needed on first collision and cost one optional clause; (ii)–(iii) unevidenced; the example fixes the dotted form. | Low: additive alias; unsupported forms get clear diagnostics. | **Required** |
| **G-1 — Control flow in expression positions (syntax only)** | `grammar-rfc.md` §8.3: `while`/`for`/`loop`/`return`/`break`/`continue` parse as expressions (primary level); `return`'s operand extends maximally; `break`/`continue` take no operand. **Type behavior (unit-ness, divergence, loop values) is deliberately not fixed by this frontend contract** and is deferred (category C / consistency review §B-2). | (i) statements only; (ii) fix unit types now. | (i) the §3 `stmt` EBNF does not contain them — they must be derivable as expressions for `fn main`; (ii) typing is out of Phase 1 scope and would pre-empt the type system. | Low: syntax-only; typing decisions stay open. | **Required** |
| **G-2 — Semicolon rule (corrected)** | `grammar-rfc.md` §3.2 with the corrected decision order: `;` → statement; else `}` → **tail expression**; else block-like → semicolon-free statement; else diagnostic. No empty statements. | (i) `;` everywhere (breaks the §3 `main` example); (ii) semicolon-optional everywhere (ASI-like ambiguity); (iii) Directive 004 order (block-like before tail) — rejected: it loses tail `match`/`if` values. | Tail-before-block-like is the only order that derives `fn area` and `fn parse` correctly; pinned by T1–T7. | Low: matches the example exactly; all seven classifications pinned by conformance cases. | **Required** |
| **G-3 — Generic closer disambiguation** | `parser-ast-contract.md` §3(3): in type-argument closing only, accept `Gt`; split a leftover `Ge` into `>`+`=`; expressions keep `>=` untouched. | (i) require whitespace; (ii) forbid nested generics; (iii) lexer `>>` token. | (i) user-hostile; (ii) contradicts needed types; (iii) creates the ambiguity shifts would otherwise have. | Low: parser-local, no token added. | **Required** |
| **G-4 — Tuple/array literal expressions** | `(a, b)` tuple, `()` unit, `(e)` grouping; `[a, b]` array; no repeat form. | (i) none in Phase 1; (ii) add `[x; N]`. | (i) leaves SPECIFIED types without value syntax and forces parser churn later; (ii) unevidenced. | Low: additive. | **Required** |

---

## CATEGORY B — Rules requiring explicit amendment approval

Each row amends or resolves something that the Directive 004 draft got
wrong or left contradictory. **These require an explicit owner decision
even if Category A is approved wholesale.**

| Amendment | Recommended rule | Alternatives considered | Rationale | Compatibility risk | Owner approval |
| --- | --- | --- | --- | --- | --- |
| **B-1 — Parenthesized assignment targets** | Grouping parentheses are transparent: `(x) = 1`, `((x)) = 1`, `(o.f) = 1`, `(v[i]) = 1` are valid; `(x + y) = 1`, `f() = 1`, `42 = 1` remain `invalid assignment target` (parse-time, no type checking). | (i) Keep the Directive 004 rejection of `(x) = 1` and retain grouping in the AST to enforce it; (ii) forbid all parenthesized targets. | The AST drops grouping by design (§5 sketch has no group node), so (i) would add a node or hidden metadata purely to reject a harmless form; (ii) rejects a form users will write. Transparency is the only option consistent with the AST. | Low: newly accepted syntax; all rejections keep precise diagnostics. | **Required** |
| **B-2 — Numeric literal range representation** | The `Int` token carries **magnitude (`u64`) + radix**; lexer checks only `magnitude ≤ u64::MAX`; signed/target-type checks move to the type-system phase. Departs from the §5 `Int(i64)` sketch (explicitly justified, PROPOSED). Out-of-range magnitudes produce `Error` tokens — never a substituted `Int(0)`. | (i) Keep `Int(i64)`, reject `-9223372036854775808`; (ii) keep `Int(i64)` with saturating placeholder; (iii) arbitrary-precision payload. | (i) rejects i64::MIN — a valid, representable value whose *magnitude* exceeds i64::MAX before the unary minus applies; (ii) silently corrupts values; (iii) adds dependency/complexity unjustified for u64-bounded targets. | Low: token payload changes only; every literal accepted today (except overflow) stays accepted; `0xFF`/`255` compare equal under the printer relation. | **Required** |
| **B-3 — Floating-point overflow policy** | A `Float` token is always a **finite** `f64`. Literals rounding to ±infinity (`1e9999`) are diagnosed (`floating-point literal is not representable`) and recovered as `Error` — never substituted. Underflow to zero (`1e-9999` → `0.0`) is accepted (round-trips). | (i) Accept infinity (Directive 004 draft); (ii) reject underflow too. | (i) `inf` has no XLANG literal syntax, breaking the printer/round-trip contract; (ii) underflow still round-trips to an equal value (`0.0` → `0.0`) — rejection adds restriction without preserving an invariant. | Low: only the unrepresentable boundary is rejected; asymmetry documented. | **Required** |
| **B-4 — Interpolation and parse nesting limits (resource safety)** | Constant limits with recoverable diagnostics: lexer `MAX_INTERPOLATION_DEPTH = 64` (`interpolation nesting limit (64) exceeded`, recover to balancing `}`/EOF, `Error` token); parser `MAX_PARSE_DEPTH = 64` (`expression nesting limit (64) exceeded`, skip to expression sync set). No stack overflow for any input; limits are implementation-safety policy, not language semantics, and are exact-boundary-fuzzed (conformance obligation O4). | (i) Unbounded recursion (stack overflow = panic — violates the never-panic contract); (ii) rely on “source length bounds depth” (rejected by Directive 005 §9: length does not bound stack use); (iii) different limit values or iterative-only rewrite. | 64 is far beyond any realistic source nesting, keeps the recursive-descent design implementing cleanly, and makes the stack bound a constant independent of input. Values are trivially raisable by owner decision. | Low: only adversarial inputs hit the limit; diagnostics are precise and deterministic. | **Required** |

---

## CATEGORY C — Future semantic or language features

Not needed to resolve the Phase 1 frontend; listed so the boundary is
explicit and nothing is introduced silently. No action requested now.

- **G-5 — struct-literal construction syntax:** absent from the original;
  remains UNRESOLVED and excluded from Phase 1. Consequence (kept):
  `if x { … }` is unambiguous, and `Point { x: 1.0 }` is not valid Phase 1
  syntax. Re-opening this requires a specification change and lookahead
  design — a deliberate future owner decision.
- **Control-flow type behavior** (`while`/`for`/`loop`/`return`/`break`
  typing, divergence, loop values, `()` inference): deferred to the
  type-system phase; Phase 1 records syntax only.
- **`break` with a value; loop values** (`loop { break 1 }`): unevidenced;
  syntax reserved out.
- **Open-ended ranges** (`a..`, `..b`) and inclusive `..=`: unevidenced.
- **Bitwise/shift operators, attributes (`#[…]`), turbofish (`f::<T>(…)`),
  `pub` re-exports, struct-like enum variants, match guards/or-patterns,
  negative literal patterns:** all unevidenced; explicitly not introduced.
- **Unicode normalization policy for identifiers:** UNRESOLVED; identifies
  compare by scalar sequence for now.
- **Stable diagnostic codes (`E0xxx`) and color/width policy:** PD-010;
  untouched here.

## Approval mechanics

- Category A rows: approve as proposed / approve with amendment / reject.
- Category B rows: **explicit per-amendment decision is required** (a
  wholesale Category A approval does not cover them).
- Category C: no action; listed for the record.
- On approval, the corresponding `docs/pending-decisions.md` entries move
  to `RESOLVED` with a pointer to the recorded decision, and the proposal
  documents change status from PROPOSED to ACCEPTED.
- Implementation of `xl-lexer`, `xl-ast`, `xl-parser`, and `xc parse`
  remains **not authorized** until a separate implementation directive.
