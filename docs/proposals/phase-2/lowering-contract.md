# XLANG Phase 2 — Lowering and Desugaring Contract

**STATUS: PROPOSED — PENDING OWNER RATIFICATION**

- **Directive:** Engineering Directive 010 §7 (AST→HIR lowering and the
  required desugaring designs).
- **Companion documents:** `hir-contract.md` (representation),
  `name-resolution-contract.md` (scopes and def collection),
  `type-system-contract.md` (typing of lowered forms),
  `traits-and-exhaustiveness.md` (refutability reuse),
  `decision-packet.md` (`P2D-B5`, `P2D-B8`, `P2D-B9`, `P2D-B10`,
  `P2D-B18`, `P2D-B22`).
- **No implementation is authorized.** All rules below are PROPOSED
  unless labeled otherwise.

## 1. Lowering pipeline (order of operations)

Lowering a file proceeds in a fixed, deterministic order:

1. **Module scan** — collect module-level items in source order into
   definitions (two-pass: collect names first, then lower bodies, so
   module items are order-independent; `name-resolution-contract.md` §5).
2. **Import registration** — lower `ImportDecl`s, create module bindings,
   ask the module registry about each import target (`P2D-B7`).
3. **Signature lowering** — lower generics, parameters, and return types
   of every function/method/trait item into `TypeRef`s; lower struct
   fields and enum variants/payloads; lower impl headers.
4. **Body lowering** — for each function/method body (source order):
   create the body's scope chain, lower statements and the tail, resolving
   every name use against the scope tables as it goes (sequential
   visibility for locals; hoisted items per `P2D-B3`, §3.2), and apply the
   three desugarings (§4–6) at the point of the originating construct.
5. **Validation** — run the structural HIR validator on the result
   (`hir-contract.md` §9).

Lowering never reorders user code except where a desugaring specifies it;
diagnostics from all steps are emitted in this order.

## 2. Identity and span rules for lowered nodes

- Every HIR node produced from exactly one AST node inherits its span
  verbatim.
- **Synthesized nodes** (desugaring temporaries, desugared control flow,
  formatting wrappers) carry the span of the originating construct and an
  `Origin::Desugared { kind }` flag (`P2D-A2`). Example: every node of the
  `for` expansion carries the `for` expression's span, so a type error
  inside the desugared loop still points at the whole `for` statement
  (and diagnostics inside `BODY` — which has real spans — point at user
  text, not the expansion).
- Internal temporaries (`__state`, `__item`, `__next`, `__v`, `__e`) are
  generated as **fresh `LocalId`s**, never as strings inserted into scopes.
  They are hygienic by construction: user code cannot name them, shadow
  them, or observe them (`P2D-A2`; this is why desugaring is specified in
  HIR, which has identity-based bindings, rather than in the AST).
- Internal operations (`RangeNew`, `RangeNext`, `StrConcat`,
  `FormatPrim` — `BuiltinCall` nodes) are **compiler-owned built-ins**,
  not resolved through user scopes. The desugared `?` lowering instead
  references the prelude `Result` constructors: its `Ok`/`Err` patterns
  and the `Err` callee are synthesized uses **pre-resolved** to the
  prelude definitions (`hir-contract.md` §5.4). A user item named `Err`
  or `Ok` (shadowing the prelude) cannot capture the desugared `?`
  lowering in either case, because nothing in the expansion is resolved
  by name (`P2D-B9`).

## 3. The complete lowering table

### 3.1 Items

| AST | HIR result | Notes |
| --- | --- | --- |
| `ItemKind::Import(ImportDecl)` | import record + module binding `DefId` (kind `ModuleBinding`) | path segments as uses; alias or last segment binds; `P2D-B6`, `P2D-B7` |
| `ItemKind::Fn(FnDecl)` | `Def { kind: Fn, generics, sig, body: Some }` | `pub` recorded |
| `ItemKind::Struct` | `Def { kind: Struct, generics }` + field `Def`s | field `pub` recorded; no constructor syntax exists (G-5 UNRESOLVED) |
| `ItemKind::Enum` | `Def { kind: Enum, generics }` + variant `Def`s | payload `TypeRef`s; variant order preserved (exhaustiveness rendering order) |
| `ItemKind::Trait` | `Def { kind: Trait, generics }` + trait-item `Def`s | signature-only items (accepted grammar) |
| `ItemKind::Impl` | `Def { kind: Impl, generics, trait_ref: Option, self_ty }` + method `Def`s | `is_inherent = trait_.is_none()` from syntax; method selection is WP-C |

