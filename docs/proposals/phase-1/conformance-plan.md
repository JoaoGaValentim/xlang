# XLANG Phase 1 — Conformance Plan

**STATUS: PROPOSED — OWNER REVIEW REQUIRED**

- **Directive:** Engineering Directive 004 (created), corrected under
  Directive 005 — Syntax Contract Finalization (2026-10-12).
- **Purpose:** the proposed positive/negative syntax corpus and test
  criteria for Phase 1 implementation (`xl-lexer`, `xl-ast`, `xl-parser`,
  `xc parse`).
- **No results are claimed here.** No lexer, parser, or test exists yet;
  every "expected" statement below is a specification expectation, not an
  observed outcome.

Status codes per case (two independent axes, per Directive 005 §11F):

- **Source acceptance:** `ESTABLISHED` — the construct's acceptance derives
  directly from the original specification (§3 example / stated rules);
  `PENDING` — acceptance depends on a PROPOSED rule in
  `grammar-rfc.md` / the decision packet.
- **AST status:** `SKETCH` — the node shape is fixed by the §5
  `ExprKind`/`Span` sketches; `PROPOSED` — the shape depends on a PROPOSED
  representation (PD-00x / G-x) and requires owner approval; mixed shapes
  are noted inline.

No complete AST shape is labeled ESTABLISHED merely because the source
construct is specified — item-level nodes, statement nodes, and pattern
nodes have no §5 sketch and are PROPOSED throughout.

## 1. Positive corpus

### 1.1 The original example program

| ID | Source | Expected parse / AST shape | Source | AST |
| --- | --- | --- | --- | --- |
| P1 | `import std.io;` | `Item::Import { path: [std, io], alias: None }`. Module member `io.println` elsewhere is `Field(Path [io], Ident println)` — **never** `Path [io, println]` (Directive 005 §6) | ESTABLISHED | PROPOSED (PD-006 `ImportDecl` fields; `Field`/`Call`/`Path` from sketch) |
| P2 | `struct Point { x: f64, y: f64 }` | `Item::Struct`, 2 fields, `f64` path types | ESTABLISHED | PROPOSED (item nodes, grammar-rfc §3.1) |
| P3 | `impl Point { pub fn dist(self, o: Point) -> f64 { let dx = self.x - o.x; let dy = self.y - o.y; (dx * dx + dy * dy).sqrt() } }` | `ImplDecl { trait_: None, self_ty: Point }`; `FnDecl { vis: Public, params: [Receiver, o: Point], ret: f64 }`; tail `Call(Field(Binary(Add, Binary(Add, dx*dx, dy*dy)), sqrt), [])` — grouping parens are transparent | ESTABLISHED | PROPOSED (item/fn nodes); expression atoms from sketch |
| P4 | `enum Shape { Circle(f64), Rect(f64, f64) }` | `Item::Enum`, variants `[Circle([f64]), Rect([f64, f64])]` | ESTABLISHED | PROPOSED (grammar-rfc §3.1) |
| P5 | `fn area(s: Shape) -> f64 { match s { Shape::Circle(r) => 3.14159 * r * r, Shape::Rect(w, h) => w * h, } }` | tail `Match` (T1); 2 `Arm`s; patterns `PatKind::Path { path [Shape, Circle], args [Binding r] }`; trailing comma | ESTABLISHED | Sketch (`Match`, `Binary`); `Arm`/`Pat` PROPOSED (grammar-rfc §3.5) |
| P6 | `fn parse(s: str) -> Result<i64, str> { let n = s.to_int()?; if n < 0 { Err("negativo") } else { Ok(n) } }` | `LetStmt { pat: n, init: Try(Call(Field(Path s, to_int))) }`; tail `If` (T2) | ESTABLISHED | Sketch (`Try`, `If`); `LetStmt` PROPOSED |
| P7 | `fn main() { let mut total = 0.0; for i in 0..10 { total += i as f64; } io.println("total = {total}"); }` | `LetStmt { mutable: true, init: Float(0.0) }`; `Stmt::Expr(For { pat: i, iter: Range(0, 10), body })` (T4); body `Stmt::Expr(Assign(AddAssign, total, Cast(i, f64)))`; then `Stmt::Expr(Call(Field(Path io, println), [Str{ Text "total = ", Expr(Path total) }]))` | ESTABLISHED | `For`/`Range`/`Assign` PROPOSED (G-1/PD-005/PD-003); `Str` parts PROPOSED (PD-004) |

