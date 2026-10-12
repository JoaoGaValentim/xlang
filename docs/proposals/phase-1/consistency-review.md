# XLANG Phase 1 — Consistency Review (Directive 005)

**STATUS: PROPOSED — OWNER REVIEW REQUIRED**

- **Directive:** Engineering Directive 005 — Phase 1 Contract Correction,
  Grammar Consistency Hardening, and Final Design Review (2026-10-12).
- **Scope:** audited findings, cross-document consistency checks, and
  manual derivations for the Phase 1 proposal set. Documentation only; no
  code, manifests, or the archived specification were touched.
- **Base revision:** PR #2 head `72d0392` (Directive 004 proposals);
  corrections applied on `phase/1-syntax-contract`.

## §A Audit findings

Each finding records: identifier, affected contract, reproducible example,
the original inconsistency, the correction applied, the owner decision it
requires, and the final proposed state.

### F-1 — Block tail expressions misclassified by the directive order

- **Affected:** `parser-ast-contract.md` §3(2), `grammar-rfc.md` §3.2/§8.4,
  decision packet G-2, conformance plan §1.3.
- **Example:** `fn area(s: Shape) -> f64 { match s { … } }` and
  `fn parse(…) -> Result<i64, str> { …; if n < 0 { … } else { … } }`.
- **Inconsistency:** the Directive 004 draft checked “block-like” before
  “tail position”, so the `match`/`if` that supply `area`'s and `parse`'s
  values would have been reclassified as statements, silently emptying
  `Block.tail`.
- **Correction:** exact decision order — `;` → statement; else `}` → tail;
  else block-like → semicolon-free statement; else diagnostic. Pinned by
  conformance cases T1–T7.
- **Owner decision:** none beyond G-2 approval (the corrected order is the
  only one deriving the §3 example).
- **Final state:** corrected order documented in all four documents;
  T1–T7 classify every case unambiguously.

### F-2 — Parenthesized assignment targets contradicted the AST

- **Affected:** `grammar-rfc.md` §5(3), `parser-ast-contract.md` §3(5),
  conformance N13, decision packet (new B-1).
- **Example:** `(x) = 1;` versus `(x + y) = 1;`.
- **Inconsistency:** the draft rejected `(x) = 1`, but the AST drops
  grouping parentheses (no group node), so a validator inspecting
  Path/Field/Index cannot distinguish `x` from `(x)` — the rejection was
  unenforceable without retaining hidden grouping information.
- **Correction:** grouping is transparent; `(x)`, `((x))`, `(o.f)`,
  `(v[i])` are valid targets; `(x + y)`, `f()`, `42` remain invalid.
  N13 withdrawn; positive cases P41 added.
- **Owner decision:** **B-1 (explicit).**
- **Final state:** consistent across grammar, parser contract, corpus,
  packet.

### F-3 — `io.println` represented inconsistently (path vs member access)

- **Affected:** `conformance-plan.md` P1 and P7.
- **Example:** `io.println("total = {total}");`.
- **Inconsistency:** P1/P7 described a compound `Path [io, println]`, but
  the grammar reserves `::` for path segments and `.` for member access.
- **Correction:** AST is
  `Call(Field(Path [io], Ident println), [Str{…}])` — never
  `Path [io, println]`. No name-resolution semantics are implied (Phase 2
  decides what `io` refers to).
- **Owner decision:** none (derived from the preserved `.`/`::` split).
- **Final state:** P1/P7 corrected.

### F-4 — Function-type EBNF had a misplaced optional group

- **Affected:** `grammar-rfc.md` §3.4.
- **Example:** `fn(i64) -> i64`, `fn()`.
- **Inconsistency:** the draft production
  `"fn" "(" [ type { "," type } [ "," ] ")" ] [ "->" type ]` nested the
  closing `)` inside the optional group, so `fn()` was underivable.
- **Correction:** `"fn" "(" [ type_list ] ")" [ "->" type ]` with
  `type_list = type { "," type } [ "," ]`; grouping vs tuples separated
  (`grouped_type` / `tuple_type`). Derivations recorded in §D-7.
- **Owner decision:** none (pure EBNF correction).
- **Final state:** every listed type form derives; AST covers each.

### F-5 — Integer literals bounded to `i64` at lex time (B-2)

- **Affected:** `lexer-contract.md` §4.1, `grammar-rfc.md` §2.3,
  `parser-ast-contract.md` §2.2, corpus, packet.
- **Example:** `-9223372036854775808` (i64::MIN).
- **Inconsistency:** the draft rejected any magnitude above `i64::MAX`;
  i64::MIN's magnitude is `2^63` and exceeds it before unary minus
  applies — a valid, representable value was unlexable. The draft also
  substituted a silent `0` placeholder for overflow.
