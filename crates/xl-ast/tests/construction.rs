//! Suite A — node construction across every AST category.

mod common;

use common::*;
use xl_ast::*;
use xl_span::Span;

#[test]
fn every_expression_kind_is_constructible_and_prints() {
    let cases: Vec<Expr> = vec![
        lit_int(7),
        lit_float(1.5),
        lit_bool(true),
        leaf_expr(0, ExprKind::Lit(Lit::Char('x'))),
        var("x"),
        path_expr(&["io", "println"]),
        un(UnOp::Neg, lit_int(1)),
        un(UnOp::Not, lit_bool(false)),
        bin(BinOp::Add, lit_int(1), lit_int(2)),
        bin(BinOp::Eq, lit_int(1), lit_int(2)),
        bin(BinOp::And, lit_bool(true), lit_bool(false)),
        assign(AssignOp::AddAssign, var("x"), lit_int(1)),
        range(lit_int(0), lit_int(10)),
        cast(var("i"), ty_path("f64")),
        try_expr(call(var("f"), vec![])),
        call(var("f"), vec![lit_int(1), lit_int(2)]),
        field(var("p"), "x"),
        index(var("v"), lit_int(0)),
        tuple(vec![lit_int(1), lit_int(2)]),
        tuple(vec![]),
        array(vec![lit_int(1), lit_int(2)]),
        array(vec![]),
        ret(None),
        ret(Some(lit_int(1))),
        break_expr(),
        leaf_expr(0, ExprKind::Continue),
        leaf_expr(0, ExprKind::Block(empty_block())),
        leaf_expr(0, ExprKind::While(Box::new(lit_bool(true)), empty_block())),
        leaf_expr(0, ExprKind::Loop(empty_block())),
        leaf_expr(
            0,
            ExprKind::For(
                Box::new(binding_pat("i")),
                Box::new(lit_int(0)),
                empty_block(),
            ),
        ),
        leaf_expr(
            0,
            ExprKind::If(Box::new(lit_bool(true)), empty_block(), None),
        ),
        leaf_expr(0, ExprKind::Match(Box::new(var("s")), vec![])),
    ];
    for expr in cases {
        let printed = print_expr(&expr).expect("fixture must print");
        assert!(!printed.is_empty());
    }
}

#[test]
fn literal_string_with_parts_is_constructible() {
    let mut s = Spans::new();
    let name = s.ident("total");
    let inner = ex(
        name.span,
        ExprKind::Path(Path {
            segments: vec![name.clone()],
            span: name.span,
        }),
    );
    let expr = s.string_interp("total = ", inner);
    assert_eq!(print_expr(&expr).expect("prints"), "\"total = {total}\"");
}

#[test]
fn every_pattern_kind_is_constructible() {
    let mut s = Spans::new();
    let wildcard = Pat {
        kind: PatKind::Wildcard,
        span: fspan(0, 1),
    };
    let binding = binding_pat("x");
    let literal = Pat {
        kind: PatKind::Lit(Lit::Int(IntLit {
            magnitude: 1,
            radix: Radix::Dec,
        })),
        span: fspan(0, 1),
    };
    let path = s.path_pat(&["Shape", "Circle"], vec![binding_pat("r")]);
    let unit_path = s.path_pat(&["None"], vec![]);
    let tuple = Pat {
        kind: PatKind::Tuple(vec![binding_pat("a"), binding_pat("b")]),
        span: fspan(0, 3),
    };
    for pat in [wildcard, binding, literal, path, unit_path, tuple] {
        let printed = print_pattern(&pat).expect("fixture must print");
        assert!(!printed.is_empty());
    }
}

#[test]
fn every_type_kind_is_constructible_and_prints() {
    let mut s = Spans::new();
    let f64_ty = {
        let path = s.path_type(&["f64"], None);
        s.ty(TypeKind::Path(path))
    };
    let unit = Type {
        kind: TypeKind::Tuple(Vec::new()),
        span: fspan(0, 2),
    };
    let one_tuple = Type {
        kind: TypeKind::Tuple(vec![ty_path("i64")]),
        span: fspan(0, 4),
    };
    let tuple = Type {
        kind: TypeKind::Tuple(vec![ty_path("i64"), ty_path("bool")]),
        span: fspan(0, 8),
    };
    let slice = Type {
        kind: TypeKind::Slice(Box::new(ty_path("i64"))),
        span: fspan(0, 5),
    };
    let array = Type {
        kind: TypeKind::Array(Box::new(ty_path("i64")), Box::new(lit_int(4))),
        span: fspan(0, 7),
    };
    let fn_type = Type {
        kind: TypeKind::Fn {
            params: vec![f64_ty.clone()],
            ret: Some(Box::new(f64_ty.clone())),
        },
        span: fspan(0, 12),
    };
    let fn_unit = Type {
        kind: TypeKind::Fn {
            params: Vec::new(),
            ret: None,
        },
        span: fspan(0, 4),
    };
    let generic = {
        let path = s.path_type(&["Result"], Some(vec![ty_path("i64"), ty_path("str")]));
        s.ty(TypeKind::Path(path))
    };
    let printed: Vec<String> = [
        unit, one_tuple, tuple, slice, array, fn_type, fn_unit, generic,
    ]
    .iter()
    .map(|ty| print_type(ty).expect("fixture must print"))
    .collect();
    assert_eq!(printed[0], "()");
    assert_eq!(printed[1], "(i64,)");
    assert_eq!(printed[2], "(i64, bool)");
    assert_eq!(printed[3], "[i64]");
    assert_eq!(printed[4], "[i64; 4]");
    assert_eq!(printed[5], "fn(f64) -> f64");
    assert_eq!(printed[6], "fn()");
    assert_eq!(printed[7], "Result<i64, str>");
}

