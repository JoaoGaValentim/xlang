//! Snapshot tests (insta) for canonical printing.
//!
//! Every snapshot change must correspond to an intentional contract change.

mod common;

use common::*;
use xl_ast::*;

#[test]
fn snapshot_canonical_example_module() {
    let items = example_program();
    insta::assert_snapshot!(
        "canonical_example_module",
        print_items(&items).expect("prints")
    );
}

#[test]
fn snapshot_precedence_battery() {
    let cases: Vec<Expr> = vec![
        bin(BinOp::Add, var("a"), bin(BinOp::Mul, var("b"), var("c"))),
        bin(BinOp::Mul, bin(BinOp::Add, var("a"), var("b")), var("c")),
        bin(BinOp::Sub, var("a"), bin(BinOp::Sub, var("b"), var("c"))),
        bin(BinOp::Or, var("a"), bin(BinOp::And, var("b"), var("c"))),
        bin(BinOp::Lt, bin(BinOp::Lt, var("a"), var("b")), var("c")),
        range(range(var("a"), var("b")), var("c")),
        assign(
            AssignOp::Assign,
            var("a"),
            assign(AssignOp::Assign, var("b"), var("c")),
        ),
        un(UnOp::Neg, bin(BinOp::Mul, var("x"), var("y"))),
        cast(bin(BinOp::Add, var("a"), var("b")), ty_path("f64")),
        try_expr(call(field(var("s"), "to_int"), Vec::new())),
        ret(Some(bin(BinOp::Add, var("a"), var("b")))),
        bin(BinOp::Add, ret(Some(var("x"))), lit_int(1)),
    ];
    let printed: Vec<String> = cases
        .iter()
        .map(|expr| print_expr(expr).expect("prints"))
        .collect();
    insta::assert_snapshot!("precedence_battery", printed.join("\n"));
}

#[test]
fn snapshot_block_classification_battery() {
    let for_stmt = expr_stmt(ex(
        fspan(0, 10),
        ExprKind::For(
            Box::new(binding_pat("i")),
            Box::new(range(lit_int(0), lit_int(10))),
            empty_block(),
        ),
    ));
    let if_tail = ex(
        fspan(0, 10),
        ExprKind::If(
            Box::new(var("c")),
            block_with(Vec::new(), Some(lit_int(1))),
            None,
        ),
    );
    let cases = [
        block_with(vec![for_stmt.clone()], None),
        block_with(vec![for_stmt], Some(call(var("finish"), Vec::new()))),
        block_with(Vec::new(), Some(if_tail)),
    ];
    let printed: Vec<String> = cases
        .iter()
        .map(|block| print_block(block).expect("prints"))
        .collect();
    insta::assert_snapshot!("block_classification_battery", printed.join("\n---\n"));
}
