//! Suite A: items and declarations.

mod common;

use common::{clean, find_fn, messages, parse_source};
use xl_ast::{ExprKind, ImplItem, ItemKind, Lit, Param, StmtKind, TypeKind, Visibility};

#[test]
fn import_forms() {
    let parsed = clean("import std.io; import my.pkg as p;");
    let ItemKind::Import(first) = &parsed.output.items[0].kind else {
        panic!("import");
    };
    assert_eq!(
        first
            .path
            .iter()
            .map(|ident| ident.name.as_str())
            .collect::<Vec<_>>(),
        ["std", "io"]
    );
    assert!(first.alias.is_none());
    let ItemKind::Import(second) = &parsed.output.items[1].kind else {
        panic!("import");
    };
    assert_eq!(
        second.alias.as_ref().map(|ident| ident.name.as_str()),
        Some("p")
    );
}

#[test]
fn malformed_import_reports_once() {
    assert_eq!(messages("import a::b;"), vec!["expected '.' or ';'"]);
    assert_eq!(messages("import a.{b};"), vec!["expected identifier"]);
}

#[test]
fn function_declaration_with_generics_and_visibility() {
    let parsed = clean("pub fn g<T: Ord, U>(a: T, b: U) -> T { a }");
    let ItemKind::Fn(decl) = &parsed.output.items[0].kind else {
        panic!("fn");
    };
    assert!(matches!(decl.vis, Visibility::Public(_)));
    let generics = decl.generics.as_ref().expect("generics");
    assert_eq!(generics.params.len(), 2);
    assert!(generics.params[0].bound.is_some());
    assert!(generics.params[1].bound.is_none());
    assert_eq!(decl.sig.params.len(), 2);
    assert!(decl.sig.ret.is_some());
    assert!(matches!(
        decl.body.tail.as_ref().map(|e| &e.kind),
        Some(ExprKind::Path(_))
    ));
}

#[test]
fn receiver_and_named_parameters() {
    let parsed = clean("impl P { pub fn m(self, o: Point, x: i64) -> i64 { x } }");
    let ItemKind::Impl(imp) = &parsed.output.items[0].kind else {
        panic!("impl");
    };
    let ImplItem::Fn(decl) = &imp.items[0];
    assert!(matches!(decl.sig.params[0], Param::Receiver(_)));
    assert!(matches!(decl.sig.params[1], Param::Named { .. }));
    assert!(matches!(decl.vis, Visibility::Public(_)));
}

#[test]
fn struct_fields_visibility_and_trailing_comma() {
    let parsed = clean("pub struct S<T> { pub x: i64, y: [i64; 4], z: Vec<T>, }");
    let ItemKind::Struct(decl) = &parsed.output.items[0].kind else {
        panic!("struct");
    };
    assert_eq!(decl.fields.len(), 3);
    assert!(matches!(decl.fields[0].vis, Visibility::Public(_)));
    assert!(matches!(decl.fields[1].vis, Visibility::Private));
    assert!(matches!(decl.fields[1].ty.kind, TypeKind::Array(..)));
    assert!(decl.generics.is_some());
}

#[test]
fn empty_struct_and_enum_are_allowed() {
    let parsed = clean("struct S {} enum E {}");
    assert_eq!(parsed.output.items.len(), 2);
    let ItemKind::Struct(s) = &parsed.output.items[0].kind else {
        panic!("struct");
    };
    assert!(s.fields.is_empty());
    let ItemKind::Enum(e) = &parsed.output.items[1].kind else {
        panic!("enum");
    };
    assert!(e.variants.is_empty());
}

#[test]
fn enum_unit_and_tuple_variants() {
    let parsed = clean("enum E { A, B(i64), C(i64, bool) }");
    let ItemKind::Enum(decl) = &parsed.output.items[0].kind else {
        panic!("enum");
    };
    assert_eq!(decl.variants.len(), 3);
    assert!(decl.variants[0].payload.is_empty());
    assert_eq!(decl.variants[1].payload.len(), 1);
    assert_eq!(decl.variants[2].payload.len(), 2);
}

