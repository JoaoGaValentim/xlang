//! Expression and statement lowering coverage: every accepted construct
//! (`Directive 011` §8).

mod common;

use common::{body_of_fn_named, def_named, expr_of, lower_ok, pat_of, stmt_of, tail_of};
use xl_hir::{BuiltinOp, ExprKind, FieldSel, HirLit, PatKind, StmtKind};

#[test]
fn literals_lower_with_their_payloads() {
    let output = lower_ok("fn f() { 1; 1.5; true; 'x'; \"hi\"; 0xFF; 0b101; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let kinds: Vec<&HirLit> = body
        .stmts
        .iter()
        .map(|stmt| match stmt_of(module, *stmt) {
            StmtKind::Expr(expr) => match expr_of(module, *expr) {
                ExprKind::Lit(lit) => lit,
                other => panic!("expected a literal, got {other:?}"),
            },
            other => panic!("expected an expression statement, got {other:?}"),
        })
        .collect();
    assert!(matches!(
        kinds[0],
        HirLit::Int {
            magnitude: 1,
            radix: xl_hir::Radix::Dec
        }
    ));
    assert!(matches!(kinds[1], HirLit::Float(value) if (*value - 1.5).abs() < f64::EPSILON));
    assert!(matches!(kinds[2], HirLit::Bool(true)));
    assert!(matches!(kinds[3], HirLit::Char('x')));
    assert!(matches!(kinds[4], HirLit::Str(string) if string.text == "hi"));
    assert!(matches!(
        kinds[5],
        HirLit::Int {
            magnitude: 255,
            radix: xl_hir::Radix::Hex
        }
    ));
    assert!(matches!(
        kinds[6],
        HirLit::Int {
            magnitude: 5,
            radix: xl_hir::Radix::Bin
        }
    ));
}

#[test]
fn operators_keep_their_identity() {
    let output = lower_ok(
        "fn f() { -1; !true; 1 + 2; 1 == 2; 1 < 2; true && false; true || false; 1 * 2; }",
    );
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let kinds: Vec<String> = body
        .stmts
        .iter()
        .map(|stmt| match stmt_of(module, *stmt) {
            StmtKind::Expr(expr) => format!("{:?}", expr_of(module, *expr)),
            other => panic!("expected an expression statement, got {other:?}"),
        })
        .collect();
    assert!(kinds[0].contains("Unary(Neg"));
    assert!(kinds[1].contains("Unary(Not"));
    assert!(kinds[2].contains("Binary(Add"));
    assert!(kinds[3].contains("Binary(Eq"));
    assert!(kinds[4].contains("Binary(Lt"));
    assert!(kinds[5].contains("Binary(And"));
    assert!(kinds[6].contains("Binary(Or"));
    assert!(kinds[7].contains("Binary(Mul"));
}

#[test]
fn paths_record_one_use_per_segment() {
    let output = lower_ok("fn f() { a::b::c; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Expr(expr) = stmt_of(module, body.stmts[0]) else {
        panic!("expected an expression statement");
    };
    let ExprKind::Path { segments } = expr_of(module, *expr) else {
        panic!("expected a path");
    };
    assert_eq!(segments.len(), 3);
    let names: Vec<&str> = segments
        .iter()
        .map(|use_| module.use_(*use_).unwrap().name.as_str())
        .collect();
    assert_eq!(names, ["a", "b", "c"]);
}

#[test]
fn calls_fields_indexes_and_casts_lower() {
    let output = lower_ok("fn f() { g(1, 2); p.x; a[i]; x as i64; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Expr(call) = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *call), ExprKind::Call(_, args) if args.len() == 2));

    let StmtKind::Expr(field) = stmt_of(module, body.stmts[1]) else {
        panic!()
    };
    let ExprKind::Field(base, selection) = expr_of(module, *field) else {
        panic!("expected a field access");
    };
    assert!(matches!(expr_of(module, *base), ExprKind::Path { .. }));
    let FieldSel::Unresolved { name } = selection else {
        panic!("structural lowering leaves member selection unresolved");
    };
    assert_eq!(module.use_(*name).unwrap().name, "x");

    let StmtKind::Expr(index) = stmt_of(module, body.stmts[2]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *index), ExprKind::Index(_, _)));

    let StmtKind::Expr(cast) = stmt_of(module, body.stmts[3]) else {
        panic!()
    };
    let ExprKind::Cast(inner, ty) = expr_of(module, *cast) else {
        panic!("expected a cast");
    };
    assert!(matches!(expr_of(module, *inner), ExprKind::Path { .. }));
    assert!(module.type_ref(*ty).is_some());
}

#[test]
fn ranges_become_range_new_builtins() {
    let output = lower_ok("fn f() { let r = 0..10; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    let ExprKind::BuiltinCall { op, args } = expr_of(module, *init) else {
        panic!("expected the range builtin");
    };
    assert_eq!(*op, BuiltinOp::RangeNew);
    assert_eq!(args.len(), 2);
}

