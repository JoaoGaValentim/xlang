# XLANG Phase 1 — Grammar and Language-Surface RFC

**STATUS: PROPOSED — OWNER REVIEW REQUIRED**

- **Directive:** Engineering Directive 004 — Phase 1 Syntax Architecture and
  Specification Gate (2026-10-12)
- **Authoritative sources:** `docs/spec-v0.1-original.txt` (v0.1, archived,
  SHA-256 `c3c6bce5b3e804b5c6441398e4665ac9271df4f2ca2af262d8019bcb4b343770`)
  and accepted ADRs (`docs/decisions/`).
- **Scope:** language surface only (tokens, grammar, precedence, structural
  rules). No semantic resolution, no type checking, no implementation.
- **Related documents:** `lexer-contract.md`, `parser-ast-contract.md`,
  `conformance-plan.md`, `decision-packet.md` (this directory).

Evidence labels used throughout:

- **SPECIFIED** — directly stated or shown by the original specification.
- **DERIVED** — forced by a combination of SPECIFIED facts (for example, the
  `for` statement in the original `main` example has no terminating `;`, so
  block-like statements cannot require one).
- **PROPOSED** — a concrete design choice filling a gap the original leaves
  open. Requires owner approval; nothing here is approved.
- **UNRESOLVED** — the original is silent and no Phase 1 rule is proposed
  (or a proposal is deliberately withheld).

## 1. Evidence base from the original

The single complete original example (SPECIFIED, §3) is the primary
conformance artifact:

```xlang
import std.io;

struct Point { x: f64, y: f64 }

impl Point {
    pub fn dist(self, o: Point) -> f64 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape { Circle(f64), Rect(f64, f64) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rect(w, h) => w * h,
    }
}

fn parse(s: str) -> Result<i64, str> {
    let n = s.to_int()?;
    if n < 0 { Err("negativo") } else { Ok(n) }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 { total += i as f64; }
    io.println("total = {total}");
}
```

Additional SPECIFIED surface facts: keywords (§3), primitive and compound
types (§3), the EBNF sketch with the Pratt chain (§3), the `Span` struct and
`TokenKind`/`ExprKind` sketches (§5), and the statement form
`stmt = "let" ["mut"] pattern [":" type] "=" expr ";" | expr ";" | item`
(§3).

## 2. Token inventory (PD-002)

### 2.1 Keywords (SPECIFIED — §3)

`fn let mut if else match while for in loop break continue return struct
enum impl trait import pub as true false`

Two explicit gaps between the §3 keyword list and the §5 `TokenKind` sketch
are resolved by restoring the keywords (PROPOSED — restoration of SPECIFIED
content):

| Gap | §3 keyword list | §5 sketch | Resolution |
| --- | --- | --- | --- |
| `break` | present | **missing** | add `Break` token |
| `continue` | present | **missing** | add `Continue` token |

`self` is **not** in the §3 keyword list and is therefore lexed as an
identifier with contextual meaning (SPECIFIED omission → DERIVED): it is
recognized as a method receiver in parameter position and as a path root in
expressions. Treating `self` as a keyword would contradict §3.

### 2.2 Identifiers and the wildcard (DERIVED + PROPOSED)

- Identifier start: `XID_Start` **or** `_`; identifier continue:
  `XID_Continue` (start rule extended by `_` to permit names such as
  `_unused`; PROPOSED, low risk).
- A lone `_` is the token `Underscore`, reserved for the wildcard pattern
  (`pattern = "_" | …`, SPECIFIED §3). `Underscore` is not an expression.

### 2.3 Literals (SPECIFIED categories, PROPOSED detailed forms)

