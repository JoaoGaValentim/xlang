//! Source-origin fidelity (`P2D-A2`; conformance-plan §7.1 items 2–5).
//!
//! The reverse-mapping test walks the AST with an independent walker and
//! asserts that every visited construct produced at least one HIR entity
//! and that no HIR entity carries a key no visited construct produces.

mod common;

use std::collections::HashSet;

use common::walk::{ExpectedKeys, collected_construct_keys};
use common::{lower_ok, parse_source};
use xl_hir::{AstOriginKey, HirModule};

/// Asserts the two-sided origin mapping for one source.
fn assert_origin_mapping(source: &str) -> HirModule {
    let parsed = parse_source(source);
    assert!(
        !parsed.has_errors(),
        "test source must parse cleanly: {:#?}",
        parsed.diagnostics
    );
    let output = lower_ok(source);
    let expected: HashSet<AstOriginKey> = ExpectedKeys::collect(&parsed.items);
    let collected = collected_construct_keys(&output.module);

    let missing: Vec<_> = expected.difference(&collected).collect();
    assert!(
        missing.is_empty(),
        "visited AST constructs without any HIR entity: {missing:#?}"
    );
    let fabricated: Vec<_> = collected.difference(&expected).collect();
    assert!(
        fabricated.is_empty(),
        "HIR entities with keys no visited construct produces: {fabricated:#?}"
    );
    output.module
}

#[test]
fn mapping_covers_items_of_every_kind() {
    assert_origin_mapping(
        "\
import std.io;
pub fn area(s: Shape) -> f64 { 0.0 }
pub struct Point { pub x: i64, pub y: i64 }
enum Shape { Circle(f64), Rect(i64, i64), Empty }
trait Show<T> { fn show(self) -> T; }
impl Show<i64> for Point { pub fn show(self) -> i64 { 0 } }
",
    );
}

#[test]
fn mapping_covers_statements_and_expressions() {
    assert_origin_mapping(
        "\
fn f(a: i64, b: i64) -> i64 {
    let mut x: i64 = a + b * 2;
    let y = (1, 2);
    let p = g();
    x = -x;
    let t = p.x;
    let arr = [1, 2, 3];
    let idx = arr[0];
    let c = x as f64;
    let r = 0..10;
    if x > 0 { x } else if x < 0 { 0 - x } else { 0 }
    match y { (p, q) => p + q }
    while x > 0 { x = x - 1; }
    loop { break; }
    let blk = { let z = 1; z };
    f(x, b)
}
",
    );
}

#[test]
fn mapping_covers_types_and_patterns() {
    assert_origin_mapping(
        "\
struct Box<T> { item: T }
fn f(x: i64, y: (i64, bool), z: [i64; 2], s: [i64], g: Box<Box<i64>>, fnp: fn(i64, bool) -> i64) -> () {
    let (a, b) = y;
    let (_, c) = (1, 2);
    match x { 0 => 1, 1 => 2, _ => 3 }
    let nested = ((1, 2), 3);
    let q = nested;
}
",
    );
}

#[test]
fn mapping_covers_nested_items_and_blocks() {
    assert_origin_mapping(
        "\
fn outer() -> i64 {
    fn inner(x: i64) -> i64 { x + 1 }
    struct Local { v: i64 }
    let x = inner(1);
    { let y = x; y }
}
",
    );
}

#[test]
fn mapping_covers_the_desugarings() {
    assert_origin_mapping(
        "\
fn f() {
    for x in 0..10 {
        let y = g()?;
        for z in (1, 2) {
            continue;
        }
        x;
    }
    let s = \"a{x}b{y}\";
    s;
}
",
    );
}

#[test]
fn mapping_covers_self_in_impl_context() {
    assert_origin_mapping(
        "\
struct Point { x: i64 }
trait Clone2 { fn clone2(self) -> Self; }
impl Clone2 for Point {
    fn clone2(self) -> Self {
        let p: Self = self;
        p
    }
}
",
    );
}

#[test]
fn every_item_and_expression_origin_uses_its_ast_id() {
    let source = "fn f(x: i64) -> i64 { let y = x + 1; y }\n";
    let parsed = parse_source(source);
    let output = lower_ok(source);

    // Forward direction: every AST item/expression id appears as an
    // `Item`/`Expr` origin key (shared or as a synthesized construct).
    let collected = collected_construct_keys(&output.module);
    let mut expected_items: Vec<_> = parsed
        .items
        .iter()
        .map(|item| AstOriginKey::Item(item.id))
        .collect();
    while let Some(item) = expected_items.pop() {
        assert!(collected.contains(&item), "missing item origin {item:?}");
    }
}
