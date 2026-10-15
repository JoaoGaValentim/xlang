//! Deterministic textual rendering of a lowered module.
//!
//! The dump is a debugging and snapshot-testing tool: a stable,
//! line-oriented format covering every arena entity, its payload, its
//! span, and its AST origin. It contains no memory addresses and no
//! hash-order-dependent output; identical modules render identically.

use std::fmt::Write;

use crate::hir::{
    BodyId, DefKind, ExprKind, FnSigId, HirKind, HirLit, HirModule, ModuleBindingTarget, PatKind,
    Resolution, Resolutions, StmtKind, TypeRefKind, UseId,
};
use crate::origin::{AnchorId, AstOrigin, AstOriginKey};

/// Renders the whole module in the stable dump format.
#[must_use]
pub fn dump_module(module: &HirModule) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "module {} package {} file {}",
        module.module, module.package, module.file
    );
    out.push_str("items:\n");
    for item in &module.items {
        let _ = writeln!(
            out,
            "  {} kind={} def={} span={} origin={}",
            item.id,
            item_kind(item.kind),
            item.def,
            span(item.span),
            origin(module.origins.item(item.id)),
        );
    }
    out.push_str("imports:\n");
    for record in &module.imports {
        let _ = writeln!(
            out,
            "  import item={} binding={} path=[{}] span={}",
            record.item,
            record.binding,
            join_ids(&record.path),
            span(record.decl_span),
        );
    }
    out.push_str("defs:\n");
    for def in &module.defs.defs {
        let _ = writeln!(
            out,
            "  {} name={:?} kind={} parent={} vis={} span={} origin={}",
            def.id,
            def.name,
            def_kind(&def.kind),
            match def.parent {
                Some(parent) => parent.to_string(),
                None => "-".to_string(),
            },
            visibility(def.visibility.clone()),
            span(def.span),
            origin(module.origins.def(def.id)),
        );
    }
    out.push_str("uses:\n");
    for (index, use_) in module.uses.uses.iter().enumerate() {
        let id = UseId::from_index(index as u32);
        let _ = writeln!(
            out,
            "  {} name={:?} span={} origin={}",
            id,
            use_.name,
            span(use_.span),
            origin(module.origins.use_(id)),
        );
    }
    out.push_str("bodies:\n");
    for (index, body) in module.bodies.bodies.iter().enumerate() {
        let id = BodyId::from_index(index as u32);
        let _ = writeln!(
            out,
            "  {} stmts=[{}] tail={} origin={}",
            id,
            join_ids(&body.stmts),
            optional(body.tail),
            origin(module.origins.body(id)),
        );
    }
    out.push_str("nodes:\n");
    for node in &module.nodes.nodes {
        let _ = writeln!(
            out,
            "  {} {} span={} origin={}",
            node.id,
            node_kind(&node.kind),
            span(node.span),
            origin(module.origins.node(node.id)),
        );
    }
    out.push_str("types:\n");
    for type_ref in &module.types.types {
        let _ = writeln!(
            out,
            "  {} {} span={} origin={}",
            type_ref.id,
            type_kind(&type_ref.kind),
            span(type_ref.span),
            origin(module.origins.type_ref(type_ref.id)),
        );
    }
    out.push_str("sigs:\n");
    for (index, sig) in module.sigs.sigs.iter().enumerate() {
        let id = FnSigId::from_index(index as u32);
        let _ = writeln!(
            out,
            "  {} name={} params=[{}] ret={} span={}",
            id,
            sig.name,
            join_ids(&sig.params),
            optional(sig.ret),
            span(sig.span),
        );
    }
    out
}

/// Renders the resolution side table in the stable dump format.
#[must_use]
pub fn dump_resolutions(module: &HirModule, resolutions: &Resolutions) -> String {
    let mut out = String::new();
    out.push_str("resolutions:\n");
    for (index, resolution) in resolutions.uses.iter().enumerate() {
        let id = UseId::from_index(index as u32);
        let name = module
            .uses
            .uses
            .get(index)
            .map_or("?", |use_| use_.name.as_str());
        let _ = writeln!(
            out,
            "  {} name={:?} -> {}",
            id,
            name,
            resolution_text(resolution)
        );
    }
    out
}

fn resolution_text(resolution: &Resolution) -> String {
    match resolution {
        Resolution::Def(def) => format!("def {def}"),
        Resolution::Global(global) => {
            format!("global ({}, {})", global.module, global.definition)
        }
        Resolution::Internal(internal) => format!("internal {internal:?}"),
        Resolution::Unresolved(reason) => format!("unresolved {reason:?}"),
    }
}

fn span(span: xl_span::Span) -> String {
    format!("{}..{}", span.lo, span.hi)
}

fn join_ids<T: std::fmt::Display>(ids: &[T]) -> String {
    let mut out = String::new();
    for (index, id) in ids.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{id}");
    }
    out
}

