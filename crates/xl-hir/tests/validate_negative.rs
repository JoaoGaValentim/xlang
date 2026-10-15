//! Negative validation: deliberately malformed HIR values must be
//! rejected (`Directive 011` §12; `hir-contract.md` §11).
//!
//! Each case lowers a clean source, confirms the cleaned module validates,
//! mutates one detail of a clone, and requires the expected violation kind.

mod common;

use common::lower_ok;
use xl_hir::{
    AnchorId, AstOrigin, DefKind, ExprKind, HirId, HirKind, HirLit, LowerLimits, Resolution,
    StmtKind, TypeRefKind, UnresolvedReason, ViolationKind, validate, validate_resolutions,
    validate_with_limits,
};

fn sample() -> xl_hir::StructuralLowerOutput {
    lower_ok(
        "import std.io;
        struct S { x: f64 }
        fn f() -> f64 {
            let mut total = 0.0;
            for v in 0..3 { total += v as f64; }
            total
        }",
    )
}

fn find_stmt_node(module: &xl_hir::HirModule, predicate: impl Fn(&StmtKind) -> bool) -> HirId {
    module
        .nodes
        .nodes
        .iter()
        .find(|node| match &node.kind {
            HirKind::Stmt(kind) => predicate(kind),
            _ => false,
        })
        .map(|node| node.id)
        .expect("statement node")
}

fn find_expr_node(module: &xl_hir::HirModule, predicate: impl Fn(&ExprKind) -> bool) -> HirId {
    module
        .nodes
        .nodes
        .iter()
        .find(|node| match &node.kind {
            HirKind::Expr(kind) => predicate(kind),
            _ => false,
        })
        .map(|node| node.id)
        .expect("expression node")
}

#[test]
fn clean_sample_validates() {
    let output = sample();
    assert!(validate(&output.module).is_empty());
    assert!(validate_resolutions(&output.module, &output.resolutions).is_empty());
}

#[test]
fn orphaned_arena_node_with_invalid_reference_is_rejected() {
    let mut output = sample();
    let index = output.module.nodes.nodes.len() as u32;
    let first = output.module.nodes.nodes[0].clone();
    output.module.nodes.nodes.push(xl_hir::HirNode {
        id: HirId::from_index(index),
        span: first.span,
        kind: HirKind::Expr(ExprKind::Call(HirId::from_index(u32::MAX), vec![])),
    });
    output
        .module
        .origins
        .nodes
        .push(AstOrigin::Expr(xl_ast::ExprId::from_index(u32::MAX)));
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::IndexOutOfBounds),
        "orphaned arena entry escaped reference validation: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::UnreachableArenaEntry)
    );
}

#[test]
fn orphaned_body_with_invalid_statement_is_rejected() {
    let mut output = sample();
    let mut orphan = output.module.bodies.bodies[0].clone();
    orphan.stmts = vec![HirId::from_index(u32::MAX)];
    orphan.tail = None;
    output.module.bodies.bodies.push(orphan);
    let origin = output.module.origins.bodies[0].clone();
    output.module.origins.bodies.push(origin);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::UnreachableArenaEntry)
    );
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::IndexOutOfBounds)
    );
}

#[test]
fn orphaned_type_with_invalid_element_is_rejected() {
    let mut output = sample();
    let mut orphan = output.module.types.types[0].clone();
    orphan.id = xl_hir::TypeRefId::from_index(output.module.types.types.len() as u32);
    orphan.kind = TypeRefKind::Slice(xl_hir::TypeRefId::from_index(u32::MAX));
    output.module.types.types.push(orphan);
    let origin = output.module.origins.types[0].clone();
    output.module.origins.types.push(origin);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::UnreachableArenaEntry)
    );
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::IndexOutOfBounds)
    );
}

#[test]
fn orphaned_signature_with_invalid_parameter_is_rejected() {
    let mut output = sample();
    let mut orphan = output.module.sigs.sigs[0].clone();
    orphan.params = vec![xl_hir::DefId::from_index(u32::MAX)];
    output.module.sigs.sigs.push(orphan);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::UnreachableArenaEntry),
        "unreachable signature escaped validation: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::IndexOutOfBounds),
        "invalid signature reference escaped validation: {violations:?}"
    );
}

#[test]
fn out_of_bounds_references_are_rejected() {
    let mut output = sample();
    let stmt = find_stmt_node(&output.module, |kind| matches!(kind, StmtKind::Let { .. }));
    output.module.nodes.nodes[stmt.index() as usize].kind =
        HirKind::Stmt(StmtKind::Expr(HirId::from_index(9999)));
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::IndexOutOfBounds)
    );
}

