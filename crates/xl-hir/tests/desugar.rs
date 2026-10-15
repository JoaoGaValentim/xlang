//! Ratified desugarings: `for`, `?`, and string interpolation
//! (`P2D-B8`, `P2D-B9`, `P2D-B10`; `lowering-contract.md` §4–6).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{body_of_fn_named, expr_of, lower_ok, pat_of, stmt_of, tail_of};
use xl_hir::{
    AstOrigin, BuiltinOp, DefKind, ExprKind, HirId, HirLit, InternalDef, PatKind, Resolution,
    StmtKind,
};

/// Collects `(desugar kind, expansion)` per originating construct.
fn expansions_of(module: &xl_hir::HirModule) -> BTreeMap<String, BTreeSet<(String, u32)>> {
    let mut map: BTreeMap<String, BTreeSet<(String, u32)>> = BTreeMap::new();
    for origin in module
        .origins
        .nodes
        .iter()
        .chain(&module.origins.defs)
        .chain(&module.origins.uses)
        .chain(&module.origins.bodies)
        .chain(&module.origins.types)
    {
        if let AstOrigin::Synthesized {
            construct,
            expansion,
            ..
        } = origin
        {
            let source = format!("{:?}", construct.source);
            map.entry(source)
                .or_default()
                .insert((format!("{:?}", construct.kind), expansion.index()));
        }
    }
    map
}

/// Counts nodes with the given AST expression origin.
fn node_count_with_expr_origin(module: &xl_hir::HirModule, expr: xl_ast::ExprId) -> usize {
    module
        .origins
        .nodes
        .iter()
        .filter(|origin| matches!(origin, AstOrigin::Expr(id) if *id == expr))
        .count()
}

#[test]
fn for_lowers_to_the_ratified_shape() {
    let output = lower_ok("fn f() { for x in 0..10 { x; } }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    // The `for` is the body's tail: a block expression.
    let for_block = tail_of(module, body);
    let ExprKind::Block(outer_body) = expr_of(module, for_block) else {
        panic!("expected the expanded block");
    };
    let outer_body = module.body(*outer_body).expect("outer body");
    assert_eq!(outer_body.stmts.len(), 1, "state binding plus loop tail");
    assert!(outer_body.tail.is_some());
    let StmtKind::Let { pattern, init } = stmt_of(module, outer_body.stmts[0]) else {
        panic!("expected the state binding");
    };
    let PatKind::Binding { local, .. } = pat_of(module, *pattern) else {
        panic!("expected the state binding pattern");
    };
    let state = module.def(*local).expect("state");
    assert_eq!(state.name, "__state");
    assert!(matches!(state.kind, DefKind::Local { mutable: true, .. }));
    // The iterable is evaluated exactly once: its nodes appear once.
    assert!(matches!(
        expr_of(module, *init),
        ExprKind::BuiltinCall {
            op: BuiltinOp::RangeNew,
            ..
        }
    ));
    // The loop is the block's tail.
    let loop_expr = tail_of(module, outer_body);
    let ExprKind::Loop(loop_body) = expr_of(module, loop_expr) else {
        panic!("expected the loop");
    };
    let loop_body = module.body(*loop_body).expect("loop body");
    assert_eq!(loop_body.stmts.len(), 1);
    let StmtKind::Expr(match_expr) = stmt_of(module, loop_body.stmts[0]) else {
        panic!("expected the match statement");
    };
    let ExprKind::Match(scrutinee, arms) = expr_of(module, *match_expr) else {
        panic!("expected the match");
    };
    assert!(matches!(
        expr_of(module, *scrutinee),
        ExprKind::BuiltinCall {
            op: BuiltinOp::RangeNext,
            ..
        }
    ));
    assert_eq!(arms.len(), 2);
    // Some arm: the internal expansion.
    let PatKind::Variant { path, args } = pat_of(module, arms[0].pat) else {
        panic!("expected the Some pattern");
    };
    let some_use = *path.last().unwrap();
    assert_eq!(module.use_(some_use).unwrap().name, "Some");
    assert_eq!(
        output.resolutions.get(some_use),
        Some(&Resolution::Internal(InternalDef::OptionSome))
    );
    assert_eq!(args.len(), 1);
    let ExprKind::Block(arm_body) = expr_of(module, arms[0].body) else {
        panic!("expected the arm block");
    };
    let arm_body = module.body(*arm_body).expect("arm body");
    assert_eq!(
        arm_body.stmts.len(),
        4,
        "pair destructuring, pattern binding, state advance, body block"
    );
    // None arm: `break` targeting the loop.
    let PatKind::Variant { path, args } = pat_of(module, arms[1].pat) else {
        panic!("expected the None pattern");
    };
    let none_use = *path.last().unwrap();
    assert_eq!(module.use_(none_use).unwrap().name, "None");
    assert_eq!(
        output.resolutions.get(none_use),
        Some(&Resolution::Internal(InternalDef::OptionNone))
    );
    assert!(args.is_empty());
    assert!(matches!(
        expr_of(module, arms[1].body),
        ExprKind::Break { target } if *target == loop_expr
    ));
}

#[test]
fn for_evaluates_its_iterable_exactly_once() {
    let source = "fn f() { for x in g() { x; } }";
    let parsed = common::parse_source(source);
    let output = common::lower_ok(source);
    // The iterable is the call `g()`: exactly one HIR node carries its
    // origin (and exactly one `g` use exists).
    let call_ids: Vec<_> = parsed
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            xl_ast::ItemKind::Fn(decl) => decl.body.tail.as_ref().and_then(|expr| {
                if let xl_ast::ExprKind::For(_, iterable, _) = &expr.kind {
                    return Some(iterable.id);
                }
                None
            }),
            _ => None,
        })
        .collect();
    assert_eq!(call_ids.len(), 1);
    assert_eq!(node_count_with_expr_origin(&output.module, call_ids[0]), 1);
}