| Token | Forms (proposal) | Evidence |
| --- | --- | --- |
| `Int` | decimal `1_000`; hex `0xFF`; binary `0b1010`. Token carries **magnitude (`u64`) + radix**; signed/target-type range checks are deferred to the type-system phase (amendment B-2, `lexer-contract.md` §4.1). The §5 sketch's `Int(i64)` payload is amended for this reason | SPECIFIED §3; representation amended (PROPOSED) |
| `Float` | `3.14159`, `0.0`, `1.5e3`, `1e5`. Tokens always carry a **finite** `f64`; literals that overflow to infinity are diagnosed (amendment B-3, `lexer-contract.md` §4.2). `Float(f64)` payload follows the §5 sketch | SPECIFIED category "floats"; forms/policy PROPOSED (see `lexer-contract.md` §4) |
| `Str` | structured string with escapes and `{expr}` interpolation | SPECIFIED §3; structure per PD-004 |
| `Char` | `'a'`, escapes | SPECIFIED §3 |
| `true`, `false` | boolean literals (keywords) | SPECIFIED §3 |

No octal literals, no suffixes (`1u32`), no digit separators beyond `_`
(SPECIFIED: only the listed exemplars).

### 2.4 Operators and punctuation

| Token | Lexeme | Evidence |
| --- | --- | --- |
| `Plus Minus Star Slash Percent` | `+ - * / %` | SPECIFIED §5 + example |
| `Eq EqEq NotEq Lt Le Gt Ge` | `= == != < <= > >=` | SPECIFIED §5 + example |
| `AndAnd OrOr Bang` | `&& || !` | SPECIFIED §5 |
| `PlusEq` | `+=` | **REQUIRED by the §3 example** (`total += i as f64;`); the §5 sketch omits it — explicit gap |
| `MinusEq StarEq SlashEq PercentEq` | `-= *= /= %=` | PROPOSED completion of the compound set opened by `+=` (see decision packet PD-002) |
| `Arrow FatArrow` | `-> =>` | SPECIFIED §5; `=>` used by match arms, `->` by function returns |
| `Question` | `?` | SPECIFIED §5 + example (`s.to_int()?`) |
| `Dot DotDot ColonColon` | `. .. ::` | SPECIFIED §5 + example (`0..10`, `Shape::Circle`) |
| `LParen RParen LBrace RBrace LBracket RBracket` | `( ) { } [ ]` | SPECIFIED §5 |
| `Comma Colon Semi` | `, : ;` | SPECIFIED §5 |
| `Underscore` | `_` | DERIVED (wildcard pattern, §3) |
| `Eof` | end of input | SPECIFIED §5 |
| `Error` | (recovery token, carries a span and a diagnostic) | PROPOSED — lexer recovery contract |

Operators **deliberately not introduced** (no evidence in the original; each
would be an invented feature): bitwise `& | ^ ~`, shifts `<< >>`, compound
`<<= >>= &= |= ^=`, inclusive range `..=`, rest patterns `...`, attributes
`#[…]`, lifetime/borrow operators `&`/`&&(type)`, and any operator not in the
§5 sketch or the §3 example.

Range: `..` only, as a binary operator (SPECIFIED `0..10`); open-ended forms
(`a..`, `..b`) and `..=` are **UNRESOLVED** and not proposed.

## 3. Complete proposed grammar

Conventions: `{ x }` = zero or more; `[ x ]` = optional; `|` alternation.
Every rule below is traceable to §1 evidence plus the labeled additions.

### 3.1 Items

```ebnf
item        = import_decl | fn_decl | struct_decl | enum_decl
            | impl_decl | trait_decl ;

import_decl = "import" module_path [ "as" IDENT ] ";" ;        (* PD-006 *)
module_path = IDENT { "." IDENT } ;

fn_decl     = [ "pub" ] "fn" IDENT [ generics ] "(" [ params ] ")"
              [ "->" type ] block ;
params      = param { "," param } [ "," ] ;
param       = "self" | IDENT ":" type ;

generics    = "<" generic_param { "," generic_param } [ "," ] ">" ;
generic_param = IDENT [ ":" path_type ] ;

struct_decl = [ "pub" ] "struct" IDENT [ generics ]
              "{" [ fields ] "}" ;
fields      = field { "," field } [ "," ] ;
field       = [ "pub" ] IDENT ":" type ;

enum_decl   = [ "pub" ] "enum" IDENT [ generics ]
              "{" [ variants ] "}" ;
variants    = variant { "," variant } [ "," ] ;
variant     = IDENT [ "(" type_list ")" ] ;

impl_decl   = "impl" [ generics ] path_type [ "for" path_type ]
              "{" { impl_item } "}" ;
impl_item   = [ "pub" ] "fn" IDENT "(" [ params ] ")" [ "->" type ]
              block ;

trait_decl  = [ "pub" ] "trait" IDENT [ generics ]
              "{" { trait_item } "}" ;
trait_item  = "fn" IDENT "(" [ params ] ")" [ "->" type ] ";" ;
```