### 3.2 Statements, blocks, patterns

| AST | HIR result | Notes |
| --- | --- | --- |
| `StmtKind::Let(LetStmt)` | `StmtKind::Let { local(s), init }` | annotation → `Local.ty`; `mutable` → `Local.mutable` for **every** binding the pattern introduces (`P2D-B17`); init lowered with the pattern's uses recorded |
| `StmtKind::Expr(e)` | `StmtKind::Expr(hir)` | value discarded (convention unchanged) |
| `StmtKind::Item(item)` | `StmtKind::Item` | nested items: hoisted inside their block (`P2D-B3`) |
| `Block { stmts, tail }` | `BodyId` with `stmts` + `tail` | tail/value split preserved by construction (SPECIFIED) |
| `PatKind::Wildcard` | `PatKind::Wildcard` | binds nothing |
| `PatKind::Binding(ident)` | `PatKind::Binding { local }` **or** `PatKind::Variant { variant }` for a unit-variant name | final classification is semantic (`P2D-B5`); lowering records a *binding candidate* use and resolution decides (`name-resolution-contract.md` §7.4) |
| `PatKind::Lit(lit)` | `PatKind::Lit` | literal payloads copied with magnitude/radix |
| `PatKind::Path { path, args }` | `PatKind::Variant { variant: use, args }` | always a constructor candidate (path with `::` and/or args) |
| `PatKind::Tuple(pats)` | `PatKind::Tuple` | unit pattern `()` = empty tuple |

### 3.3 Expressions (accepted syntax → HIR)

| AST `ExprKind` | HIR result | Notes |
| --- | --- | --- |
| `Lit(Int/Float/Bool/Char)` | `Lit` | magnitude/radix preserved (B-2); floats finite (B-3) |
| `Lit(Str)` all-literal | `Lit(Str)` | decoded text |
| `Lit(Str)` with interpolation | `BuiltinCall { op: StrConcat / FormatPrim, … }` expansion | §6 |
| `Path` | `Path(UseId)` | one use per segment; `Enum::Variant` resolved in WP-B |
| `Unary`, `Binary` | same kinds | grouping already transparent in the AST |
| `Call(callee, args)` | `Call(callee_hir, args_hir)` | arity/type checking in WP-C; built-in calls have no source syntax (see §2 hygiene rule) |
| `Field(base, name)` | `Field(base_hir, FieldSel::Unresolved { name })` | module member vs field vs method decided in WP-C (`P2D-A3`) |
| `Index(base, index)` | `Index` | typing restricted per `P2D-B18` |
| `Assign(op, lhs, rhs)` | `Assign` | target shape was validated at parse time (accepted B-1); legality (mutability) is WP-C |
| `Range(a, b)` | `BuiltinCall { op: RangeNew, args: [a, b] }` | internal built-in, `P2D-B8`; `Range` is not nameable |
| `Cast(e, ty)` | `Cast(e, TypeRef)` | conversions per `P2D-B12` |
| `Try(e)` | desugared match | §5 |
| `If(c, b, e)` | `If(c, BodyId, Option<HirId>)` | else-if nests naturally |
| `Match(s, arms)` | `Match` | arms in source order (exhaustiveness order) |
| `Block(b)` | `Block(BodyId)` | |
| `While(c, b)` | `While(c, BodyId)` | |
| `For(p, s, b)` | desugared loop | §4 |
| `Loop(b)` | `Loop(BodyId)` | |
| `Return(e)` | `Return(Option<HirId>)` | |
| `Break` / `Continue` | `Break/Continue { target }` | `target` resolved structurally to the nearest enclosing lowering-time loop (never across a body boundary, because no closures exist) |
| `Tuple` / `Array` | `Tuple` / `Array` | unit = empty tuple; empty array allowed syntactically (typing per `P2D-B18`) |
| recovered placeholder | `ExprKind::Error` | never diagnosable again (poison; `hir-contract.md` §7) |

### 3.4 Types

