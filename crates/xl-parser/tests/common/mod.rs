//! Shared helpers for the `xl-parser` test suites.
#![allow(dead_code)]

/// The complete original example from `docs/spec-v0.1-original.txt` §3.
pub const SPEC_EXAMPLE: &str = r#"import std.io;

struct Point { x: f64, y: f64 }

impl Point {
    pub fn dist(self, o: Point) -> f64 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape { Circle(f64), Rect(f64, f64) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rect(w, h) => w * h,
    }
}

fn parse(s: str) -> Result<i64, str> {
    let n = s.to_int()?;
    if n < 0 { Err("negativo") } else { Ok(n) }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 { total += i as f64; }
    io.println("total = {total}");
}
"#;

use xl_ast::{
    Arm, AssignOp, BinOp, Block, EnumDecl, Expr, ExprKind, Field, FnDecl, FnSig, Generics, Ident,
    ImplDecl, ImplItem, ImportDecl, Item, ItemKind, LetStmt, Lit, Param, Pat, PatKind, Path,
    PathType, Stmt, StmtKind, StringLit, StringPart, StructDecl, TraitDecl, TraitItem, Type,
    TypeKind, Variant, Visibility,
};
use xl_diag::Diagnostic;
use xl_parser::{ParseOutput, parse};
use xl_span::{FileId, SourceMap};

/// A parsed source file with its map and identifier.
pub struct Parsed {
    /// The owning source map.
    pub sources: SourceMap,
    /// The file identifier used for the parse.
    pub file: FileId,
    /// The parser output.
    pub output: ParseOutput,
}

impl Parsed {
    /// All diagnostic messages in order.
    #[must_use]
    pub fn messages(&self) -> Vec<String> {
        self.output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message().to_string())
            .collect()
    }

    /// Rendered diagnostics (for failure output).
    #[must_use]
    pub fn rendered(&self) -> String {
        self.output
            .diagnostics
            .iter()
            .map(|diagnostic| xl_diag::render(diagnostic, &self.sources))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Asserts the parse produced no diagnostics at all.
    pub fn assert_clean(&self, source: &str) {
        assert!(
            self.output.diagnostics.is_empty(),
            "expected a clean parse for:\n{source}\ndiagnostics:\n{}",
            self.rendered()
        );
    }

    /// Returns the printed canonical source when printing succeeds.
    pub fn printed(&self) -> String {
        xl_ast::print_items(&self.output.items).expect("canonical printing must succeed")
    }
}

/// Parses `source` as a single file named `test.x`.
#[must_use]
pub fn parse_source(source: &str) -> Parsed {
    let mut sources = SourceMap::new();
    let file = sources
        .add_file("test.x", source)
        .expect("source registration must succeed");
    let output = parse(sources.file(file).expect("registered"), file);
    Parsed {
        sources,
        file,
        output,
    }
}

/// Returns the diagnostic messages for `source`.
#[must_use]
pub fn messages(source: &str) -> Vec<String> {
    parse_source(source).messages()
}

/// Asserts a clean parse and returns it.
#[must_use]
pub fn clean(source: &str) -> Parsed {
    let parsed = parse_source(source);
    parsed.assert_clean(source);
    parsed
}

/// Extracts the top-level function with `name`, if present.
#[must_use]
pub fn find_fn<'a>(items: &'a [Item], name: &str) -> Option<&'a FnDecl> {
    items.iter().find_map(|item| match &item.kind {
        ItemKind::Fn(decl) if decl.sig.name.name == name => Some(decl),
        _ => None,
    })
}

/// Extracts a binary operator kind when `expr` is a `Binary`.
#[must_use]
pub fn binary_op(expr: &Expr) -> Option<BinOp> {
    match expr.kind {
        ExprKind::Binary(op, _, _) => Some(op),
        _ => None,
    }
}

/// Extracts an assignment operator kind when `expr` is an `Assign`.
#[must_use]
pub fn assign_op(expr: &Expr) -> Option<AssignOp> {
    match expr.kind {
        ExprKind::Assign(op, _, _) => Some(op),
        _ => None,
    }
}

/// Parses the body of the function `name` and returns its block.
#[must_use]
pub fn fn_body<'a>(items: &'a [Item], name: &str) -> &'a Block {
    &find_fn(items, name).expect("function must exist").body
}

// ----------------------------------------------------------------------
// Structural equivalence (parser contract §5 item 6)
// ----------------------------------------------------------------------

type Result = std::result::Result<(), String>;

fn fail(message: impl Into<String>) -> Result {
    Err(message.into())
}

