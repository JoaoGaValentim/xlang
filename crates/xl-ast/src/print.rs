//! Deterministic canonical printing of XLANG syntax trees.
//!
//! The printer is a syntax-preserving debugging and validation tool for
//! `xc parse`; it is **not** the future Phase 8 source formatter. Its
//! contract (`parser-ast-contract.md` §5, ratified):
//!
//! - Output is deterministic and AST-preserving: parentheses are inserted
//!   exactly where precedence, associativity, and the non-associative tiers
//!   (assignment, range, comparison) require them, so re-parsing recovers
//!   the same tree structure.
//! - Formatting is canonical: four-space indentation, one statement per
//!   line, single spaces around binary and assignment operators, no spaces
//!   inside `()`/`[]`, `, ` between list elements, no trailing commas,
//!   blocks always braced, `else` on the closing brace's line.
//! - `Block::tail` is printed without a semicolon; block-like expression
//!   statements are printed without `;` except in the one position where
//!   that would reclassify them as a tail (last statement with no tail),
//!   where `;` is emitted to preserve the classification.
//! - Literals round-trip: integers print in their stored radix without
//!   separators, finite floats print via shortest-round-trip notation valid
//!   for the XLANG lexer (including `-0.0`), strings re-escape decoded text
//!   so re-lexing yields the same segments, and `inf`/`NaN` are **never**
//!   emitted — a non-finite float is a structured error.
//! - The printer never panics. Malformed trees (empty paths/names/generic
//!   lists, non-place assignment targets, non-block `if` else-branches) and
//!   trees deeper than [`MAX_PRINT_DEPTH`] produce [`PrintError`].

use std::fmt;

use xl_span::Span;

use crate::ast::{
    Arm, AssignOp, BinOp, Block, EnumDecl, Expr, ExprKind, Field, FnDecl, FnSig, Generics, Ident,
    ImplDecl, ImplItem, ImportDecl, Item, ItemKind, Lit, Param, Pat, PatKind, Path, PathType, Stmt,
    StmtKind, StructDecl, TraitDecl, TraitItem, Type, TypeKind, UnOp, Variant, Visibility,
};

/// Maximum node-visit depth supported by the printer and validator.
///
/// The accepted parser bounds its own nesting at 64 (amendment B-4); this
/// limit is deliberately generous for manually constructed trees while
/// still guaranteeing bounded stack use. Exceeding it yields
/// [`PrintErrorKind::DepthLimitExceeded`] (printing) or
/// [`crate::ViolationKind::DepthExceeded`] (validation) instead of
/// unbounded recursion.
pub const MAX_PRINT_DEPTH: u32 = 128;

/// Category of a [`PrintError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintErrorKind {
    /// The tree exceeds [`MAX_PRINT_DEPTH`] node visits on one path.
    DepthLimitExceeded,
    /// The tree cannot be printed as valid XLANG source (empty path,
    /// empty identifier, empty generic list, non-place assignment target,
    /// non-block `if` else-branch).
    InvalidStructure,
    /// A `Float` literal is not finite; `inf`/`NaN` are not XLANG literal
    /// syntax and are never emitted.
    NonFiniteFloat,
}

/// Structured printing failure. Printing never panics and never silently
/// produces invalid source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintError {
    kind: PrintErrorKind,
    span: Span,
    message: String,
}

impl PrintError {
    fn new(kind: PrintErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
        }
    }

    /// Returns the error category.
    #[must_use]
    pub fn kind(&self) -> PrintErrorKind {
        self.kind
    }

    /// Returns the span of the offending node.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns a human-readable description.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PrintError {}

/// Prints one item as canonical XLANG source.
///
/// # Errors
///
/// Returns a [`PrintError`] for malformed trees or depth-limit violations
/// (see the module documentation).
pub fn print_item(item: &Item) -> Result<String, PrintError> {
    let mut printer = Printer::new();
    printer.item(item)?;
    Ok(printer.out)
}

/// Prints a sequence of items (a parsed file's root) joined by one blank
/// line, without a trailing newline.
///
/// # Errors
///
/// Same as [`print_item`].
pub fn print_items(items: &[Item]) -> Result<String, PrintError> {
    let mut printer = Printer::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            printer.out.push_str("\n\n");
        }
        printer.item(item)?;
    }
    Ok(printer.out)
}

