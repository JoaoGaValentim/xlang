//! An independent AST walker mirroring the documented child-index schema
//! (`xl_hir::origin` module docs).
//!
//! The walker computes, for every AST construct the lowering traversal is
//! documented to visit, the expected [`AstOriginKey`]. The origin tests
//! assert the *reverse mapping*: every expected key has at least one HIR
//! entity whose origin (or synthesized construct source) is that key, and
//! no HIR entity carries a key that no visited construct produces.
//!
//! The walker is deliberately written from the schema documentation (not
//! from the lowering code) so a schema drift fails the tests.
#![allow(dead_code)]

use std::collections::HashSet;

use xl_ast::{
    Block, EnumDecl, Expr, ExprKind, FnDecl, FnSig, Generics, ImplDecl, ImplItem, Item, ItemKind,
    Pat, PatKind, PathType, Stmt, StmtKind, StringPart, StructDecl, TraitDecl, Type, TypeKind,
};
use xl_hir::{AnchorId, AstOrigin, AstOriginKey, HirModule};
use xl_span::Span;

/// The origin test helper: collects expected keys from items.
#[derive(Default)]
pub struct ExpectedKeys {
    keys: HashSet<AstOriginKey>,
    in_self: bool,
}

impl ExpectedKeys {
    /// Collects the expected keys of a module's items.
    pub fn collect(items: &[Item]) -> HashSet<AstOriginKey> {
        let mut collector = Self::default();
        for item in items {
            collector.item(item);
        }
        collector.keys
    }

    fn insert(&mut self, key: AstOriginKey) {
        self.keys.insert(key);
    }

    fn structural(&mut self, anchor: AnchorId, path: Vec<u32>) {
        self.insert(AstOriginKey::Structural { anchor, path });
    }

    // ------------------------------------------------------------------

    fn item(&mut self, item: &Item) {
        let anchor = AnchorId::Item(item.id);
        self.insert(AstOriginKey::Item(item.id));
        let saved_self = self.in_self;
        match &item.kind {
            ItemKind::Fn(decl) => {
                self.fn_decl(anchor, &[], decl, true);
            }
            ItemKind::Struct(decl) => self.struct_decl(anchor, decl),
            ItemKind::Enum(decl) => self.enum_decl(anchor, decl),
            ItemKind::Trait(decl) => {
                self.in_self = true;
                self.trait_decl(anchor, decl);
            }
            ItemKind::Impl(decl) => {
                self.in_self = true;
                self.impl_decl(anchor, decl);
            }
            ItemKind::Import(decl) => {
                for (index, _) in decl.path.iter().enumerate() {
                    // Path segment uses sit at child `index`.
                    self.structural(anchor, vec![index as u32]);
                }
                match (decl.alias.is_some(), decl.path.len()) {
                    (true, n) => self.structural(anchor, vec![n as u32]),
                    (false, 0) => {}
                    (false, n) => self.structural(anchor, vec![n as u32 - 1]),
                }
            }
        }
        self.in_self = saved_self;
    }

    /// Walks a function-like declaration under `prefix` (the declaration's
    /// child slots: `0` generics, `1` signature, `2` body).
    fn fn_decl(&mut self, anchor: AnchorId, prefix: &[u32], decl: &FnDecl, with_body: bool) {
        let mut slot = prefix.to_vec();
        slot.push(0);
        self.generics(anchor, &mut slot, &decl.generics);
        slot.pop();
        slot.push(1);
        self.fn_sig(anchor, &mut slot, &decl.sig);
        slot.pop();
        if with_body {
            slot.push(2);
            self.block_at(anchor, &mut slot, &decl.body);
            slot.pop();
        }
    }

    fn generics(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, generics: &Option<Generics>) {
        if let Some(generics) = generics {
            for (index, param) in generics.params.iter().enumerate() {
                prefix.push(index as u32);
                // The parameter definition sits at the parameter slot; the
                // bound (when written) is child 1.
                self.structural(anchor, prefix.clone());
                if let Some(bound) = &param.bound {
                    prefix.push(1);
                    self.path_type(anchor, prefix, bound);
                    prefix.pop();
                }
                prefix.pop();
            }
        }
    }

