//! Phase 1 regression: the archived specification example (`docs/
//! spec-v0.1-original.txt` §3) must keep parsing and now lower cleanly
//! (`Directive 011` §14.G).

mod common;

use common::{lower_ok, parse_source};
use xl_hir::{DefKind, ExprKind, HirKind, ModuleBindingTarget};

/// The complete original example (identical to the parser suite's copy).
const SPEC_EXAMPLE: &str = r#"import std.io;

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
"#;

#[test]
fn specification_example_parses_and_lowers_cleanly() {
    let parsed = parse_source(SPEC_EXAMPLE);
    assert!(
        !parsed.has_errors(),
        "the archived example must parse: {:#?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.items.len(), 7);
    let output = lower_ok(SPEC_EXAMPLE);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.fate, xl_hir::StageFate::Complete);
}

#[test]
fn specification_example_structure_is_faithful() {
    let output = lower_ok(SPEC_EXAMPLE);
    let module = &output.module;

    // The import binds `io` with an unresolved target.
    let record = &module.imports[0];
    let binding = module.def(record.binding).expect("binding");
    let DefKind::ModuleBinding(binding) = &binding.kind else {
        panic!("expected a module binding");
    };
    assert_eq!(binding.path, vec!["std".to_string(), "io".to_string()]);
    assert_eq!(binding.target, ModuleBindingTarget::Unresolved);

    // The method resolves `self` receiver and returns f64.
    let dist = common::def_named(module, "dist");
    let DefKind::Method { sig, body, .. } = &dist.kind else {
        panic!("expected a method");
    };
    assert!(body.is_some());
    let signature = module.sig(*sig).expect("sig");
    assert!(matches!(
        module.def(signature.params[0]).unwrap().kind,
        DefKind::Receiver { .. }
    ));

    // `parse` contains the ratified `?` expansion.
    assert!(
        module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Expr(ExprKind::Match(..))))
    );
    // `main` contains the `for` expansion and the interpolation concat.
    assert!(
        module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Expr(ExprKind::Loop(_))))
    );
    assert!(module.nodes.nodes.iter().any(|node| matches!(
        node.kind,
        HirKind::Expr(ExprKind::BuiltinCall {
            op: xl_hir::BuiltinOp::StrConcat,
            ..
        })
    )));
    // `io.println("...")` keeps its field selection unresolved.
    assert!(module.nodes.nodes.iter().any(|node| matches!(
        node.kind,
        HirKind::Expr(ExprKind::Field(_, xl_hir::FieldSel::Unresolved { .. }))
    )));
}

#[test]
fn specification_example_is_deterministic() {
    let first = lower_ok(SPEC_EXAMPLE);
    let second = lower_ok(SPEC_EXAMPLE);
    assert_eq!(first.module, second.module);
    assert_eq!(first.resolutions, second.resolutions);
}
