//! Minimal structural validation for XLANG syntax trees.
//!
//! The validator checks **structural** invariants only (`Directive 007
//! §10`): span sanity and containment, file consistency within a tree,
//! presence of required names/paths/generic lists, block tail separation,
//! interpolation part ordering, and traversal depth. It does **not** touch
//! a [`xl_span::SourceMap`] (source-map validity), resolve names, infer
//! types, or apply any semantic rule; those belong to later phases.
//!
//! Recovered parser nodes are intentionally tolerated: zero-width spans and
//! empty optional lists are valid wherever the grammar permits them.
//! Violations are returned as data (never panics), so callers can report
//! them with `xl-diag` if they wish.

use std::fmt;

use xl_span::Span;

use crate::ast::{
    Arm, Block, EnumDecl, Expr, ExprKind, Field, FnDecl, FnSig, Generics, Ident, ImplDecl,
    ImplItem, ImportDecl, Item, ItemKind, Lit, Param, Pat, PatKind, Path, PathType, Stmt, StmtKind,
    StructDecl, TraitDecl, TraitItem, Type, TypeKind, Variant,
};

/// Maximum node-visit depth supported by the validator (see
/// [`crate::print::MAX_PRINT_DEPTH`] for the rationale).
pub const MAX_VALIDATE_DEPTH: u32 = 128;

/// Category of a [`Violation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// A span has `lo > hi`.
    InvertedSpan,
    /// A child span is not contained in its parent's span.
    ChildOutsideParent,
    /// A child span refers to a different file than its parent.
    FileMismatch,
    /// A required identifier has an empty name.
    EmptyIdentifier,
    /// A path (expression, type, or import) has no segments.
    EmptyPath,
    /// A generic parameter or argument list is empty.
    EmptyGenerics,
    /// A block tail expression begins before the last statement ends.
    TailOrdering,
    /// String-literal parts are not in non-decreasing source order or lie
    /// outside the literal.
    InterpolationOrder,
    /// The tree exceeds [`MAX_VALIDATE_DEPTH`] on one path.
    DepthExceeded,
}

/// A structural violation with its location and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The violation category.
    pub kind: ViolationKind,
    /// The span of the offending node (or inner span).
    pub span: Span,
    /// A human-readable description.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Validates one item tree. An empty result means no structural violation
/// was found.
#[must_use]
pub fn validate_item(item: &Item) -> Vec<Violation> {
    let mut validator = Validator::new();
    validator.item(item, item.span);
    validator.violations
}

/// Validates one standalone expression tree.
#[must_use]
pub fn validate_expr(expr: &Expr) -> Vec<Violation> {
    let mut validator = Validator::new();
    validator.expr(expr, expr.span);
    validator.violations
}

/// Validates one standalone type tree.
#[must_use]
pub fn validate_type(ty: &Type) -> Vec<Violation> {
    let mut validator = Validator::new();
    validator.ty(ty, ty.span);
    validator.violations
}

/// Validates one standalone pattern tree.
#[must_use]
pub fn validate_pattern(pat: &Pat) -> Vec<Violation> {
    let mut validator = Validator::new();
    validator.pat(pat, pat.span);
    validator.violations
}

struct Validator {
    violations: Vec<Violation>,
    depth: u32,
}

