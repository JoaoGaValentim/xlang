//! Suite H — determinism: identical trees always print identical output.

mod common;

use common::*;
use xl_ast::*;

fn battery() -> Vec<Expr> {
    vec![
        bin(BinOp::Add, bin(BinOp::Mul, var("a"), var("b")), var("c")),
        un(UnOp::Neg, bin(BinOp::Sub, var("x"), lit_int(1))),
        assign(
            AssignOp::AddAssign,
            var("total"),
            cast(var("i"), ty_path("f64")),
        ),
        range(lit_int(0), bin(BinOp::Add, var("n"), lit_int(1))),
        try_expr(call(field(var("s"), "to_int"), Vec::new())),
        ex(
            fspan(0, 10),
            ExprKind::Match(
                Box::new(var("s")),
                vec![Arm {
                    pat: binding_pat("a"),
                    body: lit_int(1),
                    span: fspan(0, 5),
                }],
            ),
        ),
    ]
}

#[test]
fn repeated_expression_prints_are_identical() {
    for expr in battery() {
        let first = print_expr(&expr).expect("prints");
        let second = print_expr(&expr).expect("prints");
        assert_eq!(first, second);
    }
}

#[test]
fn repeated_item_prints_are_identical() {
    let items = example_program();
    let first = print_items(&items).expect("prints");
    let second = print_items(&items).expect("prints");
    assert_eq!(first, second);
}

#[test]
fn equivalent_constructions_produce_identical_output() {
    // Same tree built through different helper paths.
    let via_direct = bin(BinOp::Add, lit_int(1), var("x"));
    let mut s = Spans::new();
    let one = s.int_expr(1, Radix::Dec);
    let x = s.path_expr(&["x"]);
    let via_cursor = ex_cover(
        &[one.span, x.span],
        ExprKind::Binary(BinOp::Add, Box::new(one), Box::new(x)),
    );
    assert_eq!(
        print_expr(&via_direct).expect("prints"),
        print_expr(&via_cursor).expect("prints")
    );
}

#[test]
fn printing_is_independent_of_construction_order() {
    let tree_a = bin(BinOp::Add, var("a"), bin(BinOp::Mul, var("b"), var("c")));
    // Build the same shape bottom-up in a different statement order.
    let mul = bin(BinOp::Mul, var("b"), var("c"));
    let add = bin(BinOp::Add, var("a"), mul);
    assert_eq!(
        print_expr(&tree_a).expect("prints"),
        print_expr(&add).expect("prints")
    );
}