- **Correction:** token carries magnitude (`u64`) + radix; only
  `magnitude ≤ u64::MAX` is checked lexically; signed/target checks move to
  the type-system phase; overflow yields an explicitly erroneous `Error`
  token, never a substituted value. Departs from the §5 `Int(i64)` sketch —
  justified and labeled PROPOSED.
- **Owner decision:** **B-2 (explicit).**
- **Final state:** representation, AST payload (`IntLit`), corpus P42/N33,
  printer rule aligned.

### F-6 — Float overflow broke the printer/round-trip contract (B-3)

- **Affected:** `lexer-contract.md` §4.2, contract §5 printer items, corpus.
- **Example:** `1e9999` → `f64::INFINITY` under the draft.
- **Inconsistency:** accepting infinity means the canonical printer must
  reproduce a value for which no XLANG literal syntax exists (`inf`), so
  “parse(print(x)) ≡ x” cannot hold for accepted literals.
- **Correction:** finite-only `Float` tokens; representability diagnostic +
  `Error` token on overflow; underflow-to-zero accepted (round-trips);
  printer emits shortest-round-trip decimal that the lexer accepts.
- **Owner decision:** **B-3 (explicit).**
- **Final state:** corpus P43/N32; printer rule documented.

### F-7 — Recursion/stack-safety claim was not implementable as stated;
span invariants were flat

- **Affected:** `lexer-contract.md` §5.2(11)/§5.4/§9,
  `parser-ast-contract.md` §4.
- **Example:** thousands of nested `(`; nested interpolation; embedded
  tokens inside a `Str` token.
- **Inconsistency:** “bounded by source length” does not bound native stack
  use; and “token spans cover the input minus trivia” is impossible for a
  structured `Str` token whose embedded tokens lie inside the outer span.
- **Correction:** constant `MAX_INTERPOLATION_DEPTH = 64` (lexer) and
  `MAX_PARSE_DEPTH = 64` (parser) with recoverable diagnostics; separate
  invariants for top-level tokens, embedded tokens, segments, trivia, and
  `Eof`.
- **Owner decision:** **B-4 (explicit).**
- **Final state:** limits, diagnostics, fuzz obligations, and invariant
  taxonomy documented.

### F-8 — Semantics (unit-typing) leaked into the G-1 proposal

- **Affected:** `grammar-rfc.md` §8.3, decision packet G-1.
- **Example:** “`while`/`for`/`loop` … all of unit type”.
- **Inconsistency:** type behavior is Phase 2+ scope; asserting unit types
  in the frontend contract pre-empts the type system and violates the
  Phase 1/no-semantics boundary.
- **Correction:** G-1 now asserts **syntax only** (these forms may appear
  in expression positions); typing/divergence/value semantics explicitly
  deferred (category C).
- **Owner decision:** none beyond the amended G-1 text.
- **Final state:** semantics-boundary statement in packet G-1, grammar §8.3,
  and this review §B-2.

### F-9 — Corpus labeled complete AST shapes as ESTABLISHED

- **Affected:** `conformance-plan.md` §1.
- **Example:** P5 “ESTABLISHED” covering `Arm`/`Pat` shapes that have no §5
  sketch.
- **Inconsistency:** source-level acceptance and AST-shape approval were
  conflated.
- **Correction:** two independent status axes (Source: ESTABLISHED/PENDING;
  AST: SKETCH/PROPOSED) applied to every row.
- **Owner decision:** none (classification hygiene).
- **Final state:** every row carries both statuses.

### F-10 — Coverage claims were broader than the enumerated cases

- **Affected:** `conformance-plan.md` §4.
- **Example:** “remaining compounds exercised by extension cases in
  implementation”; `&&`/`||`, `%`, `<=`, `>`, `>=`, `!=` had no dedicated
  case.
- **Inconsistency:** claimed coverage without enumerated cases.
- **Correction:** explicit operator→case table; new cases P38–P45; binary
  `/` and others not covered are listed as future obligations F1–F6.
- **Owner decision:** none (already implied by PD-002/PD-005 approvals).
- **Final state:** coverage is exactly what the table enumerates.

## §B Consistency checks

### B-1 Grammar ↔ AST mapping

Every top-level production has a representation; syntax-only items are
listed (per Directive 005 §11A):

- **Represented:** items (Fn/Struct/Enum/Impl/Trait/Import), stmts
  (Let/Expr/Item), all expression variants, all type variants, all pattern
  variants, generics, visibility.
- **Syntax-only (no node, documented):** grouping parens (expression and
  type — §2.3(6) of the parser contract; the basis of B-1), trailing
  commas, operator tokens, delimiter tokens, comments/whitespace (trivia),
  absence of `pub` (becomes `Visibility::Private`).
- No production lacks a representation or a listed reason.

### B-2 Semantics boundary

