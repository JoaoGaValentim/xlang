# XLANG Phase 1 — Conformance Plan

**STATUS: PROPOSED — OWNER REVIEW REQUIRED**

- **Directive:** Engineering Directive 004 — Phase 1 Syntax Architecture and
  Specification Gate (2026-10-12)
- **Purpose:** the proposed positive/negative syntax corpus and test
  criteria for Phase 1 implementation (`xl-lexer`, `xl-ast`, `xl-parser`,
  `xc parse`).
- **No results are claimed here.** No lexer, parser, or test exists yet;
  every "expected" statement below is a specification expectation, not an
  observed outcome.

Status codes used per case:

- **ESTABLISHED** — acceptance derives directly from the original
  specification (the §3 example, §5 sketches); the rule is SPECIFIED.
- **PENDING** — acceptance depends on a PROPOSED rule in
  `grammar-rfc.md` / the decision packet and requires owner approval.
- **UNRESOLVED** — behavior not proposed; listed to document the boundary.

## 1. Positive corpus

### 1.1 The original example program (all ESTABLISHED)

The seven top-level constructs of the §3 example are the primary
conformance artifact. Each is reproduced (whitespace-normalized) with its
expected shape.

| ID | Source | Expected parse / AST shape | Status |
| --- | --- | --- | --- |
| P1 | `import std.io;` | `Item::Import { path: [std, io], alias: None }`; module access `io.println` later = `Path [io, println]` + `Call` | ESTABLISHED |
| P2 | `struct Point { x: f64, y: f64 }` | `Item::Struct`, 2 fields, types `f64` (path types) | ESTABLISHED |
| P3 | `impl Point { pub fn dist(self, o: Point) -> f64 { let dx = self.x - o.x; let dy = self.y - o.y; (dx * dx + dy * dy).sqrt() } }` | `Item::Impl { trait_: None, self_ty: Point }`; item `FnDecl { vis: Public, params: [Receiver, o: Point], ret: f64 }`; body statements: 2 × `LetStmt`; tail: `Call(Field(Block? no — parenthesized Binary expr, sqrt))` — precisely `Call(Field(Paren? grouping drops to inner expr, sqrt), [])` over `Binary(Add, Binary(Add, dx*dx, dy*dy))` | ESTABLISHED |
| P4 | `enum Shape { Circle(f64), Rect(f64, f64) }` | `Item::Enum`, variants `[Circle([f64]), Rect([f64, f64])]` | ESTABLISHED |
| P5 | `fn area(s: Shape) -> f64 { match s { Shape::Circle(r) => 3.14159 * r * r, Shape::Rect(w, h) => w * h, } }` | `FnDecl`; tail `ExprKind::Match` with 2 `Arm`s; patterns `PatKind::Path { path [Shape, Circle], args [Binding r] }`; bodies binary `*` trees; trailing comma accepted | ESTABLISHED |
| P6 | `fn parse(s: str) -> Result<i64, str> { let n = s.to_int()?; if n < 0 { Err("negativo") } else { Ok(n) } }` | `FnDecl`; `LetStmt { pat: n, init: Try(Call(Field(s, to_int))) }`; tail `ExprKind::If`; condition `Binary(Lt, n, 0)`; branches `Call(Path [Err])` / `Call(Path [Ok])` | ESTABLISHED |
| P7 | `fn main() { let mut total = 0.0; for i in 0..10 { total += i as f64; } io.println("total = {total}"); }` | `LetStmt { mutable: true, init: Float(0.0) }`; `ExprKind::For { pat: Binding i, iter: Range(0, 10), body }` with body `Stmt::Expr(Assign(AddAssign, total, Cast(i, f64)))`; then `Stmt::Expr(Call(Path [io, println], [Str{ Text "total = ", Expr(Path total) }]))` | ESTABLISHED (semicolon-optional-for-block-like and interpolation structure are DERIVED/PROPOSED — see the row level notes) |

Notes: P3's `(dx * dx + dy * dy).sqrt()` is `Field` applied to a
parenthesized expression (grouping), then `Call`; P6's `?` is postfix `Try`;
P7's `for` statement has no `;` (DERIVED semicolon rule, packet G-2).

### 1.2 Surface-feature corpus (fragments exercising every rule)