#[test]
fn break_and_continue_in_a_for_target_the_desugared_loop() {
    let output = lower_ok("fn f() { for x in 0..10 { if x > 1 { break; } continue; } }");
    let module = &output.module;
    // Find the loop expression node.
    let loop_id = module
        .nodes
        .nodes
        .iter()
        .find(|node| {
            matches!(node.kind, xl_hir::HirKind::Expr(ExprKind::Loop(_)))
                && module
                    .origins
                    .node(node.id)
                    .is_some_and(AstOrigin::is_desugared)
        })
        .map(|node| node.id)
        .expect("desugared loop");
    let mut breaks = 0;
    let mut continues = 0;
    for node in &module.nodes.nodes {
        match &node.kind {
            xl_hir::HirKind::Expr(ExprKind::Break { target }) => {
                assert_eq!(*target, loop_id, "break must target the desugared loop");
                breaks += 1;
            }
            xl_hir::HirKind::Expr(ExprKind::Continue { target }) => {
                assert_eq!(*target, loop_id, "continue must target the desugared loop");
                continues += 1;
            }
            _ => {}
        }
    }
    // One user break plus the None-arm break; one user continue.
    assert_eq!(breaks, 2);
    assert_eq!(continues, 1);
}

#[test]
fn nested_for_loops_take_their_own_targets() {
    let output = lower_ok("fn f() { for x in 0..2 { for y in 0..2 { break; } } }");
    let module = &output.module;
    let loops: Vec<HirId> = module
        .nodes
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, xl_hir::HirKind::Expr(ExprKind::Loop(_))))
        .map(|node| node.id)
        .collect();
    assert_eq!(loops.len(), 2);
    let breaks: Vec<HirId> = module
        .nodes
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            xl_hir::HirKind::Expr(ExprKind::Break { target }) => Some(*target),
            _ => None,
        })
        .collect();
    // The two None-arm breaks target the two distinct loops; the user
    // break targets the inner loop.
    assert_eq!(breaks.len(), 3);
    assert_eq!(
        breaks.iter().filter(|target| **target == loops[1]).count(),
        2,
        "inner None-arm break and the user break"
    );
    assert_eq!(
        breaks.iter().filter(|target| **target == loops[0]).count(),
        1,
        "outer None-arm break"
    );
}

#[test]
fn nested_desugarings_record_increasing_expansions() {
    let output = lower_ok("fn f() -> i64 { for x in g()? { let y = h()?; } 0 }");
    let expansions = expansions_of(&output.module);
    let fors: Vec<_> = expansions
        .values()
        .filter(|set| set.iter().any(|(kind, _)| kind == "For"))
        .collect();
    assert_eq!(fors.len(), 1, "one `for` construct");
    assert!(fors[0].contains(&("For".to_string(), 0)));
    let tries: Vec<_> = expansions
        .values()
        .filter(|set| set.iter().any(|(kind, _)| kind == "Try"))
        .collect();
    assert_eq!(tries.len(), 2, "iterable `?` and body `?` constructs");
    for try_expansions in tries {
        assert!(
            try_expansions.iter().all(|(_, expansion)| *expansion > 0),
            "a `?` inside a desugared `for` body gets expansion > 0: {try_expansions:?}"
        );
    }
}