/// Checks structural equivalence modulo spans and parser-assigned node IDs;
/// integer radix is not required to round-trip (`0xFF ≡ 255`), float values
/// compare by bit pattern, and string parts compare by decoded text and
/// structure.
pub fn items_equivalent(a: &[Item], b: &[Item]) -> Result {
    if a.len() != b.len() {
        return fail(format!("item count mismatch: {} != {}", a.len(), b.len()));
    }
    for (index, (left, right)) in a.iter().zip(b).enumerate() {
        eq_item(left, right).map_err(|error| format!("item[{index}]: {error}"))?;
    }
    Ok(())
}

fn eq_item(a: &Item, b: &Item) -> Result {
    match (&a.kind, &b.kind) {
        (ItemKind::Fn(x), ItemKind::Fn(y)) => eq_fn(x, y),
        (ItemKind::Struct(x), ItemKind::Struct(y)) => eq_struct(x, y),
        (ItemKind::Enum(x), ItemKind::Enum(y)) => eq_enum(x, y),
        (ItemKind::Impl(x), ItemKind::Impl(y)) => eq_impl(x, y),
        (ItemKind::Trait(x), ItemKind::Trait(y)) => eq_trait(x, y),
        (ItemKind::Import(x), ItemKind::Import(y)) => eq_import(x, y),
        _ => fail("item kind mismatch"),
    }
}

fn eq_vis(a: &Visibility, b: &Visibility) -> Result {
    match (a, b) {
        (Visibility::Private, Visibility::Private) => Ok(()),
        (Visibility::Public(_), Visibility::Public(_)) => Ok(()),
        _ => fail("visibility mismatch"),
    }
}

fn eq_ident(a: &Ident, b: &Ident) -> Result {
    if a.name == b.name {
        Ok(())
    } else {
        fail(format!("identifier mismatch: {:?} != {:?}", a.name, b.name))
    }
}

fn eq_generics(a: &Option<Generics>, b: &Option<Generics>) -> Result {
    match (a, b) {
        (None, None) => Ok(()),
        (Some(x), Some(y)) => {
            if x.params.len() != y.params.len() {
                return fail("generic parameter count mismatch");
            }
            for (left, right) in x.params.iter().zip(&y.params) {
                eq_ident(&left.name, &right.name)?;
                match (&left.bound, &right.bound) {
                    (None, None) => {}
                    (Some(p), Some(q)) => eq_path_type(p, q)?,
                    _ => return fail("generic bound presence mismatch"),
                }
            }
            Ok(())
        }
        _ => fail("generics presence mismatch"),
    }
}

fn eq_path(a: &Path, b: &Path) -> Result {
    if a.segments.len() != b.segments.len() {
        return fail("path segment count mismatch");
    }
    for (left, right) in a.segments.iter().zip(&b.segments) {
        eq_ident(left, right)?;
    }
    Ok(())
}

fn eq_path_type(a: &PathType, b: &PathType) -> Result {
    if a.segments.len() != b.segments.len() {
        return fail("type path segment count mismatch");
    }
    for (left, right) in a.segments.iter().zip(&b.segments) {
        eq_ident(left, right)?;
    }
    match (&a.generics, &b.generics) {
        (None, None) => Ok(()),
        (Some(x), Some(y)) => {
            if x.len() != y.len() {
                return fail("generic argument count mismatch");
            }
            for (left, right) in x.iter().zip(y) {
                eq_type(left, right)?;
            }
            Ok(())
        }
        _ => fail("generic argument presence mismatch"),
    }
}

fn eq_type(a: &Type, b: &Type) -> Result {
    match (&a.kind, &b.kind) {
        (TypeKind::Path(x), TypeKind::Path(y)) => eq_path_type(x, y),
        (TypeKind::Tuple(x), TypeKind::Tuple(y)) => {
            if x.len() != y.len() {
                return fail("tuple type arity mismatch");
            }
            for (left, right) in x.iter().zip(y) {
                eq_type(left, right)?;
            }
            Ok(())
        }
        (TypeKind::Slice(x), TypeKind::Slice(y)) => eq_type(x, y),
        (TypeKind::Array(x, lx), TypeKind::Array(y, ly)) => {
            eq_type(x, y)?;
            eq_expr(lx, ly)
        }
        (
            TypeKind::Fn {
                params: px,
                ret: rx,
            },
            TypeKind::Fn {
                params: py,
                ret: ry,
            },
        ) => {
            if px.len() != py.len() {
                return fail("fn type arity mismatch");
            }
            for (left, right) in px.iter().zip(py) {
                eq_type(left, right)?;
            }
            match (rx, ry) {
                (None, None) => Ok(()),
                (Some(x), Some(y)) => eq_type(x, y),
                _ => fail("fn type return presence mismatch"),
            }
        }
        _ => fail("type kind mismatch"),
    }
}

