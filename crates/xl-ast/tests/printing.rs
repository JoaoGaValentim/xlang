//! Suite D — canonical printing: exact outputs for representative trees.

mod common;

use common::*;
use xl_ast::*;

#[test]
fn import_variants_print() {
    let plain = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Import(ImportDecl {
            path: vec![ident("std", 0), ident("io", 0)],
            alias: None,
            span: fspan(0, 8),
        }),
        span: fspan(0, 8),
    };
    assert_eq!(print_item(&plain).expect("prints"), "import std.io;");

    let aliased = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Import(ImportDecl {
            path: vec![ident("my", 0), ident("pkg", 0)],
            alias: Some(ident("p", 0)),
            span: fspan(0, 12),
        }),
        span: fspan(0, 12),
    };
    assert_eq!(print_item(&aliased).expect("prints"), "import my.pkg as p;");
}

#[test]
fn struct_prints_fields_one_per_line_without_trailing_comma() {
    let mut s = Spans::new();
    let name = s.ident("Point");
    let mut fields = Vec::new();
    for field_name in ["x", "y"] {
        let fname = s.ident(field_name);
        let ty = {
            let path = s.path_type(&["f64"], None);
            s.ty(TypeKind::Path(path))
        };
        let span = cover(fname.span, ty.span);
        fields.push(Field {
            vis: Visibility::Private,
            name: fname,
            ty,
            span,
        });
    }
    let item = s.item(ItemKind::Struct(StructDecl {
        vis: Visibility::Private,
        name,
        generics: None,
        fields,
        span: fspan(0, 0),
    }));
    assert_eq!(
        print_item(&item).expect("prints"),
        "struct Point {\n    x: f64,\n    y: f64\n}"
    );
}

#[test]
fn enum_prints_variants_including_unit_and_tuple_forms() {
    let mut s = Spans::new();
    let name = s.ident("E");
    let unit = {
        let vname = s.ident("A");
        let span = vname.span;
        Variant {
            name: vname,
            payload: Vec::new(),
            span,
        }
    };
    let tuple_variant = {
        let vname = s.ident("B");
        let path = s.path_type(&["i64"], None);
        let ty = s.ty(TypeKind::Path(path));
        let span = cover(vname.span, ty.span);
        Variant {
            name: vname,
            payload: vec![ty],
            span,
        }
    };
    let item = s.item(ItemKind::Enum(EnumDecl {
        vis: Visibility::Private,
        name,
        generics: None,
        variants: vec![unit, tuple_variant],
        span: fspan(0, 0),
    }));
    assert_eq!(
        print_item(&item).expect("prints"),
        "enum E {\n    A,\n    B(i64)\n}"
    );
}

#[test]
fn impl_prints_methods_with_visibility() {
    let mut s = Spans::new();
    let self_ty = s.path_type(&["Point"], None);
    let vis = Visibility::Public(s.leaf(3));
    let sig = sig_ex(ident("dist", 0), Vec::new(), Some(ty_path("f64")));
    let body = empty_block();
    let fn_decl = FnDecl {
        vis,
        generics: None,
        sig,
        body,
        span: fspan(0, 40),
    };
    let item = s.item(ItemKind::Impl(ImplDecl {
        generics: None,
        trait_: None,
        self_ty,
        items: vec![ImplItem::Fn(fn_decl)],
        span: fspan(0, 0),
    }));
    assert_eq!(
        print_item(&item).expect("prints"),
        "impl Point {\n    pub fn dist() -> f64 {}\n}"
    );
}

#[test]
fn trait_impl_prints_for_clause() {
    let mut s = Spans::new();
    let trait_path = s.path_type(&["Show"], None);
    let self_ty = s.path_type(&["Point"], None);
    let item = s.item(ItemKind::Impl(ImplDecl {
        generics: None,
        trait_: Some(trait_path),
        self_ty,
        items: Vec::new(),
        span: fspan(0, 0),
    }));
    assert_eq!(print_item(&item).expect("prints"), "impl Show for Point {}");
}

#[test]
fn trait_declaration_prints_signature_only_items() {
    let mut s = Spans::new();
    let name = s.ident("Show");
    let sig = sig_ex(ident("show", 0), Vec::new(), Some(ty_path("str")));
    let span = cover(name.span, sig.span);
    let sig_span = sig.span;
    let item = s.item(ItemKind::Trait(TraitDecl {
        vis: Visibility::Private,
        name,
        generics: None,
        items: vec![TraitItem {
            sig,
            span: sig_span,
        }],
        span,
    }));
    assert_eq!(
        print_item(&item).expect("prints"),
        "trait Show {\n    fn show() -> str;\n}"
    );
}