/// Prints one expression as canonical XLANG source (root context: never
/// parenthesized).
///
/// # Errors
///
/// Same as [`print_item`].
pub fn print_expr(expr: &Expr) -> Result<String, PrintError> {
    let mut printer = Printer::new();
    printer.expr(expr, 1)?;
    Ok(printer.out)
}

/// Prints one type as canonical XLANG source.
///
/// # Errors
///
/// Same as [`print_item`].
pub fn print_type(ty: &Type) -> Result<String, PrintError> {
    let mut printer = Printer::new();
    printer.ty(ty)?;
    Ok(printer.out)
}

/// Prints one pattern as canonical XLANG source.
///
/// # Errors
///
/// Same as [`print_item`].
pub fn print_pattern(pat: &Pat) -> Result<String, PrintError> {
    let mut printer = Printer::new();
    printer.pat(pat)?;
    Ok(printer.out)
}

/// Prints one block as canonical XLANG source.
///
/// # Errors
///
/// Same as [`print_item`].
pub fn print_block(block: &Block) -> Result<String, PrintError> {
    let mut printer = Printer::new();
    printer.block(block)?;
    Ok(printer.out)
}

// Precedence levels of the accepted table (grammar-rfc §4). Higher binds
// tighter. `Return`/`Break`/`Continue` sit below assignment so that any
// nested use is parenthesized (their operand extends maximally; a bare root
// use is never parenthesized because roots use `min == 1`).
const L_ASSIGN: u8 = 1;
const L_RANGE: u8 = 2;
const L_OR: u8 = 3;
const L_AND: u8 = 4;
const L_CMP: u8 = 5;
const L_ADD: u8 = 6;
const L_MUL: u8 = 7;
const L_UNARY: u8 = 8;
const L_POSTFIX: u8 = 9;
const L_PRIMARY: u8 = 10;

struct Printer {
    out: String,
    indent: usize,
    depth: u32,
}