#[test]
fn every_item_kind_is_constructible() {
    // Function.
    let fn_decl = fn_item("f", Vec::new(), Some(ty_path("i64")), empty_block());
    assert!(matches!(fn_decl.kind, ItemKind::Fn(_)));

    // Import.
    let import = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Import(ImportDecl {
            path: vec![ident("std", 0), ident("io", 0)],
            alias: None,
            span: fspan(0, 8),
        }),
        span: fspan(0, 8),
    };
    assert!(matches!(import.kind, ItemKind::Import(_)));

    // Struct with fields (one public).
    let struct_decl = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Struct(StructDecl {
            vis: Visibility::Public(fspan(0, 3)),
            name: ident("Point", 4),
            generics: None,
            fields: vec![
                Field {
                    vis: Visibility::Private,
                    name: ident("x", 0),
                    ty: ty_path("f64"),
                    span: fspan(0, 2),
                },
                Field {
                    vis: Visibility::Public(fspan(0, 3)),
                    name: ident("y", 0),
                    ty: ty_path("f64"),
                    span: fspan(0, 5),
                },
            ],
            span: fspan(0, 20),
        }),
        span: fspan(0, 20),
    };
    assert!(matches!(struct_decl.kind, ItemKind::Struct(_)));

    // Enum with unit and tuple variants.
    let enum_decl = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Enum(EnumDecl {
            vis: Visibility::Private,
            name: ident("E", 0),
            generics: None,
            variants: vec![
                Variant {
                    name: ident("A", 0),
                    payload: Vec::new(),
                    span: fspan(0, 1),
                },
                Variant {
                    name: ident("B", 0),
                    payload: vec![ty_path("i64")],
                    span: fspan(0, 3),
                },
            ],
            span: fspan(0, 12),
        }),
        span: fspan(0, 12),
    };
    assert!(matches!(enum_decl.kind, ItemKind::Enum(_)));

    // Trait with a signature-only item.
    let trait_decl = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Trait(trait_item_with(
            "Show",
            sig_ex(ident("show", 0), Vec::new(), Some(ty_path("str"))),
        )),
        span: fspan(0, 30),
    };
    assert!(matches!(trait_decl.kind, ItemKind::Trait(_)));

    // Inherent impl and trait impl.
    let inherent = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Impl(ImplDecl {
            generics: None,
            trait_: None,
            self_ty: {
                let mut s = Spans::new();
                s.path_type(&["Point"], None)
            },
            items: Vec::new(),
            span: fspan(0, 12),
        }),
        span: fspan(0, 12),
    };
    let trait_impl = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Impl(ImplDecl {
            generics: None,
            trait_: Some({
                let mut s = Spans::new();
                s.path_type(&["Show"], None)
            }),
            self_ty: {
                let mut s = Spans::new();
                s.path_type(&["Point"], None)
            },
            items: Vec::new(),
            span: fspan(0, 20),
        }),
        span: fspan(0, 20),
    };
    for item in [inherent, trait_impl] {
        assert!(matches!(item.kind, ItemKind::Impl(_)));
        assert!(!print_item(&item).expect("prints").is_empty());
    }
}

#[test]
fn generics_with_bounds_are_constructible() {
    let mut s = Spans::new();
    let name = s.ident("T");
    let bound = s.path_type(&["Ord"], None);
    let param_span = cover(name.span, bound.span);
    let param = GenericParam {
        name,
        bound: Some(bound),
        span: param_span,
    };
    let generics = Generics {
        span: param_span,
        params: vec![param],
    };
    let sig = sig_ex(ident("f", 0), Vec::new(), None);
    let decl = FnDecl {
        vis: Visibility::Private,
        generics: Some(generics),
        sig,
        body: empty_block(),
        span: fspan(0, 40),
    };
    let item = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Fn(decl),
        span: fspan(0, 40),
    };
    assert_eq!(print_item(&item).expect("prints"), "fn f<T: Ord>() {}");
}

#[test]
fn blocks_distinguish_statements_from_tails() {
    let with_tail = block_with(Vec::new(), Some(lit_int(1)));
    let with_stmt = block_with(vec![expr_stmt(lit_int(1))], None);
    assert_eq!(print_block(&with_tail).expect("prints"), "{\n    1\n}");
    assert_eq!(print_block(&with_stmt).expect("prints"), "{\n    1;\n}");
}

#[test]
fn helper_span_constructors_are_consistent() {
    let a = fspan(3, 7);
    let b = fspan(5, 9);
    assert_eq!(cover(a, b), fspan(3, 9));
    assert_eq!(cover_all(&[a, b]), fspan(3, 9));
    assert_eq!(cover_all(&[] as &[Span]), fspan(0, 0));
    assert_eq!(span_of(&lit_int(1)), fspan(0, 1));
}