impl Validator {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            depth: 0,
        }
    }

    fn push(&mut self, kind: ViolationKind, span: Span, message: &str) {
        self.violations.push(Violation {
            kind,
            span,
            message: message.to_string(),
        });
    }

    /// Checks one span against its parent. Returns `false` when the span is
    /// inverted (containment cannot be judged).
    fn span(&mut self, span: Span, parent: Span) -> bool {
        if span.lo > span.hi {
            self.push(ViolationKind::InvertedSpan, span, "span is inverted");
            return false;
        }
        if span.file != parent.file {
            self.push(
                ViolationKind::FileMismatch,
                span,
                "node span refers to a different file than its parent",
            );
            return false;
        }
        if span.lo < parent.lo || span.hi > parent.hi {
            self.push(
                ViolationKind::ChildOutsideParent,
                span,
                "child span is not contained in its parent's span",
            );
        }
        true
    }

    /// Depth guard: returns `false` (and records one violation) when the
    /// subtree must not be descended into.
    fn at_limit(&mut self, span: Span) -> bool {
        if self.depth >= MAX_VALIDATE_DEPTH {
            self.push(
                ViolationKind::DepthExceeded,
                span,
                "syntax tree exceeds the supported validation depth (128)",
            );
            return true;
        }
        false
    }

    fn ident(&mut self, ident: &Ident, parent: Span) {
        self.span(ident.span, parent);
        if ident.name.is_empty() {
            self.push(
                ViolationKind::EmptyIdentifier,
                ident.span,
                "identifier name is empty",
            );
        }
    }

    // ----------------------------------------------------------------
    // Items
    // ----------------------------------------------------------------

    fn item(&mut self, item: &Item, parent: Span) {
        if self.at_limit(item.span) {
            return;
        }
        self.span(item.span, parent);
        self.depth += 1;
        let own = item.span;
        match &item.kind {
            ItemKind::Fn(f) => self.fn_decl(f, own),
            ItemKind::Struct(s) => self.struct_decl(s, own),
            ItemKind::Enum(e) => self.enum_decl(e, own),
            ItemKind::Impl(i) => self.impl_decl(i, own),
            ItemKind::Trait(t) => self.trait_decl(t, own),
            ItemKind::Import(i) => self.import_decl(i, own),
        }
        self.depth -= 1;
    }

    fn vis(&mut self, vis: &crate::ast::Visibility, parent: Span) {
        if let crate::ast::Visibility::Public(span) = vis {
            self.span(*span, parent);
        }
    }

    fn fn_decl(&mut self, decl: &FnDecl, parent: Span) {
        if self.at_limit(decl.span) {
            return;
        }
        self.span(decl.span, parent);
        self.depth += 1;
        let own = decl.span;
        self.vis(&decl.vis, own);
        if let Some(generics) = &decl.generics {
            self.generics(generics, own);
        }
        self.fn_sig(&decl.sig, own);
        self.block(&decl.body, own);
        self.depth -= 1;
    }

    fn fn_sig(&mut self, sig: &FnSig, parent: Span) {
        if self.at_limit(sig.span) {
            return;
        }
        self.span(sig.span, parent);
        self.depth += 1;
        let own = sig.span;
        self.ident(&sig.name, own);
        for param in &sig.params {
            match param {
                Param::Receiver(span) => {
                    self.span(*span, own);
                }
                Param::Named { name, ty, span } => {
                    self.span(*span, own);
                    self.ident(name, own);
                    self.ty(ty, own);
                }
            }
        }
        if let Some(ret) = &sig.ret {
            self.ty(ret, own);
        }
        self.depth -= 1;
    }

    fn generics(&mut self, generics: &Generics, parent: Span) {
        if self.at_limit(generics.span) {
            return;
        }
        self.span(generics.span, parent);
        if generics.params.is_empty() {
            self.push(
                ViolationKind::EmptyGenerics,
                generics.span,
                "generic parameter list is empty",
            );
            return;
        }
        self.depth += 1;
        for param in &generics.params {
            self.span(param.span, generics.span);
            self.ident(&param.name, param.span);
            if let Some(bound) = &param.bound {
                self.path_type(bound, param.span);
            }
        }
        self.depth -= 1;
    }

    fn struct_decl(&mut self, decl: &StructDecl, parent: Span) {
        if self.at_limit(decl.span) {
            return;
        }
        self.span(decl.span, parent);
        self.depth += 1;
        let own = decl.span;
        self.vis(&decl.vis, own);
        self.ident(&decl.name, own);
        if let Some(generics) = &decl.generics {
            self.generics(generics, own);
        }
        for field in &decl.fields {
            self.field(field, own);
        }
        self.depth -= 1;
    }

    fn field(&mut self, field: &Field, parent: Span) {
        if self.at_limit(field.span) {
            return;
        }
        self.span(field.span, parent);
        self.depth += 1;
        self.vis(&field.vis, field.span);
        self.ident(&field.name, field.span);
        self.ty(&field.ty, field.span);
        self.depth -= 1;
    }

    fn enum_decl(&mut self, decl: &EnumDecl, parent: Span) {
        if self.at_limit(decl.span) {
            return;
        }
        self.span(decl.span, parent);
        self.depth += 1;
        let own = decl.span;
        self.vis(&decl.vis, own);
        self.ident(&decl.name, own);
        if let Some(generics) = &decl.generics {
            self.generics(generics, own);
        }
        for variant in &decl.variants {
            self.variant(variant, own);
        }
        self.depth -= 1;
    }

    fn variant(&mut self, variant: &Variant, parent: Span) {
        if self.at_limit(variant.span) {
            return;
        }
        self.span(variant.span, parent);
        self.depth += 1;
        self.ident(&variant.name, variant.span);
        for ty in &variant.payload {
            self.ty(ty, variant.span);
        }
        self.depth -= 1;
    }

    fn impl_decl(&mut self, decl: &ImplDecl, parent: Span) {
        if self.at_limit(decl.span) {
            return;
        }
        self.span(decl.span, parent);
        self.depth += 1;
        let own = decl.span;
        if let Some(generics) = &decl.generics {
            self.generics(generics, own);
        }
        self.path_type(&decl.self_ty, own);
        if let Some(trait_path) = &decl.trait_ {
            self.path_type(trait_path, own);
        }
        for item in &decl.items {
            match item {
                ImplItem::Fn(f) => self.fn_decl(f, own),
            }
        }
        self.depth -= 1;
    }

    fn trait_decl(&mut self, decl: &TraitDecl, parent: Span) {
        if self.at_limit(decl.span) {
            return;
        }
        self.span(decl.span, parent);
        self.depth += 1;
        let own = decl.span;
        self.vis(&decl.vis, own);
        self.ident(&decl.name, own);
        if let Some(generics) = &decl.generics {
            self.generics(generics, own);
        }
        for item in &decl.items {
            self.trait_item(item, own);
        }
        self.depth -= 1;
    }

    fn trait_item(&mut self, item: &TraitItem, parent: Span) {
        if self.at_limit(item.span) {
            return;
        }
        self.span(item.span, parent);
        self.depth += 1;
        self.fn_sig(&item.sig, item.span);
        self.depth -= 1;
    }

    fn import_decl(&mut self, decl: &ImportDecl, parent: Span) {
        if self.at_limit(decl.span) {
            return;
        }
        self.span(decl.span, parent);
        self.depth += 1;
        if decl.path.is_empty() {
            self.push(
                ViolationKind::EmptyPath,
                decl.span,
                "import path has no segments",
            );
        }
        for segment in &decl.path {
            self.ident(segment, decl.span);
        }
        if let Some(alias) = &decl.alias {
            self.ident(alias, decl.span);
        }
        self.depth -= 1;
    }

    // ----------------------------------------------------------------
    // Statements and blocks
    // ----------------------------------------------------------------

    fn block(&mut self, block: &Block, parent: Span) {
        if self.at_limit(block.span) {
            return;
        }
        self.span(block.span, parent);
        self.depth += 1;
        let own = block.span;
        for stmt in &block.stmts {
            self.stmt(stmt, own);
        }
        if let Some(tail) = &block.tail {
            if let Some(last) = block.stmts.last()
                && tail.span.lo < last.span.hi
            {
                self.push(
                    ViolationKind::TailOrdering,
                    tail.span,
                    "tail expression begins before the last statement ends",
                );
            }
            self.expr(tail, own);
        }
        self.depth -= 1;
    }

    fn stmt(&mut self, stmt: &Stmt, parent: Span) {
        if self.at_limit(stmt.span) {
            return;
        }
        self.span(stmt.span, parent);
        self.depth += 1;
        let own = stmt.span;
        match &stmt.kind {
            StmtKind::Let(l) => {
                self.pat(&l.pat, own);
                if let Some(ty) = &l.ty {
                    self.ty(ty, own);
                }
                self.expr(&l.init, own);
            }
            StmtKind::Expr(expr) => self.expr(expr, own),
            StmtKind::Item(item) => self.item(item, own),
        }
        self.depth -= 1;
    }

    // ----------------------------------------------------------------
    // Expressions
    // ----------------------------------------------------------------

    fn expr(&mut self, expr: &Expr, parent: Span) {
        if self.at_limit(expr.span) {
            return;
        }
        self.span(expr.span, parent);
        self.depth += 1;
        let own = expr.span;
        match &expr.kind {
            ExprKind::Lit(lit) => self.lit(lit, own),
            ExprKind::Path(path) => self.path(path, own),
            ExprKind::Unary(_, operand) | ExprKind::Try(operand) => self.expr(operand, own),
            ExprKind::Binary(_, left, right) | ExprKind::Range(left, right) => {
                self.expr(left, own);
                self.expr(right, own);
            }
            ExprKind::Assign(_, target, value) => {
                self.expr(target, own);
                self.expr(value, own);
            }
            ExprKind::Cast(inner, ty) => {
                self.expr(inner, own);
                self.ty(ty, own);
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee, own);
                for arg in args {
                    self.expr(arg, own);
                }
            }
            ExprKind::Field(base, name) => {
                self.expr(base, own);
                self.ident(name, own);
            }
            ExprKind::Index(base, index) => {
                self.expr(base, own);
                self.expr(index, own);
            }
            ExprKind::If(cond, then, else_) => {
                self.expr(cond, own);
                self.block(then, own);
                if let Some(else_expr) = else_ {
                    self.expr(else_expr, own);
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.expr(scrutinee, own);
                for arm in arms {
                    self.arm(arm, own);
                }
            }
            ExprKind::Block(block) | ExprKind::Loop(block) => self.block(block, own),
            ExprKind::While(cond, body) => {
                self.expr(cond, own);
                self.block(body, own);
            }
            ExprKind::For(pat, iterable, body) => {
                self.pat(pat, own);
                self.expr(iterable, own);
                self.block(body, own);
            }
            ExprKind::Return(operand) => {
                if let Some(operand) = operand {
                    self.expr(operand, own);
                }
            }
            ExprKind::Break | ExprKind::Continue => {}
            ExprKind::Tuple(elements) | ExprKind::Array(elements) => {
                for element in elements {
                    self.expr(element, own);
                }
            }
        }
        self.depth -= 1;
    }

    fn arm(&mut self, arm: &Arm, parent: Span) {
        if self.at_limit(arm.span) {
            return;
        }
        self.span(arm.span, parent);
        self.depth += 1;
        self.pat(&arm.pat, arm.span);
        self.expr(&arm.body, arm.span);
        self.depth -= 1;
    }

    // ----------------------------------------------------------------
    // Patterns, types, literals
    // ----------------------------------------------------------------

    fn pat(&mut self, pat: &Pat, parent: Span) {
        if self.at_limit(pat.span) {
            return;
        }
        self.span(pat.span, parent);
        self.depth += 1;
        let own = pat.span;
        match &pat.kind {
            PatKind::Wildcard => {}
            PatKind::Binding(ident) => self.ident(ident, own),
            PatKind::Lit(lit) => self.lit(lit, own),
            PatKind::Path { path, args } => {
                self.path(path, own);
                if let Some(args) = args {
                    for arg in args {
                        self.pat(arg, own);
                    }
                }
            }
            PatKind::Tuple(elements) => {
                for element in elements {
                    self.pat(element, own);
                }
            }
        }
        self.depth -= 1;
    }

    fn ty(&mut self, ty: &Type, parent: Span) {
        if self.at_limit(ty.span) {
            return;
        }
        self.span(ty.span, parent);
        self.depth += 1;
        let own = ty.span;
        match &ty.kind {
            TypeKind::Path(path_type) => self.path_type(path_type, own),
            TypeKind::Tuple(elements) => {
                for element in elements {
                    self.ty(element, own);
                }
            }
            TypeKind::Slice(element) => self.ty(element, own),
            TypeKind::Array(element, length) => {
                self.ty(element, own);
                self.expr(length, own);
            }
            TypeKind::Fn { params, ret } => {
                for param in params {
                    self.ty(param, own);
                }
                if let Some(ret) = ret {
                    self.ty(ret, own);
                }
            }
        }
        self.depth -= 1;
    }

    fn path(&mut self, path: &Path, parent: Span) {
        if self.at_limit(path.span) {
            return;
        }
        self.span(path.span, parent);
        if path.segments.is_empty() {
            self.push(ViolationKind::EmptyPath, path.span, "path has no segments");
            return;
        }
        for segment in &path.segments {
            self.ident(segment, path.span);
        }
    }

    fn path_type(&mut self, path_type: &PathType, parent: Span) {
        if self.at_limit(path_type.span) {
            return;
        }
        self.span(path_type.span, parent);
        if path_type.segments.is_empty() {
            self.push(
                ViolationKind::EmptyPath,
                path_type.span,
                "type path has no segments",
            );
        }
        for segment in &path_type.segments {
            self.ident(segment, path_type.span);
        }
        if let Some(generics) = &path_type.generics {
            if generics.is_empty() {
                self.push(
                    ViolationKind::EmptyGenerics,
                    path_type.span,
                    "generic argument list is empty",
                );
            }
            for argument in generics {
                self.ty(argument, path_type.span);
            }
        }
    }

    fn lit(&mut self, lit: &Lit, parent: Span) {
        self.depth += 1;
        match lit {
            Lit::Str(string) => {
                self.span(string.span, parent);
                let mut previous_end = string.span.lo;
                for part in &string.parts {
                    let part_span = match part {
                        crate::ast::StringPart::Text(text) => text.span,
                        crate::ast::StringPart::Expr(expr) => expr.span,
                    };
                    let ok = self.span(part_span, string.span);
                    if ok {
                        if part_span.lo < previous_end {
                            self.push(
                                ViolationKind::InterpolationOrder,
                                part_span,
                                "string parts are not in source order",
                            );
                        }
                        previous_end = part_span.hi;
                    }
                    if let crate::ast::StringPart::Expr(expr) = part {
                        self.expr(expr, string.span);
                    }
                }
            }
            Lit::Int(_) | Lit::Float(_) | Lit::Bool(_) | Lit::Char(_) => {}
        }
        self.depth -= 1;
    }
}