impl Printer {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
            depth: 0,
        }
    }

    fn enter(&mut self, span: Span) -> Result<(), PrintError> {
        self.depth += 1;
        if self.depth > MAX_PRINT_DEPTH {
            return Err(PrintError::new(
                PrintErrorKind::DepthLimitExceeded,
                span,
                format!("syntax tree exceeds the supported printing depth ({MAX_PRINT_DEPTH})"),
            ));
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }

    fn invalid(span: Span, message: impl Into<String>) -> PrintError {
        PrintError::new(PrintErrorKind::InvalidStructure, span, message)
    }

    fn name<'a>(&self, ident: &'a Ident) -> Result<&'a str, PrintError> {
        if ident.name.is_empty() {
            return Err(Self::invalid(ident.span, "identifier must not be empty"));
        }
        Ok(&ident.name)
    }

    // ----------------------------------------------------------------
    // Items
    // ----------------------------------------------------------------

    fn item(&mut self, item: &Item) -> Result<(), PrintError> {
        self.enter(item.span)?;
        match &item.kind {
            ItemKind::Fn(f) => self.fn_decl(f)?,
            ItemKind::Struct(s) => self.struct_decl(s)?,
            ItemKind::Enum(e) => self.enum_decl(e)?,
            ItemKind::Impl(i) => self.impl_decl(i)?,
            ItemKind::Trait(t) => self.trait_decl(t)?,
            ItemKind::Import(i) => self.import_decl(i)?,
        }
        self.exit();
        Ok(())
    }

    fn vis(&mut self, vis: &Visibility) {
        if matches!(vis, Visibility::Public(_)) {
            self.out.push_str("pub ");
        }
    }

    fn fn_decl(&mut self, f: &FnDecl) -> Result<(), PrintError> {
        self.vis(&f.vis);
        self.fn_header(&f.generics, &f.sig)?;
        self.out.push(' ');
        self.block(&f.body)
    }

    fn fn_header(&mut self, generics: &Option<Generics>, sig: &FnSig) -> Result<(), PrintError> {
        self.out.push_str("fn ");
        self.out.push_str(self.name(&sig.name)?);
        if let Some(generics) = generics {
            self.generics(generics)?;
        }
        self.out.push('(');
        for (index, param) in sig.params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            match param {
                Param::Receiver(_) => self.out.push_str("self"),
                Param::Named { name, ty, .. } => {
                    self.out.push_str(self.name(name)?);
                    self.out.push_str(": ");
                    self.ty(ty)?;
                }
            }
        }
        self.out.push(')');
        if let Some(ret) = &sig.ret {
            self.out.push_str(" -> ");
            self.ty(ret)?;
        }
        Ok(())
    }

    fn generics(&mut self, generics: &Generics) -> Result<(), PrintError> {
        if generics.params.is_empty() {
            return Err(Self::invalid(
                generics.span,
                "generic parameter list must not be empty",
            ));
        }
        self.out.push('<');
        for (index, param) in generics.params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            self.out.push_str(self.name(&param.name)?);
            if let Some(bound) = &param.bound {
                self.out.push_str(": ");
                self.path_type(bound)?;
            }
        }
        self.out.push('>');
        Ok(())
    }

    fn struct_decl(&mut self, s: &StructDecl) -> Result<(), PrintError> {
        self.vis(&s.vis);
        self.out.push_str("struct ");
        self.out.push_str(self.name(&s.name)?);
        if let Some(generics) = &s.generics {
            self.generics(generics)?;
        }
        self.braced_list(&s.fields, |printer, field| printer.field(field))
    }

    fn field(&mut self, field: &Field) -> Result<(), PrintError> {
        self.vis(&field.vis);
        self.out.push_str(self.name(&field.name)?);
        self.out.push_str(": ");
        self.ty(&field.ty)
    }

    fn enum_decl(&mut self, e: &EnumDecl) -> Result<(), PrintError> {
        self.vis(&e.vis);
        self.out.push_str("enum ");
        self.out.push_str(self.name(&e.name)?);
        if let Some(generics) = &e.generics {
            self.generics(generics)?;
        }
        self.braced_list(&e.variants, |printer, variant| printer.variant(variant))
    }

    fn variant(&mut self, variant: &Variant) -> Result<(), PrintError> {
        self.out.push_str(self.name(&variant.name)?);
        if !variant.payload.is_empty() {
            self.out.push('(');
            for (index, ty) in variant.payload.iter().enumerate() {
                if index > 0 {
                    self.out.push_str(", ");
                }
                self.ty(ty)?;
            }
            self.out.push(')');
        }
        Ok(())
    }

    fn impl_decl(&mut self, i: &ImplDecl) -> Result<(), PrintError> {
        self.out.push_str("impl");
        if let Some(generics) = &i.generics {
            self.generics(generics)?;
        }
        self.out.push(' ');
        if let Some(trait_path) = &i.trait_ {
            self.path_type(trait_path)?;
            self.out.push_str(" for ");
        }
        self.path_type(&i.self_ty)?;
        if i.items.is_empty() {
            self.out.push_str(" {}");
            return Ok(());
        }
        self.out.push_str(" {\n");
        self.indent += 1;
        for (index, item) in i.items.iter().enumerate() {
            self.push_indent();
            match item {
                ImplItem::Fn(f) => self.fn_decl(f)?,
            }
            if index + 1 < i.items.len() {
                self.out.push('\n');
            }
        }
        self.indent -= 1;
        self.out.push('\n');
        self.push_indent();
        self.out.push('}');
        Ok(())
    }

    fn trait_decl(&mut self, t: &TraitDecl) -> Result<(), PrintError> {
        self.vis(&t.vis);
        self.out.push_str("trait ");
        self.out.push_str(self.name(&t.name)?);
        if let Some(generics) = &t.generics {
            self.generics(generics)?;
        }
        if t.items.is_empty() {
            self.out.push_str(" {}");
            return Ok(());
        }
        self.out.push_str(" {\n");
        self.indent += 1;
        for (index, item) in t.items.iter().enumerate() {
            self.push_indent();
            self.trait_item(item)?;
            if index + 1 < t.items.len() {
                self.out.push('\n');
            }
        }
        self.indent -= 1;
        self.out.push('\n');
        self.push_indent();
        self.out.push('}');
        Ok(())
    }

    fn trait_item(&mut self, item: &TraitItem) -> Result<(), PrintError> {
        self.enter(item.span)?;
        self.fn_header(&None, &item.sig)?;
        self.out.push(';');
        self.exit();
        Ok(())
    }

    fn import_decl(&mut self, import: &ImportDecl) -> Result<(), PrintError> {
        if import.path.is_empty() {
            return Err(Self::invalid(import.span, "import path must not be empty"));
        }
        self.out.push_str("import ");
        for (index, segment) in import.path.iter().enumerate() {
            if index > 0 {
                self.out.push('.');
            }
            self.out.push_str(self.name(segment)?);
        }
        if let Some(alias) = &import.alias {
            self.out.push_str(" as ");
            self.out.push_str(self.name(alias)?);
        }
        self.out.push(';');
        Ok(())
    }

    /// Prints ` { item, item }` (compact `{}` when empty).
    fn braced_list<T>(
        &mut self,
        items: &[T],
        mut render: impl FnMut(&mut Self, &T) -> Result<(), PrintError>,
    ) -> Result<(), PrintError> {
        if items.is_empty() {
            self.out.push_str(" {}");
            return Ok(());
        }
        self.out.push_str(" {\n");
        self.indent += 1;
        for (index, item) in items.iter().enumerate() {
            self.push_indent();
            render(self, item)?;
            if index + 1 < items.len() {
                self.out.push(',');
            }
            self.out.push('\n');
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push('}');
        Ok(())
    }

    // ----------------------------------------------------------------
    // Statements and blocks
    // ----------------------------------------------------------------

    fn stmt(&mut self, stmt: &Stmt, is_last: bool, tail_is_none: bool) -> Result<(), PrintError> {
        self.enter(stmt.span)?;
        match &stmt.kind {
            StmtKind::Let(l) => {
                self.out.push_str("let ");
                if l.mutable {
                    self.out.push_str("mut ");
                }
                self.pat(&l.pat)?;
                if let Some(ty) = &l.ty {
                    self.out.push_str(": ");
                    self.ty(ty)?;
                }
                self.out.push_str(" = ");
                self.expr(&l.init, 1)?;
                self.out.push(';');
            }
            StmtKind::Expr(expr) => {
                self.expr(expr, 1)?;
                if block_like(expr) {
                    // A semicolon-free block-like statement in final position
                    // with no tail would re-parse as a tail expression;
                    // emit `;` to preserve the classification.
                    if is_last && tail_is_none {
                        self.out.push(';');
                    }
                } else {
                    self.out.push(';');
                }
            }
            StmtKind::Item(item) => self.item(item)?,
        }
        self.exit();
        Ok(())
    }

    fn block(&mut self, block: &Block) -> Result<(), PrintError> {
        self.enter(block.span)?;
        if block.stmts.is_empty() && block.tail.is_none() {
            self.out.push_str("{}");
            self.exit();
            return Ok(());
        }
        self.out.push_str("{\n");
        self.indent += 1;
        let last_index = block.stmts.len().checked_sub(1);
        for (index, stmt) in block.stmts.iter().enumerate() {
            self.push_indent();
            let is_last = Some(index) == last_index;
            self.stmt(stmt, is_last, block.tail.is_none())?;
            self.out.push('\n');
        }
        if let Some(tail) = &block.tail {
            self.push_indent();
            self.expr(tail, 1)?;
            self.out.push('\n');
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push('}');
        self.exit();
        Ok(())
    }

    // ----------------------------------------------------------------
    // Expressions
    // ----------------------------------------------------------------

    fn expr(&mut self, expr: &Expr, min: u8) -> Result<(), PrintError> {
        self.enter(expr.span)?;
        let level = expr_level(expr);
        let parens = level < min;
        if parens {
            self.out.push('(');
        }
        match &expr.kind {
            ExprKind::Lit(lit) => self.lit(lit, expr.span)?,
            ExprKind::Path(path) => self.path(path)?,
            ExprKind::Unary(op, operand) => {
                self.out.push_str(match op {
                    UnOp::Neg => "-",
                    UnOp::Not => "!",
                });
                self.expr(operand, L_UNARY)?;
            }
            ExprKind::Binary(op, left, right) => {
                let (tier, left_assoc) = binary_tier(*op);
                self.expr(left, if left_assoc { tier } else { tier + 1 })?;
                self.out.push(' ');
                self.out.push_str(binary_symbol(*op));
                self.out.push(' ');
                self.expr(right, tier + 1)?;
            }
            ExprKind::Assign(op, target, value) => {
                if !matches!(
                    target.kind,
                    ExprKind::Path(_) | ExprKind::Field(..) | ExprKind::Index(..)
                ) {
                    return Err(Self::invalid(
                        target.span,
                        "assignment target must be a place expression",
                    ));
                }
                self.expr(target, L_RANGE)?;
                self.out.push(' ');
                self.out.push_str(assign_symbol(*op));
                self.out.push(' ');
                self.expr(value, L_RANGE)?;
            }
            ExprKind::Range(start, end) => {
                self.expr(start, L_RANGE + 1)?;
                self.out.push_str("..");
                self.expr(end, L_RANGE + 1)?;
            }
            ExprKind::Cast(inner, ty) => {
                self.expr(inner, L_POSTFIX)?;
                self.out.push_str(" as ");
                self.ty(ty)?;
            }
            ExprKind::Try(inner) => {
                self.expr(inner, L_POSTFIX)?;
                self.out.push('?');
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee, L_POSTFIX)?;
                self.expr_list('(', ')', args)?;
            }
            ExprKind::Field(base, name) => {
                self.expr(base, L_POSTFIX)?;
                self.out.push('.');
                self.out.push_str(self.name(name)?);
            }
            ExprKind::Index(base, index) => {
                self.expr(base, L_POSTFIX)?;
                self.out.push('[');
                self.expr(index, 1)?;
                self.out.push(']');
            }
            ExprKind::If(cond, then, else_) => {
                self.out.push_str("if ");
                self.expr(cond, 1)?;
                self.out.push(' ');
                self.block(then)?;
                if let Some(else_expr) = else_ {
                    self.out.push_str(" else ");
                    match &else_expr.kind {
                        ExprKind::If(..) => self.expr(else_expr, 1)?,
                        ExprKind::Block(block) => self.block(block)?,
                        _ => {
                            return Err(Self::invalid(
                                else_expr.span,
                                "`if` else-branch must be a block or `if` expression",
                            ));
                        }
                    }
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.out.push_str("match ");
                self.expr(scrutinee, 1)?;
                self.match_arms(arms, expr.span)?;
            }
            ExprKind::Block(block) => self.block(block)?,
            ExprKind::While(cond, body) => {
                self.out.push_str("while ");
                self.expr(cond, 1)?;
                self.out.push(' ');
                self.block(body)?;
            }
            ExprKind::For(pat, iterable, body) => {
                self.out.push_str("for ");
                self.pat(pat)?;
                self.out.push_str(" in ");
                self.expr(iterable, 1)?;
                self.out.push(' ');
                self.block(body)?;
            }
            ExprKind::Loop(body) => {
                self.out.push_str("loop ");
                self.block(body)?;
            }
            ExprKind::Return(operand) => {
                self.out.push_str("return");
                if let Some(operand) = operand {
                    self.out.push(' ');
                    self.expr(operand, 1)?;
                }
            }
            ExprKind::Break => self.out.push_str("break"),
            ExprKind::Continue => self.out.push_str("continue"),
            ExprKind::Tuple(elements) => {
                self.out.push('(');
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        self.out.push_str(", ");
                    }
                    self.expr(element, 1)?;
                }
                if elements.len() == 1 {
                    self.out.push(',');
                }
                self.out.push(')');
            }
            ExprKind::Array(elements) => {
                self.expr_list('[', ']', elements)?;
            }
        }
        if parens {
            self.out.push(')');
        }
        self.exit();
        Ok(())
    }

    fn match_arms(&mut self, arms: &[Arm], match_span: Span) -> Result<(), PrintError> {
        if arms.is_empty() {
            self.out.push_str(" {}");
            return Ok(());
        }
        self.out.push_str(" {\n");
        self.indent += 1;
        for (index, arm) in arms.iter().enumerate() {
            self.enter(arm.span).map_err(|_| {
                PrintError::new(
                    PrintErrorKind::DepthLimitExceeded,
                    match_span,
                    format!("syntax tree exceeds the supported printing depth ({MAX_PRINT_DEPTH})"),
                )
            })?;
            self.push_indent();
            self.pat(&arm.pat)?;
            self.out.push_str(" => ");
            self.expr(&arm.body, 1)?;
            self.exit();
            if index + 1 < arms.len() {
                self.out.push(',');
            }
            self.out.push('\n');
        }
        self.indent -= 1;
        self.push_indent();
        self.out.push('}');
        Ok(())
    }

    fn expr_list(&mut self, open: char, close: char, elements: &[Expr]) -> Result<(), PrintError> {
        self.out.push(open);
        for (index, element) in elements.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            self.expr(element, 1)?;
        }
        self.out.push(close);
        Ok(())
    }

    // ----------------------------------------------------------------
    // Patterns and types
    // ----------------------------------------------------------------

    fn pat(&mut self, pat: &Pat) -> Result<(), PrintError> {
        self.enter(pat.span)?;
        match &pat.kind {
            PatKind::Wildcard => self.out.push('_'),
            PatKind::Binding(ident) => self.out.push_str(self.name(ident)?),
            PatKind::Lit(lit) => self.lit(lit, pat.span)?,
            PatKind::Path { path, args } => {
                self.path(path)?;
                if let Some(args) = args {
                    self.out.push('(');
                    for (index, arg) in args.iter().enumerate() {
                        if index > 0 {
                            self.out.push_str(", ");
                        }
                        self.pat(arg)?;
                    }
                    self.out.push(')');
                }
            }
            PatKind::Tuple(elements) => {
                self.out.push('(');
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        self.out.push_str(", ");
                    }
                    self.pat(element)?;
                }
                if elements.len() == 1 {
                    self.out.push(',');
                }
                self.out.push(')');
            }
        }
        self.exit();
        Ok(())
    }

    fn ty(&mut self, ty: &Type) -> Result<(), PrintError> {
        self.enter(ty.span)?;
        match &ty.kind {
            TypeKind::Path(path_type) => self.path_type(path_type)?,
            TypeKind::Tuple(elements) => {
                self.out.push('(');
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        self.out.push_str(", ");
                    }
                    self.ty(element)?;
                }
                if elements.len() == 1 {
                    self.out.push(',');
                }
                self.out.push(')');
            }
            TypeKind::Slice(element) => {
                self.out.push('[');
                self.ty(element)?;
                self.out.push(']');
            }
            TypeKind::Array(element, length) => {
                self.out.push('[');
                self.ty(element)?;
                self.out.push_str("; ");
                self.expr(length, 1)?;
                self.out.push(']');
            }
            TypeKind::Fn { params, ret } => {
                self.out.push_str("fn(");
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        self.out.push_str(", ");
                    }
                    self.ty(param)?;
                }
                self.out.push(')');
                if let Some(ret) = ret {
                    self.out.push_str(" -> ");
                    self.ty(ret)?;
                }
            }
        }
        self.exit();
        Ok(())
    }

    fn path(&mut self, path: &Path) -> Result<(), PrintError> {
        if path.segments.is_empty() {
            return Err(Self::invalid(
                path.span,
                "path must have at least one segment",
            ));
        }
        for (index, segment) in path.segments.iter().enumerate() {
            if index > 0 {
                self.out.push_str("::");
            }
            self.out.push_str(self.name(segment)?);
        }
        Ok(())
    }

    fn path_type(&mut self, path_type: &PathType) -> Result<(), PrintError> {
        if path_type.segments.is_empty() {
            return Err(Self::invalid(
                path_type.span,
                "type path must have at least one segment",
            ));
        }
        for (index, segment) in path_type.segments.iter().enumerate() {
            if index > 0 {
                self.out.push_str("::");
            }
            self.out.push_str(self.name(segment)?);
        }
        if let Some(generics) = &path_type.generics {
            if generics.is_empty() {
                return Err(Self::invalid(
                    path_type.span,
                    "generic argument list must not be empty",
                ));
            }
            self.out.push('<');
            for (index, argument) in generics.iter().enumerate() {
                if index > 0 {
                    self.out.push_str(", ");
                }
                self.ty(argument)?;
            }
            self.out.push('>');
        }
        Ok(())
    }

    // ----------------------------------------------------------------
    // Literals
    // ----------------------------------------------------------------

    fn lit(&mut self, lit: &Lit, span: Span) -> Result<(), PrintError> {
        match lit {
            Lit::Int(int) => {
                let text = match int.radix {
                    crate::ast::Radix::Dec => format!("{}", int.magnitude),
                    crate::ast::Radix::Hex => format!("0x{:x}", int.magnitude),
                    crate::ast::Radix::Bin => format!("0b{:b}", int.magnitude),
                };
                self.out.push_str(&text);
            }
            Lit::Float(value) => {
                if !value.is_finite() {
                    return Err(PrintError::new(
                        PrintErrorKind::NonFiniteFloat,
                        span,
                        "non-finite floats have no XLANG literal syntax",
                    ));
                }
                // Shortest round-trip decimal notation; always a valid
                // XLANG float (`1.0`, `0.1`, `1e300`, `1e-10`, `-0.0`).
                self.out.push_str(&format!("{value:?}"));
            }
            Lit::Bool(value) => self.out.push_str(if *value { "true" } else { "false" }),
            Lit::Char(value) => {
                self.out.push('\'');
                self.out.push_str(&escape_char(*value));
                self.out.push('\'');
            }
            Lit::Str(string) => {
                self.out.push('"');
                for part in &string.parts {
                    match part {
                        crate::ast::StringPart::Text(text) => {
                            self.out.push_str(&escape_text(&text.text));
                        }
                        crate::ast::StringPart::Expr(expr) => {
                            self.out.push('{');
                            self.expr(expr, 1)?;
                            self.out.push('}');
                        }
                    }
                }
                self.out.push('"');
            }
        }
        Ok(())
    }
}