Labels: `fn_decl`, `block`, `stmt`, `pattern`, `path`, `"pub"`, `"fn"`,
`"->"` are SPECIFIED (§3 EBNF). `import … [as IDENT]`, fields/variants
lists, generics syntax, `impl Trait for Type`, signature-only `trait_item`,
and `[ "pub" ]` on struct/enum/trait/field are PROPOSED completions (§8
records each gap with an example). Trailing commas in every comma-separated
list: DERIVED (match arms in the §3 example end with a trailing comma) and
extended uniformly (PROPOSED, low risk).

### 3.2 Statements and blocks

```ebnf
block       = "{" { stmt } [ expr ] "}" ;                      (* SPECIFIED *)
stmt        = let_stmt | expr_stmt | item ;
let_stmt    = "let" [ "mut" ] pattern [ ":" type ] "=" expr ";" ;
expr_stmt   = expr ( ";" | ε ) ;        (* see the semicolon rule below *)
```

**Semicolon rule (DERIVED from the §3 example; corrected decision order per
Directive 005 §4, restated precisely in `parser-ast-contract.md` §3):** in
statement position, parse a full expression, then decide in this exact
order:

1. `;` follows → the expression is an expression statement (`;` consumed).
2. otherwise, the next token is the enclosing block's `}` → the expression
   is the block's **tail expression** (its value is preserved).
3. otherwise, the expression is *block-like* (`If`, `Match`, `While`,
   `For`, `Loop`, `Block`) → semicolon-free expression statement.
4. otherwise → diagnostic `expected ';'` with deterministic recovery.

Rule 2 precedes rule 3 deliberately: a block-like expression in tail
position — the `match` in `fn area`, the `if` in `fn parse` — must remain
the block's value and must not be reclassified as a statement. Evidence: in
`fn main` the `for` statement is followed directly by the next statement
with no `;`, while `total += i as f64;` and `io.println(…);` carry one.

There is no empty statement: a lone `;` is not a `stmt` (DERIVED — the §3
`stmt` production cannot derive it).

### 3.3 Expressions

Precedence tiers are numbered lowest (1) to highest (9); the original chain
(`atribuição < || < && < comparação < + - < * / % < unário < pós-fixo`,
SPECIFIED §3) is preserved exactly, with the range tier inserted between
assignment and `||` (PROPOSED — see PD-005).

```ebnf
expr        = assignment ;
assignment  = range [ assign_op range ] ;              (* non-assoc, tier 1 *)
assign_op   = "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;
range       = or_expr [ ".." or_expr ] ;               (* non-assoc, tier 2 *)
or_expr     = and_expr { "||" and_expr } ;             (* left,  tier 3 *)
and_expr    = cmp_expr { "&&" cmp_expr } ;             (* left,  tier 4 *)
cmp_expr    = add_expr [ cmp_op add_expr ] ;           (* non-assoc, tier 5 *)
cmp_op      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
add_expr    = mul_expr { ( "+" | "-" ) mul_expr } ;    (* left,  tier 6 *)
mul_expr    = unary_expr { ( "*" | "/" | "%" ) unary_expr } ; (* left, tier 7 *)
unary_expr  = ( "-" | "!" ) unary_expr | postfix_expr ; (* tier 8 *)
postfix_expr = primary
    { "(" [ expr_list ] ")"      (* call *)
    | "." IDENT                  (* field *)
    | "[" expr "]"               (* index *)
    | "?"                        (* error propagation *)
    | "as" type                  (* cast *)
    } ;                          (* tier 9, strictly left-to-right *)
```

Primary expressions:

