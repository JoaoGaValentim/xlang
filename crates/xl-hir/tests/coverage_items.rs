//! Item-lowering coverage: every accepted item kind (`Directive 011` §8;
//! conformance-plan §7.1).

mod common;

use common::{def_named, lower_ok, maybe_def_named};
use xl_hir::{DefKind, HirItemKind, ModuleBindingTarget, TypeRefKind, Visibility};

#[test]
fn import_without_alias_binds_the_last_segment() {
    let output = lower_ok("import std.io;\nfn f() {}");
    let module = &output.module;
    assert_eq!(module.items.len(), 2);
    assert_eq!(module.items[0].kind, HirItemKind::Import);
    assert_eq!(module.imports.len(), 1);
    let record = &module.imports[0];
    let def = module.def(record.binding).expect("binding");
    assert_eq!(def.name, "io");
    let DefKind::ModuleBinding(binding) = &def.kind else {
        panic!("expected a module binding, got {:?}", def.kind);
    };
    assert_eq!(binding.path, vec!["std".to_string(), "io".to_string()]);
    assert_eq!(binding.alias, None);
    assert_eq!(binding.target, ModuleBindingTarget::Unresolved);
    assert_eq!(record.path.len(), 2, "one use per segment");
    let names: Vec<&str> = record
        .path
        .iter()
        .map(|use_| module.use_(*use_).expect("use").name.as_str())
        .collect();
    assert_eq!(names, ["std", "io"]);
    // The path uses are unresolved, pending name resolution.
    for use_ in &record.path {
        assert!(matches!(
            output.resolutions.get(*use_),
            Some(xl_hir::Resolution::Unresolved(_))
        ));
    }
}

#[test]
fn import_with_alias_binds_the_alias() {
    let output = lower_ok("import std.io as io2;\nfn f() {}");
    let module = &output.module;
    let record = &module.imports[0];
    let def = module.def(record.binding).expect("binding");
    assert_eq!(def.name, "io2");
    let DefKind::ModuleBinding(binding) = &def.kind else {
        panic!("expected a module binding");
    };
    assert_eq!(binding.alias.as_deref(), Some("io2"));
}

#[test]
fn function_signatures_carry_generics_params_and_returns() {
    let output =
        lower_ok("pub fn f<T: Show>(self: i64, x: T, y: (i64, bool)) -> T { x }\nfn g() {}");
    let module = &output.module;
    let def = def_named(module, "f");
    assert!(matches!(def.visibility, Visibility::Public(_)));
    let DefKind::Fn {
        generics,
        sig,
        body,
    } = &def.kind
    else {
        panic!("expected a function");
    };
    assert_eq!(generics.len(), 1);
    assert!(body.is_some());
    let generic = module.def(generics[0]).expect("generic");
    assert_eq!(generic.name, "T");
    assert_eq!(generic.parent, Some(def.id));
    let DefKind::GenericParam { bound } = &generic.kind else {
        panic!("expected a generic parameter");
    };
    let bound = bound.expect("bound");
    let TypeRefKind::Path { segments, .. } = &module.type_ref(bound).expect("bound type").kind
    else {
        panic!("expected a path bound");
    };
    assert_eq!(module.use_(*segments.last().unwrap()).unwrap().name, "Show");

    let signature = module.sig(*sig).expect("sig");
    assert_eq!(signature.params.len(), 3);
    assert_eq!(signature.ret, Some(signature.ret.unwrap()));
    let first = module.def(signature.params[0]).expect("param");
    assert!(matches!(first.kind, DefKind::Param { index: 0, .. }));
    assert_eq!(first.name, "self");
}

#[test]
fn receiver_parameters_are_recorded() {
    let output = lower_ok("struct P { x: i64 }\nimpl P { fn m(self) -> i64 { self.x } }");
    let module = &output.module;
    let method = def_named(module, "m");
    let DefKind::Method { sig, .. } = &method.kind else {
        panic!("expected a method");
    };
    let signature = module.sig(*sig).expect("sig");
    let receiver = module.def(signature.params[0]).expect("receiver");
    assert!(matches!(receiver.kind, DefKind::Receiver { index: 0 }));
    assert_eq!(receiver.name, "self");
    assert_eq!(receiver.parent, Some(method.id));
}