fn visibility(visibility: xl_ast::Visibility) -> &'static str {
    match visibility {
        xl_ast::Visibility::Private => "private",
        xl_ast::Visibility::Public(_) => "pub",
    }
}

fn item_kind(kind: crate::hir::HirItemKind) -> &'static str {
    match kind {
        crate::hir::HirItemKind::Fn => "fn",
        crate::hir::HirItemKind::Struct => "struct",
        crate::hir::HirItemKind::Enum => "enum",
        crate::hir::HirItemKind::Trait => "trait",
        crate::hir::HirItemKind::Impl => "impl",
        crate::hir::HirItemKind::Import => "import",
    }
}

fn def_kind(kind: &DefKind) -> String {
    match kind {
        DefKind::Fn {
            generics,
            sig,
            body,
        } => format!(
            "fn generics=[{}] sig={} body={}",
            join_ids(generics),
            sig,
            optional(*body)
        ),
        DefKind::Struct { generics, fields } => format!(
            "struct generics=[{}] fields=[{}]",
            join_ids(generics),
            join_ids(fields)
        ),
        DefKind::Enum { generics, variants } => format!(
            "enum generics=[{}] variants=[{}]",
            join_ids(generics),
            join_ids(variants)
        ),
        DefKind::Trait { generics, items } => format!(
            "trait generics=[{}] items=[{}]",
            join_ids(generics),
            join_ids(items)
        ),
        DefKind::Impl {
            generics,
            trait_ref,
            self_ty,
            is_inherent,
        } => format!(
            "impl generics=[{}] trait={} self_ty={} inherent={}",
            join_ids(generics),
            optional(*trait_ref),
            self_ty,
            is_inherent
        ),
        DefKind::Variant { payload } => format!("variant payload=[{}]", join_ids(payload)),
        DefKind::Field { ty } => format!("field ty={ty}"),
        DefKind::Method {
            generics,
            sig,
            body,
        } => format!(
            "method generics=[{}] sig={} body={}",
            join_ids(generics),
            sig,
            optional(*body)
        ),
        DefKind::TraitItem { sig } => format!("trait-item sig={sig}"),
        DefKind::GenericParam { bound } => format!("generic-param bound={}", optional(*bound)),
        DefKind::Param { ty, index } => format!("param ty={ty} index={index}"),
        DefKind::Receiver { index } => format!("receiver index={index}"),
        DefKind::Local { mutable, ty, body } => {
            format!("local mutable={mutable} ty={} body={}", optional(*ty), body)
        }
        DefKind::ModuleBinding(binding) => {
            let target = match binding.target {
                ModuleBindingTarget::Resolved(module) => format!("resolved {module}"),
                ModuleBindingTarget::Unavailable(module) => format!("unavailable {module}"),
                ModuleBindingTarget::Missing => "missing".to_string(),
                ModuleBindingTarget::Unresolved => "unresolved".to_string(),
            };
            format!(
                "module-binding path=[{}] alias={} target={}",
                binding.path.join(","),
                match &binding.alias {
                    Some(alias) => format!("{alias:?}"),
                    None => "-".to_string(),
                },
                target
            )
        }
        DefKind::Error => "error".to_string(),
    }
}

fn optional<T: std::fmt::Display>(id: Option<T>) -> String {
    match id {
        Some(id) => id.to_string(),
        None => "-".to_string(),
    }
}

fn node_kind(kind: &HirKind) -> String {
    match kind {
        HirKind::Expr(kind) => expr_kind(kind),
        HirKind::Stmt(kind) => match kind {
            StmtKind::Let { pattern, init } => {
                format!("stmt let pattern={pattern} init={init}")
            }
            StmtKind::Expr(expr) => format!("stmt expr {expr}"),
            StmtKind::Item(item) => format!("stmt item {item}"),
        },
        HirKind::Pat(kind) => pat_kind(kind),
    }
}

fn expr_kind(kind: &ExprKind) -> String {
    match kind {
        ExprKind::Lit(lit) => format!("expr lit {}", lit_text(lit)),
        ExprKind::Path { segments } => format!("expr path [{}]", join_ids(segments)),
        ExprKind::Unary(op, operand) => format!("expr unary {op:?} {operand}"),
        ExprKind::Binary(op, left, right) => format!("expr binary {op:?} {left} {right}"),
        ExprKind::Assign(op, target, value) => format!("expr assign {op:?} {target} {value}"),
        ExprKind::Call(callee, args) => {
            format!("expr call callee={callee} args=[{}]", join_ids(args))
        }
        ExprKind::Field(base, selection) => {
            format!("expr field base={base} sel={}", field_sel(*selection))
        }
        ExprKind::Index(base, index) => format!("expr index {base} {index}"),
        ExprKind::Cast(inner, ty) => format!("expr cast {inner} as {ty}"),
        ExprKind::If(cond, then_body, else_expr) => format!(
            "expr if cond={cond} then={then_body} else={}",
            optional(*else_expr)
        ),
        ExprKind::Match(scrutinee, arms) => {
            let arms: Vec<String> = arms
                .iter()
                .map(|arm| format!("({} => {})", arm.pat, arm.body))
                .collect();
            format!("expr match {scrutinee} [{}]", arms.join(","))
        }
        ExprKind::Block(body) => format!("expr block {body}"),
        ExprKind::While(cond, body) => format!("expr while cond={cond} body={body}"),
        ExprKind::Loop(body) => format!("expr loop {body}"),
        ExprKind::Return(operand) => format!("expr return {}", optional(*operand)),
        ExprKind::Break { target } => format!("expr break target={target}"),
        ExprKind::Continue { target } => format!("expr continue target={target}"),
        ExprKind::Tuple(elements) => format!("expr tuple [{}]", join_ids(elements)),
        ExprKind::Array(elements) => format!("expr array [{}]", join_ids(elements)),
        ExprKind::Error => "expr error".to_string(),
        ExprKind::BuiltinCall { op, args } => {
            format!("expr builtin {op:?} [{}]", join_ids(args))
        }
    }
}