```ebnf
primary     = literal
            | "(" [ expr { "," expr } [ "," ] ] ")"        (* unit/tuple/parens *)
            | path_expr
            | if_expr | match_expr | while_expr | for_expr | loop_expr
            | block                                         (* block expression *)
            | "return" [ expr ]            (* operand extends maximally *)
            | "break" | "continue"
            | array_lit ;

literal     = INT | FLOAT | STR | CHAR | "true" | "false" ;
path_expr   = IDENT { "::" IDENT } ;
expr_list   = expr { "," expr } [ "," ] ;
array_lit   = "[" [ expr_list ] "]" ;

if_expr     = "if" expr block [ "else" ( if_expr | block ) ] ;
match_expr  = "match" expr "{" [ arms ] "}" ;
arms        = arm { "," arm } [ "," ] ;
arm         = pattern "=>" expr ;
while_expr  = "while" expr block ;
for_expr    = "for" pattern "in" expr block ;
loop_expr   = "loop" block ;
```

Labels: `ExprKind` sketch atoms (`Lit`, `Path`, `Unary`, `Binary`, `Call`,
`Field`, `Index`, `If`, `Match`, `Block`, `Try`, `Cast`) are SPECIFIED (§5).
`Assign`, `Range`, `Tuple`, `Array`, `While`, `For`, `Loop`, `Break`,
`Continue`, `Return` are PROPOSED additions to the §5 sketch needed for
parseability of SPECIFIED surface (`+=`, `for`, `loop`/`while`/`break`/
`continue`/`return` keywords, tuple types) — see §8 and decision packet
G-1/G-4.

### 3.4 Types

```ebnf
type         = path_type | grouped_type | tuple_type
             | "[" type [ ";" expr ] "]"
             | "fn" "(" [ type_list ] ")" [ "->" type ] ;

grouped_type = "(" type ")" ;
tuple_type   = "(" ")" | "(" type "," [ type { "," type } [ "," ] ] ")" ;
type_list    = type { "," type } [ "," ] ;
path_type    = IDENT { "::" IDENT } [ generic_args ] ;
generic_args = "<" type { "," type } [ "," ] ">" ;     (* closer rule, §8.7 *)
```

Derivable forms (checked in `consistency-review.md` §D-3): `()`, `(i64)`
(grouping — same node as `i64`), `(i64,)`, `(i64, bool)`, `[i64]`,
`[i64; 4]`, `Result<i64, str>`, `Vec<Vec<i64>>`, `fn()`, `fn(i64)`,
`fn(i64) -> i64`, `fn(i64, bool) -> Result<i64, str>`. Grouping is
syntax-only: `(T)` yields the inner type node (no AST wrapper). The
function type has an optional (possibly empty) parameter list and an
optional return type; the three optionals are independent.

Labels: SPECIFIED §3 type list (`i8..i64`, `u8..u64`, `isize`, `usize`,
`f32`, `f64`, `bool`, `char`, `str`, `()`, `[T]`, `[T; N]`, `(A, B)`,
`Option<T>`, `Result<T, E>`, `fn(A) -> B`). Primitive type names are
**identifiers**, not keywords (they are absent from the §3 keyword list) and
are resolved in a prelude namespace (DERIVED). `[T; N]`: `N` is an
expression (SPECIFIED form). No const generics, no `dyn`/`impl Trait` in
types (no evidence; UNRESOLVED).

### 3.5 Patterns

```ebnf
pattern     = "_" | IDENT | literal_pat | path_pat | tuple_pat ;
literal_pat = INT | FLOAT | STR | CHAR | "true" | "false" ;
path_pat    = IDENT { "::" IDENT } [ "(" [ pattern { "," pattern } [ "," ] ] ")" ] ;
tuple_pat   = "(" pattern { "," pattern } [ "," ] ")" ;
```

SPECIFIED by the §3 `pattern` production (`"_" | IDENT | literal | path
["(" patterns ")"] | "(" patterns ")"`). Restrictions: no struct patterns,
no or-patterns, no guards, no negative literal patterns, no `ref`/`mut`
bindings — all UNRESOLVED (not proposed, not rejected). An `IDENT` pattern
binds a new name; a `path_pat` with `::` or an uppercase prelude path denotes
a constructor (name-resolution heuristic is a semantic phase concern; the
parser records the syntactic distinction only as written).