#[test]
fn struct_fields_record_visibility_types_and_parents() {
    let output = lower_ok("pub struct Point { pub x: f64, y: f64 }");
    let module = &output.module;
    let def = def_named(module, "Point");
    let DefKind::Struct { generics, fields } = &def.kind else {
        panic!("expected a struct");
    };
    assert!(generics.is_empty());
    assert_eq!(fields.len(), 2);
    let x = module.def(fields[0]).expect("field x");
    assert_eq!(x.name, "x");
    assert!(matches!(x.visibility, Visibility::Public(_)));
    assert_eq!(x.parent, Some(def.id));
    let DefKind::Field { ty } = &x.kind else {
        panic!("expected a field");
    };
    let TypeRefKind::Path { segments, .. } = &module.type_ref(*ty).unwrap().kind else {
        panic!("expected a path type");
    };
    assert_eq!(module.use_(*segments.last().unwrap()).unwrap().name, "f64");
    let y = module.def(fields[1]).expect("field y");
    assert!(matches!(y.visibility, Visibility::Private));
}

#[test]
fn enum_variants_record_payloads() {
    let output = lower_ok("enum Shape { Circle(f64), Rect(f64, f64), Empty }");
    let module = &output.module;
    let def = def_named(module, "Shape");
    let DefKind::Enum { variants, .. } = &def.kind else {
        panic!("expected an enum");
    };
    assert_eq!(variants.len(), 3);
    let circle = module.def(variants[0]).expect("Circle");
    assert_eq!(circle.parent, Some(def.id));
    let DefKind::Variant { payload } = &circle.kind else {
        panic!("expected a variant");
    };
    assert_eq!(payload.len(), 1);
    let empty = module.def(variants[2]).expect("Empty");
    let DefKind::Variant { payload } = &empty.kind else {
        panic!("expected a variant");
    };
    assert!(payload.is_empty());
}

#[test]
fn traits_and_trait_items_are_recorded() {
    let output = lower_ok("trait Show<T> { fn show(self) -> T; }");
    let module = &output.module;
    let def = def_named(module, "Show");
    let DefKind::Trait { generics, items } = &def.kind else {
        panic!("expected a trait");
    };
    assert_eq!(generics.len(), 1);
    assert_eq!(items.len(), 1);
    let item = module.def(items[0]).expect("trait item");
    assert_eq!(item.name, "show");
    assert_eq!(item.parent, Some(def.id));
    assert!(matches!(item.kind, DefKind::TraitItem { .. }));
}

#[test]
fn inherent_and_trait_impls_are_distinguished() {
    let output = lower_ok(
        "struct P { x: i64 }\ntrait T { fn t(self) -> i64; }\nimpl P { fn m(self) -> i64 { self.x } }\nimpl T for P { fn t(self) -> i64 { 0 } }",
    );
    let module = &output.module;
    let impls: Vec<_> = module
        .defs
        .iter()
        .filter(|def| matches!(def.kind, DefKind::Impl { .. }))
        .collect();
    assert_eq!(impls.len(), 2);
    let DefKind::Impl {
        is_inherent,
        trait_ref,
        self_ty,
        ..
    } = &impls[0].kind
    else {
        unreachable!()
    };
    assert!(*is_inherent);
    assert!(trait_ref.is_none());
    assert!(module.type_ref(*self_ty).is_some());
    let DefKind::Impl {
        is_inherent,
        trait_ref,
        ..
    } = &impls[1].kind
    else {
        unreachable!()
    };
    assert!(!*is_inherent);
    assert!(trait_ref.is_some());
    let method = module
        .defs
        .iter()
        .find(|def| def.name == "t" && matches!(def.kind, DefKind::Method { .. }))
        .expect("impl method t");
    assert_eq!(method.parent, Some(impls[1].id));
    let trait_item = module
        .defs
        .iter()
        .find(|def| def.name == "t" && matches!(def.kind, DefKind::TraitItem { .. }))
        .expect("trait item t");
    assert_ne!(method.id, trait_item.id, "same name, distinct identities");
}

#[test]
fn nested_items_are_item_slots_with_their_own_definitions() {
    let output = lower_ok("fn outer() -> i64 { fn inner(x: i64) -> i64 { x + 1 } inner(1) }");
    let module = &output.module;
    assert_eq!(module.items.len(), 2, "module fn plus nested fn");
    assert_eq!(module.items[1].kind, HirItemKind::Fn);
    let inner = def_named(module, "inner");
    assert!(matches!(inner.kind, DefKind::Fn { .. }));
    assert!(inner.parent.is_none());
    let _ = maybe_def_named(module, "outer").expect("outer");
}

#[test]
fn pattern_bindings_introduce_locals_with_bodies() {
    let output = lower_ok("fn f() -> i64 { let (a, b) = (1, 2); a }");
    let module = &output.module;
    let a = def_named(module, "a");
    let DefKind::Local { mutable, ty, body } = &a.kind else {
        panic!("expected a local");
    };
    assert!(!mutable);
    assert!(ty.is_none());
    assert!(module.body(*body).is_some());
    let name = |id: xl_hir::DefId| module.def(id).unwrap().name.clone();
    let b = def_named(module, "b");
    assert_eq!(name(a.id), "a");
    assert_eq!(name(b.id), "b");
}