#[test]
fn consecutive_try_operators_are_distinct_constructs() {
    let output = lower_ok("fn f() { let a = g()?; let b = h()?; }");
    let expansions = expansions_of(&output.module);
    let tries: Vec<_> = expansions
        .values()
        .filter(|set| set.iter().any(|(kind, _)| kind == "Try"))
        .collect();
    assert_eq!(tries.len(), 2);
    for set in tries {
        assert!(set.contains(&("Try".to_string(), 0)));
    }
}

#[test]
fn try_evaluates_its_operand_exactly_once() {
    let source = "fn f() { let x = g()?; }";
    let parsed = common::parse_source(source);
    let output = common::lower_ok(source);
    let operand = parsed
        .items
        .iter()
        .find_map(|item| match &item.kind {
            xl_ast::ItemKind::Fn(decl) => decl.body.stmts.iter().find_map(|stmt| {
                if let xl_ast::StmtKind::Let(let_stmt) = &stmt.kind
                    && let xl_ast::ExprKind::Try(operand) = &let_stmt.init.kind
                {
                    return Some(operand.id);
                }
                None
            }),
            _ => None,
        })
        .expect("try operand");
    let _ = &operand;
    assert_eq!(node_count_with_expr_origin(&output.module, operand), 1);
}

#[test]
fn try_within_for_keeps_its_structure() {
    let output = lower_ok("fn f() -> i64 { for x in 0..2 { let y = g()?; } 0 }");
    let module = &output.module;
    let matches = module
        .nodes
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, xl_hir::HirKind::Expr(ExprKind::Match(..))))
        .count();
    // The `for` expansion match plus the `?` expansion match.
    assert_eq!(matches, 2);
}

#[test]
fn interpolation_merges_text_and_wraps_parts() {
    let output = lower_ok("fn f() { let s = \"a{x}b{y}\"; }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    let ExprKind::BuiltinCall { op, args } = expr_of(module, *init) else {
        panic!("expected the concatenation");
    };
    assert_eq!(*op, BuiltinOp::StrConcat);
    assert_eq!(args.len(), 4);
    let part = |id: HirId| -> String {
        match expr_of(module, id) {
            ExprKind::Lit(HirLit::Str(string)) => format!("{:?}", string.text),
            ExprKind::BuiltinCall {
                op: BuiltinOp::FormatPrim,
                ..
            } => "format".to_string(),
            other => format!("{other:?}"),
        }
    };
    let rendered: Vec<String> = args.iter().map(|arg| part(*arg)).collect();
    assert_eq!(rendered, ["\"a\"", "format", "\"b\"", "format"]);
}

#[test]
fn adjacent_interpolations_have_no_empty_text_parts() {
    let output = lower_ok("fn f() { let s = \"{a}{b}\"; }");
    let module = &output.module;
    let arg_count = module
        .nodes
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            xl_hir::HirKind::Expr(ExprKind::BuiltinCall {
                op: BuiltinOp::StrConcat,
                args,
            }) => Some(args.len()),
            _ => None,
        })
        .expect("concat");
    assert_eq!(arg_count, 2, "no empty text literals between parts");
}

#[test]
fn a_single_interpolation_is_still_a_concatenation() {
    let output = lower_ok("fn f() { let s = \"{a}\"; }");
    let module = &output.module;
    let node = module
        .nodes
        .nodes
        .iter()
        .find(|node| {
            matches!(
                node.kind,
                xl_hir::HirKind::Expr(ExprKind::BuiltinCall {
                    op: BuiltinOp::StrConcat,
                    ..
                })
            )
        })
        .expect("concat");
    let xl_hir::HirKind::Expr(ExprKind::BuiltinCall { args, .. }) = &node.kind else {
        unreachable!()
    };
    assert_eq!(args.len(), 1);
}

#[test]
fn literal_strings_never_expand() {
    let output = lower_ok("fn f() { let a = \"hello\"; let b = \"\"; }");
    for node in &output.module.nodes.nodes {
        assert!(
            !matches!(
                node.kind,
                xl_hir::HirKind::Expr(ExprKind::BuiltinCall {
                    op: BuiltinOp::StrConcat | BuiltinOp::FormatPrim,
                    ..
                })
            ),
            "all-literal strings stay literals"
        );
    }
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { init, .. } = stmt_of(module, body.stmts[1]) else {
        panic!()
    };
    assert!(matches!(
        expr_of(module, *init),
        ExprKind::Lit(HirLit::Str(string)) if string.text.is_empty()
    ));
}