fn eq_lit(a: &Lit, b: &Lit) -> Result {
    match (a, b) {
        (Lit::Int(x), Lit::Int(y)) => {
            if x.magnitude == y.magnitude {
                Ok(())
            } else {
                fail(format!(
                    "int magnitude mismatch: {} != {}",
                    x.magnitude, y.magnitude
                ))
            }
        }
        (Lit::Float(x), Lit::Float(y)) => {
            if x.to_bits() == y.to_bits() {
                Ok(())
            } else {
                fail(format!("float mismatch: {x} != {y}"))
            }
        }
        (Lit::Bool(x), Lit::Bool(y)) => {
            if x == y {
                Ok(())
            } else {
                fail("bool mismatch")
            }
        }
        (Lit::Char(x), Lit::Char(y)) => {
            if x == y {
                Ok(())
            } else {
                fail("char mismatch")
            }
        }
        (Lit::Str(x), Lit::Str(y)) => eq_string_lit(x, y),
        _ => fail("literal kind mismatch"),
    }
}

fn eq_string_lit(a: &StringLit, b: &StringLit) -> Result {
    if a.parts.len() != b.parts.len() {
        return fail("string part count mismatch");
    }
    for (left, right) in a.parts.iter().zip(&b.parts) {
        match (left, right) {
            (StringPart::Text(x), StringPart::Text(y)) => {
                if x.text != y.text {
                    return fail(format!(
                        "string text mismatch: {:?} != {:?}",
                        x.text, y.text
                    ));
                }
            }
            (StringPart::Expr(x), StringPart::Expr(y)) => eq_expr(x, y)?,
            _ => return fail("string part kind mismatch"),
        }
    }
    Ok(())
}

fn eq_expr(a: &Expr, b: &Expr) -> Result {
    match (&a.kind, &b.kind) {
        (ExprKind::Lit(x), ExprKind::Lit(y)) => eq_lit(x, y),
        (ExprKind::Path(x), ExprKind::Path(y)) => eq_path(x, y),
        (ExprKind::Unary(op1, x), ExprKind::Unary(op2, y)) => {
            if op1 != op2 {
                return fail("unary operator mismatch");
            }
            eq_expr(x, y)
        }
        (ExprKind::Binary(op1, l1, r1), ExprKind::Binary(op2, l2, r2)) => {
            if op1 != op2 {
                return fail("binary operator mismatch");
            }
            eq_expr(l1, l2)?;
            eq_expr(r1, r2)
        }
        (ExprKind::Call(f1, a1), ExprKind::Call(f2, a2)) => {
            eq_expr(f1, f2)?;
            eq_exprs(a1, a2)
        }
        (ExprKind::Field(x1, i1), ExprKind::Field(x2, i2)) => {
            eq_expr(x1, x2)?;
            eq_ident(i1, i2)
        }
        (ExprKind::Index(x1, y1), ExprKind::Index(x2, y2)) => {
            eq_expr(x1, x2)?;
            eq_expr(y1, y2)
        }
        (ExprKind::Assign(op1, l1, r1), ExprKind::Assign(op2, l2, r2)) => {
            if op1 != op2 {
                return fail("assignment operator mismatch");
            }
            eq_expr(l1, l2)?;
            eq_expr(r1, r2)
        }
        (ExprKind::Range(l1, r1), ExprKind::Range(l2, r2)) => {
            eq_expr(l1, l2)?;
            eq_expr(r1, r2)
        }
        (ExprKind::Cast(x1, t1), ExprKind::Cast(x2, t2)) => {
            eq_expr(x1, x2)?;
            eq_type(t1, t2)
        }
        (ExprKind::Try(x), ExprKind::Try(y)) => eq_expr(x, y),
        (ExprKind::If(c1, b1, e1), ExprKind::If(c2, b2, e2)) => {
            eq_expr(c1, c2)?;
            eq_block(b1, b2)?;
            match (e1, e2) {
                (None, None) => Ok(()),
                (Some(x), Some(y)) => eq_expr(x, y),
                _ => fail("else presence mismatch"),
            }
        }
        (ExprKind::Match(s1, a1), ExprKind::Match(s2, a2)) => {
            eq_expr(s1, s2)?;
            if a1.len() != a2.len() {
                return fail("match arm count mismatch");
            }
            for (left, right) in a1.iter().zip(a2) {
                eq_arm(left, right)?;
            }
            Ok(())
        }
        (ExprKind::Block(x), ExprKind::Block(y)) => eq_block(x, y),
        (ExprKind::While(c1, b1), ExprKind::While(c2, b2)) => {
            eq_expr(c1, c2)?;
            eq_block(b1, b2)
        }
        (ExprKind::For(p1, i1, b1), ExprKind::For(p2, i2, b2)) => {
            eq_pat(p1, p2)?;
            eq_expr(i1, i2)?;
            eq_block(b1, b2)
        }
        (ExprKind::Loop(x), ExprKind::Loop(y)) => eq_block(x, y),
        (ExprKind::Return(x), ExprKind::Return(y)) => match (x, y) {
            (None, None) => Ok(()),
            (Some(a), Some(b)) => eq_expr(a, b),
            _ => fail("return operand presence mismatch"),
        },
        (ExprKind::Break, ExprKind::Break) => Ok(()),
        (ExprKind::Continue, ExprKind::Continue) => Ok(()),
        (ExprKind::Tuple(x), ExprKind::Tuple(y)) => eq_exprs(x, y),
        (ExprKind::Array(x), ExprKind::Array(y)) => eq_exprs(x, y),
        _ => fail("expression kind mismatch"),
    }
}

