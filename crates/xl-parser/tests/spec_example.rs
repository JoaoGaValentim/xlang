//! The original specification example: clean parse, classification, and
//! canonical round-trip.

mod common;

use common::{SPEC_EXAMPLE, clean, fn_body, items_equivalent};
use xl_ast::{ExprKind, ItemKind, StmtKind};

/// The complete original example from `docs/spec-v0.1-original.txt` §3.
pub const EXAMPLE: &str = SPEC_EXAMPLE;

#[test]
fn specification_example_parses_without_diagnostics() {
    let parsed = clean(EXAMPLE);
    assert_eq!(
        parsed.output.items.len(),
        7,
        "import + struct + impl + enum + area + parse + main"
    );
}

#[test]
fn specification_items_have_expected_shapes() {
    let parsed = clean(EXAMPLE);
    let items = &parsed.output.items;

    let ItemKind::Import(import) = &items[0].kind else {
        panic!("item 0 must be the import");
    };
    assert_eq!(
        import
            .path
            .iter()
            .map(|ident| ident.name.as_str())
            .collect::<Vec<_>>(),
        ["std", "io"]
    );
    assert!(import.alias.is_none());

    let ItemKind::Struct(point) = &items[1].kind else {
        panic!("item 1 must be struct Point");
    };
    assert_eq!(point.name.name, "Point");
    assert_eq!(point.fields.len(), 2);
    assert_eq!(point.fields[1].name.name, "y");

    let ItemKind::Impl(imp) = &items[2].kind else {
        panic!("item 2 must be impl Point");
    };
    assert!(imp.trait_.is_none());
    assert_eq!(imp.self_ty.segments[0].name, "Point");
    assert_eq!(imp.items.len(), 1);

    let ItemKind::Enum(shape) = &items[3].kind else {
        panic!("item 3 must be enum Shape");
    };
    assert_eq!(shape.variants.len(), 2);
    assert_eq!(shape.variants[0].payload.len(), 1);
    assert_eq!(shape.variants[1].payload.len(), 2);
}

#[test]
fn dist_has_transparent_grouping_and_tail_call() {
    let parsed = clean(EXAMPLE);
    let ItemKind::Impl(imp) = &parsed.output.items[2].kind else {
        panic!("item 2 must be impl Point");
    };
    let xl_ast::ImplItem::Fn(dist) = &imp.items[0];
    let body = &dist.body;
    assert_eq!(body.stmts.len(), 2);
    let tail = body.tail.as_ref().expect("dist has a tail expression");
    // `(dx * dx + dy * dy).sqrt()` — grouping is transparent, so the tail
    // is a Call on a Field access on a Binary(Add(Add, Mul, Mul)).
    let ExprKind::Call(callee, args) = &tail.kind else {
        panic!("tail must be a call");
    };
    assert!(args.is_empty());
    let ExprKind::Field(base, name) = &callee.kind else {
        panic!("callee must be a field access");
    };
    assert_eq!(name.name, "sqrt");
    let ExprKind::Binary(add, _, _) = &base.kind else {
        panic!("receiver must be the grouped addition");
    };
    assert_eq!(*add, xl_ast::BinOp::Add);
}

#[test]
fn area_tail_is_match() {
    let parsed = clean(EXAMPLE);
    let body = fn_body(&parsed.output.items, "area");
    assert!(body.stmts.is_empty());
    let tail = body.tail.as_ref().expect("area tail");
    assert!(matches!(tail.kind, ExprKind::Match(..)));
    let ExprKind::Match(_, arms) = &tail.kind else {
        unreachable!()
    };
    assert_eq!(arms.len(), 2);
}

#[test]
fn parse_tail_is_if() {
    let parsed = clean(EXAMPLE);
    let body = fn_body(&parsed.output.items, "parse");
    assert_eq!(body.stmts.len(), 1);
    let tail = body.tail.as_ref().expect("parse tail");
    assert!(matches!(tail.kind, ExprKind::If(..)));
}

#[test]
fn main_has_no_tail_and_expected_statements() {
    let parsed = clean(EXAMPLE);
    let body = fn_body(&parsed.output.items, "main");
    assert!(body.tail.is_none());
    assert_eq!(body.stmts.len(), 3, "let, for, println call");
    assert!(matches!(body.stmts[0].kind, StmtKind::Let(..)));
    match &body.stmts[1].kind {
        StmtKind::Expr(expr) => assert!(matches!(expr.kind, ExprKind::For(..))),
        other => panic!("statement 1 must be the for loop, got {other:?}"),
    }
    match &body.stmts[2].kind {
        StmtKind::Expr(expr) => {
            let ExprKind::Call(callee, _) = &expr.kind else {
                panic!("println must be a call");
            };
            let ExprKind::Field(base, name) = &callee.kind else {
                panic!("io.println must be field access on io");
            };
            assert_eq!(name.name, "println");
            let ExprKind::Path(path) = &base.kind else {
                panic!("io must be a path");
            };
            assert_eq!(path.segments.len(), 1);
            assert_eq!(path.segments[0].name, "io");
        }
        other => panic!("statement 2 must be the println call, got {other:?}"),
    }
}

#[test]
fn canonical_round_trip_of_the_specification_example() {
    let parsed = clean(EXAMPLE);
    let printed = parsed.printed();
    let reparsed = clean(&printed);
    items_equivalent(&parsed.output.items, &reparsed.output.items)
        .unwrap_or_else(|error| panic!("round-trip mismatch: {error}\nprinted:\n{printed}"));
}

#[test]
fn printed_example_is_printed_again_identically() {
    let first = clean(EXAMPLE).printed();
    let second = clean(&first).printed();
    assert_eq!(first, second, "canonical printing must be idempotent");
}

#[test]
fn interpolation_in_main_is_structured() {
    let parsed = clean(EXAMPLE);
    let body = fn_body(&parsed.output.items, "main");
    let StmtKind::Expr(call) = &body.stmts[2].kind else {
        panic!("println statement");
    };
    let ExprKind::Call(_, args) = &call.kind else {
        panic!("call");
    };
    let ExprKind::Lit(xl_ast::Lit::Str(string)) = &args[0].kind else {
        panic!("string argument");
    };
    assert_eq!(string.parts.len(), 2, "text + interpolation");
    match &string.parts[0] {
        xl_ast::StringPart::Text(text) => assert_eq!(text.text, "total = "),
        other => panic!("first part must be text, got {other:?}"),
    }
    match &string.parts[1] {
        xl_ast::StringPart::Expr(expr) => {
            assert!(matches!(expr.kind, ExprKind::Path(..)));
        }
        other => panic!("second part must be an expression, got {other:?}"),
    }
}

#[test]
fn every_item_span_is_ordered_and_within_file() {
    let parsed = clean(EXAMPLE);
    let len = EXAMPLE.len() as u32;
    for item in &parsed.output.items {
        assert!(item.span.lo <= item.span.hi);
        assert!(item.span.hi <= len);
        assert_eq!(item.span.file, parsed.file);
    }
}
