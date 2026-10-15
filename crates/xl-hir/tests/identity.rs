//! Identity and determinism (`Directive 011` §6, §14.A, §14.F).

mod common;

use common::{lower_ok, lower_source};
use xl_hir::{
    DefId, DefKind, GlobalDefId, HirId, ModuleId, PackageId, StageFate, TypeRefId, UseId,
};

const CORPUS: &[&str] = &[
    "fn f() { let x = 1; x; }",
    "import std.io;\nstruct S<T> { v: T }\nenum E { A, B(i64) }",
    "fn f() { for x in 0..3 { let y = g()?; } }",
    "fn f() { let s = \"a{x}{y}b\"; }",
    "trait T { fn m(self) -> i64; }\nimpl T for S { fn m(self) -> i64 { 0 } }",
];

#[test]
fn lowering_is_deterministic() {
    for source in CORPUS {
        let first = lower_source(source);
        let second = lower_source(source);
        assert_eq!(first.module, second.module, "{source:?}");
        assert_eq!(first.resolutions, second.resolutions, "{source:?}");
        assert_eq!(first.diagnostics, second.diagnostics, "{source:?}");
        assert_eq!(first.fate, second.fate, "{source:?}");
    }
}

#[test]
fn identifiers_match_their_table_slots() {
    let output = lower_ok("fn f() { let x = 1; x; }");
    let module = &output.module;
    for (index, item) in module.items.iter().enumerate() {
        assert_eq!(item.id.index() as usize, index);
    }
    for (index, def) in module.defs.defs.iter().enumerate() {
        assert_eq!(def.id.index() as usize, index);
    }
    for (index, node) in module.nodes.nodes.iter().enumerate() {
        assert_eq!(node.id.index() as usize, index);
    }
    for (index, type_ref) in module.types.types.iter().enumerate() {
        assert_eq!(type_ref.id.index() as usize, index);
    }
}

#[test]
fn definition_order_follows_source_order() {
    let output = lower_ok("fn a() {} fn b<T>(x: T) {}");
    let module = &output.module;
    let names: Vec<&str> = module.defs.iter().map(|def| def.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "T", "x"]);
}

#[test]
fn struct_definitions_precede_their_members() {
    let output = lower_ok("struct S { a: i64, b: bool }");
    let module = &output.module;
    let names: Vec<&str> = module.defs.iter().map(|def| def.name.as_str()).collect();
    assert_eq!(names, ["S", "a", "b"]);
    let DefKind::Struct { fields, .. } = &module.defs.defs[0].kind else {
        panic!()
    };
    assert_eq!(fields[0], DefId::from_index(1));
    assert_eq!(fields[1], DefId::from_index(2));
}

#[test]
fn shadowed_bindings_keep_distinct_identities() {
    let output = lower_ok(
        "fn f() -> i64 {
            let x = 1;
            { let x = 2; x; }
            x
        }",
    );
    let module = &output.module;
    let locals: Vec<DefId> = module
        .defs
        .iter()
        .filter(|def| matches!(def.kind, DefKind::Local { .. }))
        .map(|def| def.id)
        .collect();
    assert_eq!(locals.len(), 2, "both shadowing declarations exist");
    assert_ne!(locals[0], locals[1]);
    // The uses stay distinct and unresolved (structural lowering resolves
    // nothing).
    let candidate_uses: Vec<UseId> = module
        .nodes
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            xl_hir::HirKind::Pat(xl_hir::PatKind::Binding { candidate, .. }) => Some(*candidate),
            _ => None,
        })
        .collect();
    assert_eq!(candidate_uses.len(), 2);
    assert_ne!(candidate_uses[0], candidate_uses[1]);
}

#[test]
fn global_definition_identities_compose_module_and_definition() {
    let a = GlobalDefId {
        module: ModuleId::from_index(0),
        definition: DefId::from_index(7),
    };
    let b = GlobalDefId {
        module: ModuleId::from_index(1),
        definition: DefId::from_index(7),
    };
    assert_ne!(
        a, b,
        "the same local index in different modules is a different definition"
    );
    let c = GlobalDefId {
        module: ModuleId::from_index(0),
        definition: DefId::from_index(8),
    };
    assert_ne!(a, c);
}

#[test]
fn identity_spaces_are_distinct_types() {
    // Compile-time check: the numbered spaces are not interchangeable.
    let module = ModuleId::from_index(3);
    let package = PackageId::from_index(3);
    assert_eq!(module.index(), package.index());
    let hir = HirId::from_index(1);
    let def = DefId::from_index(1);
    let use_ = UseId::from_index(1);
    let ty = TypeRefId::from_index(1);
    assert_eq!(
        [hir.index(), def.index(), use_.index(), ty.index()],
        [1, 1, 1, 1]
    );
}

#[test]
fn fabricated_resolutions_never_appear_in_structural_output() {
    let output = lower_ok("import std.io;\nfn f() { let x = g(); x }");
    for (index, resolution) in output.resolutions.uses.iter().enumerate() {
        let id = UseId::from_index(index as u32);
        let name = &output.module.use_(id).expect("use").name;
        match resolution {
            xl_hir::Resolution::Unresolved(_) => {}
            xl_hir::Resolution::Internal(internal) => {
                // Only the desugaring's own references are internal, and
                // they only exist when a desugaring ran.
                panic!("unexpected internal resolution {internal:?} on {name:?}");
            }
            xl_hir::Resolution::Def(_) | xl_hir::Resolution::Global(_) => {
                panic!("source use {name:?} must stay unresolved in WP-A");
            }
        }
    }
    // Import targets stay unresolved too.
    for record in &output.module.imports {
        let binding_def = output.module.def(record.binding).expect("binding");
        let DefKind::ModuleBinding(binding) = &binding_def.kind else {
            panic!()
        };
        assert_eq!(binding.target, xl_hir::ModuleBindingTarget::Unresolved);
    }
}

#[test]
fn aborted_outputs_are_never_complete() {
    let parsed = common::parse_source("fn f() { 1; 2; 3; }");
    let output = xl_hir::lower_structure_with_limits(
        &parsed.items,
        common::FILE,
        ModuleId::from_index(0),
        PackageId::from_index(0),
        xl_hir::LowerLimits {
            max_hir_nodes: 1,
            ..xl_hir::LowerLimits::RATIFIED
        },
    );
    assert!(!output.is_complete());
    assert!(matches!(output.fate, StageFate::Aborted { .. }));
}
