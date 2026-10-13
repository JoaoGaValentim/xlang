//! Snapshot tests: canonical printer output and rendered diagnostics for
//! contract-stability (insta).

mod common;

use common::{SPEC_EXAMPLE, parse_source};

#[test]
fn canonical_output_of_the_specification_example() {
    let parsed = common::clean(SPEC_EXAMPLE);
    let printed = parsed.printed();
    insta::assert_snapshot!("spec_example_canonical", printed);
}

#[test]
fn diagnostics_battery_rendering() {
    let source = "\
fn broken( { }
fn f() {
    let x = ;
    let y = a < b < c;
    let z: Vec<T>> = v;
}
";
    let parsed = parse_source(source);
    let rendered = parsed.rendered();
    insta::assert_snapshot!("diagnostics_battery", rendered);
}

#[test]
fn interpolation_battery_rendering() {
    let source = "\
fn f() {
    let a = \"{}\";
    let b = \"{1 2}\";
    let c = \"tail\";
}
";
    let parsed = parse_source(source);
    insta::assert_snapshot!("interpolation_battery", parsed.rendered());
}

#[test]
fn chained_operator_recovery_rendering() {
    let source = "\
fn f() {
    let a = x < y < z;
    let b = p..q..r;
    let c = m = n = o;
}
";
    let parsed = parse_source(source);
    insta::assert_snapshot!("chained_operator_recovery", parsed.rendered());
}