#[test]
fn conditionals_match_and_loop_forms_lower() {
    let output = lower_ok(
        "fn f(x: i64) -> i64 {
            let a = if x > 0 { 1 } else { 2 };
            let b = match x { 0 => 1, _ => 2 };
            while x > 0 { x = x - 1; }
            loop { break; }
            a + b
        }",
    );
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    assert_eq!(body.stmts.len(), 4);
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    let ExprKind::If(cond, then_body, else_expr) = expr_of(module, *init) else {
        panic!("expected an if");
    };
    assert!(matches!(expr_of(module, *cond), ExprKind::Binary(..)));
    assert!(module.body(*then_body).is_some());
    assert!(else_expr.is_some());
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[1]) else {
        panic!()
    };
    let ExprKind::Match(scrutinee, arms) = expr_of(module, *init) else {
        panic!("expected a match");
    };
    assert!(matches!(expr_of(module, *scrutinee), ExprKind::Path { .. }));
    assert_eq!(arms.len(), 2);
    assert!(matches!(pat_of(module, arms[0].pat), PatKind::Lit(..)));
    assert!(matches!(pat_of(module, arms[1].pat), PatKind::Wildcard));
    let StmtKind::Expr(while_expr) = stmt_of(module, body.stmts[2]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *while_expr), ExprKind::While(..)));
    let StmtKind::Expr(loop_expr) = stmt_of(module, body.stmts[3]) else {
        panic!()
    };
    let ExprKind::Loop(loop_body) = expr_of(module, *loop_expr) else {
        panic!("expected a loop");
    };
    let loop_body = module.body(*loop_body).unwrap();
    let StmtKind::Expr(break_expr) = stmt_of(module, loop_body.stmts[0]) else {
        panic!()
    };
    assert!(matches!(
        expr_of(module, *break_expr),
        ExprKind::Break { target } if *target == *loop_expr
    ));
}

#[test]
fn break_and_continue_target_the_nearest_loop() {
    let output = lower_ok(
        "fn f() {
            loop {
                while true { break; }
                continue;
            }
        }",
    );
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let outer_loop = tail_of(module, body);
    let ExprKind::Loop(outer_body) = expr_of(module, outer_loop) else {
        panic!("expected the loop")
    };
    let outer_body = module.body(*outer_body).unwrap();
    let StmtKind::Expr(while_expr) = stmt_of(module, outer_body.stmts[0]) else {
        panic!()
    };
    let ExprKind::While(_, while_body) = expr_of(module, *while_expr) else {
        panic!()
    };
    let while_body = module.body(*while_body).unwrap();
    let StmtKind::Expr(inner_break) = stmt_of(module, while_body.stmts[0]) else {
        panic!()
    };
    assert!(matches!(
        expr_of(module, *inner_break),
        ExprKind::Break { target } if *target == *while_expr
    ));
    let _ = &outer_loop;
    let StmtKind::Expr(continue_expr) = stmt_of(module, outer_body.stmts[1]) else {
        panic!()
    };
    assert!(matches!(
        expr_of(module, *continue_expr),
        ExprKind::Continue { target } if *target == outer_loop
    ));
}

#[test]
fn returns_and_blocks_lower() {
    let output = lower_ok("fn f() -> i64 { let b = { 1 }; return b; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *init), ExprKind::Block(_)));
    let StmtKind::Expr(ret) = stmt_of(module, body.stmts[1]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *ret), ExprKind::Return(Some(_))));
    assert_eq!(body.tail, None);
}

#[test]
fn tuples_arrays_and_tails_lower() {
    let output = lower_ok("fn f() -> i64 { let t = (1, 2); let a = [1, 2, 3]; t; a; 7 }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *init), ExprKind::Tuple(elements) if elements.len() == 2));
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[1]) else {
        panic!()
    };
    assert!(matches!(expr_of(module, *init), ExprKind::Array(elements) if elements.len() == 3));
    let tail = tail_of(module, body);
    assert!(matches!(
        expr_of(module, tail),
        ExprKind::Lit(HirLit::Int { magnitude: 7, .. })
    ));
}

#[test]
fn mutable_lets_and_annotations_reach_every_binding() {
    let output = lower_ok("fn f() { let mut (a, b): (i64, i64) = (1, 2); a = b; }");
    let module = &output.module;
    let a = def_named(module, "a");
    let b = def_named(module, "b");
    let xl_hir::DefKind::Local {
        mutable,
        ty: ty_a,
        body: body_a,
    } = &a.kind
    else {
        panic!("expected a local");
    };
    let xl_hir::DefKind::Local {
        mutable: mb,
        ty: ty_b,
        body: body_b,
    } = &b.kind
    else {
        panic!("expected a local");
    };
    assert!(*mutable && *mb);
    assert_eq!(ty_a, ty_b, "the annotation is shared by every binding");
    assert_eq!(body_a, body_b);
    assert!(ty_a.is_some());
}

#[test]
fn assignment_operators_are_preserved() {
    let output = lower_ok("fn f() { let mut x = 1; x += 1; x = x + 1; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    for (index, expected) in [(1, "AddAssign"), (2, "Assign")] {
        let StmtKind::Expr(expr) = stmt_of(module, body.stmts[index]) else {
            panic!()
        };
        let text = format!("{:?}", expr_of(module, *expr));
        assert!(text.contains(expected), "{text}");
    }
}

#[test]
fn expression_statements_discard_values_structurally() {
    let output = lower_ok("fn f() { 1 + 2; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    assert!(matches!(stmt_of(module, body.stmts[0]), StmtKind::Expr(_)));
    assert!(body.tail.is_none());
}