#[test]
fn inherent_and_trait_impls() {
    let parsed = clean(
        "impl Point { fn a(self) -> f64 { 1.0 } }\n\
         impl Show for Point { pub fn show(self) -> str { \"s\" } }",
    );
    let ItemKind::Impl(inherent) = &parsed.output.items[0].kind else {
        panic!("impl");
    };
    assert!(inherent.trait_.is_none());
    assert_eq!(inherent.self_ty.segments[0].name, "Point");

    let ItemKind::Impl(trait_impl) = &parsed.output.items[1].kind else {
        panic!("impl");
    };
    let trait_ = trait_impl.trait_.as_ref().expect("trait path");
    assert_eq!(trait_.segments[0].name, "Show");
    assert_eq!(trait_impl.self_ty.segments[0].name, "Point");
}

#[test]
fn trait_declaration_with_signature_only_items() {
    let parsed = clean("trait Show { fn show(self) -> str; fn name(self) -> str; }");
    let ItemKind::Trait(decl) = &parsed.output.items[0].kind else {
        panic!("trait");
    };
    assert_eq!(decl.items.len(), 2);
    assert_eq!(decl.items[0].sig.name.name, "show");
}

#[test]
fn generic_struct_and_impl() {
    let parsed = clean("struct W<T> { v: T }\nimpl<T> W<T> { fn get(self) -> T { self.v } }");
    let ItemKind::Impl(imp) = &parsed.output.items[1].kind else {
        panic!("impl");
    };
    assert!(imp.generics.is_some());
    assert_eq!(imp.self_ty.segments[0].name, "W");
    assert_eq!(
        imp.self_ty.generics.as_ref().expect("generic args").len(),
        1
    );
}

#[test]
fn nested_items_inside_blocks() {
    let source = "fn outer() { fn inner() -> i64 { 1 } struct S { x: i64 } }";
    let parsed = clean(source);
    let outer = find_fn(&parsed.output.items, "outer").expect("outer");
    assert_eq!(outer.body.stmts.len(), 2);
    assert!(matches!(outer.body.stmts[0].kind, StmtKind::Item(_)));
    match &outer.body.stmts[0].kind {
        StmtKind::Item(item) => assert!(matches!(item.kind, ItemKind::Fn(_))),
        _ => unreachable!(),
    }
    match &outer.body.stmts[1].kind {
        StmtKind::Item(item) => assert!(matches!(item.kind, ItemKind::Struct(_))),
        _ => unreachable!(),
    }
}

#[test]
fn literal_variants_parse_in_items() {
    let parsed =
        clean("fn f() { let a = 1; let b = 1.5; let c = 'x'; let d = true; let e = \"s\"; }");
    let body = &find_fn(&parsed.output.items, "f").expect("f").body;
    assert_eq!(body.stmts.len(), 5);
    let kinds: Vec<&ExprKind> = body
        .stmts
        .iter()
        .map(|stmt| match &stmt.kind {
            StmtKind::Let(let_stmt) => &let_stmt.init.kind,
            _ => panic!("let expected"),
        })
        .collect();
    assert!(matches!(kinds[0], ExprKind::Lit(Lit::Int(_))));
    assert!(matches!(kinds[1], ExprKind::Lit(Lit::Float(_))));
    assert!(matches!(kinds[2], ExprKind::Lit(Lit::Char(_))));
    assert!(matches!(kinds[3], ExprKind::Lit(Lit::Bool(true))));
    assert!(matches!(kinds[4], ExprKind::Lit(Lit::Str(_))));
}

#[test]
fn visibility_forms_parse() {
    let parsed = clean(
        "pub fn a() {}\npub struct S { pub x: i64 }\npub enum E { A }\npub trait T { fn f(self); }",
    );
    assert_eq!(parsed.output.items.len(), 4);
    for item in &parsed.output.items {
        let is_public = match &item.kind {
            ItemKind::Fn(decl) => matches!(decl.vis, Visibility::Public(_)),
            ItemKind::Struct(decl) => matches!(decl.vis, Visibility::Public(_)),
            ItemKind::Enum(decl) => matches!(decl.vis, Visibility::Public(_)),
            ItemKind::Trait(decl) => matches!(decl.vis, Visibility::Public(_)),
            _ => panic!("unexpected item"),
        };
        assert!(is_public, "item must be public");
    }
}

#[test]
fn empty_input_and_trivia_only_input_parse_clean() {
    let parsed = parse_source("");
    assert!(parsed.output.items.is_empty());
    assert!(parsed.output.diagnostics.is_empty());
    let parsed = clean("// nothing\n/* here */\n");
    assert!(parsed.output.items.is_empty());
}