#[test]
fn wrong_node_kinds_are_rejected() {
    let mut output = sample();
    // Replace the body's tail with a statement node.
    let stmt = find_stmt_node(&output.module, |kind| matches!(kind, StmtKind::Let { .. }));
    let f = common::def_named(&output.module, "f");
    let DefKind::Fn { body, .. } = f.kind else {
        panic!()
    };
    output.module.bodies.bodies[body.unwrap().index() as usize].tail = Some(stmt);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::KindMismatch)
    );
}

#[test]
fn unfilled_definitions_are_rejected() {
    let mut output = sample();
    let f = common::def_named(&output.module, "f").id;
    output.module.defs.defs[f.index() as usize].kind = DefKind::Error;
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::UnfilledDefinition)
    );
}

#[test]
fn shortened_origin_tables_are_rejected() {
    let mut output = sample();
    output.module.origins.nodes.pop();
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::OriginMissing)
    );
}

#[test]
fn duplicated_source_occurrences_are_rejected() {
    let mut output = sample();
    let first = find_expr_node(&output.module, |kind| matches!(kind, ExprKind::Lit(_)));
    let second = find_expr_node(&output.module, |kind| {
        matches!(kind, ExprKind::BuiltinCall { .. })
    });
    let copied = output.module.origins.nodes[first.index() as usize].clone();
    output.module.origins.nodes[second.index() as usize] = copied;
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::DuplicateNodeOccurrence)
    );
}

#[test]
fn duplicated_synthesized_occurrences_are_rejected() {
    let mut output = sample();
    // Two synthesized entities forced to the same complete key.
    let ids: Vec<usize> = output
        .module
        .origins
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, origin)| origin.is_desugared())
        .map(|(index, _)| index)
        .take(2)
        .collect();
    assert!(ids.len() >= 2);
    let stolen = output.module.origins.nodes[ids[0]].clone();
    output.module.origins.nodes[ids[1]] = stolen;
    let violations = validate(&output.module);
    assert!(violations.iter().any(|violation| {
        violation.kind == ViolationKind::DuplicateSynthesizedKey
            || violation.kind == ViolationKind::DuplicateNodeOccurrence
    }));
}

#[test]
fn empty_structural_paths_are_rejected() {
    let mut output = sample();
    let stmt = find_stmt_node(&output.module, |kind| matches!(kind, StmtKind::Let { .. }));
    let anchor = match &output.module.origins.nodes[stmt.index() as usize] {
        AstOrigin::Structural { anchor, .. } => *anchor,
        other => panic!("expected a structural origin, got {other:?}"),
    };
    output.module.origins.nodes[stmt.index() as usize] = AstOrigin::Structural {
        anchor,
        path: Vec::new(),
    };
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::OriginMalformed)
    );
    let _ = AnchorId::Item(xl_ast::ItemId::from_index(0));
}

#[test]
fn non_finite_floats_are_rejected() {
    let mut output = sample();
    let float = find_expr_node(&output.module, |kind| {
        matches!(kind, ExprKind::Lit(HirLit::Float(_)))
    });
    output.module.nodes.nodes[float.index() as usize].kind =
        HirKind::Expr(ExprKind::Lit(HirLit::Float(f64::NAN)));
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::NonFiniteFloat)
    );
}

#[test]
fn foreign_loop_targets_are_rejected() {
    let mut output = sample();
    let break_node = find_expr_node(&output.module, |kind| {
        matches!(kind, ExprKind::Break { .. })
    });
    let literal = find_expr_node(&output.module, |kind| matches!(kind, ExprKind::Lit(_)));
    output.module.nodes.nodes[break_node.index() as usize].kind =
        HirKind::Expr(ExprKind::Break { target: literal });
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::LoopTargetNotEnclosing)
    );
}

#[test]
fn span_violations_are_rejected() {
    let mut output = sample();
    let node = find_expr_node(&output.module, |kind| matches!(kind, ExprKind::Lit(_)));
    output.module.nodes.nodes[node.index() as usize].span.lo = 100;
    output.module.nodes.nodes[node.index() as usize].span.hi = 10;
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::SpanInverted)
    );

    let mut output = sample();
    let node = find_expr_node(&output.module, |kind| matches!(kind, ExprKind::Lit(_)));
    output.module.nodes.nodes[node.index() as usize].span.file = xl_span::FileId::from_index(9);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::SpanFileMismatch)
    );
}

#[test]
fn entity_id_mismatches_are_rejected() {
    let mut output = sample();
    let f = common::def_named(&output.module, "f").id;
    output.module.defs.defs[f.index() as usize].id = xl_hir::DefId::from_index(1234);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::EntityIdMismatch)
    );
}

#[test]
fn inconsistent_parents_are_rejected() {
    let mut output = sample();
    let field = common::def_named(&output.module, "x").id;
    let f = common::def_named(&output.module, "f").id;
    output.module.defs.defs[field.index() as usize].parent = Some(f);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::ParentKindMismatch)
    );
}

#[test]
fn import_record_corruption_is_rejected() {
    let mut output = sample();
    output.module.imports.clear();
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::ImportRecordMismatch)
    );
}

