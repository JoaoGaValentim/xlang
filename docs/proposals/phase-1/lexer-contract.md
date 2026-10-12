# XLANG Phase 1 — Lexer Contract

**STATUS: PROPOSED — OWNER REVIEW REQUIRED**

- **Directive:** Engineering Directive 004 — Phase 1 Syntax Architecture and
  Specification Gate (2026-10-12)
- **Authoritative sources:** `docs/spec-v0.1-original.txt` §3 (lexical
  notes), §5 (`TokenKind` sketch), and the accepted ADR-0002 (source
  position conventions) plus `crates/xl-span` public contracts.
- **Scope:** the `xl-lexer` contract (source text → token stream). No
  implementation. The lexer must not depend on the parser.
- **Related:** `grammar-rfc.md` (token inventory), `parser-ast-contract.md`
  (consumer contract), `conformance-plan.md` (corpus).

## 1. Crate responsibility and dependencies

`xl-lexer` is a total, deterministic function:

```text
lex(source: &SourceFile) -> LexOutput
LexOutput { tokens: Vec<Token>, diagnostics: Vec<Diagnostic> }
```

- Dependencies: `xl-span` (spans, `FileId`, `SourceFile`) and `xl-diag`
  (diagnostics). No dependency on `xl-parser`, `xl-ast`, or any later crate.
- Contract: **no panic, ever**, for any input `&str` (including invalid
  UTF-8 positions never occur — the input type guarantees UTF-8).
- Determinism: identical input bytes ⇒ identical token stream and identical
  diagnostics (ADR-0002; INV-007). No timestamps, no environment input.
- Byte offsets are UTF-8 byte positions (`u32`), spans are half-open
  `[lo, hi)` (ADR-0002). Every token span covers exactly its lexeme; the
  `Eof` span is zero-width at `source.len()`.
- Identifiers and string data are interned in the compilation session
  (SPECIFIED §4 recommends `lasso`); the exact symbol type is an
  implementation detail of the implementation directive.

## 2. Token stream

- `Token { kind: TokenKind, span: Span }`, token kinds as listed in
  `grammar-rfc.md` §2.
- The stream always ends with exactly one `Eof` token.
- Whitespace and comments are **trivia**: skipped, not emitted (PROPOSED —
  Phase 1's parser does not need trivia; a trivia-preserving mode for the
  Phase 8 formatter is deliberately out of scope and must not be designed
  here).
- The lexer emits `Error` tokens (see §8) so that the parser can recover
  without special casing raw bytes.

## 3. Identifiers, keywords, and `_`

1. Identifier start: `XID_Start` (UAX #31) or `_`; continue: `XID_Continue`
   (SPECIFIED "Identificadores Unicode (XID)"; the `_` start extension is
   PROPOSED for names such as `_unused`).
2. A maximal identifier run equal to `_` alone is the `Underscore` token
   (DERIVED: wildcard patterns, `pattern = "_" | …`).
3. Keyword recognition: exact match against the §3 keyword list
   (`grammar-rfc.md` §2.1); keywords are case-sensitive; `self` is **not** a
   keyword (SPECIFIED omission).
4. Unicode normalization is **not** applied: identifiers compare by scalar
   sequence (deterministic; normalization policy is UNRESOLVED and would be
   a specification change).
5. Non-XID letters (e.g. `①`, emoji) cannot start or continue identifiers;
   they are illegal-character diagnostics.

## 4. Numeric literals

Underscore placement everywhere: `_` must be **between digits**
(`1_000`, `0xFF_FF`, `1_0.5_5`). Violations (`_1`, `1_`, `0x_1`, `1__0`)
are malformed-literal diagnostics.

### 4.1 Integers

```text
int        = dec_int | hex_int | bin_int ;
dec_int    = dec_digit { dec_digit | "_" dec_digit } ;
dec_digit  = "0"…"9" ;
hex_int    = ("0x" | "0X") hex_digit { hex_digit | "_" hex_digit } ;
bin_int    = ("0b" | "0B") bin_digit { bin_digit | "_" bin_digit } ;
```

- SPECIFIED exemplars: `1_000`, `0xFF`, `0b1010`. Lower- and upper-case
  prefixes and hex digits are accepted (PROPOSED, low risk).
- Leading zeros are permitted in decimal (`007` = 7) — PROPOSED (no rule in
  the original against them).
- Value semantics: the literal's value must fit `i64` (SPECIFIED type list;
  default integer type is `i64`, §7). Out-of-range values are a diagnostic
  (`integer literal out of range for i64`), reported with the literal span;
  the token is still produced with a saturated/`0` placeholder value for
  recovery determinism.