| ID | Source | Expected parse / AST shape | Status |
| --- | --- | --- | --- |
| P8 | `let mut total = 0.0;` | `LetStmt { mutable: true }` | ESTABLISHED |
| P9 | `let n = s.to_int()?;` | `Try(Call(Field(Path s, to_int)))` | ESTABLISHED |
| P10 | `0..10` | `Range(Int 0, Int 10)` | ESTABLISHED (range tier position is PENDING, packet PD-005) |
| P11 | `total += i as f64` | `Assign(AddAssign, Path total, Cast(Path i, f64))` | ESTABLISHED (`+=`); compound-set extensions PENDING (PD-002) |
| P12 | `"total = {total}"` | `Lit(Str { parts: [Text "total = ", Expr(Path total)] })` | ESTABLISHED form; segment AST PENDING (PD-004) |
| P13 | `// line` and `/* a /* b */ c */` | no tokens (trivia) | ESTABLISHED (nested comments) |
| P14 | `1_000 0xFF 0b1010 3.14159 1e5 'x' '\n' '\u{1F980}' "a\nb"` | `Int(1000) Int(255) Int(10) Float(…) Float(…) Char('x') Char('\n') Char('🦀') Str{Text "a\nb"}` | forms PENDING where marked in lexer-contract §4–5 |
| P15 | `if c { a } else if d { b } else { c2 }` | `If(cond, block, Some(If(…)))` | PENDING (else-if recursion, PD-005/G-1 context) |
| P16 | `match x { 1 => "a", _ => "b", }` | `Match`, arms literals + wildcard; trailing comma | ESTABLISHED patterns; `_` token PENDING (PD-002) |
| P17 | `while n < 10 { n += 1; }` / `loop { break; }` | `While(Binary(Lt…), block)` / `Loop` containing `Continue/Break` statements (no `;` needed for block-like; `break;` inside block has `;` as expr-stmt… `break` is not block-like → needs `;` unless tail) | PENDING (G-1, G-2) |
| P18 | `fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }` | fn-type in params; `Call(Path f, [Path x])` | ESTABLISHED type form; params/commas PENDING (grammar-rfc §3.1) |
| P19 | `struct S { pub x: i64, y: [i64; 4] }` | field vis + array type with length expression | Array type ESTABLISHED; `pub` on field PENDING (PD-006) |
| P20 | `enum E { A, B(i64) }` | unit variant + tuple variant | Unit variant PENDING (struct-like variants excluded, grammar-rfc §8.10) |
| P21 | `impl Shape { fn area(self) -> f64 { match self { Shape::Circle(r) => r } } }` | second inherent impl; `self` receiver + `self` path in match scrutinee | ESTABLISHED |
| P22 | `let (a, b) = pair;` | `LetStmt` with `PatKind::Tuple` | ESTABLISHED (pattern grammar) |
| P23 | `let xs = [1, 2, 3];` | `Lit`-free `ExprKind::Array` | PENDING (G-4) |
| P24 | `let p = (1, 2.5);` | `ExprKind::Tuple` | PENDING (G-4) |
| P25 | `let r: Result<i64, str> = parse(s);` | type `PathType { Result, generics [i64, str] }` | ESTABLISHED |
| P26 | `fn f() -> Vec<Vec<i64>> { v }` | nested generic closers `>>` | PENDING (G-3 closer rule) |
| P27 | `let x: Vec<i64>= v;` | `Ge` split into `>` + `=` in type-close context | PENDING (G-3) |
| P28 | `a == b != c` (no) — see negative N9 | — | — |
| P29 | `-x + !y` | `Binary(Add, Unary(Neg, Path x), Unary(Not, Path y))` | ESTABLISHED (`- !` unary) |
| P30 | `a - b - c` | `Binary(Sub, Binary(Sub, a, b), c)` (left) | ESTABLISHED (left assoc additive) |
| P31 | `a < b` / `a <= b` / `a >= b` | comparisons | ESTABLISHED; non-assoc is PENDING (PD-005) |
| P32 | `x.y.z(1, 2)?` | `Try(Call(Field(Field(x, y), z), [1, 2]))` | ESTABLISHED (postfix chain) |
| P33 | `import std.io; import my.pkg as p;` | imports with/without alias | first ESTABLISHED; alias PENDING (PD-006) |
| P34 | `trait Show { fn show(self) -> str; }` | signature-only trait item | PENDING (grammar-rfc §3.1) |
| P35 | `impl Show for Point { pub fn show(self) -> str { s } }` | trait impl form | PENDING (design decision, grammar-rfc §8.8) |
| P36 | `fn g<T: Ord>(a: T) -> T { a }` | generics with bound (SPECIFIED Phase 5 form) | PENDING (surface accepted in Phase 1, grammar-rfc §3.1) |
| P37 | `return n < 0` (inside fn) | `Return(Some(Binary(Lt,…)))` — operand extends maximally | PENDING (G-1) |

## 2. Negative corpus

Each case lists the expected **first** diagnostic (message class), the
recovery behavior, and the rule that classifies it.