fn eq_exprs(a: &[Expr], b: &[Expr]) -> Result {
    if a.len() != b.len() {
        return fail(format!(
            "expression count mismatch: {} != {}",
            a.len(),
            b.len()
        ));
    }
    for (left, right) in a.iter().zip(b) {
        eq_expr(left, right)?;
    }
    Ok(())
}

fn eq_arm(a: &Arm, b: &Arm) -> Result {
    eq_pat(&a.pat, &b.pat)?;
    eq_expr(&a.body, &b.body)
}

fn eq_pat(a: &Pat, b: &Pat) -> Result {
    match (&a.kind, &b.kind) {
        (PatKind::Wildcard, PatKind::Wildcard) => Ok(()),
        (PatKind::Binding(x), PatKind::Binding(y)) => eq_ident(x, y),
        (PatKind::Lit(x), PatKind::Lit(y)) => eq_lit(x, y),
        (PatKind::Path { path: p1, args: a1 }, PatKind::Path { path: p2, args: a2 }) => {
            eq_path(p1, p2)?;
            match (a1, a2) {
                (None, None) => Ok(()),
                (Some(x), Some(y)) => {
                    if x.len() != y.len() {
                        return fail("pattern argument count mismatch");
                    }
                    for (left, right) in x.iter().zip(y) {
                        eq_pat(left, right)?;
                    }
                    Ok(())
                }
                _ => fail("pattern argument presence mismatch"),
            }
        }
        (PatKind::Tuple(x), PatKind::Tuple(y)) => {
            if x.len() != y.len() {
                return fail("tuple pattern arity mismatch");
            }
            for (left, right) in x.iter().zip(y) {
                eq_pat(left, right)?;
            }
            Ok(())
        }
        _ => fail("pattern kind mismatch"),
    }
}

fn eq_block(a: &Block, b: &Block) -> Result {
    if a.stmts.len() != b.stmts.len() {
        return fail("statement count mismatch");
    }
    for (left, right) in a.stmts.iter().zip(&b.stmts) {
        eq_stmt(left, right)?;
    }
    match (&a.tail, &b.tail) {
        (None, None) => Ok(()),
        (Some(x), Some(y)) => eq_expr(x, y),
        _ => fail("block tail presence mismatch"),
    }
}

fn eq_stmt(a: &Stmt, b: &Stmt) -> Result {
    match (&a.kind, &b.kind) {
        (StmtKind::Let(x), StmtKind::Let(y)) => eq_let(x, y),
        (StmtKind::Expr(x), StmtKind::Expr(y)) => eq_expr(x, y),
        (StmtKind::Item(x), StmtKind::Item(y)) => eq_item(x, y),
        _ => fail("statement kind mismatch"),
    }
}

fn eq_let(a: &LetStmt, b: &LetStmt) -> Result {
    if a.mutable != b.mutable {
        return fail("mutability mismatch");
    }
    eq_pat(&a.pat, &b.pat)?;
    match (&a.ty, &b.ty) {
        (None, None) => {}
        (Some(x), Some(y)) => eq_type(x, y)?,
        _ => return fail("let type annotation presence mismatch"),
    }
    eq_expr(&a.init, &b.init)
}