| AST `TypeKind` | HIR `TypeRefKind` | Notes |
| --- | --- | --- |
| `Path(PathType)` | `Path { segments: uses, generics }` | each segment a use; generics lowered recursively |
| `Tuple` | `Tuple` | `()` = empty tuple |
| `Slice` | `Slice` | `[T]` nameable; no Phase 2 expression has slice type (`P2D-B18`) |
| `Array(elem, len_expr)` | `Array { elem, len }` | `len` is a lowered expression, checked per `P2D-B18` |
| `Fn { params, ret }` | `Fn` | optional return (unit when absent) |
| name `Self` in trait/impl context | `SelfTy` | contextual, not a keyword (`P2D-B14`) |

## 4. Desugaring: `for` loops (P2D-B8)

Source (SPECIFIED surface, §3 example):

```xlang
for item in sequence {
    process(item);
}
```

### 4.1 Internal iteration protocol (Phase 2 scope)

The original mandates “`for` → `loop` + iterator” (SPECIFIED §6 Phase 2)
but the language defines no Iterator trait, no associated types, no
`mut self` receivers, and no standard library yet. Phase 2 therefore
defines a **compiler-owned, internal protocol**, not a user-visible
trait:

- Internal type `Range<T>` (not nameable in source; produced only by the
  `..` operator lowering). `T` is an integer primitive (`i8`…`i64`,
  `u8`…`u64`, `isize`, `usize`).
- `range_new(a: T, b: T) -> Range<T>` — built-in; evaluates `a` then `b`,
  each exactly once.
- `range_next(r: Range<T>) -> Option<(T, Range<T>)>` — built-in, internal
  semantics: `None` when `r.start >= r.end` (empty or exhausted);
  otherwise `Some((r.start, Range { start: start + 1, end }))`.
  **No overflow is possible:** this branch requires `start < end`, so
  `start + 1 <= end <= T::MAX` for any integer `T` (DERIVED invariant).

The protocol is *internal*: `range_new`/`range_next` cannot be named or
called by user code and are not subject to shadowing (§2). When Phase 5/6
introduce traits and the standard library, this internal protocol is the
seam to be replaced by (or exposed as) the real iteration traits under a
future owner decision (Category C).

**Phase 2 iterable set:** exactly `Range<T>` from `a..b` expressions.
Iterating arrays, structs, or user types is **FUTURE** (owner decision
`P2D-B8`; the alternative — adding arrays now — is recorded in the
packet).

### 4.2 Lowering

```xlang
for pat in seq { BODY }
```