| ID | Source | Expected diagnostic | Recovery | Status |
| --- | --- | --- | --- | --- |
| N1 | `let x = ;` | `expected expression` at `;` | zero-width expr placeholder; `;` consumed; continue | PENDING (messages) |
| N2 | `let x` (no `=`) | `expected '='` | insert initializer placeholder; sync at `;` | ESTABLISHED (`=` required by EBNF) |
| N3 | `fn f( { }` | `expected parameter` / unmatched `(` | sync at `{`; parse body; report once | PENDING (messages) |
| N4 | `struct { }` | `expected identifier` | skip to `}`; build empty struct with placeholder name | PENDING |
| N5 | `"abc` | lexer: `unterminated string literal` | `Error` token to line end | PENDING (lexer contract §8) |
| N6 | `/* abc` | lexer: `unterminated block comment` | resume at EOF | PENDING |
| N7 | `0x` | lexer: `expected digits after '0x'` | `Error` token | PENDING |
| N8 | `1_` | lexer: `'_' must appear between digits` | `Error` token | PENDING |
| N9 | `a < b < c` | `comparison operators cannot be chained; use parentheses` | left-nested recovery `((a<b)<c)` | PENDING (PD-005) |
| N10 | `a..b..c` | `range cannot be chained; use parentheses` | left-nested recovery | PENDING (PD-005) |
| N11 | `a = b = c` | `chained assignment; parenthesize the right-hand side` | right-nested recovery | PENDING (PD-003) |
| N12 | `1 = x;` | `invalid assignment target` | build assignment anyway | PENDING (PD-003) |
| N13 | `(x) = 1;` | `invalid assignment target` | build assignment anyway | PENDING (PD-003) |
| N14 | `import a::b;` | `expected '.' or ';'` after module segment | sync at `;` | PENDING (PD-006) |
| N15 | `import a.{b};` | `expected identifier` | sync at `;` | PENDING (PD-006) |
| N16 | `match s { }` | *parses* (empty arms list); exhaustiveness is Phase 2 | n/a | PENDING (grammar permits empty arms; flagged) |
| N17 | `match s { A => }` | `expected expression` | arm with placeholder body | PENDING |
| N18 | `"{}"` | `empty interpolation` at segment span | `Expr` placeholder in segment | PENDING (lexer/parser split) |
| N19 | `"\q"` | lexer: `unknown escape sequence '\q'` | literal `q` accepted | PENDING |
| N20 | `fn f() { return x y }` | `expected ';'` at `y` | sync; `return x` statement built | PENDING (G-2) |
| N21 | `x.0` | `expected field name` after `.` | field placeholder | PENDING |
| N22 | `1.` | *lexes* as `Int(1) .`; parser: `expected field name` | field placeholder | PENDING (munch rules) |
| N23 | `.5` | *lexes* as `. Int(5)`; parser: `expected expression` | placeholder | PENDING |
| N24 | `&x` | lexer: `unexpected character '&'` | `Error` token | ESTABLISHED-ish (no `&` token; PENDING message) |
| N25 | `x..=;`? — `..=` | lexes `..` `=`; expression error `expected expression` | placeholder | PENDING (no `..=` operator) |
| N26 | `let x: ;` | `expected type` | type placeholder | PENDING |
| N27 | `let x: Vec<T>> = v;`? — see P26 for valid form; `Vec<T>>` alone | `unexpected '>'` | skip excess closer | PENDING (G-3) |
| N28 | `fn f(,) {}` | `expected parameter` | skip comma | PENDING |
| N29 | `;;` | `expected expression` (no empty statements) | skip `;` | PENDING (DERIVED) |
| N30 | `struct S { x: }` | `expected type` | placeholder type; sync at `,`/`}` | PENDING |

No negative case may require type information to classify (Phase 1 rule).

## 3. Test criteria (implementation phase)

1. **Token snapshots (insta):** exact `(TokenKind, Span)` sequences for all
   §1 sources, including a full-source snapshot of the §3 example
   (P1–P7 concatenated as the original program).
2. **AST snapshots:** canonical printer output (parser contract §5) for all
   §1 entries; exact-string compare.
3. **Round-trip property:** `parse(print(ast)) == ast` for corpus ASTs and
   generated ASTs (spans excluded).
4. **Negative snapshots:** exact `xl-diag` rendering for §2 cases
   (message + span + recovery tree), so drift fails loudly.
5. **Fuzz (cargo-fuzz):** pipeline over arbitrary inputs: no panic, no
   hang, one trailing `Eof`, valid spans, progress guarantee
   (parser contract §4).
6. **Acceptance gate (SPECIFIED §6 Phase 1):** 100% of the §3 example
   constructs parse (P1–P7) and 1 hour of fuzzing without failures — to be
   executed in the implementation directive, not claimed here.

## 4. Coverage cross-check

- Every operator in `grammar-rfc.md` §2.4 is exercised: `+ - * / %`
  (P5/P7), `== != < <= > >=` (P6, P31), `&& || !` (P29 plus
  `if n < 0 && ok`-style derivations in guards — none in original; P29
  covers `!`), `= += -= *= /= %=` (P7/P11; remaining compounds exercised by
  extension cases in implementation), `..` (P7/P10), `. :: ? as`
  (P3/P5/P6/P11), braces/parens/brackets (throughout).
- Every §3 example line appears in §1.1 (P1–P7).
- Every proposed token addition has a positive case (P11 for `+=`,
  P17 for `break`/`continue`) or is a recovery-only token (`Error`, §2).
- Every negative class in §2 maps to a diagnostic catalog entry in
  `lexer-contract.md` §8 or `parser-ast-contract.md` §3–§4.