## 4. Operator precedence and associativity (PD-005)

### 4.1 Binding powers (proposal; concrete values for the Pratt parser)

| Tier | Operators | Associativity | `lbp` | `rbp` | Rule for same-tier repetition |
| --- | --- | --- | --- | --- | --- |
| 1 | `= += -= *= /= %=` | non-associative (syntax) | 2 | 3 | diagnostic + right-nested recovery |
| 2 | `..` | non-associative | 4 | 5 | diagnostic + left-nested recovery |
| 3 | `\|\|` | left | 6 | 7 | — |
| 4 | `&&` | left | 8 | 9 | — |
| 5 | `== != < <= > >=` | non-associative | 10 | 11 | diagnostic + left-nested recovery |
| 6 | `+ -` | left | 12 | 13 | — |
| 7 | `* / %` | left | 14 | 15 | — |
| 8 | prefix `- !` | (prefix) | — | 16 | operand parsed at min-bp 16 |
| 9 | postfix `( )` `.` `[ ]` `?` `as` | left-to-right chain | — | — | applied immediately after each primary/unary |

Non-associative tiers are enforced explicitly by the Pratt loop: after one
application of a tier-1/2/5 operator, a following operator **of the same
tier** produces a diagnostic (`operators of this tier cannot be chained; use
parentheses`) and parsing continues with a deterministic recovery shape
(right-nested for assignment, left-nested otherwise). The parser never
relies on implicit precedence for these tiers.

### 4.2 Precedence-sensitive examples (different ASTs)

| Source | Parses as | Would-be alternative (rejected) |
| --- | --- | --- |
| `a + b * c` | `a + (b * c)` | `(a + b) * c` |
| `a - b - c` | `(a - b) - c` | `a - (b - c)` |
| `a \|\| b && c` | `a \|\| (b && c)` | `(a \|\| b) && c` |
| `!a && b` | `(!a) && b` | `!(a && b)` |
| `0..n + 1` | `0..(n + 1)` | `(0..n) + 1` |
| `total += i as f64` | `total += (i as f64)` | `(total += i) as f64` |
| `-x * y` | `(-x) * y` | `-(x * y)` |
| `f(x)?` | `(f(x))?` | `f(x?)` |
| `a = b + c` | `a = (b + c)` | `(a = b) + c` |
| `s.to_int()? + 1` | `(s.to_int()?) + 1` | `s.to_int()? + 1` under any other chain |
| `a < b < c` | **diagnostic** (non-assoc); recovery `(a < b) < c` | `a < (b < c)` |
| `a..b..c` | **diagnostic** (non-assoc); recovery `(a..b)..c` | `a..(b..c)` |
| `a = b = c` | **diagnostic** (non-assoc); recovery `a = (b = c)` | — |

## 5. Assignment semantics at the syntax level (PD-003)

PROPOSED contract (full rationale in the decision packet):

1. **Assignment is an expression** (tier 1, the lowest), consistent with the
   SPECIFIED Pratt chain (§3) and the expression-oriented model (§1–§2).
2. **Operators:** `=` plus the compound set `+= -= *= /= %=` (`+=` is
   REQUIRED by the example; the remainder complete the family).
3. **Left-hand side:** restricted at parse time to *place expressions*:
   `path_expr` (identifier or `self`), field access (`e.name`), or index
   (`e[i]`). Grouping parentheses are **transparent** for this
   classification (amendment B-1, Directive 005 §5): the AST does not
   retain grouping nodes, so `(x) = 1`, `((x)) = 1`, `(obj.field) = 1`, and
   `(items[i]) = 1` are valid assignment forms exactly like their
   unparenthesized counterparts. `(x + y) = 1`, `f() = 1`, and `42 = 1`
   remain **syntax errors** (`invalid assignment target`), reported without
   any type checking. Recovery: the node is still built so that parsing
   continues. (The Directive 004 draft rejected `(x) = 1`, contradicting
   the AST that drops grouping; B-1 resolves the contradiction in favor of
   transparency.)