lowers to the following structure (all synthesized nodes carry the `for`
expression's span unless they wrap user code):

```text
{
    let mut __state = ⟨seq⟩;                 // type Range<T>; seq evaluated exactly once
    loop {
        match range_next(__state) {
            Some(__pair) => {                // prelude Option::Some (internal reference)
                let (__item, __next) = __pair;
                let pat = __item;            // user pattern binds here (fresh scope per iteration)
                __state = __next;            // state advanced BEFORE BODY
                ⟨BODY⟩                       // body statements; tail value discarded
            }
            None => break,
        }
    }
}
```

Contract points:

1. **Evaluation order.** `seq` is evaluated exactly once, before the
   first `range_next` call. Each iteration evaluates `range_next(__state)`
   once. `pat` is bound before `BODY` runs. Nothing else is evaluated.
2. **Temporary bindings.** `__state` is a fresh mutable internal local
   (identity-based; §2). `__pair`, `__item`, `__next` are fresh internal
   locals; `__next` is assigned back into `__state` before `BODY`.
3. **Scope.** `pat`'s bindings are visible exactly inside `BODY` and are
   fresh for each iteration concept (no leakage after the loop, no
   cross-iteration state other than `__state`, which user code cannot
   name). The loop's expanded block introduces one scope; `BODY`'s own
   block scope nests inside it.
4. **`break`/`continue`/`return`.** Because the state update precedes
   `BODY`, `continue` is correct: the next iteration observes the advanced
   state. `break` exits the desugared `loop` (target recorded at lowering,
   §3.3). `return` exits the function. No value semantics are invented:
   `for` is an expression with type `()` (`type-system-contract.md` §10).
5. **Error behavior.** If `seq` is an error node, the whole `for` lowers
   to `Error` (poison). If `pat` is an error node, it lowers to
   `Wildcard` with poison marking; no additional diagnostics. If `range_next`
   cannot be typed (non-`Range` operand), exactly one diagnostic is
   emitted at the `for`/operand span by WP-C; the loop still type-checks
   to prevent cascades.
6. **Refutable patterns.** The `let pat = __item;` pattern must be
   irrefutable for the item type; a refutable pattern is an error
   (`P2D-B22`, using the exhaustiveness machinery of
   `traits-and-exhaustiveness.md`). Example: `for Some(x) in 0..10` is
   rejected because the item type `i64` cannot match `Some(x)`.
7. **Diagnostics inside `BODY`** point at real user spans; diagnostics
   about the iteration itself point at the `for` construct or `seq`.

## 5. Desugaring: error propagation `?` (P2D-B9)

Source (SPECIFIED surface, §3 example):

```xlang
let value = operation()?;
```

### 5.1 Lowering

```text
⟨expr⟩?   ⟹   match ⟨expr⟩ {
                  Ok(__v) => __v,
                  Err(__e) => return Err(__e),
              }
```

- `Ok`/`Err` here are **internal references** to the prelude `Result`
  constructors, not name lookups: shadowing `Ok`/`Err` in user code
  cannot capture the expansion (§2 hygiene).
- `⟨expr⟩` is evaluated exactly once. `__v`/`__e` are fresh internal
  locals.
- The `?` expression's type is the payload type of `Ok`.

### 5.2 Typing rules (checked in WP-C)

1. The operand type must unify with `Result<T, E>` for fresh `T`, `E`;
   otherwise: `the '?' operator requires a 'Result<T, E>' operand, found
   'X'` (a `?` on `Option` is **not** introduced — UNRESOLVED/FUTURE).
2. The `?` expression has type `T`.
3. The enclosing function's declared return type must unify with
   `Result<T1, E1>` with **`E ≡ E1` exactly** — no implicit conversions,
   no `From`-style conversion, no exception semantics (SPECIFIED §2:
   errors are values; §7: no implicit conversions). If the enclosing
   return type is not a `Result`, or `E` does not unify with `E1`, the
   diagnostic is emitted at the `?` construct with the enclosing
   function's return type as a secondary label.
4. `?` is never allowed outside a function body (the grammar admits
   `postfix ?` only in expression positions; module-level expressions do
   not exist, so this is unreachable — documented for completeness).
5. Error conversion (e.g. an explicit `map_err`-style mechanism) is
   **FUTURE**, contingent on the standard library and trait machinery
   (Category C). Phase 2 requires exact error-type equality.

### 5.3 Error behavior

- Error operand → whole `?` is `Error` (poison), no diagnostics.
- Mismatched error type → one diagnostic; the `?` expression still
  type-checks as `T` to prevent cascades.

## 6. Desugaring: string interpolation (P2D-B10)

Source (SPECIFIED surface, §3 example):

```xlang
let message = "result = {value}";
```

### 6.1 Semantic representation and lowering

The accepted AST carries `StringLit { parts: StringPart::Text |
StringPart::Expr }` (ratified PD-004). Lowering:

1. Consecutive `Text` parts (and the text around interpolations) are
   merged into single `Lit(Str)` nodes (a `Lit(Str)` with the *decoded*
   value; the accepted lexer already decoded escapes).
2. Each interpolated expression part lowers to
   `BuiltinCall { op: FormatPrim, args: [part] }` **unless** the whole
   string is all-literal text with no interpolations (then the string
   stays a single `Lit(Str)`).
3. All parts are combined with the internal n-ary `str_concat` built-in:
   `BuiltinCall { op: StrConcat, args: [part_0, …, part_n] }`; a string
   with zero parts lowers to `Lit(Str(""))`.
4. Evaluation order: parts are evaluated **left to right**, exactly once,
   in source order. (DERIVED from ordinary expression order and from the
   left-to-right structure of the accepted lexer segments.)

### 6.2 Typing rules (WP-C)

- `StrConcat`: every part must be `str`; result `str`. (Concatenation is
  internal: the `+` operator is **not** defined for `str`; no new operator
  semantics are introduced.)
- `FormatPrim` (one interpolation part):
  - `part: str` → type `str`; identity formatting (no runtime call needed
    after optimization; the HIR node is a marker).
  - `part` of one of the **primitive formatting set** → type `str`. The
    text produced for a value is the proposal below.
  - any other type → error: `cannot format value of type 'X' in a string
    literal`, with a note that formatting for user types arrives with the
    standard library (FUTURE Display machinery, Category C).

**Primitive formatting contract (PROPOSAL, `P2D-B10` — the original is
silent):** for the primitive types `i8`…`i64`, `u8`…`u64`, `isize`,
`usize`, `f32`, `f64`, `bool`, `char`, `str`:

| Type | Textual form |
| --- | --- |
| integer types | decimal digits with a leading `-` for negative values (value-based; literal radix is irrelevant) |
| `bool` | `true` / `false` |
| `char` | the character itself (no quotes, no escapes) |
| `str` | the string’s text unchanged |
| `f32`/`f64` | the shortest decimal representation that round-trips to the same value and is accepted by the Phase 1 float grammar (the same rule the accepted canonical printer uses for float literals, B-3); finite values only |

Rationale: the §3 example interpolates an `f64` (`"total = {total}"`)
and an `i64`-like accumulator, so the intent to format primitives is
evidenced; restricting the set to primitives avoids inventing `Display`
semantics for user types, and reusing the accepted printer’s float rule
avoids a second, conflicting numeric-formatting definition. The owner may
instead choose the conservative alternative (str-only interpolation) or a
different primitive contract — see `P2D-B10` in the decision packet.

### 6.3 Guaranteed Phase 2 behavior vs. future dependencies

- **Guaranteed (Phase 2):** lowering shape, evaluation order, typing of
  `str` parts and the primitive set, diagnostics for non-formatable
  types. Phase 2 checks; it does not run anything.
- **Future (dependencies):** runtime formatting and concatenation
  implementations (Phase 4 VM); `Display`/`Debug` traits and formatting
  of user-defined types (Phase 6); any user-visible `Iterator`/
  `IntoIterator` surface for `for` (Phase 5/6). None of these are assumed
  by the Phase 2 contract.

## 7. Desugaring hygiene and non-goals

- No other desugarings exist in Phase 2. Specifically: `while`/`loop`
  are **not** desugared (the original mandates only `for`), `if` is not
  desugared, operators are not desugared into trait calls (no operator
  traits exist; `P2D-B15`), `return`/`break`/`continue` remain nodes.
- No implicit conversions, temporaries with user-visible names, or
  exception/panic semantics are introduced anywhere in this contract
  (SPECIFIED §2, §7).
- The desugared forms are internal representation; the canonical printer
  and `xc parse` (Phase 1, accepted) are unaffected, and no new source
  syntax is introduced by any desugaring.

## 8. Test obligations (WP-A, for the future implementation)

1. **Golden lowering snapshots:** HIR dumps (deterministic debug format)
   for the accepted conformance corpus, including the §3 example.
2. **Desugaring tests:** for each of the three desugarings, a table of
   source fragments with expected HIR shapes (structure only), covering:
   `for` evaluation order (side-effecting iterator expression? — Phase 2
   has no side-effecting iterables beyond the expression itself; test
   single evaluation via a call expression operand structure), nested
   `for` (target resolution of `break`/`continue`), `?` in nested blocks
   and inside `for` bodies, interpolation with multiple and adjacent
   parts, empty strings, strings that are only interpolation (`"{x}"`).
3. **Hygiene tests:** user code declaring items/locals named `Ok`, `Err`,
   `Some`, `None`, `range_next`, `str_concat` cannot capture or break
   desugared expansions.
4. **Poison tests:** error nodes anywhere in the input produce complete
   HIR with `Error` nodes and no additional lowering diagnostics.
5. **Determinism tests:** lowering twice yields byte-identical HIR dumps.

## 9. Traceability

| Contract item | Source |
| --- | --- |
| `for` → `loop` + iterator; `?` → `match`; interpolation → concatenation | SPECIFIED §6 Phase 2 |
| No exceptions, no implicit conversions; errors as values | SPECIFIED §2, §7 |
| Result<T, E> model; `Ok`/`Err` surface | SPECIFIED §3 example, §6 Phase 5 |
| Range expression `0..10` and iteration in the example | SPECIFIED §3 example |
| String parts/decoding; no Phase 1 desugaring | ratified PD-004; lexer contract §5 |
| Grouping transparency; target validation at parse time | ratified B-1, PD-003 |
| Never panic; complete trees; placeholder discipline | SPECIFIED §4; accepted recovery contract |
| Printer float rule reused for formatting | ratified B-3; parser/AST contract §5(5) |
