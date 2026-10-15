//! Error tolerance: recovered ASTs, structural dead ends, deep inputs, and
//! flat inputs (`Directive 011` §12–13; `lowering-contract.md` §7).

mod common;

use common::{lower_ok, lower_source, parse_source};
use xl_hir::{ExprKind, HirKind, PatKind, StageFate};

#[test]
fn the_empty_module_lowers_and_validates() {
    let output = lower_ok("");
    assert!(output.module.items.is_empty());
    assert!(output.module.defs.defs.is_empty());
    assert!(output.module.nodes.nodes.is_empty());
    assert!(output.diagnostics.is_empty());
}

/// A list of sources that parse with recovery; lowering must stay total,
/// produce error nodes where placeholders were, and add no diagnostics of
/// its own (the parser already reported).
#[test]
fn recovered_asts_lower_without_new_diagnostics() {
    let cases = [
        "fn f() { let x = ; }",
        "fn f(x: ) {}",
        "fn f() { for x in ; { } }",
        "fn f() { let = 1; }",
        "fn f() {} {}",
        "fn () {}",
    ];
    for source in cases {
        let parsed = parse_source(source);
        assert!(parsed.has_errors(), "the case must exercise recovery");
        let output = lower_source(source);
        assert!(output.is_complete(), "{source:?} must lower completely");
        assert!(
            output.diagnostics.is_empty(),
            "{source:?} added diagnostics: {:#?}",
            output.diagnostics
        );
        let violations = xl_hir::validate(&output.module);
        assert!(
            violations.is_empty(),
            "{source:?} does not validate: {violations:#?}"
        );
    }
}

#[test]
fn placeholder_expressions_become_error_nodes() {
    let output = lower_source("fn f() { let x = ; }");
    assert!(
        output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Expr(ExprKind::Error))),
        "the missing initializer must be an error node"
    );
}

#[test]
fn placeholder_patterns_become_error_nodes() {
    let output = lower_source("fn f() { let = 1; }");
    assert!(
        output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Pat(PatKind::Error))),
        "the missing pattern must be an error node"
    );
}

#[test]
fn placeholder_types_become_error_type_references() {
    let output = lower_source("fn f(x: ) {}");
    assert!(
        output
            .module
            .types
            .types
            .iter()
            .any(|type_ref| matches!(type_ref.kind, xl_hir::TypeRefKind::Error)),
        "the missing parameter type must be an error type reference"
    );
}

#[test]
fn a_placeholder_iterable_poisons_the_whole_for() {
    let output = lower_source("fn f() { for x in ; { } }");
    let errors = output
        .module
        .nodes
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, HirKind::Expr(ExprKind::Error)))
        .count();
    assert!(errors >= 1);
    // No desugared structure for that `for`.
    assert!(
        !output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Expr(ExprKind::BuiltinCall { .. }))),
        "a poisoned `for` builds no expansion"
    );
}

#[test]
fn placeholder_patterns_in_a_for_become_wildcards() {
    let output = lower_source("fn f() { for in 0..2 { } }");
    assert!(output.is_complete());
    assert!(
        output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Pat(PatKind::Wildcard))),
        "the recovered for-pattern lowers to a wildcard"
    );
}

#[test]
fn break_and_continue_outside_a_loop_fail_closed() {
    let output = lower_source("fn f() { break; }");
    assert_eq!(output.fate, StageFate::Complete);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.diagnostics[0].message(), "'break' outside of a loop");
    assert!(xl_hir::validate(&output.module).is_empty());
    assert!(
        output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Expr(ExprKind::Error)))
    );

    let output = lower_source("fn f() { continue; }");
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "'continue' outside of a loop"
    );
}

#[test]
fn break_inside_a_loop_stays_valid() {
    let output = lower_ok("fn f() { loop { break; } }");
    assert!(output.diagnostics.is_empty());
}

#[test]
fn nested_item_bodies_cannot_target_outer_loops() {
    let output = lower_source("fn f() { loop { fn g() { break; } } }");
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.diagnostics[0].message(), "'break' outside of a loop");
}

#[test]
fn interpolation_in_a_pattern_fails_closed() {
    let output = lower_source("fn f(s: str) { match s { \"a{x}\" => 1, _ => 2 } }");
    assert_eq!(output.fate, StageFate::Complete);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "string interpolation is not allowed in patterns"
    );
    assert!(
        output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, HirKind::Pat(PatKind::Error)))
    );
    assert!(xl_hir::validate(&output.module).is_empty());
}

#[test]
fn empty_interpolations_survive_recovery() {
    let output = lower_source("fn f() { let s = \"{}\"; }");
    assert!(output.is_complete());
    assert!(xl_hir::validate(&output.module).is_empty());
}

#[test]
fn deep_hand_built_expressions_are_poisoned_at_the_limit() {
    // 300 nested arrays exceed MAX_HIR_DEPTH (256).
    let items = vec![common::deep_array_item("f", 0, 0, 300)];
    let output = xl_hir::lower_structure(
        &items,
        common::FILE,
        xl_hir::ModuleId::from_index(0),
        xl_hir::PackageId::from_index(0),
    );
    assert_eq!(output.fate, StageFate::Complete);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "HIR nesting limit (256) exceeded"
    );
    assert!(xl_hir::validate(&output.module).is_empty());
}

#[test]
fn deep_inputs_within_the_limit_pass_cleanly() {
    let items = vec![common::deep_array_item("f", 0, 0, 100)];
    let output = xl_hir::lower_structure(
        &items,
        common::FILE,
        xl_hir::ModuleId::from_index(0),
        xl_hir::PackageId::from_index(0),
    );
    assert!(output.is_complete());
    assert!(output.diagnostics.is_empty());
    assert!(xl_hir::validate(&output.module).is_empty());
}

#[test]
fn large_flat_modules_lower_within_budget() {
    let mut source = String::from("fn f() {\n");
    for index in 0..3000 {
        source.push_str(&format!("    let v{index} = {index};\n"));
    }
    source.push_str("}\n");
    let output = lower_ok(&source);
    assert!(output.module.defs.defs.len() >= 3000);
    assert!(output.module.nodes.nodes.len() >= 6000);
}

#[test]
fn deeply_nested_parser_input_stays_below_the_hir_limit() {
    // The parser's own depth limit is 64; lowering's budget is 256. Take
    // the deepest expression nesting the parser accepts and lower it.
    let depth = 60;
    let mut source = String::from("fn f() -> i64 { ");
    for _ in 0..depth {
        source.push('(');
    }
    source.push('1');
    for _ in 0..depth {
        source.push(')');
    }
    source.push_str(" }");
    let output = lower_ok(&source);
    assert!(output.diagnostics.is_empty());
    // Right-nested arrays are the deepest counted chains.
    let mut source = String::from("fn f() -> i64 { ");
    for _ in 0..depth {
        source.push('[');
    }
    source.push('1');
    for _ in 0..depth {
        source.push(']');
    }
    source.push_str(" }");
    let output = lower_ok(&source);
    assert!(output.diagnostics.is_empty());
}
