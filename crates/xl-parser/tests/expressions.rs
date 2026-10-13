//! Suite C: Pratt expressions — precedence, associativity, non-associative
//! chain diagnostics, and assignment targets.

mod common;

use common::{clean, fn_body, parse_source};
use xl_ast::{AssignOp, BinOp, ExprKind, StmtKind, UnOp};

/// Parses `let v = <source>;` into the initializer's kind (clean parse).
fn init(source: &str) -> ExprKind {
    let full = format!("fn f() {{ let v = {source}; }}");
    let parsed = clean(&full);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let statement expected");
    };
    let_stmt.init.kind.clone()
}

/// Parses `let v = <source>;` without requiring a clean parse.
fn init_raw(source: &str) -> (Vec<String>, ExprKind) {
    let full = format!("fn f() {{ let v = {source}; }}");
    let parsed = parse_source(&full);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let statement expected");
    };
    (parsed.messages(), let_stmt.init.kind.clone())
}

fn binary(kind: &ExprKind) -> (BinOp, &ExprKind, &ExprKind) {
    match kind {
        ExprKind::Binary(op, lhs, rhs) => (*op, &lhs.kind, &rhs.kind),
        other => panic!("expected binary, got {other:?}"),
    }
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    let kind = init("a + b * c");
    let (op, lhs, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Add);
    assert!(matches!(lhs, ExprKind::Path(_)));
    let (inner, _, _) = binary(rhs);
    assert_eq!(inner, BinOp::Mul);
}

#[test]
fn subtraction_is_left_associative() {
    let kind = init("a - b - c");
    let (op, lhs, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Sub);
    assert!(matches!(rhs, ExprKind::Path(_)), "right-most must be c");
    let (inner, _, _) = binary(lhs);
    assert_eq!(inner, BinOp::Sub);
}

#[test]
fn explicit_grouping_preserves_structure() {
    let kind = init("a - (b - c)");
    let (op, _, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Sub);
    let (inner, _, _) = binary(rhs);
    assert_eq!(inner, BinOp::Sub, "grouped subtraction stays on the right");
}

#[test]
fn logical_and_binds_tighter_than_or() {
    let kind = init("a || b && c");
    let (op, _, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Or);
    let (inner, _, _) = binary(rhs);
    assert_eq!(inner, BinOp::And);
}

#[test]
fn not_binds_tighter_than_and() {
    let kind = init("!a && b");
    let (op, lhs, _) = binary(&kind);
    assert_eq!(op, BinOp::And);
    match lhs {
        ExprKind::Unary(UnOp::Not, _) => {}
        other => panic!("expected unary not on the left, got {other:?}"),
    }
}

#[test]
fn range_operand_extends_over_additive() {
    let kind = init("0..n + 1");
    match kind {
        ExprKind::Range(lhs, rhs) => {
            assert!(matches!(lhs.kind, ExprKind::Lit(_)));
            let (op, _, _) = binary(&rhs.kind);
            assert_eq!(op, BinOp::Add);
        }
        other => panic!("expected range, got {other:?}"),
    }
}