#[test]
fn repeated_interpolated_variables_stay_distinct_uses() {
    let output = lower_ok("fn f() { let s = \"{x}-{x}\"; }");
    let module = &output.module;
    let formats: Vec<HirId> = module
        .nodes
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            xl_hir::HirKind::Expr(ExprKind::BuiltinCall {
                op: BuiltinOp::FormatPrim,
                args,
            }) => Some(args[0]),
            _ => None,
        })
        .collect();
    assert_eq!(formats.len(), 2);
    let origins: Vec<_> = formats
        .iter()
        .map(|id| module.origins.node(*id).cloned())
        .collect();
    assert_ne!(
        origins[0], origins[1],
        "the two written occurrences are distinct AST expressions"
    );
}

#[test]
fn unicode_text_parts_preserve_decoded_text_and_spans() {
    let source = "fn f() { let s = \"héllo {x} wörld\"; }";
    let output = common::lower_ok(source);
    let module = &output.module;
    let mut texts = Vec::new();
    for node in &module.nodes.nodes {
        if let xl_hir::HirKind::Expr(ExprKind::Lit(HirLit::Str(string))) = &node.kind {
            let slice = &source[string.span.lo as usize..string.span.hi as usize];
            // The merged span covers exactly the raw text segments the
            // lexer produced (edge segments include the quotes).
            assert!(
                slice.contains(&string.text),
                "span slice {slice:?} must cover the decoded text {:?}",
                string.text
            );
            texts.push(string.text.clone());
        }
    }
    assert_eq!(texts, vec!["héllo ".to_string(), " wörld".to_string()]);
}

#[test]
fn hygiene_source_names_cannot_capture_internal_bindings() {
    // The user deliberately uses the internal names and constructor names.
    let output = lower_ok(
        "fn Ok() -> i64 { 0 }
        fn f() -> i64 {
            let __state = 1;
            let __v = 2;
            for __item in 0..2 {
                let y = g()?;
                __item;
            }
            0
        }",
    );
    let module = &output.module;
    // Internal uses resolve to internal identities; source uses stay
    // pending even when their text matches an internal name.
    let mut internal_locals = 0;
    let mut pending_named = 0;
    for (index, resolution) in output.resolutions.uses.iter().enumerate() {
        let use_ = &module.uses.uses[index];
        match resolution {
            Resolution::Internal(_) => {
                assert!(matches!(
                    module.origins.use_(xl_hir::UseId::from_index(index as u32)),
                    Some(AstOrigin::Synthesized { .. })
                ));
            }
            Resolution::Def(def) => {
                let origin = module
                    .origins
                    .use_(xl_hir::UseId::from_index(index as u32))
                    .expect("origin");
                assert!(
                    origin.is_desugared(),
                    "established resolutions only exist on synthesized uses"
                );
                let target = module.def(*def).unwrap();
                if matches!(target.kind, DefKind::Local { .. }) {
                    internal_locals += 1;
                }
            }
            Resolution::Unresolved(_) => {
                if use_.name.starts_with("__") || use_.name == "Ok" {
                    pending_named += 1;
                }
            }
            Resolution::Global(_) => {
                panic!("structural output never fabricates global resolutions")
            }
        }
    }
    assert!(internal_locals > 0, "internal references resolve to temps");
    assert!(
        pending_named > 0,
        "source uses named like internal bindings still resolve through scopes"
    );
    // Every synthesized local uses an internal name; the user's locals
    // with colliding text are distinct identities.
    for def in module.defs.iter() {
        let desugared = module.origins.def(def.id).unwrap().is_desugared();
        if desugared {
            assert!(matches!(def.kind, DefKind::Local { .. }));
            assert!(def.name.starts_with("__"), "{:?}", def.name);
        }
    }
    let states: Vec<_> = module
        .defs
        .iter()
        .filter(|def| def.name == "__state")
        .collect();
    assert_eq!(states.len(), 2, "the user local and the internal temp");
    assert_ne!(states[0].id, states[1].id);
    let desugared_count = states
        .iter()
        .filter(|def| module.origins.def(def.id).unwrap().is_desugared())
        .count();
    assert_eq!(desugared_count, 1);
}

#[test]
fn binding_resolutions_stay_pending_and_internal_ones_are_not() {
    let output = lower_ok("fn f() { for x in 0..2 { x; } }");
    for (index, resolution) in output.resolutions.uses.iter().enumerate() {
        let origin = output
            .module
            .origins
            .use_(xl_hir::UseId::from_index(index as u32))
            .expect("origin");
        if origin.is_desugared() {
            assert!(
                !matches!(resolution, Resolution::Unresolved(_)),
                "synthesized uses are established at creation"
            );
        }
    }
}