    fn fn_sig(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, sig: &FnSig) {
        for (index, param) in sig.params.iter().enumerate() {
            prefix.push(index as u32);
            // Every parameter (receiver or named) has its definition here.
            self.structural(anchor, prefix.clone());
            if let xl_ast::Param::Named { ty, .. } = param {
                prefix.push(0);
                self.ty(anchor, prefix, ty);
                prefix.pop();
            }
            prefix.pop();
        }
        if let Some(ret) = &sig.ret {
            prefix.push(sig.params.len() as u32);
            self.ty(anchor, prefix, ret);
            prefix.pop();
        }
    }

    fn struct_decl(&mut self, anchor: AnchorId, decl: &StructDecl) {
        self.generics(anchor, &mut vec![0], &decl.generics);
        for (index, field) in decl.fields.iter().enumerate() {
            let mut prefix = vec![1 + index as u32];
            self.structural(anchor, prefix.clone());
            prefix.push(0);
            self.ty(anchor, &mut prefix, &field.ty);
        }
    }

    fn enum_decl(&mut self, anchor: AnchorId, decl: &EnumDecl) {
        self.generics(anchor, &mut vec![0], &decl.generics);
        for (index, variant) in decl.variants.iter().enumerate() {
            let mut prefix = vec![1 + index as u32];
            self.structural(anchor, prefix.clone());
            for (payload_index, ty) in variant.payload.iter().enumerate() {
                prefix.push(payload_index as u32);
                self.ty(anchor, &mut prefix, ty);
                prefix.pop();
            }
        }
    }

    fn trait_decl(&mut self, anchor: AnchorId, decl: &TraitDecl) {
        self.generics(anchor, &mut vec![0], &decl.generics);
        for (index, trait_item) in decl.items.iter().enumerate() {
            let mut prefix = vec![1 + index as u32];
            self.structural(anchor, prefix.clone());
            prefix.push(0);
            self.fn_sig(anchor, &mut prefix, &trait_item.sig);
        }
    }

    fn impl_decl(&mut self, anchor: AnchorId, decl: &ImplDecl) {
        self.generics(anchor, &mut vec![0], &decl.generics);
        if let Some(trait_path) = &decl.trait_ {
            self.path_type(anchor, &mut vec![1], trait_path);
        }
        self.path_type(anchor, &mut vec![2], &decl.self_ty);
        for (index, impl_item) in decl.items.iter().enumerate() {
            let ImplItem::Fn(method) = impl_item;
            let prefix = vec![3 + index as u32];
            self.structural(anchor, prefix.clone());
            self.fn_decl(anchor, &prefix, method, true);
        }
    }

    // ------------------------------------------------------------------
    // Statements and blocks
    // ------------------------------------------------------------------

    fn block_at(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, block: &Block) {
        self.structural(anchor, prefix.clone());
        self.block_children(anchor, prefix, block);
    }

    fn block_children(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, block: &Block) {
        for (index, stmt) in block.stmts.iter().enumerate() {
            prefix.push(index as u32);
            self.stmt(anchor, prefix, stmt);
            prefix.pop();
        }
        if let Some(tail) = &block.tail {
            prefix.push(block.stmts.len() as u32);
            self.expr(anchor, prefix, tail);
            prefix.pop();
        }
    }

