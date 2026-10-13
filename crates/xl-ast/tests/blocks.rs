//! Suite F — block structure: tails, statements, and classification
//! preservation.

mod common;

use common::*;
use xl_ast::*;

fn if_one() -> Expr {
    ex(
        fspan(0, 10),
        ExprKind::If(
            Box::new(var("flag")),
            block_with(Vec::new(), Some(lit_int(1))),
            Some(Box::new(ex(
                fspan(0, 6),
                ExprKind::Block(block_with(Vec::new(), Some(lit_int(2)))),
            ))),
        ),
    )
}

#[test]
fn tail_if_is_value_and_tail_none_gets_no_semicolon() {
    let with_tail = block_with(Vec::new(), Some(if_one()));
    let printed = print_block(&with_tail).expect("prints");
    assert_eq!(
        printed,
        "{\n    if flag {\n        1\n    } else {\n        2\n    }\n}"
    );
}

#[test]
fn block_like_statement_followed_by_tail_has_no_semicolon() {
    // stmts: [for ...], tail: call
    let for_stmt = expr_stmt(ex(
        fspan(0, 12),
        ExprKind::For(
            Box::new(binding_pat("i")),
            Box::new(range(lit_int(0), lit_int(10))),
            empty_block(),
        ),
    ));
    let tail = call(var("finish"), Vec::new());
    let block = block_with(vec![for_stmt], Some(tail));
    assert_eq!(
        print_block(&block).expect("prints"),
        "{\n    for i in 0..10 {}\n    finish()\n}"
    );
}

#[test]
fn final_block_like_statement_without_tail_gets_a_semicolon() {
    // A trailing `for` statement with tail == None must print `;` so that
    // re-parsing classifies it as a statement, not a tail.
    let for_stmt = expr_stmt(ex(
        fspan(0, 12),
        ExprKind::For(
            Box::new(binding_pat("i")),
            Box::new(range(lit_int(0), lit_int(10))),
            empty_block(),
        ),
    ));
    let block = block_with(vec![for_stmt], None);
    assert_eq!(
        print_block(&block).expect("prints"),
        "{\n    for i in 0..10 {};\n}"
    );
}

#[test]
fn every_block_like_kind_gets_the_semicolon_rule() {
    let block_like: Vec<Expr> = vec![
        if_one(),
        ex(fspan(0, 6), ExprKind::Loop(empty_block())),
        ex(
            fspan(0, 8),
            ExprKind::While(Box::new(var("c")), empty_block()),
        ),
        ex(
            fspan(0, 10),
            ExprKind::Match(Box::new(var("s")), Vec::new()),
        ),
        ex(fspan(0, 4), ExprKind::Block(empty_block())),
    ];
    for expr in block_like {
        let stmt = expr_stmt(expr);
        let block = block_with(vec![stmt], None);
        let printed = print_block(&block).expect("prints");
        assert!(
            printed.contains("};"),
            "trailing block-like statement must end with `}};`: {printed:?}"
        );
    }
}

#[test]
fn non_block_like_statements_always_get_semicolons() {
    let stmts = vec![
        expr_stmt(call(var("g"), Vec::new())),
        expr_stmt(assign(AssignOp::Assign, var("x"), lit_int(1))),
    ];
    let block = block_with(stmts, None);
    assert_eq!(
        print_block(&block).expect("prints"),
        "{\n    g();\n    x = 1;\n}"
    );
}

#[test]
fn tail_position_expression_statement_without_semicolon_is_never_tail_reclassified() {
    // Sanity for classification: a plain call in tail position prints bare.
    let block = block_with(
        vec![expr_stmt(call(var("g"), Vec::new()))],
        Some(lit_int(1)),
    );
    assert_eq!(
        print_block(&block).expect("prints"),
        "{\n    g();\n    1\n}"
    );
}

