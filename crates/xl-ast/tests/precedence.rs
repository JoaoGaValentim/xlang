//! Suite E — precedence, associativity, and required parentheses.

mod common;

use common::*;
use xl_ast::*;

#[test]
fn multiplicative_binds_tighter_than_additive() {
    // a + b * c
    let tree = bin(BinOp::Add, var("a"), bin(BinOp::Mul, var("b"), var("c")));
    assert_eq!(print_expr(&tree).expect("prints"), "a + b * c");
    // (a + b) * c must keep its parentheses.
    let grouped = bin(BinOp::Mul, bin(BinOp::Add, var("a"), var("b")), var("c"));
    assert_eq!(print_expr(&grouped).expect("prints"), "(a + b) * c");
}

#[test]
fn left_associative_tiers_omit_left_parentheses_but_keep_right_ones() {
    // (a - b) - c
    let left = bin(BinOp::Sub, bin(BinOp::Sub, var("a"), var("b")), var("c"));
    assert_eq!(print_expr(&left).expect("prints"), "a - b - c");
    // a - (b - c) must be parenthesized on the right.
    let right = bin(BinOp::Sub, var("a"), bin(BinOp::Sub, var("b"), var("c")));
    assert_eq!(print_expr(&right).expect("prints"), "a - (b - c)");
    // Same for addition: a + (b + c) is a different tree from (a + b) + c.
    let add_right = bin(BinOp::Add, var("a"), bin(BinOp::Add, var("b"), var("c")));
    assert_eq!(print_expr(&add_right).expect("prints"), "a + (b + c)");
}

#[test]
fn logical_precedence_matches_the_table() {
    // a || (b && c)
    let tree = bin(BinOp::Or, var("a"), bin(BinOp::And, var("b"), var("c")));
    assert_eq!(print_expr(&tree).expect("prints"), "a || b && c");
    // (a || b) && c keeps parentheses.
    let grouped = bin(BinOp::And, bin(BinOp::Or, var("a"), var("b")), var("c"));
    assert_eq!(print_expr(&grouped).expect("prints"), "(a || b) && c");
}

#[test]
fn comparison_is_non_associative_and_nested_trees_are_parenthesized() {
    let left_nested = bin(BinOp::Lt, bin(BinOp::Lt, var("a"), var("b")), var("c"));
    assert_eq!(print_expr(&left_nested).expect("prints"), "(a < b) < c");
    let right_nested = bin(BinOp::Eq, var("a"), bin(BinOp::Ne, var("b"), var("c")));
    assert_eq!(print_expr(&right_nested).expect("prints"), "a == (b != c)");
}

#[test]
fn range_is_non_associative_and_below_logical_tiers() {
    let nested = range(range(var("a"), var("b")), var("c"));
    assert_eq!(print_expr(&nested).expect("prints"), "(a..b)..c");
    let inner = range(var("a"), range(var("b"), var("c")));
    assert_eq!(print_expr(&inner).expect("prints"), "a..(b..c)");
    // Additive operands are not parenthesized: 0..(n + 1) prints bare.
    let additive = range(lit_int(0), bin(BinOp::Add, var("n"), lit_int(1)));
    assert_eq!(print_expr(&additive).expect("prints"), "0..n + 1");
}

#[test]
fn assignment_is_lowest_and_rhs_nesting_is_parenthesized() {
    let plain = assign(
        AssignOp::AddAssign,
        var("total"),
        cast(var("i"), ty_path("f64")),
    );
    assert_eq!(print_expr(&plain).expect("prints"), "total += i as f64");

    // a = (b = c)
    let nested = assign(
        AssignOp::Assign,
        var("a"),
        assign(AssignOp::Assign, var("b"), var("c")),
    );
    assert_eq!(print_expr(&nested).expect("prints"), "a = (b = c)");

    // A range may ride unparenthesized on the right of an assignment.
    let range_rhs = assign(AssignOp::Assign, var("a"), range(var("b"), var("c")));
    assert_eq!(print_expr(&range_rhs).expect("prints"), "a = b..c");
}

#[test]
fn unary_binds_above_multiplicative() {
    // (-x) * y
    let tree = bin(BinOp::Mul, un(UnOp::Neg, var("x")), var("y"));
    assert_eq!(print_expr(&tree).expect("prints"), "-x * y");
    // -(x * y)
    let grouped = un(UnOp::Neg, bin(BinOp::Mul, var("x"), var("y")));
    assert_eq!(print_expr(&grouped).expect("prints"), "-(x * y)");
    // !(a && b)
    let not_group = un(UnOp::Not, bin(BinOp::And, var("a"), var("b")));
    assert_eq!(print_expr(&not_group).expect("prints"), "!(a && b)");
    // Prefix chains need no parentheses.
    let not_not = un(UnOp::Not, un(UnOp::Not, var("x")));
    assert_eq!(print_expr(&not_not).expect("prints"), "!!x");
}

#[test]
fn postfix_chains_print_left_to_right_without_parens() {
    // s.to_int()?
    let chain = try_expr(call(field(var("s"), "to_int"), Vec::new()));
    assert_eq!(print_expr(&chain).expect("prints"), "s.to_int()?");
    // x.y.z(1, 2)?
    let longer = try_expr(call(
        field(field(var("x"), "y"), "z"),
        vec![lit_int(1), lit_int(2)],
    ));
    assert_eq!(print_expr(&longer).expect("prints"), "x.y.z(1, 2)?");
    // Postfix applies to a parenthesized binary base.
    let grouped = call(field(bin(BinOp::Add, var("a"), var("b")), "f"), Vec::new());
    assert_eq!(print_expr(&grouped).expect("prints"), "(a + b).f()");
    // Casts chain on the left.
    let double_cast = cast(cast(var("x"), ty_path("f64")), ty_path("i64"));
    assert_eq!(print_expr(&double_cast).expect("prints"), "x as f64 as i64");
    // Cast binds tighter than multiplication.
    let cast_mul = bin(BinOp::Mul, cast(var("i"), ty_path("f64")), lit_float(2.0));
    assert_eq!(print_expr(&cast_mul).expect("prints"), "i as f64 * 2.0");
}

#[test]
fn return_operand_extends_maximally() {
    // return a + b (the operand is the whole binary expression).
    let root = ret(Some(bin(BinOp::Add, var("a"), var("b"))));
    assert_eq!(print_expr(&root).expect("prints"), "return a + b");
    // A return nested as an operand is parenthesized.
    let nested = bin(BinOp::Add, ret(Some(var("x"))), lit_int(1));
    assert_eq!(print_expr(&nested).expect("prints"), "(return x) + 1");
    // Bare returns nested likewise.
    let nested_bare = bin(BinOp::Add, ret(None), lit_int(1));
    assert_eq!(print_expr(&nested_bare).expect("prints"), "(return) + 1");
}

#[test]
fn block_like_primaries_need_no_parentheses_as_operands() {
    let if_operand = bin(
        BinOp::Add,
        ex(
            fspan(0, 10),
            ExprKind::If(
                Box::new(var("c")),
                block_with(Vec::new(), Some(lit_int(1))),
                None,
            ),
        ),
        lit_int(3),
    );
    assert_eq!(
        print_expr(&if_operand).expect("prints"),
        "if c {\n    1\n} + 3"
    );
}

#[test]
fn comparison_inside_range_operand_is_not_parenthesized() {
    // Range(a, b == c): `==` (tier 5) binds tighter than `..` (tier 2).
    let tree = range(var("a"), bin(BinOp::Eq, var("b"), var("c")));
    assert_eq!(print_expr(&tree).expect("prints"), "a..b == c");
}