### 1.2 Surface-feature corpus

| ID | Source | Expected parse / AST shape | Source | AST |
| --- | --- | --- | --- | --- |
| P8 | `let mut total = 0.0;` | `LetStmt { mutable: true }` | ESTABLISHED | PROPOSED |
| P9 | `let n = s.to_int()?;` | `Try(Call(Field(Path s, to_int)))` | ESTABLISHED | SKETCH |
| P10 | `0..10` | `Range(Int 0, Int 10)` | ESTABLISHED (slot position PENDING, PD-005) | PROPOSED |
| P11 | `total += i as f64` | `Assign(AddAssign, Path total, Cast(Path i, f64))` | ESTABLISHED (`+=`) | PROPOSED (PD-003) |
| P12 | `"total = {total}"` | `Lit(Str { parts: [Text "total = ", Expr(Path total)] })` | ESTABLISHED form | PROPOSED (PD-004) |
| P13 | `// line` and `/* a /* b */ c */` | no tokens (trivia) | ESTABLISHED | n/a (trivia) |
| P14 | `1_000 0xFF 0b1010 3.14159 1e5 'x' '\n' '\u{1F980}' "a\nb"` | `Int(mag, radix)`×3, `Float`, `Char`, `Char`, `Char('🦀')`, `Str{Text}` | forms PENDING where noted in lexer-contract §4–5 | PROPOSED (`IntLit`, B-2) |
| P15 | `if c { a } else if d { b } else { c2 }` | `If(cond, block, Some(If(…)))` | PENDING (else-if recursion) | SKETCH (If) |
| P16 | `match x { 1 => "a", _ => "b", }` | `Match`, literal + wildcard arms; trailing comma | ESTABLISHED | SKETCH + Arm PROPOSED |
| P17 | `while n < 10 { n += 1; }` / `loop { break; }` | `While(…)`; `Loop` with `Break` (semicolon required after `break` inside a block — not block-like) | PENDING (G-1) | PROPOSED (G-1) |
| P18 | `fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }` | fn-type in params; `Call(Path f, [Path x])` | ESTABLISHED type form | PROPOSED (params); `TypeKind::Fn` PROPOSED |
| P19 | `struct S { pub x: i64, y: [i64; 4] }` | field vis + array type with length expression | array type ESTABLISHED; field `pub` PENDING (PD-006) | PROPOSED |
| P20 | `enum E { A, B(i64) }` | unit variant + tuple variant | unit variant PENDING | PROPOSED |
| P21 | `impl Shape { fn area(self) -> f64 { match self { Shape::Circle(r) => r } } }` | second inherent impl; `self` receiver and `self` path | ESTABLISHED | PROPOSED |
| P22 | `let (a, b) = pair;` | `LetStmt` with `PatKind::Tuple` | ESTABLISHED (pattern grammar) | PROPOSED |
| P23 | `let xs = [1, 2, 3];` | `ExprKind::Array` | PENDING (G-4) | PROPOSED |
| P24 | `let p = (1, 2.5);` | `ExprKind::Tuple` | PENDING (G-4) | PROPOSED |
| P25 | `let r: Result<i64, str> = parse(s);` | `PathType { Result, generics [i64, str] }` | ESTABLISHED | PROPOSED (PathType) |
| P26 | `fn f() -> Vec<Vec<i64>> { v }` | nested generic closers `>>` (two `Gt`) | PENDING (G-3) | PROPOSED |
| P27 | `let x: Vec<i64>= v;` | `Ge` split into `>` + `=` in type-close context | PENDING (G-3) | PROPOSED |
| P29 | `-x + !y` | `Binary(Add, Unary(Neg, x), Unary(Not, y))` | ESTABLISHED | SKETCH |
| P30 | `a - b - c` | `Binary(Sub, Binary(Sub, a, b), c)` (left) | ESTABLISHED | SKETCH |
| P31 | `a < b` / `a <= b` / `a > b` / `a >= b` / `a == b` / `a != b` | six independent comparison applications | ESTABLISHED | SKETCH |
| P32 | `x.y.z(1, 2)?` | `Try(Call(Field(Field(x, y), z), [1, 2]))` | ESTABLISHED | SKETCH |
| P33 | `import std.io; import my.pkg as p;` | imports with/without alias | first ESTABLISHED; alias PENDING (PD-006) | PROPOSED |
| P34 | `trait Show { fn show(self) -> str; }` | signature-only trait item | PENDING | PROPOSED |
| P35 | `impl Show for Point { pub fn show(self) -> str { s } }` | trait impl form | PENDING | PROPOSED |
| P36 | `fn g<T: Ord>(a: T) -> T { a }` | generics with bound | PENDING (surface) | PROPOSED |
| P37 | `return n < 0` | `Return(Some(Binary(Lt, …)))` — operand extends maximally | PENDING (G-1) | PROPOSED |
| P38 | `x -= 1; x *= 2; x /= 3; x %= 4;` | four `Assign(SubAssign/MulAssign/DivAssign/RemAssign, Path x, Int)` statements | PENDING (PD-002 family) | PROPOSED (PD-003) |
| P39 | `a && b \|\| !c` | `Binary(Or, Binary(And, a, b), Unary(Not, c))` (`&&` binds tighter) | ESTABLISHED | SKETCH |
| P40 | `w % 2` | `Binary(Rem, w, 2)` (the `%` operator, enumerated separately from P5/P7) | ESTABLISHED | SKETCH |
| P41 | `(x) = 1; ((x)) = 1; (o.f) = 1; (v[i]) = 1;` | four `Assign` statements; grouping transparent (B-1) | PENDING (B-1 amendment) | PROPOSED |
| P42 | `let a = -9223372036854775808; let b: u64 = 18446744073709551615; let c = 0xFFFF;` | `Int(mag 2^63)` under `Unary(Neg)`; `Int(mag 2^64−1)`; `Int(65535, Hex)` — all lexically accepted under B-2 | PENDING (B-2) | PROPOSED (`IntLit`) |
| P43 | `let d = 1e300; let e = 1e-9999;` | finite `Float(1e300)`; underflow accepted as `Float(0.0)` (B-3) | PENDING (B-3) | PROPOSED |
| P44 | `"literal \{brace\}"; "a { "b {c} d" } e"; "café \u{1F980}"` | re-parseable escaped text; nested interpolation through embedded string; unicode escapes | PENDING (PD-004) | PROPOSED |
| P45 | `let café = 1; let δ = café + 1;` | Unicode (XID) identifiers; `Binary(Add, Path café, 1)` | ESTABLISHED (XID note §3) | PROPOSED |