/// Returns `true` for expressions whose statement form does not require a
/// semicolon (`grammar-rfc.md` §3.2).
fn block_like(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::If(..)
            | ExprKind::Match(..)
            | ExprKind::While(..)
            | ExprKind::For(..)
            | ExprKind::Loop(..)
            | ExprKind::Block(..)
    )
}

/// Precedence level of an expression per the accepted Pratt table.
fn expr_level(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Assign(..) | ExprKind::Return(_) | ExprKind::Break | ExprKind::Continue => {
            L_ASSIGN
        }
        ExprKind::Range(..) => L_RANGE,
        ExprKind::Binary(op, ..) => binary_tier(*op).0,
        ExprKind::Unary(..) => L_UNARY,
        ExprKind::Call(..)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::Try(_)
        | ExprKind::Cast(..) => L_POSTFIX,
        _ => L_PRIMARY,
    }
}

/// Returns `(tier, left_associative)` for a binary operator.
fn binary_tier(op: BinOp) -> (u8, bool) {
    match op {
        BinOp::Or => (L_OR, true),
        BinOp::And => (L_AND, true),
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => (L_CMP, false),
        BinOp::Add | BinOp::Sub => (L_ADD, true),
        BinOp::Mul | BinOp::Div | BinOp::Rem => (L_MUL, true),
    }
}