#[test]
fn function_with_generics_and_parameters_prints() {
    let mut s = Spans::new();
    let name = s.ident("g");
    let t_name = s.ident("T");
    let bound = s.path_type(&["Ord"], None);
    let param_span = cover(t_name.span, bound.span);
    let generics = Generics {
        params: vec![GenericParam {
            name: t_name,
            bound: Some(bound),
            span: param_span,
        }],
        span: param_span,
    };
    let param_name = s.ident("a");
    let a_ty = {
        let path = s.path_type(&["T"], None);
        s.ty(TypeKind::Path(path))
    };
    let ret = {
        let path = s.path_type(&["T"], None);
        s.ty(TypeKind::Path(path))
    };
    let sig = sig_ex(name, vec![named_param(param_name, a_ty)], Some(ret));
    let inner = s.path_expr(&["a"]);
    let stmt = s.stmt(StmtKind::Expr(inner));
    let body = s.block(vec![stmt], None);
    let item = s.item(ItemKind::Fn(FnDecl {
        vis: Visibility::Private,
        generics: Some(generics),
        sig,
        body,
        span: fspan(0, 0),
    }));
    assert_eq!(
        print_item(&item).expect("prints"),
        "fn g<T: Ord>(a: T) -> T {\n    a;\n}"
    );
}

#[test]
fn receivers_and_parameters_print() {
    let mut s = Spans::new();
    let receiver = s.leaf(4);
    let o = s.ident("o");
    let point = {
        let path = s.path_type(&["Point"], None);
        s.ty(TypeKind::Path(path))
    };
    let sig = sig_ex(
        ident("dist", 0),
        vec![Param::Receiver(receiver), named_param(o, point)],
        None,
    );
    let item = fn_item_with_sig(sig, empty_block());
    assert_eq!(
        print_item(&item).expect("prints"),
        "fn dist(self, o: Point) {}"
    );
}

fn fn_item_with_sig(sig: FnSig, body: Block) -> Item {
    let span = cover(sig.span, body.span);
    Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Fn(FnDecl {
            vis: Visibility::Private,
            generics: None,
            sig,
            body,
            span,
        }),
        span,
    }
}

#[test]
fn expression_forms_print_exactly() {
    assert_eq!(print_expr(&lit_int(1)).expect("prints"), "1");
    assert_eq!(print_expr(&var("x")).expect("prints"), "x");
    assert_eq!(
        print_expr(&path_expr(&["io", "println"])).expect("prints"),
        "io::println"
    );
    assert_eq!(print_expr(&un(UnOp::Neg, var("x"))).expect("prints"), "-x");
    assert_eq!(print_expr(&un(UnOp::Not, var("x"))).expect("prints"), "!x");
    assert_eq!(
        print_expr(&bin(BinOp::Add, var("a"), lit_int(1))).expect("prints"),
        "a + 1"
    );
    assert_eq!(
        print_expr(&assign(AssignOp::Assign, var("x"), lit_int(1))).expect("prints"),
        "x = 1"
    );
    assert_eq!(
        print_expr(&assign(AssignOp::RemAssign, var("x"), lit_int(2))).expect("prints"),
        "x %= 2"
    );
    assert_eq!(
        print_expr(&range(lit_int(0), var("n"))).expect("prints"),
        "0..n"
    );
    assert_eq!(
        print_expr(&cast(var("i"), ty_path("f64"))).expect("prints"),
        "i as f64"
    );
    assert_eq!(
        print_expr(&try_expr(call(var("f"), Vec::new()))).expect("prints"),
        "f()?"
    );
    assert_eq!(print_expr(&field(var("p"), "x")).expect("prints"), "p.x");
    assert_eq!(
        print_expr(&index(var("v"), lit_int(0))).expect("prints"),
        "v[0]"
    );
    assert_eq!(
        print_expr(&call(var("f"), vec![lit_int(1), lit_int(2)])).expect("prints"),
        "f(1, 2)"
    );
    assert_eq!(
        print_expr(&tuple(vec![lit_int(1), lit_int(2)])).expect("prints"),
        "(1, 2)"
    );
    assert_eq!(print_expr(&tuple(Vec::new())).expect("prints"), "()");
    assert_eq!(
        print_expr(&tuple(vec![lit_int(1)])).expect("prints"),
        "(1,)"
    );
    assert_eq!(
        print_expr(&array(vec![lit_int(1), lit_int(2)])).expect("prints"),
        "[1, 2]"
    );
    assert_eq!(print_expr(&array(Vec::new())).expect("prints"), "[]");
    assert_eq!(print_expr(&ret(None)).expect("prints"), "return");
    assert_eq!(
        print_expr(&ret(Some(lit_int(1)))).expect("prints"),
        "return 1"
    );
    assert_eq!(print_expr(&break_expr()).expect("prints"), "break");
    assert_eq!(
        print_expr(&leaf_expr(0, ExprKind::Continue)).expect("prints"),
        "continue"
    );
}

