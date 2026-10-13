//! Suite D: types, generic closing (G-3), and patterns.

mod common;

use common::{clean, fn_body, parse_source};
use xl_ast::{ExprKind, PatKind, StmtKind, TypeKind};

/// Parses `let x: <ty> = v;` and returns the annotation's kind.
fn annotation(ty: &str) -> TypeKind {
    let source = format!("fn f() {{ let x: {ty} = v; }}");
    let parsed = clean(&source);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    let ty = let_stmt.ty.as_ref().expect("annotation");
    ty.kind.clone()
}

#[test]
fn path_types_with_generic_arguments() {
    let kind = annotation("Result<i64, str>");
    let TypeKind::Path(path) = kind else {
        panic!("path type");
    };
    assert_eq!(path.segments[0].name, "Result");
    let generics = path.generics.expect("generic args");
    assert_eq!(generics.len(), 2);
}

#[test]
fn nested_generic_closers_are_two_gt_tokens() {
    let parsed = clean("fn f() -> Vec<Vec<i64>> { v }");
    let decl = common::find_fn(&parsed.output.items, "f").expect("f");
    let ret = decl.sig.ret.as_ref().expect("return type");
    let TypeKind::Path(outer) = &ret.kind else {
        panic!("path");
    };
    let outer_args = outer.generics.as_ref().expect("args");
    let TypeKind::Path(inner) = &outer_args[0].kind else {
        panic!("inner path");
    };
    assert_eq!(inner.segments[0].name, "Vec");
    assert!(inner.generics.is_some());
}

#[test]
fn ge_split_in_type_closing_context() {
    // `Vec<i64>= v;` lexes `>=` as one token; the type-closing context
    // splits it into `>` + `=` so the let initializer parses.
    let parsed = clean("fn f() { let x: Vec<i64>= v; }");
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(let_stmt.ty.is_some());
    assert!(matches!(let_stmt.init.kind, ExprKind::Path(_)));
}

#[test]
fn double_closers_before_ge_split() {
    // `Vec<Vec<i64>>= v;` lexes `>` `>=`; inner closer consumes the `>`,
    // outer closer splits the `>=`.
    let parsed = clean("fn f() { let x: Vec<Vec<i64>>= v; }");
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(matches!(let_stmt.init.kind, ExprKind::Path(_)));
}

#[test]
fn ge_in_expression_is_not_split() {
    let parsed = clean("fn f() { let b = a >= c; }");
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert_eq!(common::binary_op(&let_stmt.init), Some(xl_ast::BinOp::Ge));
}

#[test]
fn excess_generic_closer_is_reported_and_skipped() {
    let parsed = parse_source("fn f() { let x: Vec<T>> = v; }");
    assert_eq!(parsed.messages(), vec!["unexpected '>'"]);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(matches!(let_stmt.init.kind, ExprKind::Path(_)));
}

#[test]
fn excess_closer_inside_return_type() {
    let parsed = parse_source("fn f() -> Vec<T>> { 0 }");
    assert_eq!(parsed.messages(), vec!["unexpected '>'"]);
}

#[test]
fn all_derivable_type_forms() {
    type Check = fn(&TypeKind) -> bool;
    let cases: &[(&str, Check)] = &[
        (
            "()",
            |k| matches!(k, TypeKind::Tuple(items) if items.is_empty()),
        ),
        ("(i64)", |k| matches!(k, TypeKind::Path(_))),
        (
            "(i64,)",
            |k| matches!(k, TypeKind::Tuple(items) if items.len() == 1),
        ),
        (
            "(i64, bool)",
            |k| matches!(k, TypeKind::Tuple(items) if items.len() == 2),
        ),
        ("[i64]", |k| matches!(k, TypeKind::Slice(_))),
        ("[i64; 4]", |k| matches!(k, TypeKind::Array(..))),
        (
            "fn()",
            |k| matches!(k, TypeKind::Fn { params, ret: None } if params.is_empty()),
        ),
        (
            "fn(i64)",
            |k| matches!(k, TypeKind::Fn { params, ret: None } if params.len() == 1),
        ),
        ("fn(i64) -> i64", |k| {
            matches!(k, TypeKind::Fn { ret: Some(_), .. })
        }),
        (
            "fn(i64, bool) -> Result<i64, str>",
            |k| matches!(k, TypeKind::Fn { params, ret: Some(_) } if params.len() == 2),
        ),
        ("Vec<Vec<i64>>", |k| matches!(k, TypeKind::Path(_))),
    ];
    for (source, check) in cases {
        let kind = annotation(source);
        assert!(check(&kind), "{source}: {kind:?}");
    }
}

#[test]
fn function_type_in_parameter_position() {
    let parsed = clean("fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }");
    let decl = common::find_fn(&parsed.output.items, "apply").expect("apply");
    let xl_ast::Param::Named { ty, .. } = &decl.sig.params[0] else {
        panic!("named param");
    };
    assert!(matches!(ty.kind, TypeKind::Fn { .. }));
}

#[test]
fn array_type_length_is_an_expression() {
    let kind = annotation("[i64; 4]");
    match kind {
        TypeKind::Array(_, length) => {
            assert!(matches!(length.kind, ExprKind::Lit(_)));
        }
        other => panic!("array type, got {other:?}"),
    }
}