fn eq_fn(a: &FnDecl, b: &FnDecl) -> Result {
    eq_vis(&a.vis, &b.vis)?;
    eq_generics(&a.generics, &b.generics)?;
    eq_fn_sig(&a.sig, &b.sig)?;
    eq_block(&a.body, &b.body)
}

fn eq_fn_sig(a: &FnSig, b: &FnSig) -> Result {
    eq_ident(&a.name, &b.name)?;
    if a.params.len() != b.params.len() {
        return fail("parameter count mismatch");
    }
    for (left, right) in a.params.iter().zip(&b.params) {
        match (left, right) {
            (Param::Receiver(_), Param::Receiver(_)) => {}
            (
                Param::Named {
                    name: n1, ty: t1, ..
                },
                Param::Named {
                    name: n2, ty: t2, ..
                },
            ) => {
                eq_ident(n1, n2)?;
                eq_type(t1, t2)?;
            }
            _ => return fail("parameter kind mismatch"),
        }
    }
    match (&a.ret, &b.ret) {
        (None, None) => Ok(()),
        (Some(x), Some(y)) => eq_type(x, y),
        _ => fail("return type presence mismatch"),
    }
}

fn eq_struct(a: &StructDecl, b: &StructDecl) -> Result {
    eq_vis(&a.vis, &b.vis)?;
    eq_ident(&a.name, &b.name)?;
    eq_generics(&a.generics, &b.generics)?;
    if a.fields.len() != b.fields.len() {
        return fail("field count mismatch");
    }
    for (left, right) in a.fields.iter().zip(&b.fields) {
        eq_field(left, right)?;
    }
    Ok(())
}

fn eq_field(a: &Field, b: &Field) -> Result {
    eq_vis(&a.vis, &b.vis)?;
    eq_ident(&a.name, &b.name)?;
    eq_type(&a.ty, &b.ty)
}

fn eq_enum(a: &EnumDecl, b: &EnumDecl) -> Result {
    eq_vis(&a.vis, &b.vis)?;
    eq_ident(&a.name, &b.name)?;
    eq_generics(&a.generics, &b.generics)?;
    if a.variants.len() != b.variants.len() {
        return fail("variant count mismatch");
    }
    for (left, right) in a.variants.iter().zip(&b.variants) {
        eq_variant(left, right)?;
    }
    Ok(())
}

fn eq_variant(a: &Variant, b: &Variant) -> Result {
    eq_ident(&a.name, &b.name)?;
    if a.payload.len() != b.payload.len() {
        return fail("variant payload arity mismatch");
    }
    for (left, right) in a.payload.iter().zip(&b.payload) {
        eq_type(left, right)?;
    }
    Ok(())
}

fn eq_impl(a: &ImplDecl, b: &ImplDecl) -> Result {
    eq_generics(&a.generics, &b.generics)?;
    match (&a.trait_, &b.trait_) {
        (None, None) => {}
        (Some(x), Some(y)) => eq_path_type(x, y)?,
        _ => return fail("impl trait presence mismatch"),
    }
    eq_path_type(&a.self_ty, &b.self_ty)?;
    if a.items.len() != b.items.len() {
        return fail("impl item count mismatch");
    }
    for (left, right) in a.items.iter().zip(&b.items) {
        match (left, right) {
            (ImplItem::Fn(x), ImplItem::Fn(y)) => eq_fn(x, y)?,
        }
    }
    Ok(())
}

fn eq_trait(a: &TraitDecl, b: &TraitDecl) -> Result {
    eq_vis(&a.vis, &b.vis)?;
    eq_ident(&a.name, &b.name)?;
    eq_generics(&a.generics, &b.generics)?;
    if a.items.len() != b.items.len() {
        return fail("trait item count mismatch");
    }
    for (TraitItem { sig: x, .. }, TraitItem { sig: y, .. }) in a.items.iter().zip(&b.items) {
        eq_fn_sig(x, y)?;
    }
    Ok(())
}

fn eq_import(a: &ImportDecl, b: &ImportDecl) -> Result {
    if a.path.len() != b.path.len() {
        return fail("import path length mismatch");
    }
    for (left, right) in a.path.iter().zip(&b.path) {
        eq_ident(left, right)?;
    }
    match (&a.alias, &b.alias) {
        (None, None) => Ok(()),
        (Some(x), Some(y)) => eq_ident(x, y),
        _ => fail("import alias presence mismatch"),
    }
}

/// Diagnostic helper: message text list as a slice of `&str`.
#[must_use]
pub fn message_refs(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message())
        .collect()
}