#[test]
fn control_flow_expressions_print_exactly() {
    let if_expr = ex(
        fspan(0, 20),
        ExprKind::If(
            Box::new(var("c")),
            block_with(Vec::new(), Some(lit_int(1))),
            Some(Box::new(ex(
                fspan(0, 10),
                ExprKind::Block(block_with(Vec::new(), Some(lit_int(2)))),
            ))),
        ),
    );
    assert_eq!(
        print_expr(&if_expr).expect("prints"),
        "if c {\n    1\n} else {\n    2\n}"
    );

    let while_expr = ex(
        fspan(0, 10),
        ExprKind::While(Box::new(var("c")), empty_block()),
    );
    assert_eq!(print_expr(&while_expr).expect("prints"), "while c {}");

    let for_expr = ex(
        fspan(0, 12),
        ExprKind::For(
            Box::new(binding_pat("i")),
            Box::new(range(lit_int(0), lit_int(10))),
            empty_block(),
        ),
    );
    assert_eq!(print_expr(&for_expr).expect("prints"), "for i in 0..10 {}");

    let loop_expr = ex(fspan(0, 6), ExprKind::Loop(empty_block()));
    assert_eq!(print_expr(&loop_expr).expect("prints"), "loop {}");

    let match_expr = ex(
        fspan(0, 30),
        ExprKind::Match(
            Box::new(var("s")),
            vec![
                Arm {
                    pat: binding_pat("a"),
                    body: lit_int(1),
                    span: fspan(0, 10),
                },
                Arm {
                    pat: Pat {
                        kind: PatKind::Wildcard,
                        span: fspan(0, 1),
                    },
                    body: lit_int(2),
                    span: fspan(0, 10),
                },
            ],
        ),
    );
    assert_eq!(
        print_expr(&match_expr).expect("prints"),
        "match s {\n    a => 1,\n    _ => 2\n}"
    );

    let empty_match = ex(
        fspan(0, 10),
        ExprKind::Match(Box::new(var("s")), Vec::new()),
    );
    assert_eq!(print_expr(&empty_match).expect("prints"), "match s {}");
}

#[test]
fn string_literal_expression_prints_exactly() {
    let mut s = Spans::new();
    let inner = s.path_expr(&["total"]);
    let expr = s.string_interp("total = ", inner);
    assert_eq!(print_expr(&expr).expect("prints"), "\"total = {total}\"");
}

#[test]
fn let_statements_print_exactly() {
    let let_stmt = Stmt {
        kind: StmtKind::Let(LetStmt {
            mutable: true,
            pat: binding_pat("total"),
            ty: None,
            init: lit_float(0.0),
            span: fspan(0, 12),
        }),
        span: fspan(0, 13),
    };
    let block = Block {
        stmts: vec![let_stmt],
        tail: None,
        span: fspan(0, 14),
    };
    assert_eq!(
        print_block(&block).expect("prints"),
        "{\n    let mut total = 0.0;\n}"
    );
}

#[test]
fn print_items_joins_with_blank_lines() {
    let items = example_program();
    let printed = print_items(&items).expect("prints");
    assert!(printed.contains("\n\n"));
    assert!(!printed.ends_with('\n'));
    assert!(printed.starts_with("import std.io;"));
}

#[test]
fn syntax_preserving_errors_for_malformed_trees() {
    // Empty path.
    let empty_path = ex(
        fspan(0, 0),
        ExprKind::Path(Path {
            segments: Vec::new(),
            span: fspan(0, 0),
        }),
    );
    assert_eq!(
        print_expr(&empty_path).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );

    // Non-place assignment target.
    let bad_target = assign(AssignOp::Assign, lit_int(1), lit_int(2));
    assert_eq!(
        print_expr(&bad_target).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );

    // Non-block else branch.
    let bad_else = ex(
        fspan(0, 10),
        ExprKind::If(
            Box::new(var("c")),
            empty_block(),
            Some(Box::new(lit_int(1))),
        ),
    );
    assert_eq!(
        print_expr(&bad_else).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );

    // Empty identifier.
    let bad_name = ex(
        fspan(0, 1),
        ExprKind::Path(Path {
            segments: vec![Ident {
                name: String::new(),
                span: fspan(0, 1),
            }],
            span: fspan(0, 1),
        }),
    );
    assert_eq!(
        print_expr(&bad_name).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );

    // Empty generic argument list.
    let bad_generics = Type {
        kind: TypeKind::Path(PathType {
            segments: vec![ident("Vec", 0)],
            generics: Some(Vec::new()),
            span: fspan(0, 3),
        }),
        span: fspan(0, 3),
    };
    assert_eq!(
        print_type(&bad_generics).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );
}

#[test]
fn empty_container_items_print_compactly() {
    let mut s = Spans::new();
    let name = s.ident("S");
    let item = s.item(ItemKind::Struct(StructDecl {
        vis: Visibility::Private,
        name,
        generics: None,
        fields: Vec::new(),
        span: fspan(0, 0),
    }));
    assert_eq!(print_item(&item).expect("prints"), "struct S {}");
}