### 1.3 Tail/statement classification cases (Directive 005 §4)

All rule references are to `parser-ast-contract.md` §3(2) and
`grammar-rfc.md` §3.2. Nothing here may be claimed consistent until these
classifications are unambiguous — this table is that classification.

| ID | Source | Classification | Rule | Source | AST |
| --- | --- | --- | --- | --- | --- |
| T1 | `fn area(s: Shape) -> f64 { match s { Shape::Circle(r) => r } }` | `Block.tail = Some(Match)` | 2 (`}` next) | ESTABLISHED (§3 example) | SKETCH |
| T2 | `fn f() -> i64 { if flag { 1 } else { 2 } }` | `Block.tail = Some(If)` | 2 | ESTABLISHED (§3 `fn parse`) | SKETCH |
| T3 | `fn f() { { g(); } }` | `Block.tail = Some(Block)` | 2 | DERIVED | SKETCH |
| T4 | `fn f() { for i in 0..10 { p(i); } finish(); }` | `Stmt::Expr(For)` then `Stmt::Expr(Call finish)` | 3 (next ≠ `}`, ≠ `;`) | ESTABLISHED (§3 `fn main`) | PROPOSED (G-1/PD-005) |
| T5 | `fn f() { match s { A => 1 }; }` | `Stmt::Expr(Match)` + `;` consumed; tail `None` | 1 | DERIVED | SKETCH |
| T6 | `fn f() -> i64 { g() }` | `Block.tail = Some(Call)` | 2 | DERIVED | SKETCH |
| T7 | `fn f() { g() h() }` | diagnostic `expected ';'` at `h`, deterministic recovery | 4 | DERIVED | PROPOSED (recovery) |

