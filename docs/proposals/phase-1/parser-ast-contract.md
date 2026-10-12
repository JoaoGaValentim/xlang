# XLANG Phase 1 — Parser and AST Contract

**STATUS: PROPOSED — OWNER REVIEW REQUIRED**

- **Directive:** Engineering Directive 004 — Phase 1 Syntax Architecture and
  Specification Gate (2026-10-12)
- **Authoritative sources:** `docs/spec-v0.1-original.txt` §3–§5 (grammar
  sketch, `ExprKind`/`Span` sketches, pipeline and conventions) and the
  accepted ADRs (dependency direction ADR-0001, positions ADR-0002).
- **Scope:** crate responsibilities, AST design, parsing rules, recovery,
  printing, and future `xc parse`. No implementation.

## 1. Crate responsibilities and dependency direction

| Crate | Responsibility | May depend on |
| --- | --- | --- |
| `xl-span` (exists) | source registration, `FileId`, `Span` | — |
| `xl-diag` (exists) | diagnostics model and rendering | `xl-span` |
| `xl-lexer` (Phase 1, later) | source text → `LexOutput` (tokens + lexical diagnostics) | `xl-span`, `xl-diag` |
| `xl-ast` (Phase 1, later) | syntax tree types + canonical printer; no parsing | `xl-span` |
| `xl-parser` (Phase 1, later) | token stream → AST + diagnostics | `xl-lexer`, `xl-ast`, `xl-span`, `xl-diag` |
| `xl-cli` (Phase 1, later) | `xc` binary; `xc parse` | `xl-parser`, `xl-diag`, `xl-span` (initially) |

Rules (ADR-0001 extended):

- Edges above are the complete permitted set; no cycles; `xl-lexer` never
  depends on `xl-parser`; `xl-ast` never depends on the parser.
- Future crates (`xl-hir`, `xl-types`, …) join per the specification layout
  and may depend only downward.
- The canonical printer lives in `xl-ast` (AST → `String`; no other deps) so
  that `xc parse` is `parse → print` with no extra layer. PROPOSED
  placement; moving it later is a non-breaking refactor.

## 2. AST design

### 2.1 Principles