#[test]
fn compound_assignment_with_cast() {
    let kind = init("total += i as f64");
    match kind {
        ExprKind::Assign(op, lhs, rhs) => {
            assert_eq!(op, AssignOp::AddAssign);
            assert!(matches!(lhs.kind, ExprKind::Path(_)));
            assert!(matches!(rhs.kind, ExprKind::Cast(..)));
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn cast_binds_within_multiplicative() {
    let kind = init("total * i as f64");
    let (op, lhs, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Mul);
    assert!(matches!(lhs, ExprKind::Path(_)));
    assert!(matches!(rhs, ExprKind::Cast(..)), "cast must bind to `i`");
}

#[test]
fn unary_minus_binds_before_multiplication() {
    let kind = init("-x * y");
    let (op, lhs, _) = binary(&kind);
    assert_eq!(op, BinOp::Mul);
    assert!(matches!(lhs, ExprKind::Unary(UnOp::Neg, _)));
}

#[test]
fn try_applies_to_call_result() {
    let kind = init("f(x)?");
    match kind {
        ExprKind::Try(inner) => assert!(matches!(inner.kind, ExprKind::Call(..))),
        other => panic!("expected try, got {other:?}"),
    }
}

#[test]
fn try_then_addition() {
    let kind = init("s.to_int()? + 1");
    let (op, lhs, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Add);
    assert!(matches!(lhs, ExprKind::Try(_)), "try must bind to the call");
    assert!(matches!(rhs, ExprKind::Lit(_)));
}

#[test]
fn assignment_rhs_extends_over_additive() {
    let kind = init("a = b + c");
    match kind {
        ExprKind::Assign(op, lhs, rhs) => {
            assert_eq!(op, AssignOp::Assign);
            assert!(matches!(lhs.kind, ExprKind::Path(_)));
            let (inner, _, _) = binary(&rhs.kind);
            assert_eq!(inner, BinOp::Add);
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn or_with_not_operand() {
    let kind = init("a && b || !c");
    let (op, lhs, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Or);
    let (inner, _, _) = binary(lhs);
    assert_eq!(inner, BinOp::And);
    assert!(matches!(rhs, ExprKind::Unary(UnOp::Not, _)));
}

#[test]
fn remainder_is_multiplicative() {
    let kind = init("w % 2");
    let (op, _, _) = binary(&kind);
    assert_eq!(op, BinOp::Rem);
}

#[test]
fn division_is_multiplicative() {
    let kind = init("a / b / c");
    let (op, lhs, _) = binary(&kind);
    assert_eq!(op, BinOp::Div);
    let (inner, _, _) = binary(lhs);
    assert_eq!(inner, BinOp::Div, "division is left-associative");
}

#[test]
fn postfix_chain_is_left_to_right() {
    let kind = init("x.y.z(1, 2)?");
    match kind {
        ExprKind::Try(inner) => match inner.kind {
            ExprKind::Call(callee, args) => {
                assert_eq!(args.len(), 2);
                match callee.kind {
                    ExprKind::Field(base, name) => {
                        assert_eq!(name.name, "z");
                        assert!(matches!(base.kind, ExprKind::Field(..)));
                    }
                    other => panic!("expected field, got {other:?}"),
                }
            }
            other => panic!("expected call, got {other:?}"),
        },
        other => panic!("expected try, got {other:?}"),
    }
}

#[test]
fn index_and_field_access() {
    let kind = init("v[i].name");
    match kind {
        ExprKind::Field(base, name) => {
            assert_eq!(name.name, "name");
            assert!(matches!(base.kind, ExprKind::Index(..)));
        }
        other => panic!("expected field access, got {other:?}"),
    }
}

#[test]
fn all_comparison_operators_apply() {
    for (source, expected) in [
        ("a == b", BinOp::Eq),
        ("a != b", BinOp::Ne),
        ("a < b", BinOp::Lt),
        ("a <= b", BinOp::Le),
        ("a > b", BinOp::Gt),
        ("a >= b", BinOp::Ge),
    ] {
        let kind = init(source);
        let (op, _, _) = binary(&kind);
        assert_eq!(op, expected, "{source}");
    }
}

#[test]
fn cast_does_not_swallow_comparison() {
    let kind = init("x as i64 > y");
    let (op, lhs, _) = binary(&kind);
    assert_eq!(op, BinOp::Gt, "`>` after a cast stays a comparison");
    assert!(matches!(lhs, ExprKind::Cast(..)));
}

#[test]
fn chained_comparison_is_diagnosed_with_left_nesting() {
    let (diagnostics, kind) = init_raw("a < b < c");
    assert_eq!(
        diagnostics,
        vec!["comparison operators cannot be chained; use parentheses"]
    );
    let (op, lhs, rhs) = binary(&kind);
    assert_eq!(op, BinOp::Lt);
    assert!(matches!(rhs, ExprKind::Path(_)), "outer right side is c");
    let (inner, _, _) = binary(lhs);
    assert_eq!(inner, BinOp::Lt, "left-nested recovery");
}

#[test]
fn chained_range_is_diagnosed_with_left_nesting() {
    let (diagnostics, kind) = init_raw("a..b..c");
    assert_eq!(
        diagnostics,
        vec!["range cannot be chained; use parentheses"]
    );
    match kind {
        ExprKind::Range(lhs, rhs) => {
            assert!(matches!(rhs.kind, ExprKind::Path(_)));
            assert!(matches!(lhs.kind, ExprKind::Range(..)), "left-nested");
        }
        other => panic!("expected range, got {other:?}"),
    }
}

#[test]
fn chained_assignment_is_diagnosed_with_right_nesting() {
    let (diagnostics, kind) = init_raw("a = b = c");
    assert_eq!(
        diagnostics,
        vec!["chained assignment; parenthesize the right-hand side"]
    );
    match kind {
        ExprKind::Assign(op, lhs, rhs) => {
            assert_eq!(op, AssignOp::Assign);
            assert!(matches!(lhs.kind, ExprKind::Path(_)));
            match rhs.kind {
                ExprKind::Assign(inner_op, inner_lhs, inner_rhs) => {
                    assert_eq!(inner_op, AssignOp::Assign);
                    assert!(matches!(inner_lhs.kind, ExprKind::Path(_)));
                    assert!(matches!(inner_rhs.kind, ExprKind::Path(_)));
                }
                other => panic!("expected right-nested assignment, got {other:?}"),
            }
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn triple_comparison_chain_reports_each_extra_operator() {
    let (diagnostics, _) = init_raw("a < b < c < d");
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
}

#[test]
fn grouped_non_associative_operations_are_accepted() {
    for source in ["(a < b) < c", "(a..b)..c", "a = (b = c)"] {
        let full = format!("fn f() {{ let v = {source}; }}");
        let parsed = clean(&full);
        let _ = parsed;
    }
}

#[test]
fn invalid_assignment_targets_are_reported() {
    for source in ["42 = x", "f() = x", "(x + y) = 1", "1 < 2 = x", "-x = 1"] {
        let full = format!("fn f() {{ {source}; }}");
        let parsed = parse_source(&full);
        assert!(
            parsed
                .messages()
                .iter()
                .any(|message| message == "invalid assignment target"),
            "{source}: {:?}",
            parsed.messages()
        );
    }
}

#[test]
fn grouped_assignment_targets_are_transparent() {
    let parsed = clean("fn f() { (x) = 1; ((x)) = 1; (o.f) = 1; (v[i]) = 1; }");
    let body = fn_body(&parsed.output.items, "f");
    assert_eq!(body.stmts.len(), 4);
    for stmt in &body.stmts {
        match &stmt.kind {
            StmtKind::Expr(expr) => {
                assert_eq!(common::assign_op(expr), Some(AssignOp::Assign));
            }
            other => panic!("assignment statement expected, got {other:?}"),
        }
    }
}

#[test]
fn grouped_target_span_covers_only_inner_tokens() {
    let source = "fn f() { (x) = 1; }";
    let parsed = clean(source);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Expr(expr) = &body.stmts[0].kind else {
        panic!("expr statement");
    };
    let ExprKind::Assign(_, target, _) = &expr.kind else {
        panic!("assign");
    };
    let slice = parsed
        .sources
        .span_slice(target.span)
        .expect("resolvable span");
    assert_eq!(slice, "x", "grouping must not extend the target span");
}

#[test]
fn tuple_unit_and_array_expressions() {
    let kind = init("()");
    assert!(matches!(kind, ExprKind::Tuple(ref items) if items.is_empty()));

    let kind = init("(1,)");
    match kind {
        ExprKind::Tuple(items) => assert_eq!(items.len(), 1),
        other => panic!("one-tuple expected, got {other:?}"),
    }

    let kind = init("(1, 2.5)");
    match kind {
        ExprKind::Tuple(items) => assert_eq!(items.len(), 2),
        other => panic!("tuple expected, got {other:?}"),
    }

    let kind = init("[1, 2, 3]");
    match kind {
        ExprKind::Array(items) => assert_eq!(items.len(), 3),
        other => panic!("array expected, got {other:?}"),
    }

    let kind = init("[]");
    match kind {
        ExprKind::Array(items) => assert!(items.is_empty()),
        other => panic!("empty array expected, got {other:?}"),
    }

    let kind = init("(nested)");
    assert!(matches!(kind, ExprKind::Path(_)), "grouping is transparent");

    let kind = init("((deep))");
    assert!(matches!(kind, ExprKind::Path(_)));
}

#[test]
fn nested_unary_operators() {
    let kind = init("-!a");
    match kind {
        ExprKind::Unary(UnOp::Neg, inner) => {
            assert!(matches!(inner.kind, ExprKind::Unary(UnOp::Not, _)));
        }
        other => panic!("expected nested unary, got {other:?}"),
    }
}

#[test]
fn path_expressions_with_namespace_separator() {
    let kind = init("a::b::c");
    match kind {
        ExprKind::Path(path) => {
            assert_eq!(path.segments.len(), 3);
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn all_compound_assignment_operators() {
    for (source, expected) in [
        ("x += 1", AssignOp::AddAssign),
        ("x -= 1", AssignOp::SubAssign),
        ("x *= 2", AssignOp::MulAssign),
        ("x /= 3", AssignOp::DivAssign),
        ("x %= 4", AssignOp::RemAssign),
    ] {
        let full = format!("fn f() {{ {source}; }}");
        let parsed = clean(&full);
        let body = fn_body(&parsed.output.items, "f");
        let StmtKind::Expr(expr) = &body.stmts[0].kind else {
            panic!("expr statement");
        };
        assert_eq!(common::assign_op(expr), Some(expected), "{source}");
    }
}