#[test]
fn generic_bounds_in_declaration() {
    let parsed = clean("fn g<T: Ord, U>(a: T, b: U) -> T { a }");
    let decl = common::find_fn(&parsed.output.items, "g").expect("g");
    let generics = decl.generics.as_ref().expect("generics");
    let bound = generics.params[0].bound.as_ref().expect("bound");
    assert_eq!(bound.segments[0].name, "Ord");
}

#[test]
fn trailing_comma_in_generic_arguments() {
    let kind = annotation("Result<i64, str,>");
    let TypeKind::Path(path) = kind else {
        panic!("path");
    };
    assert_eq!(path.generics.expect("args").len(), 2);
}

#[test]
fn wildcard_and_binding_patterns() {
    let parsed = clean("fn f() { let _ = 1; let x = 2; }");
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(first) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(matches!(first.pat.kind, PatKind::Wildcard));
    let StmtKind::Let(second) = &body.stmts[1].kind else {
        panic!("let");
    };
    assert!(matches!(second.pat.kind, PatKind::Binding(_)));
}

#[test]
fn literal_patterns_in_match() {
    let parsed = clean(
        "fn f() { match x { 1 => \"a\", 1.5 => \"b\", 'c' => \"d\", \"s\" => \"e\", true => \"f\", _ => \"g\", } }",
    );
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("match");
    };
    assert_eq!(arms.len(), 6);
    assert!(matches!(
        arms[0].pat.kind,
        PatKind::Lit(xl_ast::Lit::Int(_))
    ));
    assert!(matches!(
        arms[1].pat.kind,
        PatKind::Lit(xl_ast::Lit::Float(_))
    ));
    assert!(matches!(
        arms[2].pat.kind,
        PatKind::Lit(xl_ast::Lit::Char(_))
    ));
    assert!(matches!(
        arms[3].pat.kind,
        PatKind::Lit(xl_ast::Lit::Str(_))
    ));
    assert!(matches!(
        arms[4].pat.kind,
        PatKind::Lit(xl_ast::Lit::Bool(true))
    ));
    assert!(matches!(arms[5].pat.kind, PatKind::Wildcard));
}

#[test]
fn constructor_patterns_with_payloads() {
    let parsed = clean(
        "fn f() { match s { Shape::Circle(r) => 1, Shape::Rect(w, h) => 2, Shape::Unit => 3, Ok(n) => 4, } }",
    );
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("match");
    };
    match &arms[0].pat.kind {
        PatKind::Path { path, args } => {
            assert_eq!(path.segments.len(), 2);
            assert_eq!(args.as_ref().expect("payload").len(), 1);
        }
        other => panic!("constructor pattern, got {other:?}"),
    }
    match &arms[1].pat.kind {
        PatKind::Path { args, .. } => assert_eq!(args.as_ref().expect("payload").len(), 2),
        other => panic!("constructor pattern, got {other:?}"),
    }
    match &arms[2].pat.kind {
        PatKind::Path { args, .. } => assert!(args.is_none()),
        other => panic!("path pattern without payload, got {other:?}"),
    }
    match &arms[3].pat.kind {
        PatKind::Path { path, args } => {
            assert_eq!(path.segments.len(), 1);
            assert_eq!(path.segments[0].name, "Ok");
            assert!(args.is_some());
        }
        other => panic!("single-segment constructor, got {other:?}"),
    }
}

#[test]
fn tuple_patterns() {
    let parsed = clean("fn f() { let (a, b) = pair; let (c,) = one; match t { (x, y) => x } }");
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(first) = &body.stmts[0].kind else {
        panic!("let");
    };
    match &first.pat.kind {
        PatKind::Tuple(items) => assert_eq!(items.len(), 2),
        other => panic!("tuple pattern, got {other:?}"),
    }
    let StmtKind::Let(second) = &body.stmts[1].kind else {
        panic!("let");
    };
    match &second.pat.kind {
        PatKind::Tuple(items) => assert_eq!(items.len(), 1),
        other => panic!("one-tuple pattern, got {other:?}"),
    }
}

#[test]
fn for_pattern_bindings() {
    let parsed = clean("fn f() { for (i, x) in pairs { p(i); } }");
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::For(pat, _, _) = &tail.kind else {
        panic!("for");
    };
    assert!(matches!(pat.kind, PatKind::Tuple(_)));
}

#[test]
fn empty_constructor_payload_is_allowed() {
    let parsed = clean("fn f() { match x { A() => 1 } }");
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("match");
    };
    match &arms[0].pat.kind {
        PatKind::Path { args, .. } => {
            assert_eq!(args.as_ref().expect("payload").len(), 0);
        }
        other => panic!("constructor pattern, got {other:?}"),
    }
}

#[test]
fn underscore_is_not_an_expression() {
    let parsed = parse_source("fn f() { let x = _; }");
    assert_eq!(parsed.messages(), vec!["expected expression"]);
}

#[test]
fn missing_type_reports_expected_type() {
    let parsed = parse_source("fn f() { let x: ; }");
    assert_eq!(parsed.messages()[0], "expected type");
}

#[test]
fn struct_field_missing_type_reports() {
    let parsed = parse_source("struct S { x: }");
    assert_eq!(parsed.messages(), vec!["expected type"]);
}