Phase 1 fixes syntax only. Explicitly deferred: control-flow typing
(F-8), `()` inference, divergence, exhaustiveness, desugaring (Phase 2),
name resolution (Phase 2), literal range/target checking (B-2), float
semantics beyond value preservation (B-3).

### B-3 Lexer ↔ parser operator tokens

Every Pratt-tier operator has a token in `grammar-rfc.md` §2.4:
`=`→`Eq`, `+= -= *= /= %=`→`PlusEq/MinusEq/StarEq/SlashEq/PercentEq`,
`..`→`DotDot`, `||`→`OrOr`, `&&`→`AndAnd`,
`== != < <= > >=`→`EqEq/NotEq/Lt/Le/Gt/Ge`, `+ -`→`Plus/Minus`,
`* / %`→`Star/Slash/Percent`, unary `- !`→`Minus/Bang`,
postfix `(`→`LParen`, `.`→`Dot`, `[`→`LBracket`, `?`→`Question`,
`as`→`As`. No tier operator is missing a token; no token exists without a
use (except `Error`, documented recovery-only).

### B-4 Error recovery

No infinite loops (progress guarantee), no uncontrolled recursion (B-4
limits), no unchecked source positions (guarantee (e), lexer §9 span
invariants). Recovery requires no type information.

## §C Remaining inconsistencies

None known after the corrections in this document. Documents re-scanned:
grammar RFC, lexer contract, parser/AST contract, conformance plan,
decision packet. The corrections are documentation-level; no Rust code,
Cargo manifests, or the archived specification were modified.

## §D Manual derivations (Directive 005 §14)

Performed by grammar reasoning against `grammar-rfc.md` §3; no parser was
implemented. Legend: SPEC = specified rule; PROP = proposed rule pending
approval.

### D-1 — `fn f() -> i64 { if flag { 1 } else { 2 } }`

- **Tokens:** `fn f ( ) -> i64 { if flag { 1 } else { 2 } } Eof`.
- **Derivation:** item → `fn_decl` (`"fn" IDENT "(" [params] ")" "->"
  type block`), params empty, type `path_type(i64)`; `block` → one
  expression statement candidate: `if_expr` = `"if" expr block ["else"
  (if_expr|block)]`; condition `path_expr(flag)` stops at `{` (a block
  never continues an expression without an operator); then-block `{ 1 }`
  with tail `Int(1)`; else-branch is a `block` `{ 2 }`.
- **Precedence:** irrelevant (no infix).
- **Tail rule:** after the `if`, the next token is the enclosing `}` →
  rule 2 → `Block.tail = Some(If)` (F-1 order; T2).
- **AST:** `FnDecl{ sig{ ret: i64 }, body: Block{ stmts: [], tail:
  If{ cond: Path(flag), then: Block{tail 1}, else_: Some(Block{tail 2})
  } } }`.
- **Spans:** item `fn`…final `}`; `if` span `if`…else-block `}`; blocks
  include braces; `1`/`2` one byte each; no placeholders.
- **Status:** if-as-expression SPEC; recursion of `else if` PROP (P15);
  tail order = corrected G-2 (DERIVED + PROP).

### D-2 — `fn f() -> i64 { match value { 0 => 10, _ => 20, } }`

- **Tokens:** `fn f ( ) -> i64 { match value { 0 => 10 , _ => 20 , } }
  Eof` (`_` → `Underscore`, PROP token).
- **Derivation:** `match_expr` = `"match" expr "{" [arms] "}"`; scrutinee
  `path_expr(value)`; arms `pattern "=>" expr` separated by `,` with
  trailing `,`; patterns: `literal_pat(0)`, wildcard `_`.
- **Precedence:** none; `=>` is structural.
- **Tail rule:** next token after `}` is the enclosing `}` → rule 2 (T1).
- **AST:** `tail = Some(Match{ scrut: Path(value), arms:
  [Arm(Lit(0), Lit(10)), Arm(Wildcard, Lit(20))] })`.
- **Spans:** match `match`…arm-list `}`; each arm from pattern lo to expr
  hi (the separating/trailing commas are outside arm spans); wildcard span
  one byte.
- **Status:** match/literal/wildcard patterns SPEC; `Arm` node PROP;
  trailing comma DERIVED.

### D-3 — `fn f() { for i in 0..10 { process(i); } finish(); }`

- **Tokens:** `fn f ( ) { for i in 0 .. 10 { process ( i ) ; } finish
  ( ) ; } Eof` (`0..10` = `Int DotDot Int` by the munch rules).
- **Derivation:** block statements: expression starting with `for` →
  `for_expr` = `"for" pattern "in" expr block`; pattern `Binding(i)`;
  iterable `range` = `or_expr [".." or_expr]` → `Range(Int 0, Int 10)`;
  body block with `Stmt::Expr(Call(process,[i]))` + `;`. After the `for`:
  next token `finish` (not `;`, not `}`; expr is block-like) → rule 3
  semicolon-free statement (T4). `finish();` → rule 1. Final `}` closes
  the body; tail `None`.