- No suffixes (`1i64`, `1u8`) and no octal literals (not evidenced;
  UNRESOLVED).

### 4.2 Floats

```text
float      = dec_int "." dec_int [ exponent ]
           | dec_int exponent ;
exponent   = ("e" | "E") [ "+" | "-" ] dec_int ;
```

- `3.14159`, `0.0`, `1.5e3`, `1e5` are floats (PROPOSED forms; the category
  "floats" is SPECIFIED).
- `1.` and `.5` are **not** floats: `1.` lexes as `Int(1)` then `.`
  (field-access token); `.5` lexes as `.` then `Int(5)` (a parse error
  later). Rationale: keeps `1..2` (range) unambiguous; see §6.
- Value semantics: parsed as `f64` (SPECIFIED default float type). Overflow
  to infinity is accepted and yields `f64::INFINITY` (PROPOSED; no
  diagnostic — IEEE semantics documented; the exact policy is trivially
  changeable and noted for the owner).

## 5. Strings, characters, escapes, and interpolation (PD-004)

### 5.1 Character literals

```text
char    = "'" ( char_escape | char_value ) "'" ;
char_value = any Unicode scalar value except "'" "\\" CR LF ;
char_escape = "\\" ( "n" | "r" | "t" | "\\" | "'" | "\"" | "0" | unicode_escape ) ;
unicode_escape = "u{" hex_digit{1,6} "}"   (* value ≤ 0x10FFFF, no surrogates *)
```

- `'a'`, `'é'`, `'🦀'`, `'\n'`, `'\u{1F980}'` (PROPOSED escape set;
  "char" and "strings com escapes" are SPECIFIED).
- `''` (empty) and `'ab'` (multiple scalars) are diagnostics
  (`a character literal must contain exactly one character`).
- A raw CR/LF inside a character literal is an unterminated-literal
  diagnostic.

### 5.2 String literal structure and interpolation

The lexer scans the complete string literal and produces **one `Str` token**
whose payload is a structured sequence of segments. This is the PD-004
recommendation; alternatives and comparison are in §5.6.

```text
string      = '"' { text_segment | interp_segment } '"' ;
text_segment = { text_char | text_escape } ;        (* zero or more, non-empty when emitted *)
text_char   = any Unicode scalar except '"' "\\" "{" CR LF ;
text_escape = "\\" ( "n" | "r" | "t" | "\\" | "'" | "\"" | "0" | "{" | "}" | unicode_escape ) ;
interp_segment = "{" token_sequence "}" ;           (* brace-balanced, see below *)
```

Rules (PROPOSED):

1. **Escapes are decoded by the lexer**; text segments carry the decoded
   value. The parser and AST never see backslash escapes.
2. **`{` begins an interpolation.** To write a literal `{` use `\{`; `\}`
   is also accepted (both decode to the brace). A lone `}` in text is
   accepted as literal text (it cannot close anything).
3. **Embedded expressions are lexed by the lexer itself**, recursively, in
   normal token mode, producing a token sequence with **absolute source
   spans**. Nesting works naturally because embedded strings, characters,
   and comments are consumed as their own tokens. The interpolation is
   closed by the `}` that returns brace depth to zero, where depth counts
   `{`/`}` tokens of the embedded sequence (a nested none appears inside
   embedded strings/comments, which are single tokens).