fn pat_kind(kind: &PatKind) -> String {
    match kind {
        PatKind::Wildcard => "pat wildcard".to_string(),
        PatKind::Binding { local, candidate } => {
            format!("pat binding local={local} candidate={candidate}")
        }
        PatKind::Lit(lit) => format!("pat lit {}", lit_text(lit)),
        PatKind::Variant { path, args } => {
            format!(
                "pat variant path=[{}] args=[{}]",
                join_ids(path),
                join_ids(args)
            )
        }
        PatKind::Tuple(elements) => format!("pat tuple [{}]", join_ids(elements)),
        PatKind::Error => "pat error".to_string(),
    }
}

fn lit_text(lit: &HirLit) -> String {
    match lit {
        HirLit::Int { magnitude, radix } => format!("int {magnitude} {radix:?}"),
        HirLit::Float(value) => format!("float {value:?}"),
        HirLit::Bool(value) => format!("bool {value}"),
        HirLit::Char(value) => format!("char {value:?}"),
        HirLit::Str(string) => format!("str {:?}", string.text),
    }
}

fn type_kind(kind: &TypeRefKind) -> String {
    match kind {
        TypeRefKind::Path { segments, generics } => format!(
            "type path [{}] generics=[{}]",
            join_ids(segments),
            join_ids(generics)
        ),
        TypeRefKind::Tuple(elements) => format!("type tuple [{}]", join_ids(elements)),
        TypeRefKind::Slice(element) => format!("type slice {element}"),
        TypeRefKind::Array { elem, len } => format!("type array elem={elem} len={len}"),
        TypeRefKind::Fn { params, ret } => format!(
            "type fn params=[{}] ret={}",
            join_ids(params),
            optional(*ret)
        ),
        TypeRefKind::SelfTy => "type self".to_string(),
        TypeRefKind::Error => "type error".to_string(),
    }
}

fn field_sel(selection: crate::hir::FieldSel) -> String {
    match selection {
        crate::hir::FieldSel::Unresolved { name } => format!("unresolved {name}"),
        crate::hir::FieldSel::Field(def) => format!("field {def}"),
        crate::hir::FieldSel::Method(def) => format!("method {def}"),
        crate::hir::FieldSel::ModuleMember(def) => format!("module-member {def}"),
        crate::hir::FieldSel::Builtin(builtin) => format!("builtin {builtin:?}"),
    }
}

fn origin(origin: Option<&AstOrigin>) -> String {
    match origin {
        None => "missing".to_string(),
        Some(AstOrigin::Item(id)) => id.to_string(),
        Some(AstOrigin::Expr(id)) => id.to_string(),
        Some(AstOrigin::Structural { anchor, path }) => {
            format!("struct({},{:?})", anchor_text(*anchor), path)
        }
        Some(AstOrigin::Synthesized {
            construct,
            expansion,
            occurrence,
        }) => format!(
            "synth({},{},exp{},occ{})",
            desugar_text(construct.kind),
            key_text(&construct.source),
            expansion.index(),
            occurrence
        ),
    }
}

fn anchor_text(anchor: AnchorId) -> String {
    match anchor {
        AnchorId::Item(id) => id.to_string(),
        AnchorId::Expr(id) => id.to_string(),
    }
}

fn key_text(key: &AstOriginKey) -> String {
    match key {
        AstOriginKey::Item(id) => id.to_string(),
        AstOriginKey::Expr(id) => id.to_string(),
        AstOriginKey::Structural { anchor, path } => {
            format!("struct({},{:?})", anchor_text(*anchor), path)
        }
    }
}

fn desugar_text(kind: crate::origin::DesugarKind) -> &'static str {
    match kind {
        crate::origin::DesugarKind::For => "for",
        crate::origin::DesugarKind::Try => "try",
        crate::origin::DesugarKind::Interpolation => "interpolation",
    }
}
