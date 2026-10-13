//! Snapshot tests (insta) for representative token streams and diagnostics.
//!
//! Every snapshot change must correspond to an intentional contract change;
//! see the PR review checklist.

mod common;

use common::{dump_diagnostics, dump_string_token, dump_tokens};

const EXAMPLE_PROGRAM: &str = "\
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
    if n < 0 { Err(\"negativo\") } else { Ok(n) }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 { total += i as f64; }
    io.println(\"total = {total}\");
}
";

#[test]
fn snapshot_example_program_tokens() {
    insta::assert_snapshot!("example_program_tokens", dump_tokens(EXAMPLE_PROGRAM));
}

#[test]
fn snapshot_operator_battery_tokens() {
    let source = "+ - * / % += -= *= /= %= = == != < <= > >= && || ! -> => ? . .. :: \
                  ( ) { } [ ] , ; :";
    insta::assert_snapshot!("operator_battery_tokens", dump_tokens(source));
}

#[test]
fn snapshot_numeric_battery_tokens() {
    let source = "0 1_000 007 0xFF 0xFF_FF 0b1010 9223372036854775808 \
                  18446744073709551615 0.0 3.14159 1.5e3 1e5 1e300 1e-9999 1..2 1.foo";
    insta::assert_snapshot!("numeric_battery_tokens", dump_tokens(source));
}

#[test]
fn snapshot_string_structure() {
    let source = "\"total = {total} nested {if flag { 1 } else { 2 }} end\"";
    insta::assert_snapshot!("string_structure", dump_string_token(source));
}

#[test]
fn snapshot_diagnostics_battery() {
    let source = "~ \"abc\n0x 1_ 18446744073709551616 1e9999 /* open";
    insta::assert_snapshot!("diagnostics_battery", dump_diagnostics(source));
}

#[test]
fn snapshot_interpolation_limit_recovery() {
    let mut source = String::from("\"x\"");
    for _ in 0..65 {
        source = format!("\"{{ {source} }}\"");
    }
    insta::assert_snapshot!("interpolation_limit_recovery", dump_diagnostics(&source));
}