4. **Empty interpolation** (`"{}"`): the lexer produces an interpolation
   segment with an empty token sequence; the **parser** reports
   `empty interpolation` at the segment span (syntax-level diagnostic; keeps
   lexical/syntactic error ownership clean).
5. **Unterminated interpolation** (EOF before the closing `}`): diagnostic
   `unterminated interpolation in string literal` with the primary span from
   the opening `{` to the recovery point; the string token terminates at
   EOF with an `Error` segment; lexing ends with `Eof`.
6. **Unterminated string** (raw CR/LF or EOF before the closing `"`):
   diagnostic `unterminated string literal`; the recovery point is the
   newline (not consumed) or EOF; an `Error` token covers `"` to the
   recovery point.
7. **Invalid escape** (`\q`): diagnostic `unknown escape sequence \q`;
   recovery: the escaped character is taken literally; lexing continues
   within the string (this enables multiple escape errors per run).
8. **Strings are single-line**: raw CR/LF inside a string is the
   unterminated-string case (PROPOSED; multi-line string syntax is
   UNRESOLVED and not introduced).
9. **Spans:** the `Str` token spans the full literal including both quotes.
   Every text segment and every embedded token carries its precise absolute
   span (byte-exact, ADR-0002), so diagnostics inside interpolations point
   at exact source text.
10. **AST relationship:** the parser builds
    `Lit(Str(StringLit))`, `StringLit { parts: Vec<StringPart> }`,
    `StringPart::Text(String) | StringPart::Expr(Expr)`
    (`parser-ast-contract.md` §4). Phase 2 desugars interpolation to
    concatenation (SPECIFIED §6 Phase 2). Phase 1 performs no desugaring.
11. **Depth safety:** interpolation nesting is bounded by the source
    length; the implementation must avoid unbounded recursion (or prove
    stack safety under fuzzing). Implementation note only — no semantic
    limit is proposed (a hard limit would be a specification change).

### 5.3 Permitted example forms

| Source | Result |
| --- | --- |
| `"total = {total}"` | text `total = ` + expr `total` (SPECIFIED example) |
| `"a\nb"` | text with decoded newline |
| `"literal \{brace\}"` | text `literal {brace}` |
| `"{f(1)}"` | expr `f(1)` (call inside interpolation) |
| `"{ if x { 1 } else { 2 } }"` | expr with nested blocks (brace depth) |
| `"{}"` | parser diagnostic: empty interpolation |
| `"oops` | lexer diagnostic: unterminated string literal |

### 5.4 Alternatives considered (recommendation rationale)

- **Raw `Str(Symbol)` + parser re-lex of sub-ranges:** rejected — the
  parser would need lexer APIs and would duplicate escape/interpolation
  handling; error ownership blurs.
- **Flat mode-switched token stream (`StrStart`, `StrText`,
  `ExprStart`, …, `StrEnd`):** rejected — the parser must reassemble
  segments with extra state, recovery boundaries multiply, and token
  snapshots become noisy.
- **Structured `Str` token with embedded token sequences (recommended):**
  one token in, one AST node out; spans are absolute and correct; lexer
  stays parser-independent. Cost: a recursive lexer entry point, bounded by
  input size. Chosen.

## 6. Maximal munch and disambiguation rules

1. Longest match applies to operators: `==`, `!=`, `<=`, `>=`, `&&`, `||`,
   `->`, `=>`, `::`, `..`, `+=`, `-=`, `*=`, `/=`, `%=` are single tokens;
   there are no shift tokens, so `>>` is always `>` `>`.
2. Dot rules (PROPOSED, deterministic):
   - A `.` immediately followed by a digit **and** preceded by a numeric
     digit already consumed as part of a number is part of a float
     (`1.5`).
   - `..` is the range token whenever two dots are adjacent and the first is
     not consumed by rule 1 (`1..2` → `Int(1) DotDot Int(2)`; `a..b`).
   - Otherwise `.` is the field token (`x.y`, `1.foo` → `Int . IDENT`).
   - Equivalences: `1..2` ≠ `1. .2` (the latter is `Int Dot Dot?` — not a
     float; deterministic error later); `.5` is never a float.