#[test]
fn import_record_path_substitution_is_rejected() {
    let mut output = sample();
    // The recorded import path is a precise copy of the binding path,
    // not merely a sequence with the same number of segments.
    let original = output.module.imports[0].path[0];
    let other = output.module.imports[0].path[1];
    assert_ne!(original, other);
    output.module.imports[0].path[0] = other;
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::ImportRecordMismatch),
        "substituted import path was accepted: {violations:?}"
    );
}

#[test]
fn mismatched_import_spans_are_rejected() {
    let mut output = sample();
    output.module.imports[0].decl_span.lo += 1;
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::ImportRecordMismatch),
        "import record with changed declaration span was accepted: {violations:?}"
    );

    let mut output = sample();
    let binding = output.module.imports[0].binding;
    let DefKind::ModuleBinding(ref mut module_binding) =
        output.module.defs.defs[binding.index() as usize].kind
    else {
        panic!("expected import binding");
    };
    module_binding.span.file = xl_span::FileId::from_index(77);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::ImportRecordMismatch),
        "import binding with foreign span was accepted: {violations:?}"
    );
}

#[test]
fn cyclic_expression_reference_is_rejected() {
    let mut output = sample();
    let literal = find_expr_node(&output.module, |kind| matches!(kind, ExprKind::Lit(_)));
    output.module.nodes.nodes[literal.index() as usize].kind =
        HirKind::Expr(ExprKind::Unary(xl_hir::UnOp::Neg, literal));
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::CyclicReference)
    );
}

#[test]
fn cyclic_body_reference_is_rejected() {
    let mut output = sample();
    let f = common::def_named(&output.module, "f");
    let DefKind::Fn {
        body: Some(body), ..
    } = f.kind
    else {
        panic!("expected function body")
    };
    let tail = output.module.bodies.bodies[body.index() as usize]
        .tail
        .expect("tail");
    output.module.nodes.nodes[tail.index() as usize].kind = HirKind::Expr(ExprKind::Block(body));
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::CyclicReference)
    );
}

#[test]
fn cyclic_type_reference_is_rejected() {
    let mut output = sample();
    let f = common::def_named(&output.module, "f");
    let DefKind::Fn { sig, .. } = f.kind else {
        panic!("expected function")
    };
    let ty = output.module.sigs.sigs[sig.index() as usize]
        .ret
        .expect("return type");
    output.module.types.types[ty.index() as usize].kind = TypeRefKind::Slice(ty);
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::CyclicReference)
    );
}

#[test]
fn module_binding_name_mismatches_are_rejected() {
    let mut output = sample();
    let binding = output.module.imports[0].binding;
    let DefKind::ModuleBinding(ref mut mb) = output.module.defs.defs[binding.index() as usize].kind
    else {
        panic!("expected a module binding");
    };
    mb.alias = Some("other".to_string());
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::ModuleBindingNameMismatch)
    );
}

#[test]
fn resolution_table_mismatches_are_rejected() {
    let mut output = sample();
    output.resolutions.uses.pop();
    let violations = validate_resolutions(&output.module, &output.resolutions);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::ResolutionLengthMismatch)
    );

    let mut output = sample();
    let synthesized = output
        .module
        .origins
        .uses
        .iter()
        .position(AstOrigin::is_desugared)
        .expect("synthesized use");
    output.resolutions.uses[synthesized] = Resolution::Unresolved(UnresolvedReason::Pending);
    let violations = validate_resolutions(&output.module, &output.resolutions);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::SynthesizedUsePending)
    );
}

#[test]
fn depth_and_budget_violations_are_reported_from_limits() {
    let output = sample();
    let tight = LowerLimits {
        max_hir_depth: 1,
        ..LowerLimits::RATIFIED
    };
    let violations = validate_with_limits(&output.module, tight);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::DepthExceeded)
    );

    let tiny = LowerLimits {
        max_hir_nodes: 1,
        ..LowerLimits::RATIFIED
    };
    let violations = validate_with_limits(&output.module, tiny);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::BudgetExceeded)
    );
}

#[test]
fn type_reference_out_of_bounds_flow_is_rejected() {
    let mut output = sample();
    let f = common::def_named(&output.module, "f").id;
    let DefKind::Fn { sig, .. } = output.module.defs.defs[f.index() as usize].kind.clone() else {
        panic!()
    };
    let ret = output.module.sigs.sigs[sig.index() as usize]
        .ret
        .expect("return type");
    output.module.types.types[ret.index() as usize].kind = TypeRefKind::Path {
        segments: vec![xl_hir::UseId::from_index(9999)],
        generics: Vec::new(),
    };
    let violations = validate(&output.module);
    assert!(
        violations
            .iter()
            .any(|violation| violation.kind == ViolationKind::IndexOutOfBounds)
    );
}