4. **Non-associativity:** `a = b = c` is a syntax error (see §4); write
   `a = (b = c)` to nest explicitly. This removes the classic chained-
   assignment hazard without requiring Phase 2 type rules.
5. **Value and type (proposed direction only; a type-system-phase
   question, not part of the Phase 1 frontend contract):** assignment is
   *proposed* to have type `()` and produce no value; approval belongs to
   the type-system phase. `let` is the only binding form; there is no
   assignment-as-declaration.
6. **Declaration is not assignment:** `let [mut] pattern [: type] = expr;`
   (SPECIFIED) is a statement; its `=` is initializer syntax, not the
   tier-1 operator. The initializer is a full `expr` (so
   `let x = y = z;` parses and `y = z` has type `()` — Phase 2 rejects it
   unless `x: ()`).
7. **Mutability:** `mut` appears only on `let` (SPECIFIED). Assignment to a
   non-`mut` binding is a Phase 2 mutability error (SPECIFIED §6 Phase 2:
   "verificação de mutabilidade"). Phase 1 only parses.

## 6. Imports, paths, and visibility at the syntax level (PD-006)

PROPOSED syntactic contract; resolution semantics belong to Phase 2 and are
not defined here. The original gives: `import std.io;` (§3), `1 .x file = 1
module` and `xl.toml` packages (§2), `::` paths such as `Shape::Circle`
(§3), and `pub` (keyword list, example).

1. **Import grammar:** `import module_path [ "as" IDENT ] ";"` where
   `module_path = IDENT { "." IDENT }`. Dots form the module path
   (SPECIFIED by example); `::` is **not** an import separator.
2. **Import alias** (`as IDENT`): PROPOSED (needed to disambiguate
   collisions; `as` is already a keyword). Unsupported import forms (each a
   clear diagnostic, no grammar production): brace groups
   (`import a.{b, c};`), wildcard (`import a.*;`), `::` separators
   (`import a::b;`), multiple paths per statement, and `pub import`
   (re-exports are UNRESOLVED).
3. **Expression and type paths:** `IDENT { "::" IDENT }` (SPECIFIED
   `Shape::Circle`). No turbofish (`f::<T>(…)`) — explicit generic arguments
   in expressions are not evidenced and are not introduced (UNRESOLVED).
4. **Visibility syntax:** optional `pub` before `fn`, `struct`, `enum`,
   `trait`, and struct fields; `pub fn` inside `impl` blocks (SPECIFIED for
   methods by the example). `impl` blocks themselves take no visibility.
   Default visibility (private) and full visibility rules are semantic
   (Phase 2); Phase 1 fixes only where `pub` may appear.
5. **Items inside blocks:** the §3 `stmt` production includes `item`, so
   `import`, `fn`, `struct`, `enum`, `impl`, and `trait` may appear inside
   blocks (SPECIFIED by the EBNF; semantics deferred).
6. **Error recovery:** a malformed import synchronizes at `;` or `}` and
   emits one primary diagnostic with the unexpected token span; parsing
   continues at the next item.

## 7. Interpolated strings — summary (PD-004)

Full design in `lexer-contract.md` §5 and `parser-ast-contract.md` §4.
Summary of the proposal:

- The lexer scans the entire string literal and produces **one structured
  `Str` token** containing ordered segments: literal text segments (escapes
  already decoded) and embedded-expression segments, each embedded segment
  carrying its own already-lexed token sequence with absolute spans.
- `{` begins an interpolation; `\{` and `\}` are escapes for literal braces
  (a lone `}` in text is also accepted as literal); interpolations nest
  through embedded strings because embedded expressions are lexed normally.
- The parser converts the segments into `Lit(Str(StringLit))` where
  `StringLit { parts: Vec<StringPart> }` and
  `StringPart::Text(String) | StringPart::Expr(Expr)`. Phase 2 (SPECIFIED
  §6) desugars interpolation to concatenation; no desugaring happens in
  Phase 1.
- Malformed forms (empty `{}`, unterminated interpolation, unterminated
  string) have specified diagnostics and spans (see contracts).

## 8. Structural gap analysis (complete grammar consistency)

