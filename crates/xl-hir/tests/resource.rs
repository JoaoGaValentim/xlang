//! Resource safety: ratified budgets, boundary behavior, and the
//! depth-accounting alignment between lowering and the validator
//! (`Directive 011` §13; `P2D-A7`).

mod common;

use common::{FILE, parse_source};
use xl_hir::{
    AbortReason, Budget, LowerLimits, ModuleId, PackageId, StageFate, lower_structure_with_limits,
    validate_with_limits,
};

fn lower_with_limits(source: &str, limits: LowerLimits) -> xl_hir::StructuralLowerOutput {
    let parsed = parse_source(source);
    lower_structure_with_limits(
        &parsed.items,
        FILE,
        ModuleId::from_index(0),
        PackageId::from_index(0),
        limits,
    )
}

#[test]
fn node_budget_boundary_is_exact() {
    let source = "fn f() -> i64 { 1 + 2 }";
    let unlimited = lower_with_limits(source, LowerLimits::RATIFIED);
    let nodes = unlimited.module.nodes.nodes.len() as u32;
    assert!(nodes > 0);

    let exact = lower_with_limits(
        source,
        LowerLimits {
            max_hir_nodes: nodes,
            ..LowerLimits::RATIFIED
        },
    );
    assert_eq!(exact.fate, StageFate::Complete);

    let over = lower_with_limits(
        source,
        LowerLimits {
            max_hir_nodes: nodes - 1,
            ..LowerLimits::RATIFIED
        },
    );
    assert_eq!(
        over.fate,
        StageFate::Aborted {
            reason: AbortReason::BudgetExceeded {
                budget: Budget::HirNodes
            }
        }
    );
    assert_eq!(over.diagnostics.len(), 1);
    assert_eq!(
        over.diagnostics[0].message(),
        format!("HIR node limit ({}) exceeded", nodes - 1)
    );
}

#[test]
fn definition_budget_boundary_is_exact() {
    let source = "struct S { x: i64 } fn f() { let y = 1; }";
    let unlimited = lower_with_limits(source, LowerLimits::RATIFIED);
    let defs = unlimited.module.defs.defs.len() as u32;
    assert!(defs >= 4, "struct, field, fn, local");

    let exact = lower_with_limits(
        source,
        LowerLimits {
            max_defs: defs,
            ..LowerLimits::RATIFIED
        },
    );
    assert_eq!(exact.fate, StageFate::Complete);

    let over = lower_with_limits(
        source,
        LowerLimits {
            max_defs: defs - 1,
            ..LowerLimits::RATIFIED
        },
    );
    assert_eq!(
        over.fate,
        StageFate::Aborted {
            reason: AbortReason::BudgetExceeded {
                budget: Budget::Defs
            }
        }
    );
    assert_eq!(
        over.diagnostics[0].message(),
        format!("HIR definition limit ({}) exceeded", defs - 1)
    );
}

#[test]
fn use_budget_boundary_is_exact() {
    let source = "fn f() { let x = y; }";
    let unlimited = lower_with_limits(source, LowerLimits::RATIFIED);
    let uses = unlimited.module.uses.uses.len() as u32;
    assert_eq!(uses, 2, "the binding candidate and the `y` path");

    let exact = lower_with_limits(
        source,
        LowerLimits {
            max_uses: uses,
            ..LowerLimits::RATIFIED
        },
    );
    assert_eq!(exact.fate, StageFate::Complete);

    let over = lower_with_limits(
        source,
        LowerLimits {
            max_uses: uses - 1,
            ..LowerLimits::RATIFIED
        },
    );
    assert_eq!(
        over.fate,
        StageFate::Aborted {
            reason: AbortReason::BudgetExceeded {
                budget: Budget::Uses
            }
        }
    );
    assert_eq!(
        over.diagnostics[0].message(),
        format!("HIR use limit ({}) exceeded", uses - 1)
    );
}