fn binary_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn assign_symbol(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Assign => "=",
        AssignOp::AddAssign => "+=",
        AssignOp::SubAssign => "-=",
        AssignOp::MulAssign => "*=",
        AssignOp::DivAssign => "/=",
        AssignOp::RemAssign => "%=",
    }
}

/// Escapes one character for a character literal.
fn escape_char(c: char) -> String {
    match c {
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\0' => "\\0".to_string(),
        c if c.is_control() => format!("\\u{{{:x}}}", c as u32),
        c => c.to_string(),
    }
}

/// Escapes decoded text for a string-literal text segment. `{` and `}` are
/// always escaped so a literal brace can never become an interpolation when
/// re-lexed.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_tiers_match_the_accepted_table() {
        const {
            assert!(L_ASSIGN < L_RANGE && L_RANGE < L_OR && L_OR < L_AND);
            assert!(L_AND < L_CMP && L_CMP < L_ADD && L_ADD < L_MUL);
            assert!(L_MUL < L_UNARY && L_UNARY < L_POSTFIX && L_POSTFIX < L_PRIMARY);
        }
        assert_eq!(binary_tier(BinOp::Or), (L_OR, true));
        assert_eq!(binary_tier(BinOp::And), (L_AND, true));
        assert_eq!(binary_tier(BinOp::Lt), (L_CMP, false));
        assert_eq!(binary_tier(BinOp::Add), (L_ADD, true));
        assert_eq!(binary_tier(BinOp::Rem), (L_MUL, true));
    }

    #[test]
    fn escape_text_protects_braces_and_quotes() {
        assert_eq!(escape_text("a{b}c"), "a\\{b\\}c");
        assert_eq!(escape_text("\"x\""), "\\\"x\\\"");
        assert_eq!(escape_text("line\n"), "line\\n");
        assert_eq!(escape_text("plain é 🦀"), "plain é 🦀");
    }

    #[test]
    fn escape_char_covers_controls() {
        assert_eq!(escape_char('\n'), "\\n");
        assert_eq!(escape_char('\''), "\\'");
        assert_eq!(escape_char('a'), "a");
        assert_eq!(escape_char('\u{7}'), "\\u{7}");
    }
}