- Every node carries a `Span` (SPECIFIED §4: "todo erro vira Diagnostic com
  Span"; §5 nodes carry spans).
- Node identifiers (`ExprId`, `ItemId`, …) are newtype indices allocated by
  the parser (SPECIFIED §4: newtype IDs instead of pointers; §5 `Expr`
  carries `id`).
- Child edges use owned `Box`/`Vec` (SPECIFIED §5 sketch:
  `Unary(UnOp, Box<Expr>)`). Arena/Vec storage for nodes is deliberately
  **deferred** to a phase that needs index-addressable trees (HIR/MIR);
  this does not contradict §4, which is about identifier discipline, and it
  follows the §5 sketch literally.
- No `unsafe`; no global state; deterministic construction order.

### 2.2 Node sketch (contract level)

```rust
// xl-ast (names indicative; exact naming is an implementation detail)
pub struct Ident { pub name: Symbol, pub span: Span }

pub enum Visibility { Private, Public(Span) }

pub struct Item   { pub id: ItemId, pub kind: ItemKind, pub span: Span }
pub enum ItemKind {
    Fn(FnDecl), Struct(StructDecl), Enum(EnumDecl),
    Impl(ImplDecl), Trait(TraitDecl), Import(ImportDecl),
}

pub struct FnDecl  { pub vis: Visibility, pub generics: Generics,
                     pub sig: FnSig, pub body: Block, pub span: Span }
pub struct FnSig   { pub name: Ident, pub params: Vec<Param>,
                     pub ret: Option<Type>, pub span: Span }
pub enum Param     { Receiver(Span), Named { name: Ident, ty: Type, span: Span } }
pub struct Generics { pub params: Vec<GenericParam>, pub span: Span }
pub struct GenericParam { pub name: Ident, pub bound: Option<PathType>, pub span: Span }

pub struct StructDecl { pub vis: Visibility, pub name: Ident,
                        pub generics: Option<Generics>, pub fields: Vec<Field>,
                        pub span: Span }
pub struct Field { pub vis: Visibility, pub name: Ident, pub ty: Type, pub span: Span }

pub struct EnumDecl  { pub vis: Visibility, pub name: Ident,
                       pub generics: Option<Generics>, pub variants: Vec<Variant>,
                       pub span: Span }
pub struct Variant { pub name: Ident, pub payload: Vec<Type>, pub span: Span }

pub struct ImplDecl { pub generics: Option<Generics>, pub trait_: Option<PathType>,
                      pub self_ty: PathType, pub items: Vec<ImplItem>, pub span: Span }
pub enum ImplItem { Fn(FnDecl), } // visibility applies inside

pub struct TraitDecl { pub vis: Visibility, pub name: Ident,
                       pub generics: Option<Generics>,
                       pub items: Vec<TraitItem>, pub span: Span }
pub struct TraitItem { pub sig: FnSig, pub span: Span } // signature-only

pub struct ImportDecl { pub path: Vec<Ident>, pub alias: Option<Ident>,
                        pub span: Span }

pub struct Block { pub stmts: Vec<Stmt>, pub tail: Option<Box<Expr>>,
                   pub span: Span }
pub struct Stmt  { pub kind: StmtKind, pub span: Span }
pub enum StmtKind { Let(LetStmt), Expr(Expr), Item(Item) }
pub struct LetStmt { pub mutable: bool, pub pat: Pat, pub ty: Option<Type>,
                     pub init: Expr, pub span: Span }

pub struct Expr { pub id: ExprId, pub kind: ExprKind, pub span: Span }
pub enum ExprKind {
    // SPECIFIED §5 sketch atoms:
    Lit(Lit), Path(Path), Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>), Call(Box<Expr>, Vec<Expr>),
    Field(Box<Expr>, Ident), Index(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Block, Option<Box<Expr>>), Match(Box<Expr>, Vec<Arm>),
    Block(Block), Try(Box<Expr>), Cast(Box<Expr>, Type),
    // PROPOSED additions required by the grammar (grammar-rfc §8):
    Assign(AssignOp, Box<Expr>, Box<Expr>), Range(Box<Expr>, Box<Expr>),
    Tuple(Vec<Expr>), Array(Vec<Expr>),
    While(Box<Expr>, Block), For(Box<Pat>, Box<Expr>, Block),
    Loop(Block), Break, Continue, Return(Option<Box<Expr>>),
}
pub enum Lit { Int(i64), Float(f64), Bool(bool), Char(char), Str(StringLit) }
pub struct StringLit { pub parts: Vec<StringPart>, pub span: Span }
pub enum StringPart { Text(String), Expr(Expr) }
pub struct Path { pub segments: Vec<Ident>, pub span: Span }
pub struct Arm { pub pat: Pat, pub body: Expr, pub span: Span }
pub enum UnOp { Neg, Not }
pub enum BinOp { Add, Sub, Mul, Div, Rem, Eq, Ne, Lt, Le, Gt, Ge, And, Or }
pub enum AssignOp { Assign, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign }

pub struct Pat { pub kind: PatKind, pub span: Span }
pub enum PatKind { Wildcard, Binding(Ident), Lit(Lit),
                   Path { path: Path, args: Option<Vec<Pat>> },
                   Tuple(Vec<Pat>) }

pub struct Type { pub kind: TypeKind, pub span: Span }
pub enum TypeKind { Path(PathType), Tuple(Vec<Type>), Slice(Box<Type>),
                    Array(Box<Type>, Box<Expr>),
                    Fn { params: Vec<Type>, ret: Option<Box<Type>> } }
pub struct PathType { pub segments: Vec<Ident>,
                      pub generics: Option<Vec<Type>>, pub span: Span }
```

Structural requirements vs open implementation details:

- **Contract:** node kinds and their payloads must be able to represent every
  derivation of the grammar; spans per §3; `Ident` carries name + span;
  string literals carry `StringPart`s; assignments carry an explicit
  `AssignOp`.
- **Implementation detail (decidable later):** exact field names, whether
  `Option<Generics>` versus empty `Generics`, symbol representation, and
  pretty `Debug` formatting. These do not affect the grammar contract.

### 2.3 Span propagation rules

1. A node's span is `[first_token.lo, last_token.hi)` of the tokens consumed
   for that node — exact, never synthesized beyond §2.3(3).
2. List containers (block, argument list, fields) take the span of their
   delimiters; each element keeps its own span.
3. **Recovered/missing nodes** get a zero-width span at the current token's
   `lo` (insertion point). This is the only synthesized span and is
   deterministic.
4. Expression nodes record the operator token span inside the node? Not
   required by the contract (operators are implied by the variant); if a
   later diagnostic wants operator spans, it is an additive change.
5. The `Str` token's segments already carry absolute spans; the parser
   re-uses them for `StringPart::Expr` sub-expressions (no span recompute).

## 3. Parsing rules (normative)

All grammar rules are in `grammar-rfc.md` §3; this section fixes the
procedural rules the implementation must follow. All are PROPOSED unless
marked.

1. **Top level:** parse items until `Eof`; recover per §4.
2. **Statement parsing / semicolon rule** (DERIVED, grammar-rfc §8.4):
   after parsing an expression in statement position:
   - if `;` follows → consume, statement complete;
   - else if the expression is block-like (`If`/`Match`/`While`/`For`/
     `Loop`/`Block`) → statement complete without `;`;
   - else if `}` follows → the expression is the block's tail;
   - else → diagnostic `expected ';'` (primary span at the next token) and
     recovery per §4.
3. **Generic closer** (PROPOSED, packet G-3): in type-argument context,
   closing consumes `Gt`; if a `Ge` token remains and the run still needs
   closers, `Ge` is split once into `>` (consumed) plus a synthesized `=`
   for the continuation. Never applied outside type-argument closing.
   Examples: `Vec<Vec<T>>` (two `Gt`), `let x: Vec<T>= v;` (`Ge` split),
   `a >= b` (expression: untouched).
4. **Patterns:** the parser records `Path { args: None }` for a bare
   uppercase-or-`::` path and `Binding` for a lone identifier — the
   Name-resolution disambiguation is Phase 2's job; the syntax-level rule is
   only: `IDENT` alone → binding candidate; `path "(" … ")"` → constructor
   with args; `path` containing `::` → path pattern (grammar-rfc §3.5).
5. **Assignment target check** (PROPOSED, PD-003): after parsing an
   assignment's LHS, validate the shape (Path | Field | Index); otherwise
   diagnostic `invalid assignment target` at the LHS span; the node is still
   built with the offending expression for recovery.
6. **Non-associative tiers** (PROPOSED, PD-005): the Pratt loop detects a
   second operator of the same non-assoc tier and emits
   `<tier> operators cannot be chained; use parentheses`, recovering with
   the deterministic shapes in grammar-rfc §4.1.
7. **`return` operand** extends maximally: parse the operand at the full
   expression level; `return` without operand is `Return(None)`.
8. **Interpolated strings:** the parser converts each embedded token
   sequence into an `Expr` via the ordinary expression entry point; an
   empty sequence yields the `empty interpolation` diagnostic and an
   `Expr` placeholder (zero-width span).
9. **No implicit semicolons / no newline sensitivity:** newlines are
   trivia; statement boundaries come only from `;`, block-like status, and
   `}` (deterministic; avoids ASI-style ambiguity).
10. **Items as statements** parse without `;`; a stray `;` after an item is
    a diagnostic (no empty statements, grammar-rfc §3.2).

## 4. Error recovery contract

- **Guarantees** (PROPOSED): (a) parsing terminates on every token stream;
  (b) at least one token is consumed per recovery cycle (progress);
  (c) each top-level item yields at most one recovery cascade — multiple
  errors per run are expected and encouraged (SPECIFIED Phase 1 requirement
  to report several errors per execution).
- **Synchronization sets:**
  - item level: `;`, `}`, `fn`, `struct`, `enum`, `impl`, `trait`, `import`,
    `pub`, `Eof`;
  - statement level: `;`, `}` (leave `}` for the block), item keywords;
  - expression level: stop at `,`, `;`, `}`, `)`, `]`, `=>`, `Eof`
    (delimiters are never consumed by expression recovery).
- **Recovery shape:** the failing construct is closed with a placeholder
  (zero-width span per §2.3(3)) so callers always observe a complete tree;
  diagnostics carry the unexpected token as primary span.
- **No type checking during recovery** (and none anywhere in Phase 1);
  recovery never needs semantic information.
- **Error tokens:** `TokenKind::Error` is a synchronizing token; the parser
  skips it with a diagnostic if no better classification exists.

## 5. Canonical printer and round-trip strategy (future tests)

Printer contract (PROPOSED — this is *not* the Phase 8 formatter):

1. Pure function `print(&Item) -> String` (plus helpers), deterministic,
   no environment input.
2. **AST-preserving:** inserting parentheses where required by §4.1
   precedence/associativity so that `parse(print(ast)) == ast` for the
   compared fields (spans excluded — printing regenerates spans).
3. Canonical formatting rules (minimal, stable): four-space indentation;
   one statement per line; single spaces around binary/assignment
   operators; no space inside `()`/`[]`; `, ` between list elements;
   no trailing commas emitted; blocks always braced; `else` on the same
   line as the closing brace; string literals re-escaped from decoded
   values; interpolation re-printed as `{expr}`.
4. Comments are not represented in the AST and therefore not printed
   (round-trip equality is AST equality, not text identity) — stated here
   so the test criterion is exact.
5. The printer output is a debugging artifact for `xc parse`; pretty
   formatting policy belongs to the Phase 8 formatter.

Test strategy (implementation phase; SPECIFIED Phase 1 test list):

- **Snapshots (insta):** tokens and ASTs for the conformance corpus
  (`conformance-plan.md`), asserting exact `Debug`/printer output so drift
  fails loudly.
- **Round-trip property (proptest):** `parse(print(ast)) == ast` for
  generated ASTs and for `ast(print(ast(src)))` over the corpus.
- **Fuzz (cargo-fuzz):** lexer-parser pipeline over arbitrary text: no
  panics, no unbounded recursion, progress guarantee holds, diagnostics
  well-formed (valid spans per `xl-span` oracles).
- None of these tests exist yet; nothing in this document claims executed
  results.

## 6. `xc parse` — future CLI contract (Phase 1 implementation, not this
directive)

- `xc parse <file.x>`: lex + parse, print the canonical AST to stdout;
  render diagnostics to stderr via `xl-diag`.
- Exit codes (PROPOSED): `0` = no error-level diagnostics; `1` = errors
  (warnings/notes do not fail).
- Optional flags deferred (e.g. `--tokens`, JSON output) — not part of the
  Phase 1 acceptance evidence; acceptance is `xc parse arquivo.x` printing
  the AST (SPECIFIED milestone, §6 Phase 1).

## 7. Consistency checklist

- Every `ExprKind` variant maps to a grammar production in
  `grammar-rfc.md` §3; no variant exists without a production.
- Every grammar production has a node representation; no node invents
  semantics beyond syntax.
- Recovery sets contain only existing tokens; no dependency on lexer
  internals.
- No Phase 2+ concern (resolution, typing, desugaring, exhaustiveness) is
  referenced as a Phase 1 behavior.