/// The alignment property: whenever lowering completes at some depth
/// budget, the produced module must validate at that same budget. This
/// pins the desugaring-overhead constants to the validator's real-depth
/// counting.
#[test]
fn completed_modules_always_validate_at_the_same_depth_budget() {
    let corpus = [
        "fn f() {}",
        "fn f() { 1; }",
        "fn f() -> i64 { 1 + 2 * 3 }",
        "fn f() { let x = 1; let y = x; }",
        "fn f() { if 1 > 0 { 1; } else { 2; } }",
        "fn f() { while 1 > 0 { break; } }",
        "fn f() { loop { continue; } }",
        "fn f() { match 0 { 0 => 1, _ => 2 } }",
        "fn f() { let t = (1, (2, 3)); }",
        "fn f() { let a = [1, [2, 3]]; }",
        "fn f() { for x in 0..3 { x; } }",
        "fn f() { for x in 0..3 { for y in 0..3 { break; } } }",
        "fn f() -> i64 { for x in g()? { h()?; } 0 }",
        "fn f() { for x in 0..2 { let s = \"v={x}\"; } }",
        "fn f() { let s = \"a{x}b\"; }",
        "fn f() { let s = \"{a}{b}{c}\"; }",
        "fn f() { let s = \"\"; }",
        "fn f(x: fn(i64) -> i64) {}",
        "fn f(x: [i64; 2], y: (i64, [bool; 3])) {}",
        "struct S { x: i64 }\nimpl S { fn m(self) -> i64 { self.x } }",
        "fn f() { let b = { let c = { 1 }; c }; }",
        "fn f() { for x in 0..2 { if x > 0 { g()?; } } }",
        "fn f() { for x in 0..2 { for y in 0..2 { let s = \"v={y}\"; } } }",
        "fn f() { let s = \"a{b}c{d}e{f}\"; }",
    ];
    for source in corpus {
        for depth in 1..=32u32 {
            let limits = LowerLimits {
                max_hir_depth: depth,
                ..LowerLimits::RATIFIED
            };
            let output = lower_with_limits(source, limits);
            if !output.is_complete() {
                continue;
            }
            let violations = validate_with_limits(&output.module, limits);
            assert!(
                violations.is_empty(),
                "source {source:?} at depth budget {depth} produced a module that does not \
                 validate: {violations:#?}\n{}",
                xl_hir::dump::dump_module(&output.module)
            );
        }
    }
}

#[test]
fn depth_budget_poisons_desugarings_consistently() {
    // A `for` needs six structural levels; at depth 1 it must be poisoned
    // (complete, with the depth diagnostic) and the module must still
    // validate.
    let limits = LowerLimits {
        max_hir_depth: 1,
        ..LowerLimits::RATIFIED
    };
    let output = lower_with_limits("fn f() { for x in 0..2 { x; } }", limits);
    assert_eq!(output.fate, StageFate::Complete);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "HIR nesting limit (1) exceeded"
    );
    assert!(validate_with_limits(&output.module, limits).is_empty());
    assert!(
        !output
            .module
            .nodes
            .nodes
            .iter()
            .any(|node| matches!(node.kind, xl_hir::HirKind::Expr(xl_hir::ExprKind::Loop(_)))),
        "the poisoned `for` builds no loop"
    );
}

#[test]
fn at_most_one_depth_diagnostic_is_reported() {
    // Two independent over-deep chains in one module: the depth
    // diagnostic is reported once per run (deterministic suppression).
    let items = vec![
        common::deep_array_item("a", 0, 0, 300),
        common::deep_array_item("b", 1, 1000, 300),
    ];
    let output = lower_structure_with_limits(
        &items,
        FILE,
        ModuleId::from_index(0),
        PackageId::from_index(0),
        LowerLimits::RATIFIED,
    );
    assert_eq!(output.fate, StageFate::Complete);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "HIR nesting limit (256) exceeded"
    );
}
