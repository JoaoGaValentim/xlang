//! Shared helpers for the `xl-hir` integration suites.
#![allow(dead_code)]
//!
//! The suites run the real Phase 1 frontend (lexer + parser) so the
//! lowering stages are exercised on exactly the ASTs the compiler
//! produces, including recovered trees.

use xl_hir::{
    ModuleId, PackageId, StructuralLowerOutput, lower_structure, validate, validate_resolutions,
};
use xl_parser::ParseOutput;
use xl_span::{FileId, SourceMap};

pub mod walk;

/// The file identifier every test source is registered under.
pub const FILE: FileId = FileId::from_index(0);

/// Parses `source` (lexing included) into items and diagnostics.
pub fn parse_source(source: &str) -> ParseOutput {
    let mut map = SourceMap::new();
    let file = map.add_file("test.x", source).expect("register file");
    xl_parser::parse(map.file(file).expect("file"), file)
}

/// Parses and structurally lowers `source` with the ratified budgets.
pub fn lower_source(source: &str) -> StructuralLowerOutput {
    let parsed = parse_source(source);
    lower_structure(
        &parsed.items,
        FILE,
        ModuleId::from_index(0),
        PackageId::from_index(0),
    )
}

/// Lowers `source`, asserting the stage completed and the module (and its
/// resolution table) validate.
pub fn lower_ok(source: &str) -> StructuralLowerOutput {
    let output = lower_source(source);
    assert!(output.is_complete(), "lowering aborted: {:?}", output.fate);
    let violations = validate(&output.module);
    assert!(
        violations.is_empty(),
        "module does not validate: {violations:#?}"
    );
    let resolution_violations = validate_resolutions(&output.module, &output.resolutions);
    assert!(
        resolution_violations.is_empty(),
        "resolutions do not validate: {resolution_violations:#?}"
    );
    output
}

use xl_hir::{Body, Def, DefKind, FnSig, HirModule};

/// Returns the single definition with `name` (panics when absent).
pub fn def_named<'m>(module: &'m HirModule, name: &str) -> &'m Def {
    let mut found = module.defs.iter().filter(|def| def.name == name);
    let def = found
        .next()
        .unwrap_or_else(|| panic!("no definition named {name:?}"));
    assert!(
        found.next().is_none(),
        "more than one definition named {name:?}"
    );
    def
}

/// Returns the definition with `name`, when present.
pub fn maybe_def_named<'m>(module: &'m HirModule, name: &str) -> Option<&'m Def> {
    module.defs.iter().find(|def| def.name == name)
}

/// Returns the body of a function or method definition named `name`.
pub fn body_of_fn_named<'m>(module: &'m HirModule, name: &str) -> &'m Body {
    match &def_named(module, name).kind {
        DefKind::Fn { body, .. } | DefKind::Method { body, .. } => {
            module.body(body.expect("function body")).expect("body")
        }
        other => panic!("definition {name:?} is not function-like: {other:?}"),
    }
}

/// Returns the signature of a function or method definition named `name`.
pub fn sig_of_fn_named<'m>(module: &'m HirModule, name: &str) -> &'m FnSig {
    match &def_named(module, name).kind {
        DefKind::Fn { sig, .. } | DefKind::Method { sig, .. } => {
            module.sig(*sig).expect("signature")
        }
        other => panic!("definition {name:?} is not function-like: {other:?}"),
    }
}

/// Returns the expression kind for an expression node (panics otherwise).
pub fn expr_of(module: &HirModule, id: xl_hir::HirId) -> &xl_hir::ExprKind {
    module
        .nodes
        .expr(id)
        .unwrap_or_else(|| panic!("node {} is not an expression", id))
}

/// Returns the statement kind for a statement node (panics otherwise).
pub fn stmt_of(module: &HirModule, id: xl_hir::HirId) -> &xl_hir::StmtKind {
    module
        .nodes
        .stmt(id)
        .unwrap_or_else(|| panic!("node {} is not a statement", id))
}

/// Returns the pattern kind for a pattern node (panics otherwise).
pub fn pat_of(module: &HirModule, id: xl_hir::HirId) -> &xl_hir::PatKind {
    module
        .nodes
        .pat(id)
        .unwrap_or_else(|| panic!("node {} is not a pattern", id))
}

/// Returns the tail expression of a body (panics when absent).
pub fn tail_of<'m>(_module: &'m HirModule, body: &'m Body) -> xl_hir::HirId {
    body.tail.unwrap_or_else(|| panic!("body has no tail"))
}

/// Builds a hand-written function item whose body tail is `depth` nested
/// arrays around an integer literal (built bottom-up, so the builder
/// itself is iterative).
pub fn deep_array_item(name: &str, item_index: u32, expr_base: u32, depth: u32) -> xl_ast::Item {
    use xl_ast::{
        Block, Expr, ExprId, ExprKind, FnDecl, FnSig, Ident, IntLit, Item, ItemId, ItemKind, Radix,
        Visibility,
    };
    use xl_span::Span;

    let zero = Span {
        file: FILE,
        lo: 0,
        hi: 0,
    };
    let mut expr = Expr {
        id: ExprId::from_index(expr_base),
        kind: ExprKind::Lit(xl_ast::Lit::Int(IntLit {
            magnitude: 1,
            radix: Radix::Dec,
        })),
        span: zero,
    };
    for level in 1..=depth {
        expr = Expr {
            id: ExprId::from_index(expr_base + level),
            kind: ExprKind::Array(vec![expr]),
            span: zero,
        };
    }
    let decl = FnDecl {
        vis: Visibility::Private,
        generics: None,
        sig: FnSig {
            name: Ident {
                name: name.to_string(),
                span: zero,
            },
            params: Vec::new(),
            ret: None,
            span: zero,
        },
        body: Block {
            stmts: Vec::new(),
            tail: Some(Box::new(expr)),
            span: zero,
        },
        span: zero,
    };
    Item {
        id: ItemId::from_index(item_index),
        kind: ItemKind::Fn(decl),
        span: zero,
    }
}

/// Collects the ids of every node whose origin is synthesized for
/// `construct` (an AST expression).
pub fn synthesized_nodes_for(module: &HirModule, expr: xl_ast::ExprId) -> Vec<xl_hir::HirId> {
    use xl_hir::{AstOrigin, AstOriginKey};
    let key = AstOriginKey::Expr(expr);
    let mut ids = Vec::new();
    for (index, origin) in module.origins.nodes.iter().enumerate() {
        if let AstOrigin::Synthesized { construct, .. } = origin
            && construct.source == key
        {
            ids.push(xl_hir::HirId::from_index(index as u32));
        }
    }
    ids
}