3. `<`/`>`: single tokens; `>=`/`<=` are two-character tokens. The parser
   (not the lexer) resolves generic closing (§8.7 of `grammar-rfc.md`).
4. `//` starts a line comment only when not inside a string/char; `/*`
   likewise.

## 7. Comments and whitespace

- Line comment: `//` through end of line (the newline is not part of the
  comment); skipped.
- Block comment: `/* … */` with **nesting** (SPECIFIED §3: "comentários
  `//` e `/* */` aninhados"); nesting depth is tracked; an unterminated
  block comment is a diagnostic (`unterminated block comment`) whose
  primary span runs from the outermost opening `/*` to EOF.
- Whitespace: space (U+0020), tab (U+0009), carriage return (U+000D), and
  line feed (U+000A) — skipped. A single optional U+FEFF byte-order mark at
  the very start of the file is skipped (PROPOSED, low risk). Any other
  Unicode whitespace (e.g. U+00A0) is an illegal-character diagnostic
  (PROPOSED strictness; extendable later).
- Line endings: `\n` and `\r\n` both end lines; `\r` alone is treated as
  whitespace, not a line terminator (ADR-0002 line semantics remain the
  source of truth for positions).

## 8. Error model and diagnostics

- The lexer never aborts: on a lexical error it emits **one diagnostic**
  and continues from a defined recovery point, so a single run reports
  multiple errors (SPECIFIED Phase 1: "recuperação de erros … para reportar
  vários erros por execução").
- Recovery token: `Error` (span covers the offending text; the parser
  treats it as a synchronizing token, never as a valid construct).
- Diagnostics use `xl-diag` (`Diagnostic::error` + primary label); message
  texts are stable strings locked by snapshot tests (`insta`, SPECIFIED
  Phase 1 test plan). Stable numeric error codes are a PD-010 question and
  are **not** introduced.

Proposed diagnostic catalog (message template / primary span / recovery):

| Condition | Message (stable text) | Primary span | Recovery |
| --- | --- | --- | --- |
| Illegal character | `unexpected character '<c>'` | the character | skip 1 scalar |
| Unterminated string | `unterminated string literal` | `"` → line end/EOF | emit `Error`, resume at newline/EOF |
| Invalid string escape | `unknown escape sequence '\<c>'` | the escape | treat `<c>` literally, continue |
| Unterminated interpolation | `unterminated interpolation in string literal` | `{` → EOF | emit `Error`, resume at EOF |
| Unterminated block comment | `unterminated block comment` | outer `/*` → EOF | resume at EOF |
| Malformed integer (e.g. `0x`) | `expected digits after '0x'` | the prefix | emit `Error`, resume after prefix |
| Misplaced underscore | `'_' must appear between digits` | offending `_` | emit `Error`, resume at next non-digit |
| Integer overflow | `integer literal out of range for i64` | literal | value 0 placeholder |
| Empty/multi char literal | `a character literal must contain exactly one character` | literal | emit `Error` |

## 9. Fuzzing and test contract (for the implementation phase)

- Lexer fuzz target: arbitrary bytes (valid UTF-8 `&str` unify) must never
  panic, never loop forever, and always emit exactly one trailing `Eof`;
  concatenated token spans must cover the input minus trivia; every token
  span must lie on UTF-8 boundaries (`xl-span` validation is the oracle).
- Snapshot tests: token streams (kinds + spans) and diagnostics for the
  conformance corpus (`conformance-plan.md`).
- Property: `lex` is deterministic — two runs yield identical output.
- No parser dependency is permitted, including in tests (the lexer test
  suite uses tokens only).

## 10. Explicitly out of scope

- No trivia-preserving API (future formatter; separate decision).
- No symbol-interning policy decision beyond the §1 note.
- No semantic validation of interpolation contents (that is parsing).
- No normalization, no line/column computation (ADR-0002 owns positions;
  `xl-span` computes columns).
