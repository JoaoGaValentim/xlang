//! Suite I — depth limits, malformed structures, and drop behavior.

mod common;

use common::*;
use xl_ast::*;

fn deep_neg(depth: usize) -> Expr {
    let mut expr = lit_int(1);
    for _ in 0..depth {
        let span = expr.span;
        expr = ex(span, ExprKind::Unary(UnOp::Neg, Box::new(expr)));
    }
    expr
}

#[test]
fn printing_fails_cleanly_beyond_the_depth_limit() {
    let expr = deep_neg(200);
    let err = print_expr(&expr).expect_err("too deep");
    assert_eq!(err.kind(), PrintErrorKind::DepthLimitExceeded);
    assert!(err.message().contains("128"));
}

#[test]
fn validation_reports_depth_violations() {
    let expr = deep_neg(200);
    let violations = validate_expr(&expr);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::DepthExceeded)
    );
}

#[test]
fn boundary_depth_is_exactly_128_nodes() {
    // deep_neg(n) has n + 1 expression nodes; 128 nodes fit, 129 do not.
    assert!(print_expr(&deep_neg(127)).is_ok());
    assert_eq!(
        print_expr(&deep_neg(128)).expect_err("too deep").kind(),
        PrintErrorKind::DepthLimitExceeded
    );
    assert!(validate_expr(&deep_neg(127)).is_empty());
    assert!(
        validate_expr(&deep_neg(128))
            .iter()
            .any(|v| v.kind == ViolationKind::DepthExceeded)
    );
}

#[test]
fn very_deep_trees_print_validate_and_drop_inside_a_bounded_stack() {
    // 10k-deep manual trees are far beyond any parser output; run them on
    // an explicitly sized stack so the test documents (rather than assumes)
    // the depth-bounded behavior of print/validate plus recursive drop.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let expr = deep_neg(10_000);
            let err = print_expr(&expr).expect_err("too deep");
            assert_eq!(err.kind(), PrintErrorKind::DepthLimitExceeded);
            assert!(
                validate_expr(&expr)
                    .iter()
                    .any(|v| v.kind == ViolationKind::DepthExceeded)
            );
            drop(expr);
        })
        .expect("spawn succeeds")
        .join()
        .expect("no panic inside the deep-tree thread");
}

#[test]
fn moderately_deep_trees_drop_on_the_default_test_thread() {
    let expr = deep_neg(1_000);
    assert!(print_expr(&expr).is_err());
    drop(expr);
}

#[test]
fn malformed_trees_return_errors_without_panicking() {
    // Empty import path.
    let bad_import = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Import(ImportDecl {
            path: Vec::new(),
            alias: None,
            span: fspan(0, 0),
        }),
        span: fspan(0, 0),
    };
    assert_eq!(
        print_item(&bad_import).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );

    // Empty generic parameter list on a function.
    let sig = sig_ex(ident("f", 0), Vec::new(), None);
    let bad_generics = FnDecl {
        vis: Visibility::Private,
        generics: Some(Generics {
            params: Vec::new(),
            span: fspan(0, 0),
        }),
        sig,
        body: empty_block(),
        span: fspan(0, 10),
    };
    let item = Item {
        id: ItemId::from_index(0),
        kind: ItemKind::Fn(bad_generics),
        span: fspan(0, 10),
    };
    assert_eq!(
        print_item(&item).expect_err("must fail").kind(),
        PrintErrorKind::InvalidStructure
    );

    // Non-finite float inside a literal pattern.
    let bad_pattern = Pat {
        kind: PatKind::Lit(Lit::Float(f64::NAN)),
        span: fspan(0, 3),
    };
    assert_eq!(
        print_pattern(&bad_pattern).expect_err("must fail").kind(),
        PrintErrorKind::NonFiniteFloat
    );
}

#[test]
fn deep_valid_tree_at_limit_prints_successfully() {
    // A 128-node chain is inside the documented limit; its output is a
    // deterministic dash prefix, not an error.
    let printed = print_expr(&deep_neg(127)).expect("prints at the limit");
    assert!(printed.starts_with('-'));
    assert!(printed.ends_with('1'));
}