## 2. Negative corpus

Each case lists the expected **first** diagnostic (message class), the
recovery behavior, and the rule that classifies it. No negative case may
require type information to classify (Phase 1 rule).

| ID | Source | Expected diagnostic | Recovery | Status |
| --- | --- | --- | --- | --- |
| N1 | `let x = ;` | `expected expression` at `;` | zero-width expr placeholder; `;` consumed | PENDING (messages) |
| N2 | `let x` (no `=`) | `expected '='` | initializer placeholder; sync at `;` | ESTABLISHED (`=` required by EBNF) |
| N3 | `fn f( { }` | `expected parameter` | sync at `{`; parse body; report once | PENDING (messages) |
| N4 | `struct { }` | `expected identifier` | skip to `}` | PENDING |
| N5 | `"abc` | `unterminated string literal` | `Error` token to line end | PENDING (lexer §8) |
| N6 | `/* abc` | `unterminated block comment` | resume at EOF | PENDING |
| N7 | `0x` | `expected digits after '0x'` | `Error` token | PENDING |
| N8 | `1_` | `'_' must appear between digits` | `Error` token | PENDING |
| N9 | `a < b < c` | `comparison operators cannot be chained; use parentheses` | left-nested recovery `((a<b)<c)` | PENDING (PD-005) |
| N10 | `a..b..c` | `range cannot be chained; use parentheses` | left-nested recovery | PENDING (PD-005) |
| N11 | `a = b = c` | `chained assignment; parenthesize the right-hand side` | right-nested recovery | PENDING (PD-003) |
| N12 | `1 = x;` | `invalid assignment target` | build assignment anyway | PENDING (PD-003) |
| N13 | `(x) = 1;` | **withdrawn** — valid under amendment B-1; moved to positive P41 | — | Resolved by B-1 (owner approval required) |
| N14 | `import a::b;` | `expected '.' or ';'` after module segment | sync at `;` | PENDING (PD-006) |
| N15 | `import a.{b};` | `expected identifier` | sync at `;` | PENDING (PD-006) |
| N16 | `match s { }` | *parses* (empty arms list); exhaustiveness is Phase 2 | n/a | PENDING (grammar permits empty arms; flagged) |
| N17 | `match s { A => }` | `expected expression` | arm placeholder body | PENDING |
| N18 | `"{}"` | `empty interpolation` at segment span | `Expr` placeholder | PENDING (PD-004) |
| N19 | `"\q"` | `unknown escape sequence '\q'` | literal `q` accepted | PENDING |
| N20 | `fn f() { return x y }` | `expected ';'` at `y` | sync; `return x` statement built | PENDING (G-2) |
| N21 | `x.0` | `expected field name` after `.` | field placeholder | PENDING |
| N22 | `1.` | lexes `Int(1) .`; `expected field name` | field placeholder | PENDING (munch rules) |
| N23 | `.5` | lexes `. Int(5)`; `expected expression` | placeholder | PENDING |
| N24 | `&x` | `unexpected character '&'` | `Error` token | ESTABLISHED-ish (no `&` token) |
| N25 | `..=` | lexes `..` `=`; expression error | placeholder | PENDING (no `..=` operator) |
| N26 | `let x: ;` | `expected type` | type placeholder | PENDING |
| N27 | `Vec<T>>` in type position (excess closer) | `unexpected '>'` | skip excess closer | PENDING (G-3) |
| N28 | `fn f(,) {}` | `expected parameter` | skip comma | PENDING |
| N29 | `;;` | `expected expression` (no empty statements) | skip `;` | PENDING (DERIVED) |
| N30 | `struct S { x: }` | `expected type` | placeholder type; sync at `,`/`}` | PENDING |
| N31 | `((((… 1000 × `(` …))))` | `expression nesting limit (64) exceeded` | skip to expression sync set | PENDING (B-4) |
| N32 | `let x = 1e9999;` | `floating-point literal is not representable` | `Error` token | PENDING (B-3) |
| N33 | `let x = 18446744073709551616;` (2^64) | `integer literal is too large` | `Error` token (no substitution) | PENDING (B-2) |
| N34 | string containing > 64 nested `{` | `interpolation nesting limit (64) exceeded` | consume to balancing `}`/EOF; `Error` | PENDING (B-4) |

## 3. Test criteria (implementation phase)

1. **Token snapshots (insta):** exact `(TokenKind, Span)` sequences for all
   §1 sources, including a full-source snapshot of the §3 example
   (P1–P7 concatenated), honoring the lexer-contract §9 span invariants
   (top-level vs embedded levels).
2. **AST snapshots:** canonical printer output (parser contract §5) for all
   §1 entries; exact-string compare; placeholders never presented as
   successful parses.
3. **Round-trip property:** `parse(print(ast)) ≡ ast` (parser contract §5
   item 6 relation) for corpus ASTs and generated ASTs; not claimed for
   rejected inputs.
4. **Negative snapshots:** exact `xl-diag` rendering for §2 cases
   (message + span + recovery tree), so drift fails loudly.
5. **Fuzz (cargo-fuzz):** pipeline over arbitrary inputs: no panic
   (including no stack overflow — B-4), no hang, one trailing `Eof`, valid
   spans, progress guarantee, nesting limits enforced.
6. **Acceptance gate (SPECIFIED §6 Phase 1):** 100% of the §3 example
   constructs parse (P1–P7) and 1 hour of fuzzing without failures — to be
   executed in the implementation directive, not claimed here.

## 4. Coverage cross-check (enumerated, per Directive 005 §11H)

Every operator named by `grammar-rfc.md` §2.4 with its conformance case(s);
nothing is claimed covered by an example that does not exercise it.

| Operator/token | Case(s) |
| --- | --- |
| `+` | P3, P29, P45 |
| `-` (binary) | P3, P30 |
| `*` | P5, P7 |
| `/` | P38 (`/=`; binary `/` shares the tier — future obligation F1) |
| `%` | P40, P38 (`%=`) |
| `==`, `!=` | P31 |
| `<`, `<=`, `>`, `>=` | P6 (`<`), P31 (all four) |
| `&&`, `\|\|` | P39 |
| `!` | P29, P39 |
| `=` | P41 (plus P8-style `let` initializers elsewhere) |
| `+=` | P11 |
| `-=`, `*=`, `/=`, `%=` | P38 |
| `..` | P7, P10 |
| `.` | P3, P7, P9, P32, P45-adjacent |
| `::` | P5, P21 |
| `?` | P6, P9, P32 |
| `as` | P11 |
| `->` | P3, P18 |
| `=>` | P5, T1, P16 |
| `( ) { } [ ] , : ;` | throughout the corpus (structural; no dedicated case needed) |
| keywords `fn let mut if else match while for in loop break continue return struct enum impl trait import pub as true false` | all appear across P1–P45 / T1–T7 |
| `_` wildcard | P16 |
| Unicode identifiers | P45 |

Future test obligations (explicitly not covered by the corpus, assigned to
the implementation phase):

- **F1:** binary `/` division expression (only the compound `/=`, P38, and
  `* `/ `%` binaries are enumerated; `/`-binary shares the multiplicative
  tier and needs its own snapshot case).
- **F2:** `%` with float operands (same token; semantic phase).
- **F3:** precedence-combination property (random expression ASTs via
  proptest complement the fixed cases).
- **F4:** depth-limit boundaries (65th vs 64th nesting level) exactly.
- **F5:** CRLF/whitespace/BOM lexical cases (P13 covers comments only).
- **F6:** empty tuple type `()` in type position and unit expression `()`.

No claim of complete coverage is made beyond the table above.