Each entry gives a reproducible example, the ambiguity or gap, and the
proposed parsing rule. Items marked (packet) are also raised in
`decision-packet.md`.

1. **Missing `break`/`continue` tokens** — gap between §3 keywords and §5
   sketch. Resolved in §2.1 (SPECIFIED restoration; packet PD-002).
2. **Missing compound assignment tokens** — `total += i as f64;` (§3) vs
   the §5 sketch. Resolved in §2.4 (packet PD-002).
3. **Loops/control flow absent from the §3 `stmt` EBNF and the §5
   `ExprKind` sketch** — Example: `for i in 0..10 { … }` and `while x { … }`
   and `loop { … }` and `return n;` and `break;` must be derivable.
   Proposed rule (syntax only): `while`/`for`/`loop`/`return`/`break`/
   `continue` are **expressions** (primary-level); `return`'s optional
   operand extends maximally to the right; `break`/`continue` take no
   operand (`break` with a value is UNRESOLVED). Type behavior — unit
   values, divergence, value-producing loops — is **not** fixed by this
   frontend contract; it is deferred to the type-system phase (see
   `consistency-review.md` §B-2 and packet category C). (packet G-1)
4. **Statement semicolon rule** — Evidence: the `for` statement in `fn
   main` carries no `;`. Corrected rule (§3.2): exact decision order is
   `;` → statement; else `}` → **tail expression**; else block-like →
   semicolon-free statement; else diagnostic. The tail check precedes the
   block-like check so tail `match`/`if` values survive (`fn area`,
   `fn parse`). (packet G-2)
5. **Function parameters and `self`** — `pub fn dist(self, o: Point) -> f64`
   (§3). Proposed rule: `param = "self" | IDENT ":" type`; `self` allowed
   only as the first parameter of a method; `mut self` and pattern
   parameters are not introduced (UNRESOLVED); `self` remains an ordinary
   identifier lexically (§2.1).
6. **Trailing commas** — SPECIFIED evidence: the final match arm in the §3
   example ends with `,`. Proposed rule: allowed in every comma-separated
   list (params, arguments, fields, variants, arms, patterns, types,
   generics); never required.
7. **Generic angle brackets vs comparison** — `Vec<Vec<T>>` and
   `let x: Vec<Vec<T>> = v;`. Because there are no shift operators, the
   lexer emits `>` `>` and `>=` as single/adjacent tokens, not `>>`.
   Proposed rule (packet G-3): in a *type-argument closing* context the
   parser accepts `Gt` as one closer; if the closer run exhausts a `Ge`
   token (`>=`), it is split into `Gt` + `Eq` **only in that context**.
   Expressions are unaffected (`a >= b` is always one `Ge`).
   There is no `<` ambiguity in expressions because generic arguments exist
   only in type positions (no turbofish).