#[test]
fn nested_blocks_indent_correctly() {
    let inner = block_with(vec![expr_stmt(call(var("f"), Vec::new()))], None);
    let outer = block_with(
        vec![expr_stmt(ex(
            fspan(0, 20),
            ExprKind::While(Box::new(var("c")), inner),
        ))],
        None,
    );
    assert_eq!(
        print_block(&outer).expect("prints"),
        "{\n    while c {\n        f();\n    };\n}"
    );
}

#[test]
fn empty_blocks_print_compactly() {
    assert_eq!(print_block(&empty_block()).expect("prints"), "{}");
    let item = fn_item("f", Vec::new(), None, empty_block());
    assert_eq!(print_item(&item).expect("prints"), "fn f() {}");
}

#[test]
fn for_loop_body_statements_print_inside_the_block() {
    let mut s = Spans::new();
    let pat = s.binding("i");
    let start = s.int_expr(0, Radix::Dec);
    let end = s.int_expr(10, Radix::Dec);
    let iter = ex_cover(
        &[start.span, end.span],
        ExprKind::Range(Box::new(start), Box::new(end)),
    );
    let body = {
        let target = s.path_expr(&["total"]);
        let i = s.path_expr(&["i"]);
        let ty = {
            let path = s.path_type(&["f64"], None);
            s.ty(TypeKind::Path(path))
        };
        let cast = {
            let span = cover(i.span, ty.span);
            ex(span, ExprKind::Cast(Box::new(i), ty))
        };
        let assign = ex_cover(
            &[target.span, cast.span],
            ExprKind::Assign(AssignOp::AddAssign, Box::new(target), Box::new(cast)),
        );
        let stmt = s.stmt(StmtKind::Expr(assign));
        s.block(vec![stmt], None)
    };
    let for_expr = ex_cover(
        &[pat.span, iter.span, body.span],
        ExprKind::For(Box::new(pat), Box::new(iter), body),
    );
    assert_eq!(
        print_expr(&for_expr).expect("prints"),
        "for i in 0..10 {\n    total += i as f64;\n}",
        "for body statements stay inside the braces"
    );
}

#[test]
fn spec_patterns_parse_style_match_tail_prints_exactly() {
    // fn area's tail shape: match with derived spans (cursor builder).
    let mut s = Spans::new();
    let scrutinee = s.path_expr(&["s"]);
    let arm1 = {
        let r = s.binding("r");
        let pat = s.path_pat(&["Shape", "Circle"], vec![r]);
        let body = {
            let pi_like: f64 = "3.14159".parse().expect("literal parses");
            let lit = s.float_expr(pi_like);
            let r1 = s.path_expr(&["r"]);
            let first = ex_cover(
                &[lit.span, r1.span],
                ExprKind::Binary(BinOp::Mul, Box::new(lit), Box::new(r1)),
            );
            let r2 = s.path_expr(&["r"]);
            ex_cover(
                &[first.span, r2.span],
                ExprKind::Binary(BinOp::Mul, Box::new(first), Box::new(r2)),
            )
        };
        let span = cover(pat.span, body.span);
        Arm { pat, body, span }
    };
    let arm2 = {
        let w = s.binding("w");
        let h = s.binding("h");
        let pat = s.path_pat(&["Shape", "Rect"], vec![w, h]);
        let body = {
            let w = s.path_expr(&["w"]);
            let h = s.path_expr(&["h"]);
            ex_cover(
                &[w.span, h.span],
                ExprKind::Binary(BinOp::Mul, Box::new(w), Box::new(h)),
            )
        };
        let span = cover(pat.span, body.span);
        Arm { pat, body, span }
    };
    let match_expr = ex_cover(
        &[scrutinee.span, arm1.span, arm2.span],
        ExprKind::Match(Box::new(scrutinee), vec![arm1, arm2]),
    );
    assert_eq!(
        print_expr(&match_expr).expect("prints"),
        "match s {\n    Shape::Circle(r) => 3.14159 * r * r,\n    Shape::Rect(w, h) => w * h\n}"
    );
}