    fn stmt(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, stmt: &Stmt) {
        self.structural(anchor, prefix.clone());
        match &stmt.kind {
            StmtKind::Let(let_stmt) => {
                // Pattern at 0, annotation at 1, initializer at 2.
                prefix.push(0);
                self.pat(anchor, prefix, &let_stmt.pat);
                prefix.pop();
                if let Some(ty) = &let_stmt.ty {
                    prefix.push(1);
                    self.ty(anchor, prefix, ty);
                    prefix.pop();
                }
                prefix.push(2);
                self.expr(anchor, prefix, &let_stmt.init);
                prefix.pop();
            }
            StmtKind::Expr(expr) => {
                prefix.push(0);
                self.expr(anchor, prefix, expr);
                prefix.pop();
            }
            StmtKind::Item(item) => {
                self.item(item);
            }
        }
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    fn expr(&mut self, _at: AnchorId, _prefix: &mut Vec<u32>, expr: &Expr) {
        self.insert(AstOriginKey::Expr(expr.id));
        // Entering an expression resets the anchor and the path.
        let anchor = AnchorId::Expr(expr.id);
        let prefix: &mut Vec<u32> = &mut Vec::new();
        match &expr.kind {
            ExprKind::Lit(xl_ast::Lit::Str(string)) => {
                for (index, part) in string.parts.iter().enumerate() {
                    if let StringPart::Expr(part_expr) = part {
                        prefix.push(index as u32);
                        self.expr(anchor, prefix, part_expr);
                        prefix.pop();
                    }
                }
            }
            ExprKind::Lit(_) => {}
            ExprKind::Path(path) => {
                for index in 0..path.segments.len() {
                    self.structural(anchor, vec![index as u32]);
                }
            }
            ExprKind::Unary(_, operand) | ExprKind::Try(operand) => {
                self.expr(anchor, &mut vec![0], operand);
            }
            ExprKind::Binary(_, left, right) | ExprKind::Range(left, right) => {
                self.expr(anchor, &mut vec![0], left);
                self.expr(anchor, &mut vec![1], right);
            }
            ExprKind::Assign(_, target, value) => {
                self.expr(anchor, &mut vec![0], target);
                self.expr(anchor, &mut vec![1], value);
            }
            ExprKind::Call(callee, args) => {
                self.expr(anchor, &mut vec![0], callee);
                for (index, arg) in args.iter().enumerate() {
                    self.expr(anchor, &mut vec![1 + index as u32], arg);
                }
            }
            ExprKind::Field(base, name) => {
                self.expr(anchor, &mut vec![0], base);
                self.structural(anchor, vec![1]);
                let _ = name;
            }
            ExprKind::Index(base, index) => {
                self.expr(anchor, &mut vec![0], base);
                self.expr(anchor, &mut vec![1], index);
            }
            ExprKind::Cast(inner, ty) => {
                self.expr(anchor, &mut vec![0], inner);
                self.ty(anchor, &mut vec![1], ty);
            }
            ExprKind::If(cond, then_block, else_expr) => {
                self.expr(anchor, &mut vec![0], cond);
                self.block_at(anchor, &mut vec![1], then_block);
                if let Some(else_expr) = else_expr {
                    self.expr(anchor, &mut vec![2], else_expr);
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.expr(anchor, &mut vec![0], scrutinee);
                for (index, arm) in arms.iter().enumerate() {
                    self.pat(anchor, &mut vec![1 + index as u32, 0], &arm.pat);
                    self.expr(anchor, &mut vec![1 + index as u32, 1], &arm.body);
                }
            }
            ExprKind::Block(block) => self.block_at(anchor, &mut vec![0], block),
            ExprKind::While(cond, block) => {
                self.expr(anchor, &mut vec![0], cond);
                self.block_at(anchor, &mut vec![1], block);
            }
            ExprKind::Loop(block) => self.block_at(anchor, &mut vec![0], block),
            ExprKind::For(pat, iterable, block) => {
                // A recovered iterable poisons the whole construct: only
                // the `for` expression itself exists.
                if iterable.span.is_empty()
                    && matches!(&iterable.kind, ExprKind::Tuple(elements) if elements.is_empty())
                {
                    return;
                }
                self.pat(anchor, &mut vec![0], pat);
                self.expr(anchor, &mut vec![1], iterable);
                self.block_at(anchor, &mut vec![2], block);
            }
            ExprKind::Return(operand) => {
                if let Some(operand) = operand {
                    self.expr(anchor, &mut vec![0], operand);
                }
            }
            ExprKind::Break | ExprKind::Continue => {}
            ExprKind::Tuple(elements) | ExprKind::Array(elements) => {
                for (index, element) in elements.iter().enumerate() {
                    self.expr(anchor, &mut vec![index as u32], element);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Patterns and types
    // ------------------------------------------------------------------

    #[allow(clippy::ptr_arg)]
    fn pat(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, pat: &Pat) {
        self.structural(anchor, prefix.clone());
        match &pat.kind {
            PatKind::Wildcard | PatKind::Binding(_) => {}
            PatKind::Lit(_) => {
                // An interpolated string pattern is poisoned; no children.
            }
            PatKind::Path { path, args } => {
                let n = path.segments.len() as u32;
                for index in 0..n {
                    let mut inner = prefix.clone();
                    inner.push(index);
                    self.structural(anchor, inner);
                }
                if let Some(args) = args {
                    for (index, arg) in args.iter().enumerate() {
                        let mut inner = prefix.clone();
                        inner.push(n + index as u32);
                        self.pat(anchor, &mut inner, arg);
                    }
                }
            }
            PatKind::Tuple(elements) => {
                for (index, element) in elements.iter().enumerate() {
                    let mut inner = prefix.clone();
                    inner.push(index as u32);
                    self.pat(anchor, &mut inner, element);
                }
            }
        }
    }

    fn ty(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, ty: &Type) {
        self.structural(anchor, prefix.clone());
        match &ty.kind {
            TypeKind::Path(path_type) => self.path_type(anchor, prefix, path_type),
            TypeKind::Tuple(elements) => {
                for (index, element) in elements.iter().enumerate() {
                    let mut inner = prefix.clone();
                    inner.push(index as u32);
                    self.ty(anchor, &mut inner, element);
                }
            }
            TypeKind::Slice(element) => {
                let mut inner = prefix.clone();
                inner.push(0);
                self.ty(anchor, &mut inner, element);
            }
            TypeKind::Array(element, length) => {
                let mut inner = prefix.clone();
                inner.push(0);
                self.ty(anchor, &mut inner, element);
                inner.pop();
                inner.push(1);
                self.expr(anchor, &mut inner, length);
            }
            TypeKind::Fn { params, ret } => {
                for (index, param) in params.iter().enumerate() {
                    let mut inner = prefix.clone();
                    inner.push(index as u32);
                    self.ty(anchor, &mut inner, param);
                }
                if let Some(ret) = ret {
                    let mut inner = prefix.clone();
                    inner.push(params.len() as u32);
                    self.ty(anchor, &mut inner, ret);
                }
            }
        }
    }

    #[allow(clippy::ptr_arg)]
    fn path_type(&mut self, anchor: AnchorId, prefix: &mut Vec<u32>, path_type: &PathType) {
        // The type node itself (the caller may already have recorded it).
        self.structural(anchor, prefix.clone());
        // The contextual `Self` rule: a lone `Self` (no generics) inside a
        // trait/impl context is `SelfTy` and records no segment use.
        if self.in_self
            && path_type.segments.len() == 1
            && path_type.segments[0].name == "Self"
            && path_type.generics.is_none()
        {
            return;
        }
        let n = path_type.segments.len() as u32;
        for index in 0..n {
            let mut inner = prefix.clone();
            inner.push(index);
            self.structural(anchor, inner);
        }
        if let Some(arguments) = &path_type.generics {
            for (index, argument) in arguments.iter().enumerate() {
                let mut inner = prefix.clone();
                inner.push(n + index as u32);
                self.ty(anchor, &mut inner, argument);
            }
        }
    }
}

/// The source construct key of an HIR entity's origin: synthesized
/// entities count towards their originating construct.
#[must_use]
pub fn origin_construct_key(origin: &AstOrigin) -> AstOriginKey {
    match origin {
        AstOrigin::Item(id) => AstOriginKey::Item(*id),
        AstOrigin::Expr(id) => AstOriginKey::Expr(*id),
        AstOrigin::Structural { anchor, path } => AstOriginKey::Structural {
            anchor: *anchor,
            path: path.clone(),
        },
        AstOrigin::Synthesized { construct, .. } => construct.source.clone(),
    }
}

/// Collects every construct key recorded in the module's origin tables
/// (both directions are checked by set comparison in the tests).
#[must_use]
pub fn collected_construct_keys(module: &HirModule) -> HashSet<AstOriginKey> {
    let mut keys = HashSet::new();
    let tables = [
        &module.origins.items,
        &module.origins.defs,
        &module.origins.uses,
        &module.origins.bodies,
        &module.origins.nodes,
        &module.origins.types,
    ];
    for table in tables {
        for origin in table {
            keys.insert(origin_construct_key(origin));
        }
    }
    keys
}

/// Converts a span into a stable, printable description (debug helper).
#[must_use]
pub fn describe_span(span: Span) -> String {
    format!("{}..{}", span.lo, span.hi)
}