- **Precedence:** `..` tier below `+`; here operands are primaries.
- **AST:** `Block{ stmts: [Expr(For{ … }), Expr(Call(finish,[]))], tail:
  None }`.
- **Spans:** `for` span `for`…body `}`; range span `0`…`10`; body braces
  included in body span.
- **Status:** `for`/keywords SPEC; `For`/`Range` nodes PROP (G-1/PD-005);
  classification DERIVED (G-2).

### D-4 — `fn f() { let x: Result<i64, str> = parse(input); }`

- **Tokens:** `fn f ( ) { let x : Result < i64 , str > = parse ( input
  ) ; } Eof`.
- **Derivation:** `let_stmt` = `"let" ["mut"] pattern [":" type] "=" expr
  ";"`; pattern `Binding(x)`; type `path_type(Result)` + `generic_args` =
  `< type , type >`; initializer `Call(Path(parse), [Path(input)])`
  (postfix call); `;` consumed (rule 1).
- **Precedence:** none beyond postfix.
- **AST:** `LetStmt{ mutable:false, pat: x, ty: Some(PathType{Result,
  [i64, str]}), init: Call(…) }`.
- **Spans:** let-statement `let`…`;`; `Result` type from identifier start
  through `>`; call from `parse` through `)`.
- **Status:** all forms SPEC/P derived from the example (`Result<i64,
  str>` is the example's type); let/`=` rule SPEC.

### D-5 — `fn f() { let x: Vec<Vec<i64>> = values; }`

- **Tokens:** `fn f ( ) { let x : Vec < Vec < i64 > > = values ; } Eof`.
  At `>>`: no shift token exists, so the lexer emits `Gt Gt`.
- **Derivation:** nested `generic_args`; the closer run `Gt Gt` consumes
  both; if the run exhausted a `Ge` token (e.g. `Vec<i64>= v`, P27), the
  parser would split it into `>` + `=` (G-3). Expressions unaffected.
- **AST:** `ty = PathType{Vec, [PathType{Vec, [i64]}]}`.
- **Spans:** outer type spans `Vec`…final `>`.
- **Status:** PROP (G-3); required by the SPEC type system's nesting needs.

### D-6 — `let x = "value = {compute(1 + 2)}";`

- **Tokens:** `let x = STR ; Eof` where `STR` covers the full literal
  including quotes; its payload: `Text("value = ")` + one interpolation
  segment whose embedded token sequence is `compute ( 1 + 2 )` with
  **absolute** spans.
- **Derivation:** `let_stmt`; initializer `Lit(Str(StringLit))`; the
  parser runs its expression entry point over each embedded token
  sequence → `Call(Path(compute), [Binary(Add, Int(1), Int(2))])` (`+`
  is tier 6; call args are full expressions).
- **Precedence:** `+` inside the argument; postfix call outside.
- **AST:** `Lit(Str{ parts: [Text("value = "), Expr(Call(…))] })`.
- **Spans:** `Str` span includes quotes; `Text` segment covers the exact
  interior bytes; embedded tokens keep in-file offsets; the `Expr` span
  runs `compute`…`)`.
- **Status:** interpolation `{expr}` SPEC; structured segments, escapes,
  spans PROP (PD-004); lexical nesting/limits see B-4.

### D-7 — Type-form derivations (Directive 005 §7)

From `type`, `grouped_type`, `tuple_type`, `type_list`, `path_type`,
`generic_args`:

| Source | Derivation | AST |
| --- | --- | --- |
| `()` | `tuple_type` empty | `Tuple([])` |
| `(i64)` | `grouped_type` | inner `Path(i64)` (no wrapper) |
| `(i64,)` | `tuple_type` one + comma | `Tuple([i64])` |
| `(i64, bool)` | `tuple_type` two | `Tuple([i64, bool])` |
| `[i64]` | slice | `Slice(i64)` |
| `[i64; 4]` | array | `Array(i64, Int 4)` |
| `Result<i64, str>` | path + generic args | `PathType` |
| `Vec<Vec<i64>>` | nested generic args, `Gt Gt` | `PathType` nested |
| `fn()` | empty `type_list`, no `->` | `Fn{params:[], ret:None}` |
| `fn(i64)` | one param, no `->` | `Fn{params:[i64], ret:None}` |
| `fn(i64) -> i64` | one param + ret | `Fn{…, ret:Some(i64)}` |
| `fn(i64, bool) -> Result<i64, str>` | two params + ret | `Fn{…, ret:Some(PathType)}` |

All derivations match `TypeKind` in `parser-ast-contract.md` §2.2.