8. **`impl` vs trait implementations** — `impl Point { … }` (§3);
   Phase 5 requires `impl Trait for Type` (§6 Phase 5 "traits … impl Trait
   for Type"). Proposed rule: both forms parse now (`impl [generics]
   path_type [ "for" path_type ] { … }`); semantic rules are Phase 5.
9. **Trait declarations** — `trait` is an item (§3 EBNF) with no shown
   body form. Proposed rule: signature-only functions terminated by `;`
   (no bodies). Trait-associated items beyond `fn` are not introduced.
10. **Struct/enum bodies** — SPECIFIED: `struct Point { x: f64, y: f64 }`,
    `enum Shape { Circle(f64), Rect(f64, f64) }`. Proposed rule: fields are
    `[pub] IDENT ":" type`; variants are unit (`A`) or tuple-payload
    (`A(T, …)`); **struct-like variants (`A { x: T }`) are not
    introduced** (UNRESOLVED). **Struct literal construction syntax does
    not exist in the original** and is **not** proposed in Phase 1
    (UNRESOLVED — flagged informational as G-5 in the packet). Consequence:
    `if` conditions cannot be confused with struct literals, and
    `if x { … }` is unambiguous.
11. **`match` arms** — SPECIFIED shape `pat => expr,` with trailing comma.
    Proposed rule: arms comma-separated, trailing comma allowed; arm bodies
    are expressions (blocks included); guards (`pat if cond =>`) and
    or-patterns are not introduced (UNRESOLVED).
12. **`if`/`else`** — SPECIFIED as an expression (`if n < 0 { Err(…) }
    else { Ok(n) }` used as a function body). Proposed rule:
    `"if" expr block [ "else" ( if | block ) ]`; the condition is an
    unparenthesized expression and always stops before the required block;
    `else if` chains recursively. When used as a statement, the §8.4
    semicolon rule applies. A missing `else` is a *semantic* (type) issue
    when a value is required — not a Phase 1 error.
13. **Tuple and array syntax** — Types SPECIFIED (`(A, B)`, `[T]`,
    `[T; N]`). Proposed expression forms: `()` unit, `(e)` grouping,
    `(e, …)` tuple, `[e, …]` array list. `(T,)` one-tuples are allowed
    syntactically; repeat arrays (`[x; N]`) are not introduced
    (UNRESOLVED). (packet G-4)
14. **Block expressions and trailing semicolons** — SPECIFIED:
    `block = "{" { stmt } [expr] "}"`; the last expression is the value; a
    trailing `;` discards it (unit). A lone `;` is not a statement (§3.2).
15. **Nested items** — SPECIFIED by the §3 `stmt` production including
    `item`; items inside blocks take no trailing `;`.
16. **Path expressions vs types** — same `::` shape; disambiguated by
    context (type positions: after `:`, `->`, `as`, in `generic_args`,
    `impl`/`for` targets). No expression-level generic arguments exist
    (§6.3), so no expression/type split is ever required mid-stream.
17. **Integer vs float vs range vs field access** — maximal-munch rules
    (`lexer-contract.md` §4): `1..2` → `Int(1) DotDot Int(2)`; `1.5` →
    `Float`; `1.foo` → `Int(1) . foo` (field access; type error later);
    `1.e3` → `Int(1) . e3` (field access on a literal — deterministic, not
    a float); `1e5` → `Float`; `x.0` → error `expected field name`
    (tuple-field access does not exist).
18. **Unary minus and negative literals** — `-1` is
    `Unary(Neg, Lit(1))`; the lexer never produces negative literals
    (DERIVED: `-` is a distinct token in the §5 sketch).
19. **Assignment targets** — §5 (packet PD-003).
20. **`self` usage** — `self.x` parses as `path_expr(self)` + field
    (SPECIFIED example). `self` outside `impl` method receivers has no
    Phase 1 restriction beyond being an identifier; Phase 2 defines
    resolution.

## 9. Internal consistency check

- Every operator in §2.4 appears in exactly one tier of §4.1 (or is
  punctuation with a grammar rule in §3); every tier operator is in §2.4.
- The §3 example is derivable by the grammar of §3 (checked item by item in
  `conformance-plan.md` §2 and re-derived in `consistency-review.md` §D).
- No rule contradicts §1–§2 evidence; all additions carry a PROPOSED label
  and appear in the decision packet.
- The grammar introduces no Phase 2+ semantics (no resolution, no types, no
  exhaustiveness, no desugaring). Control-flow *typing* claims were removed
  under Directive 005 §10 (see `consistency-review.md` §B-2).
- Directive 005 corrections applied here: tail-expression decision order
  (§3.2), parenthesized assignment targets (§5.3, B-1), function-type EBNF
  (§3.4), numeric literal representation (§2.3, B-2/B-3). The full audit is
  `consistency-review.md`.

## 10. Open items carried to the owner

The decision packet (`decision-packet.md`) is organized in three categories:

- **A — ready for approval:** PD-002, PD-003 (core), PD-004 (core), PD-005,
  PD-006, G-1 (syntax), G-2 (corrected), G-3, G-4.
- **B — explicit amendment approvals:** B-1 parenthesized assignment
  targets, B-2 numeric literal representation, B-3 floating-point overflow
  policy, B-4 nesting/resource-safety limits.
- **C — deferred semantics/features:** G-5 struct literals (UNRESOLVED),
  control-flow type behavior, `break` with a value, open-ended ranges,
  `..=`, attributes, and any other later-phase feature.
