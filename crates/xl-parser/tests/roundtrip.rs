//! Suite H: canonical round-trip verification
//! (`parse(print(ast)) ≡ ast` under the accepted equivalence relation, and
//! `parse(print(parse(src))) ≡ parse(src)` for the corpus).

mod common;

use common::{SPEC_EXAMPLE, clean, items_equivalent};

/// The accepted round-trip corpus: specification example plus every
/// approved conformance category (nested expressions, declarations,
/// blocks/tails, generics, matches, tuples/arrays, escaped strings,
/// interpolation, integer boundaries, floats, grouped non-associative
/// expressions).
const CORPUS: &[&str] = &[
    SPEC_EXAMPLE,
    // Nested expressions and grouping.
    "fn f() { let x = a - (b - c); }",
    "fn f() { let x = (a + b) * (c - d); }",
    "fn f() { let x = ((a)); }",
    "fn f() { let x = -a + !b; }",
    // Cast tails immediately before `<` (printer integration fix,
    // Directive 008 §15): `as` types must not absorb the comparison.
    "fn f() { let x = (a * a as i64) < a; }",
    "fn f() { let x = (a as i64) < b; }",
    "fn f() { let x = (a + b as i64) < c; }",
    "fn f() { let x = (-a as i64) < b; }",
    // Block-like statements immediately before `(`/`[` continuations
    // (printer termination fix, Directive 008 fuzz regression).
    "fn area(s: Shape) -> f64 { 0.0; for i in 0..10 { total += i as f64 }; (f64, f64) }",
    "fn f() { while a {}; [1, 2] }",
    "fn f() { if a { } (x, y); }",
    "fn f() { { g(); }; (1, 2) }",
    "fn f() { loop { break }; [first, second] }",
    // Grouped non-associative operations.
    "fn f() { let x = (a < b) < c; }",
    "fn f() { let x = (a..b)..c; }",
    "fn f() { let x = a = (b = c); }",
    // Function declarations, blocks, and tails.
    "fn f() -> i64 { if a { 1 } else if b { 2 } else { 3 } }",
    "fn f() -> i64 { match v { 0 => 10, _ => 20, } }",
    "fn f() { g(); h() }",
    "fn f() { for i in 0..10 { p(i); } finish(); }",
    "fn f() -> i64 { g() }",
    "fn f() { while a { b(); } loop { break; } continue; }",
    "fn f() { return; } fn g() -> i64 { return n < 0 }",
    // Generics and types.
    "fn f(x: Vec<Vec<i64>>) -> Result<i64, str> { x }",
    "struct S<T: Ord> { pub f: [i64; 4], g: fn(i64, bool) -> (i64, i64) }",
    "enum E { A, B(i64), C(i64, bool) }",
    "trait Show { fn show(self) -> str; } impl Show for Point { pub fn show(self) -> str { \"s\" } }",
    "impl<T> W<T> { fn get(self) -> T { self.v } }",
    // Tuples and arrays.
    "fn f() { let t = (1, 2.5); let u = (); let a = [1, 2, 3]; let e = []; }",
    // Strings, escapes, interpolation.
    "fn f() { let s = \"a\\nb\" }",
    "fn f() { let s = \"literal \\{brace\\}\" }",
    "fn f() { let s = \"value = {compute(1 + 2)}\" }",
    "fn f() { let s = \"a { \"b {c} d\" } e\" }",
    // Literal boundaries.
    "fn f() { let a = 0xFFFF; let b = 0b1010; let c = 1_000; let d = 9223372036854775808; let e = 18446744073709551615; }",
    "fn f() { let a = 1e300; let b = 0.5; let c = -0.0; let d = 1e-9999; }",
    "fn f() { let c = 'x'; let d = '\\u{1F980}'; }",
    // Postfix chains and control expressions.
    "fn f() { let v = f(x)? + 1; }",
    "fn f() { let v = s.to_int()? as f64; }",
    "fn f() { let v = x.y.z(1, 2)?; }",
    "fn f() { let v = v[i].name; }",
    "fn f() { let v = total += i as f64; }",
    // Paths and nested items.
    "fn f() { let v = a::b::c; }",
    "fn outer() { fn inner() -> i64 { 1 } struct S { x: i64 } }",
    // Unicode identifiers.
    "fn f() { let café = 1; let δ = café + 1; }",
];

/// Asserts both round-trip directions for one source.
fn assert_roundtrip(source: &str) {
    let first = clean(source);
    let printed = first.printed();
    let second = clean(&printed);
    items_equivalent(&first.output.items, &second.output.items).unwrap_or_else(|error| {
        panic!("round-trip mismatch for:\n{source}\nerror: {error}\nprinted:\n{printed}")
    });
    assert_eq!(
        printed,
        second.printed(),
        "canonical printing must be idempotent for:\n{source}"
    );
}

#[test]
fn corpus_round_trips() {
    for source in CORPUS {
        assert_roundtrip(source);
    }
}

#[test]
fn spec_example_round_trip_keeps_radix_by_value_not_bytes() {
    // Integer radix is not part of the equivalence relation: `0xFFFF` and
    // `65535` compare equal. Round-trip must therefore still hold.
    let source = "fn f() { let a = 0xFF; }";
    assert_roundtrip(source);
}

#[test]
fn float_bits_round_trip() {
    for source in [
        "fn f() { let a = 0.5; }",
        "fn f() { let a = 1e300; }",
        "fn f() { let a = 1e-9999; }",
        "fn f() { let a = -0.0; }",
        "fn f() { let a = 3.14159; }",
    ] {
        assert_roundtrip(source);
    }
}

#[test]
fn grouped_chains_keep_structure_through_printing() {
    // `(a < b) < c` nests legitimately; printing must keep parentheses so
    // reparsing produces the same tree without chained-comparison errors.
    let source = "fn f() { let x = (a < b) < c; }";
    let first = clean(source);
    let printed = first.printed();
    let second = clean(&printed);
    items_equivalent(&first.output.items, &second.output.items).expect("grouped chain");
    assert!(
        printed.contains('('),
        "grouping must be preserved: {printed}"
    );
}

#[test]
fn recovered_inputs_are_never_used_for_round_trip() {
    // The equivalence relation is defined only for successfully parsed
    // trees; recovered trees carry placeholder nodes and are excluded.
    let parsed = common::parse_source("fn f() { let x = ; }");
    assert!(parsed.output.has_errors());
}

#[test]
fn interpolation_round_trips_with_nested_strings() {
    let source = "fn f() { let s = \"a { \"b {c} d\" } e\" }";
    assert_roundtrip(source);
}
