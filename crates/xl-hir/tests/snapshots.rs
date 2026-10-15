//! Deterministic lowering snapshots (`Directive 011` §14.F): HIR dumps
//! covering structure, origins, and the three desugarings. The dump
//! contains no addresses and no hash-order-dependent output.

mod common;

use common::{lower_ok, lower_source};
use xl_hir::dump::{dump_module, dump_resolutions};

#[test]
fn snapshot_function_with_a_let() {
    let output = lower_ok("fn main() { let x = 1; x }");
    insta::assert_snapshot!("fn_main_let", dump_module(&output.module));
}

#[test]
fn snapshot_for_loop() {
    let output = lower_ok("fn main() { for x in 0..10 { x; } }");
    insta::assert_snapshot!("fn_main_for", dump_module(&output.module));
    insta::assert_snapshot!(
        "fn_main_for_resolutions",
        dump_resolutions(&output.module, &output.resolutions)
    );
}

#[test]
fn snapshot_try() {
    let output = lower_ok("fn main() { let x = f()?; x }");
    insta::assert_snapshot!("fn_main_try", dump_module(&output.module));
}

#[test]
fn snapshot_interpolation() {
    let output = lower_ok("fn main() { let s = \"a{x}b\"; s }");
    insta::assert_snapshot!("fn_main_interpolation", dump_module(&output.module));
}

#[test]
fn snapshot_items_of_every_kind() {
    let output = lower_ok(
        "import std.io as io2;
        pub struct Point<T> { pub x: T, y: T }
        enum Shape { Circle(f64), Empty }
        trait Show<T> { fn show(self) -> T; }
        impl<T> Show<T> for Point<T> { pub fn show(self) -> T { self.x } }
        fn main() { let p = 1; }",
    );
    insta::assert_snapshot!("items_of_every_kind", dump_module(&output.module));
}

#[test]
fn snapshot_statements_patterns_and_types() {
    let output = lower_ok(
        "fn f(a: i64, b: (i64, bool), c: [i64; 2], d: fn(i64) -> bool) -> i64 {
            let mut x: i64 = a + 1;
            let (p, q) = b;
            match p { 0 => 1, 1 => 2, _ => 3 }
        }",
    );
    insta::assert_snapshot!("statements_patterns_types", dump_module(&output.module));
}

#[test]
fn snapshot_nested_desugarings() {
    let output = lower_ok("fn f() { for x in 0..2 { let y = g()?; } let s = \"v={x}\"; }");
    insta::assert_snapshot!("nested_desugarings", dump_module(&output.module));
}

#[test]
fn snapshot_specification_example() {
    let source = include_str!("fixtures/spec_example.x");
    assert!(
        !source.contains('\r'),
        "snapshot fixture must retain LF line endings"
    );
    let output = lower_ok(source);
    insta::assert_snapshot!("spec_example", dump_module(&output.module));
}

#[test]
fn snapshot_recovered_module() {
    // Recovered ASTs lower deterministically; the snapshot pins the error
    // nodes and the absence of fabricated structure.
    let output = lower_source("fn f() { let x = ; for y in ; { } }");
    assert!(output.is_complete());
    insta::assert_snapshot!("recovered", dump_module(&output.module));
}
