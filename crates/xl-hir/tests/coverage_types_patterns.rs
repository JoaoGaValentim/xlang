//! Type and pattern lowering coverage (`Directive 011` §8).

mod common;

use common::{body_of_fn_named, def_named, lower_ok, pat_of, stmt_of, tail_of};
use xl_hir::{DefKind, HirLit, PatKind, StmtKind, TypeRefKind};

fn param_type(output: &xl_hir::StructuralLowerOutput, name: &str, index: usize) -> TypeRefKind {
    let module = &output.module;
    let def = def_named(module, name);
    let DefKind::Fn { sig, .. } = &def.kind else {
        panic!("expected a function");
    };
    let signature = module.sig(*sig).expect("sig");
    let param = module.def(signature.params[index]).expect("param");
    let DefKind::Param { ty, .. } = &param.kind else {
        panic!("expected a param");
    };
    module.type_ref(*ty).expect("type").kind.clone()
}

#[test]
fn primitive_and_path_types_lower() {
    let output = lower_ok("fn f(a: i64, b: str, c: bool) {}");
    assert!(matches!(
        param_type(&output, "f", 0),
        TypeRefKind::Path { .. }
    ));
    let TypeRefKind::Path { segments, generics } = param_type(&output, "f", 0) else {
        unreachable!()
    };
    assert_eq!(generics.len(), 0);
    let module = &output.module;
    assert_eq!(module.use_(*segments.last().unwrap()).unwrap().name, "i64");
}

#[test]
fn generic_type_arguments_lower() {
    let output = lower_ok("struct B<T> { v: T }\nfn f(x: B<i64>, y: B<B<bool>>) {}");
    let TypeRefKind::Path { generics, .. } = param_type(&output, "f", 1) else {
        panic!("expected a path");
    };
    assert_eq!(generics.len(), 1);
    let module = &output.module;
    let inner = module.type_ref(generics[0]).expect("inner");
    assert!(matches!(inner.kind, TypeRefKind::Path { .. }));
}

#[test]
fn tuple_slice_array_unit_and_fn_types_lower() {
    let output =
        lower_ok("fn f(a: (i64, bool), b: [i64], c: [i64; 2], d: (), e: fn(i64) -> i64) {}");
    assert!(matches!(param_type(&output, "f", 0), TypeRefKind::Tuple(e) if e.len() == 2));
    assert!(matches!(param_type(&output, "f", 1), TypeRefKind::Slice(_)));
    let TypeRefKind::Array { elem, len } = param_type(&output, "f", 2) else {
        panic!("expected an array type");
    };
    let module = &output.module;
    assert!(module.type_ref(elem).is_some());
    assert!(matches!(
        module.nodes.expr(len),
        Some(xl_hir::ExprKind::Lit(HirLit::Int { magnitude: 2, .. }))
    ));
    assert!(matches!(param_type(&output, "f", 3), TypeRefKind::Tuple(e) if e.is_empty()));
    let TypeRefKind::Fn { params, ret } = param_type(&output, "f", 4) else {
        panic!("expected a fn type");
    };
    assert_eq!(params.len(), 1);
    assert!(ret.is_some());
}

#[test]
fn return_types_lower() {
    let output = lower_ok("fn f() -> i64 { 0 }");
    let module = &output.module;
    let signature = common::sig_of_fn_named(module, "f");
    assert!(signature.ret.is_some());
}

#[test]
fn self_in_impl_context_lowers_to_self_type() {
    let output = lower_ok("struct P { x: i64 }\nimpl P { fn m(self) -> Self { self } }");
    let module = &output.module;
    let method = def_named(module, "m");
    let DefKind::Method { sig, .. } = &method.kind else {
        panic!()
    };
    let signature = module.sig(*sig).expect("sig");
    let ret = signature.ret.expect("return");
    assert!(matches!(
        module.type_ref(ret).unwrap().kind,
        TypeRefKind::SelfTy
    ));
}

#[test]
fn self_outside_impl_context_is_an_ordinary_path() {
    let output = lower_ok("fn f(x: Self) {}");
    let TypeRefKind::Path { segments, .. } = param_type(&output, "f", 0) else {
        panic!("expected a path");
    };
    let module = &output.module;
    assert_eq!(module.use_(*segments.last().unwrap()).unwrap().name, "Self");
}

#[test]
fn wildcard_binding_and_literal_patterns_lower() {
    let output = lower_ok("fn f() { let _ = 1; let x = 2; match 3 { 4 => 1, _ => 2 } }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { pattern, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    assert!(matches!(pat_of(module, *pattern), PatKind::Wildcard));
    let StmtKind::Let { pattern, .. } = stmt_of(module, body.stmts[1]) else {
        panic!()
    };
    let PatKind::Binding { local, candidate } = pat_of(module, *pattern) else {
        panic!("expected a binding");
    };
    assert_eq!(module.def(*local).unwrap().name, "x");
    assert_eq!(module.use_(*candidate).unwrap().name, "x");
    assert!(
        matches!(
            output.resolutions.get(*candidate),
            Some(xl_hir::Resolution::Unresolved(_))
        ),
        "binding candidates stay pending for WP-B"
    );
}

#[test]
fn constructor_patterns_keep_their_paths_and_arguments() {
    let output =
        lower_ok("enum E { A, B(i64, bool) }\nfn f() { match x { E::A => 1, E::B(p, q) => 2 } }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let tail = tail_of(module, body);
    let xl_hir::ExprKind::Match(_, arms) = module.nodes.expr(tail).expect("match") else {
        panic!("expected a match");
    };
    let PatKind::Variant { path, args } = pat_of(module, arms[0].pat) else {
        panic!("expected a variant pattern");
    };
    assert_eq!(path.len(), 2);
    let names: Vec<&str> = path
        .iter()
        .map(|use_| module.use_(*use_).unwrap().name.as_str())
        .collect();
    assert_eq!(names, ["E", "A"]);
    assert!(args.is_empty());
    let PatKind::Variant { path, args } = pat_of(module, arms[1].pat) else {
        panic!("expected a variant pattern");
    };
    assert_eq!(path.len(), 2);
    assert_eq!(args.len(), 2);
    assert!(matches!(pat_of(module, args[0]), PatKind::Binding { .. }));
}

#[test]
fn tuple_patterns_nest() {
    let output = lower_ok("fn f() { let (a, (b, c)) = ((1, 2), (3, 4)); }");
    let module = &output.module;
    let body = body_of_fn_named(module, "f");
    let StmtKind::Let { pattern, .. } = stmt_of(module, body.stmts[0]) else {
        panic!()
    };
    let PatKind::Tuple(elements) = pat_of(module, *pattern) else {
        panic!("expected a tuple pattern");
    };
    assert_eq!(elements.len(), 2);
    let PatKind::Tuple(inner) = pat_of(module, elements[1]) else {
        panic!("expected a nested tuple pattern");
    };
    assert_eq!(inner.len(), 2);
}

#[test]
fn pattern_locals_records_are_consistent() {
    let output = lower_ok("fn f() { let (a, b) = (1, 2); }");
    let module = &output.module;
    let a = def_named(module, "a");
    let b = def_named(module, "b");
    let DefKind::Local { body: body_a, .. } = &a.kind else {
        panic!()
    };
    let DefKind::Local { body: body_b, .. } = &b.kind else {
        panic!()
    };
    assert_eq!(body_a, body_b);
    let f = def_named(module, "f");
    let DefKind::Fn { body: fn_body, .. } = &f.kind else {
        panic!()
    };
    assert_eq!(Some(*body_a), *fn_body);
}
