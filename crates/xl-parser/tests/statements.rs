//! Suite B: statements, blocks, and the accepted classification order
//! (conformance cases T1–T7).

mod common;

use common::{clean, find_fn, fn_body};
use xl_ast::{ExprKind, StmtKind};

#[test]
fn t1_tail_match_is_the_block_value() {
    let parsed = clean("fn area(s: Shape) -> f64 { match s { Shape::Circle(r) => r } }");
    let body = fn_body(&parsed.output.items, "area");
    assert!(body.stmts.is_empty());
    let tail = body.tail.as_ref().expect("tail");
    assert!(matches!(tail.kind, ExprKind::Match(..)));
}

#[test]
fn t2_tail_if_is_the_block_value() {
    let parsed = clean("fn f() -> i64 { if flag { 1 } else { 2 } }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.stmts.is_empty());
    assert!(matches!(
        body.tail.as_ref().expect("tail").kind,
        ExprKind::If(..)
    ));
}

#[test]
fn t3_tail_block_is_the_block_value() {
    let parsed = clean("fn f() { { g(); } }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.stmts.is_empty());
    assert!(matches!(
        body.tail.as_ref().expect("tail").kind,
        ExprKind::Block(..)
    ));
}

#[test]
fn t4_non_tail_for_is_a_statement_without_semicolon() {
    let parsed = clean("fn f() { for i in 0..10 { p(i); } finish(); }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.tail.is_none());
    assert_eq!(body.stmts.len(), 2);
    match &body.stmts[0].kind {
        StmtKind::Expr(expr) => assert!(matches!(expr.kind, ExprKind::For(..))),
        other => panic!("first statement must be the for loop, got {other:?}"),
    }
    match &body.stmts[1].kind {
        StmtKind::Expr(expr) => assert!(matches!(expr.kind, ExprKind::Call(..))),
        other => panic!("second statement must be the call, got {other:?}"),
    }
}

#[test]
fn t5_block_like_with_semicolon_is_a_statement() {
    let parsed = clean("fn f() { match s { A => 1 }; }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.tail.is_none());
    assert_eq!(body.stmts.len(), 1);
    match &body.stmts[0].kind {
        StmtKind::Expr(expr) => assert!(matches!(expr.kind, ExprKind::Match(..))),
        other => panic!("must be the match statement, got {other:?}"),
    }
}

#[test]
fn t6_ordinary_tail_call() {
    let parsed = clean("fn f() -> i64 { g() }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.stmts.is_empty());
    assert!(matches!(
        body.tail.as_ref().expect("tail").kind,
        ExprKind::Call(..)
    ));
}

#[test]
fn t7_missing_semicolon_reports_once_and_recovers() {
    let parsed = common::parse_source("fn f() { g() h() }");
    assert_eq!(parsed.messages(), vec!["expected ';'"]);
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.tail.is_none(), "recovery must not fabricate a tail");
    assert_eq!(body.stmts.len(), 1);
}

#[test]
fn let_forms() {
    let parsed =
        clean("fn f() { let a = 1; let mut b = 2; let c: i64 = 3; let mut (x, y) = pair; }");
    let body = fn_body(&parsed.output.items, "f");
    assert_eq!(body.stmts.len(), 4);
    let mutability: Vec<bool> = body
        .stmts
        .iter()
        .map(|stmt| match &stmt.kind {
            StmtKind::Let(let_stmt) => let_stmt.mutable,
            _ => panic!("let"),
        })
        .collect();
    assert_eq!(mutability, vec![false, true, false, true]);
    match &body.stmts[2].kind {
        StmtKind::Let(let_stmt) => assert!(let_stmt.ty.is_some()),
        _ => unreachable!(),
    }
    match &body.stmts[3].kind {
        StmtKind::Let(let_stmt) => {
            assert!(matches!(let_stmt.pat.kind, xl_ast::PatKind::Tuple(_)));
        }
        _ => unreachable!(),
    }
}

#[test]
fn let_without_initializer_reports_expected_eq() {
    let parsed = common::parse_source("fn f() { let x }");
    assert_eq!(parsed.messages(), vec!["expected '='", "expected ';'"]);
}

#[test]
fn expression_statement_semicolon_rules() {
    let parsed = clean("fn f() { g(); h() }");
    let body = fn_body(&parsed.output.items, "f");
    assert_eq!(body.stmts.len(), 1);
    assert!(body.tail.is_some());
}

#[test]
fn block_like_statements_need_no_semicolon() {
    for source in [
        "fn f() { while a { b(); } c(); }",
        "fn f() { loop { break; } c(); }",
        "fn f() { { g(); } h(); }",
        "fn f() { if a { b(); } c(); }",
    ] {
        let parsed = clean(source);
        let body = fn_body(&parsed.output.items, "f");
        assert!(body.tail.is_none(), "{source}");
        assert_eq!(body.stmts.len(), 2, "{source}");
    }
}

#[test]
fn semicolon_only_statement_is_reported() {
    let parsed = common::parse_source("fn f() { ;; }");
    assert_eq!(
        parsed.messages(),
        vec!["expected expression", "expected expression"]
    );
}

#[test]
fn item_statement_then_usage() {
    let parsed = clean("fn f() { fn g() -> i64 { 1 } g() }");
    let body = fn_body(&parsed.output.items, "f");
    assert_eq!(body.stmts.len(), 1);
    assert!(matches!(body.stmts[0].kind, StmtKind::Item(_)));
    assert!(body.tail.is_some());
}

#[test]
fn nested_blocks_parse_recursively() {
    let parsed = clean("fn f() { { { g(); } } }");
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Block(inner) = &tail.kind else {
        panic!("block");
    };
    let inner_tail = inner.tail.as_ref().expect("inner tail");
    assert!(matches!(inner_tail.kind, ExprKind::Block(..)));
}

#[test]
fn while_loop_forms() {
    let parsed = clean("fn f() { while n < 10 { n += 1; } }");
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::While(cond, block) = &tail.kind else {
        panic!("while");
    };
    assert!(matches!(cond.kind, ExprKind::Binary(..)));
    assert_eq!(block.stmts.len(), 1);
}

#[test]
fn break_and_continue_are_statements() {
    let parsed = clean("fn f() { loop { if a { break; } continue; } }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.tail.is_some());
}

#[test]
fn return_with_and_without_operand() {
    let parsed = clean("fn f() { return; } fn g() { return n < 0; } fn h() { return a + b * c; }");
    let body_f = fn_body(&parsed.output.items, "f");
    match &body_f.stmts[0].kind {
        StmtKind::Expr(expr) => {
            let ExprKind::Return(operand) = &expr.kind else {
                panic!("return");
            };
            assert!(operand.is_none());
        }
        _ => unreachable!(),
    }
    let body_g = fn_body(&parsed.output.items, "g");
    match &body_g.stmts[0].kind {
        StmtKind::Expr(expr) => {
            let ExprKind::Return(operand) = &expr.kind else {
                panic!("return");
            };
            let operand = operand.as_ref().expect("operand");
            assert!(matches!(
                operand.kind,
                ExprKind::Binary(xl_ast::BinOp::Lt, _, _)
            ));
        }
        _ => unreachable!(),
    }
    let body_h = fn_body(&parsed.output.items, "h");
    match &body_h.stmts[0].kind {
        StmtKind::Expr(expr) => {
            let ExprKind::Return(operand) = &expr.kind else {
                panic!("return");
            };
            let operand = operand.as_ref().expect("operand");
            assert!(matches!(
                operand.kind,
                ExprKind::Binary(xl_ast::BinOp::Add, _, _)
            ));
        }
        _ => unreachable!(),
    }
}

#[test]
fn return_operand_does_not_cross_statement_boundary() {
    let parsed = common::parse_source("fn f() { return x y }");
    assert_eq!(parsed.messages(), vec!["expected ';'"]);
    let body = fn_body(&parsed.output.items, "f");
    match &body.stmts[0].kind {
        StmtKind::Expr(expr) => {
            let ExprKind::Return(operand) = &expr.kind else {
                panic!("return");
            };
            assert!(operand.is_some());
        }
        _ => unreachable!(),
    }
}

#[test]
fn find_fn_helper_is_used() {
    assert!(find_fn(&clean("fn x() {}").output.items, "x").is_some());
    assert!(find_fn(&clean("fn x() {}").output.items, "y").is_none());
}
