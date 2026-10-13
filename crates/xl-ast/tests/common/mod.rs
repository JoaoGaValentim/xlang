//! Shared fixture builders for the `xl-ast` test suites.
//!
//! Two styles are available:
//!
//! - **Direct builders** (`lit_int`, `var`, `bin`, …) use fixed dummy spans
//!   derived from their children; ideal for printing and determinism tests
//!   (validation is not involved).
//! - **Cursor builders** ([`Spans`] plus `*_ex` functions) allocate leaves
//!   sequentially in construction order and derive every parent span with
//!   [`cover`], so fixtures satisfy span containment and source ordering
//!   (needed for validation and block-tail tests).
#![allow(dead_code)]

use xl_ast::{
    Arm, AssignOp, BinOp, Block, EnumDecl, Expr, ExprId, ExprKind, Field, FnDecl, FnSig, Ident,
    ImplDecl, ImplItem, ImportDecl, IntLit, Item, ItemId, ItemKind, LetStmt, Lit, Param, Pat,
    PatKind, Path, PathType, Radix, Stmt, StmtKind, StringLit, StringPart, StructDecl, TextSegment,
    TraitDecl, Type, TypeKind, UnOp, Variant, Visibility,
};
use xl_span::{FileId, Span};

/// The fixture file identifier used by every builder.
pub const FILE: FileId = FileId::from_index(0);

/// Builds a span on the fixture file.
pub fn fspan(lo: u32, hi: u32) -> Span {
    Span { file: FILE, lo, hi }
}

/// Smallest span covering both inputs.
pub fn cover(a: Span, b: Span) -> Span {
    fspan(a.lo.min(b.lo), a.hi.max(b.hi))
}

/// Smallest span covering all inputs; `(0, 0)` for an empty slice.
pub fn cover_all(spans: &[Span]) -> Span {
    match spans {
        [] => fspan(0, 0),
        [first, rest @ ..] => rest.iter().copied().fold(*first, cover),
    }
}

/// Reads an expression's span.
pub fn span_of(expr: &Expr) -> Span {
    expr.span
}

/// Expression with an explicit span.
pub fn ex(span: Span, kind: ExprKind) -> Expr {
    Expr {
        id: ExprId::from_index(span.lo),
        kind,
        span,
    }
}

/// Wraps a span list into an [`Expr`] with a covering span.
pub fn ex_cover(parts: &[Span], kind: ExprKind) -> Expr {
    ex(cover_all(parts), kind)
}

// ------------------------------------------------------------------
// Direct builders (fixed dummy spans; printing tests)
// ------------------------------------------------------------------

/// Integer literal expression.
pub fn lit_int(magnitude: u64) -> Expr {
    leaf_expr(
        0,
        ExprKind::Lit(Lit::Int(IntLit {
            magnitude,
            radix: Radix::Dec,
        })),
    )
}

/// Float literal expression.
pub fn lit_float(value: f64) -> Expr {
    leaf_expr(0, ExprKind::Lit(Lit::Float(value)))
}

/// Boolean literal expression.
pub fn lit_bool(value: bool) -> Expr {
    leaf_expr(0, ExprKind::Lit(Lit::Bool(value)))
}

/// A leaf expression with a one-byte span at `offset`.
pub fn leaf_expr(offset: u32, kind: ExprKind) -> Expr {
    ex(fspan(offset, offset + 1), kind)
}

/// Path expression from segment names.
pub fn var(name: &str) -> Expr {
    let id = ident(name, 0);
    let span = id.span;
    ex(
        span,
        ExprKind::Path(Path {
            segments: vec![id],
            span,
        }),
    )
}

/// Multi-segment path expression.
pub fn path_expr(names: &[&str]) -> Expr {
    let segments: Vec<Ident> = names
        .iter()
        .enumerate()
        .map(|(i, n)| ident(n, i as u32))
        .collect();
    let span = cover_all(&segments.iter().map(|i| i.span).collect::<Vec<_>>());
    ex(span, ExprKind::Path(Path { segments, span }))
}

/// Ident with a one-byte span at `offset`.
pub fn ident(name: &str, offset: u32) -> Ident {
    Ident {
        name: name.to_string(),
        span: fspan(offset, offset + 1),
    }
}

/// Unary expression.
pub fn un(op: UnOp, operand: Expr) -> Expr {
    let span = operand.span;
    ex(span, ExprKind::Unary(op, Box::new(operand)))
}

/// Binary expression.
pub fn bin(op: BinOp, left: Expr, right: Expr) -> Expr {
    let span = cover(left.span, right.span);
    ex(span, ExprKind::Binary(op, Box::new(left), Box::new(right)))
}

/// Assignment expression.
pub fn assign(op: AssignOp, target: Expr, value: Expr) -> Expr {
    let span = cover(target.span, value.span);
    ex(
        span,
        ExprKind::Assign(op, Box::new(target), Box::new(value)),
    )
}

/// Range expression.
pub fn range(start: Expr, end: Expr) -> Expr {
    let span = cover(start.span, end.span);
    ex(span, ExprKind::Range(Box::new(start), Box::new(end)))
}

/// Cast expression.
pub fn cast(expr: Expr, ty: Type) -> Expr {
    let span = cover(expr.span, ty.span);
    ex(span, ExprKind::Cast(Box::new(expr), ty))
}

/// `?` expression.
pub fn try_expr(inner: Expr) -> Expr {
    let span = inner.span;
    ex(span, ExprKind::Try(Box::new(inner)))
}

/// Call expression.
pub fn call(callee: Expr, args: Vec<Expr>) -> Expr {
    let mut spans: Vec<Span> = args.iter().map(span_of).collect();
    spans.push(callee.span);
    ex(cover_all(&spans), ExprKind::Call(Box::new(callee), args))
}

/// Field access.
pub fn field(base: Expr, name: &str) -> Expr {
    let name = ident(name, base.span.hi);
    let span = cover(base.span, name.span);
    ex(span, ExprKind::Field(Box::new(base), name))
}

/// Index expression.
pub fn index(base: Expr, subscript: Expr) -> Expr {
    let span = cover(base.span, subscript.span);
    ex(span, ExprKind::Index(Box::new(base), Box::new(subscript)))
}

/// Tuple expression.
pub fn tuple(elements: Vec<Expr>) -> Expr {
    let spans: Vec<Span> = elements.iter().map(span_of).collect();
    ex(cover_all(&spans), ExprKind::Tuple(elements))
}

/// Array expression.
pub fn array(elements: Vec<Expr>) -> Expr {
    let spans: Vec<Span> = elements.iter().map(span_of).collect();
    ex(cover_all(&spans), ExprKind::Array(elements))
}

/// `return` expression.
pub fn ret(operand: Option<Expr>) -> Expr {
    let span = operand.as_ref().map_or(fspan(0, 1), span_of);
    ex(span, ExprKind::Return(operand.map(Box::new)))
}

/// `break` expression.
pub fn break_expr() -> Expr {
    leaf_expr(0, ExprKind::Break)
}

/// Empty block.
pub fn empty_block() -> Block {
    Block {
        stmts: Vec::new(),
        tail: None,
        span: fspan(0, 2),
    }
}

/// Block with statements and (optionally) a tail, using derived spans.
pub fn block_with(stmts: Vec<Stmt>, tail: Option<Expr>) -> Block {
    let mut spans: Vec<Span> = stmts.iter().map(|s| s.span).collect();
    if let Some(tail) = &tail {
        spans.push(tail.span);
    }
    Block {
        stmts,
        tail: tail.map(Box::new),
        span: cover_all(&spans),
    }
}

/// Expression statement.
pub fn expr_stmt(expr: Expr) -> Stmt {
    let span = expr.span;
    Stmt {
        kind: StmtKind::Expr(expr),
        span,
    }
}

/// Binding pattern with a dummy span.
pub fn binding_pat(name: &str) -> Pat {
    let id = ident(name, 0);
    let span = id.span;
    Pat {
        kind: PatKind::Binding(id),
        span,
    }
}

/// Simple path type.
pub fn ty_path(name: &str) -> Type {
    let id = ident(name, 0);
    let span = id.span;
    Type {
        kind: TypeKind::Path(PathType {
            segments: vec![id],
            generics: None,
            span,
        }),
        span,
    }
}

// ------------------------------------------------------------------
// Cursor builders (construction-order spans)
// ------------------------------------------------------------------

/// Sequential span allocator.
pub struct Spans {
    next: u32,
}

impl Spans {
    /// Creates an allocator starting at offset 0.
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Allocates a leaf span of `width` bytes.
    pub fn leaf(&mut self, width: u32) -> Span {
        let lo = self.next;
        self.next += width;
        fspan(lo, self.next)
    }

    /// Allocates an identifier leaf.
    pub fn ident(&mut self, name: &str) -> Ident {
        let span = self.leaf(name.len() as u32 + 1);
        Ident {
            name: name.to_string(),
            span,
        }
    }

    /// Path expression; segments are allocated in order.
    pub fn path_expr(&mut self, names: &[&str]) -> Expr {
        let segments: Vec<Ident> = names.iter().map(|n| self.ident(n)).collect();
        let span = cover_all(&segments.iter().map(|i| i.span).collect::<Vec<_>>());
        ex(span, ExprKind::Path(Path { segments, span }))
    }

    /// Integer literal expression.
    pub fn int_expr(&mut self, magnitude: u64, radix: Radix) -> Expr {
        let span = self.leaf(2);
        ex(span, ExprKind::Lit(Lit::Int(IntLit { magnitude, radix })))
    }

    /// Float literal expression.
    pub fn float_expr(&mut self, value: f64) -> Expr {
        let span = self.leaf(3);
        ex(span, ExprKind::Lit(Lit::Float(value)))
    }

    /// Text-only string expression.
    pub fn string_expr(&mut self, text: &str) -> Expr {
        let text_span = self.leaf(text.len() as u32 + 2);
        let lit_span = text_span;
        ex(
            lit_span,
            ExprKind::Lit(Lit::Str(StringLit {
                parts: vec![StringPart::Text(TextSegment {
                    text: text.to_string(),
                    span: text_span,
                })],
                span: lit_span,
            })),
        )
    }

    /// String expression with a text prefix and one interpolation.
    pub fn string_interp(&mut self, text: &str, inner: Expr) -> Expr {
        let text_span = self.leaf(text.len() as u32 + 1);
        let lit_span = cover(text_span, inner.span);
        ex(
            lit_span,
            ExprKind::Lit(Lit::Str(StringLit {
                parts: vec![
                    StringPart::Text(TextSegment {
                        text: text.to_string(),
                        span: text_span,
                    }),
                    StringPart::Expr(inner),
                ],
                span: lit_span,
            })),
        )
    }

    /// Binding pattern.
    pub fn binding(&mut self, name: &str) -> Pat {
        let id = self.ident(name);
        let span = id.span;
        Pat {
            kind: PatKind::Binding(id),
            span,
        }
    }

    /// Path pattern with optional payload patterns.
    pub fn path_pat(&mut self, names: &[&str], args: Vec<Pat>) -> Pat {
        let segments: Vec<Ident> = names.iter().map(|n| self.ident(n)).collect();
        let mut spans: Vec<Span> = segments.iter().map(|i| i.span).collect();
        spans.extend(args.iter().map(|p| p.span));
        let span = cover_all(&spans);
        let path_span = cover_all(&segments.iter().map(|i| i.span).collect::<Vec<_>>());
        Pat {
            kind: PatKind::Path {
                path: Path {
                    segments,
                    span: path_span,
                },
                args: if args.is_empty() { None } else { Some(args) },
            },
            span,
        }
    }

    /// Path type with optional generic arguments.
    pub fn path_type(&mut self, names: &[&str], generics: Option<Vec<Type>>) -> PathType {
        let segments: Vec<Ident> = names.iter().map(|n| self.ident(n)).collect();
        let mut spans: Vec<Span> = segments.iter().map(|i| i.span).collect();
        if let Some(generics) = &generics {
            spans.extend(generics.iter().map(|t| t.span));
        }
        PathType {
            segments,
            generics,
            span: cover_all(&spans),
        }
    }

    /// Type carrying a covering span.
    pub fn ty(&mut self, kind: TypeKind) -> Type {
        let span = match &kind {
            TypeKind::Path(path) => path.span,
            TypeKind::Tuple(elements) => {
                cover_all(&elements.iter().map(|t| t.span).collect::<Vec<_>>())
            }
            TypeKind::Slice(element) => element.span,
            TypeKind::Array(element, length) => cover(element.span, length.span),
            TypeKind::Fn { params, ret } => {
                let mut spans: Vec<Span> = params.iter().map(|t| t.span).collect();
                if let Some(ret) = ret {
                    spans.push(ret.span);
                }
                cover_all(&spans)
            }
        };
        Type { kind, span }
    }

    /// Statement carrying a covering span (never includes a `;`).
    pub fn stmt(&mut self, kind: StmtKind) -> Stmt {
        let span = match &kind {
            StmtKind::Let(l) => {
                let mut spans = vec![l.pat.span, l.init.span];
                if let Some(ty) = &l.ty {
                    spans.push(ty.span);
                }
                cover_all(&spans)
            }
            StmtKind::Expr(e) => e.span,
            StmtKind::Item(i) => i.span,
        };
        Stmt { kind, span }
    }

    /// Block carrying a covering span of its contents.
    pub fn block(&mut self, stmts: Vec<Stmt>, tail: Option<Expr>) -> Block {
        let mut spans: Vec<Span> = stmts.iter().map(|s| s.span).collect();
        if let Some(tail) = &tail {
            spans.push(tail.span);
        }
        Block {
            stmts,
            tail: tail.map(Box::new),
            span: cover_all(&spans),
        }
    }

    /// Item carrying a covering span of its parts.
    pub fn item(&mut self, kind: ItemKind) -> Item {
        let span = match &kind {
            ItemKind::Fn(f) => cover(f.sig.span, f.body.span),
            ItemKind::Struct(s) => {
                let mut spans = vec![s.name.span];
                spans.extend(s.fields.iter().map(|f| f.span));
                if let Some(generics) = &s.generics {
                    spans.push(generics.span);
                }
                cover_all(&spans)
            }
            ItemKind::Enum(e) => {
                let mut spans = vec![e.name.span];
                spans.extend(e.variants.iter().map(|v| v.span));
                cover_all(&spans)
            }
            ItemKind::Impl(i) => {
                let mut spans = vec![i.self_ty.span];
                if let Some(trait_path) = &i.trait_ {
                    spans.push(trait_path.span);
                }
                spans.extend(i.items.iter().map(|item| match item {
                    ImplItem::Fn(f) => cover(f.sig.span, f.body.span),
                }));
                cover_all(&spans)
            }
            ItemKind::Trait(t) => {
                let mut spans = vec![t.name.span];
                spans.extend(t.items.iter().map(|item| item.span));
                cover_all(&spans)
            }
            ItemKind::Import(i) => {
                let mut spans: Vec<Span> = i.path.iter().map(|p| p.span).collect();
                if let Some(alias) = &i.alias {
                    spans.push(alias.span);
                }
                cover_all(&spans)
            }
        };
        Item {
            id: ItemId::from_index(span.lo),
            kind,
            span,
        }
    }
}

impl Default for Spans {
    fn default() -> Self {
        Self::new()
    }
}

/// Function signature with a covering span.
pub fn sig_ex(name: Ident, params: Vec<Param>, ret: Option<Type>) -> FnSig {
    let mut spans = vec![name.span];
    for param in &params {
        spans.push(match param {
            Param::Receiver(span) => *span,
            Param::Named { span, .. } => *span,
        });
    }
    if let Some(ret) = &ret {
        spans.push(ret.span);
    }
    FnSig {
        name,
        params,
        ret,
        span: cover_all(&spans),
    }
}

/// Named parameter with a covering span.
pub fn named_param(name: Ident, ty: Type) -> Param {
    let span = cover(name.span, ty.span);
    Param::Named { name, ty, span }
}

// ------------------------------------------------------------------
// The specification example program
// ------------------------------------------------------------------

/// Builds the AST for the specification's example program
/// (`docs/spec-v0.1-original.txt` §3) with source-ordered, contained spans.
pub fn example_program() -> Vec<Item> {
    let mut s = Spans::new();
    let mut items = Vec::new();

    // import std.io;
    {
        let path = vec![s.ident("std"), s.ident("io")];
        items.push(s.item(ItemKind::Import(ImportDecl {
            path,
            alias: None,
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Import(import) = &mut last.kind {
            import.span = last.span;
        }
    }

    // struct Point { x: f64, y: f64 }
    {
        let name = s.ident("Point");
        let mut fields = Vec::new();
        for field_name in ["x", "y"] {
            let fname = s.ident(field_name);
            let ty = {
                let path = s.path_type(&["f64"], None);
                s.ty(TypeKind::Path(path))
            };
            let span = cover(fname.span, ty.span);
            fields.push(Field {
                vis: Visibility::Private,
                name: fname,
                ty,
                span,
            });
        }
        items.push(s.item(ItemKind::Struct(StructDecl {
            vis: Visibility::Private,
            name,
            generics: None,
            fields,
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Struct(decl) = &mut last.kind {
            decl.span = last.span;
        }
    }

    // impl Point { pub fn dist(self, o: Point) -> f64 { ... } }
    {
        let self_ty = s.path_type(&["Point"], None);
        let vis = Visibility::Public(s.leaf(3));
        let sig = {
            let name = s.ident("dist");
            let receiver = s.leaf(4);
            let o = s.ident("o");
            let point_ty = {
                let path = s.path_type(&["Point"], None);
                s.ty(TypeKind::Path(path))
            };
            let ret = {
                let path = s.path_type(&["f64"], None);
                s.ty(TypeKind::Path(path))
            };
            sig_ex(
                name,
                vec![Param::Receiver(receiver), named_param(o, point_ty)],
                Some(ret),
            )
        };
        let body = {
            let dx = {
                let pat = s.binding("dx");
                let init = {
                    let left = {
                        let base = s.path_expr(&["self"]);
                        let fname = s.ident("x");
                        ex_cover(
                            &[base.span, fname.span],
                            ExprKind::Field(Box::new(base), fname),
                        )
                    };
                    let right = {
                        let base = s.path_expr(&["o"]);
                        let fname = s.ident("x");
                        ex_cover(
                            &[base.span, fname.span],
                            ExprKind::Field(Box::new(base), fname),
                        )
                    };
                    ex_cover(
                        &[left.span, right.span],
                        ExprKind::Binary(BinOp::Sub, Box::new(left), Box::new(right)),
                    )
                };
                let span = cover(pat.span, init.span);
                s.stmt(StmtKind::Let(LetStmt {
                    mutable: false,
                    pat,
                    ty: None,
                    init,
                    span,
                }))
            };
            let dy = {
                let pat = s.binding("dy");
                let init = {
                    let left = {
                        let base = s.path_expr(&["self"]);
                        let fname = s.ident("y");
                        ex_cover(
                            &[base.span, fname.span],
                            ExprKind::Field(Box::new(base), fname),
                        )
                    };
                    let right = {
                        let base = s.path_expr(&["o"]);
                        let fname = s.ident("y");
                        ex_cover(
                            &[base.span, fname.span],
                            ExprKind::Field(Box::new(base), fname),
                        )
                    };
                    ex_cover(
                        &[left.span, right.span],
                        ExprKind::Binary(BinOp::Sub, Box::new(left), Box::new(right)),
                    )
                };
                let span = cover(pat.span, init.span);
                s.stmt(StmtKind::Let(LetStmt {
                    mutable: false,
                    pat,
                    ty: None,
                    init,
                    span,
                }))
            };
            let tail = {
                let base = {
                    let dx2 = {
                        let dl = s.path_expr(&["dx"]);
                        let dr = s.path_expr(&["dx"]);
                        ex_cover(
                            &[dl.span, dr.span],
                            ExprKind::Binary(BinOp::Mul, Box::new(dl), Box::new(dr)),
                        )
                    };
                    let dy2 = {
                        let dl = s.path_expr(&["dy"]);
                        let dr = s.path_expr(&["dy"]);
                        ex_cover(
                            &[dl.span, dr.span],
                            ExprKind::Binary(BinOp::Mul, Box::new(dl), Box::new(dr)),
                        )
                    };
                    ex_cover(
                        &[dx2.span, dy2.span],
                        ExprKind::Binary(BinOp::Add, Box::new(dx2), Box::new(dy2)),
                    )
                };
                let fname = s.ident("sqrt");
                let field_expr = ex_cover(
                    &[base.span, fname.span],
                    ExprKind::Field(Box::new(base), fname),
                );
                ex_cover(
                    &[field_expr.span],
                    ExprKind::Call(Box::new(field_expr), Vec::new()),
                )
            };
            s.block(vec![dx, dy], Some(tail))
        };
        let fn_span = cover(vis_span(&vis), body.span);
        let fn_decl = FnDecl {
            vis,
            generics: None,
            sig,
            body,
            span: fn_span,
        };
        items.push(s.item(ItemKind::Impl(ImplDecl {
            generics: None,
            trait_: None,
            self_ty,
            items: vec![ImplItem::Fn(fn_decl)],
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Impl(decl) = &mut last.kind {
            decl.span = last.span;
        }
    }

    // enum Shape { Circle(f64), Rect(f64, f64) }
    {
        let name = s.ident("Shape");
        let mut variants = Vec::new();
        for (variant_name, arity) in [("Circle", 1_usize), ("Rect", 2)] {
            let vname = s.ident(variant_name);
            let mut payload = Vec::new();
            for _ in 0..arity {
                let path = s.path_type(&["f64"], None);
                payload.push(s.ty(TypeKind::Path(path)));
            }
            let mut spans = vec![vname.span];
            spans.extend(payload.iter().map(|t| t.span));
            variants.push(Variant {
                name: vname,
                payload,
                span: cover_all(&spans),
            });
        }
        items.push(s.item(ItemKind::Enum(EnumDecl {
            vis: Visibility::Private,
            name,
            generics: None,
            variants,
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Enum(decl) = &mut last.kind {
            decl.span = last.span;
        }
    }

    // fn area(s: Shape) -> f64 { match s { ... } }
    {
        let sig = {
            let name = s.ident("area");
            let param_name = s.ident("s");
            let param_ty = {
                let path = s.path_type(&["Shape"], None);
                s.ty(TypeKind::Path(path))
            };
            let ret = {
                let path = s.path_type(&["f64"], None);
                s.ty(TypeKind::Path(path))
            };
            sig_ex(name, vec![named_param(param_name, param_ty)], Some(ret))
        };
        let body = {
            let tail = {
                let scrutinee = s.path_expr(&["s"]);
                let arm1 = {
                    let r = s.binding("r");
                    let pat = s.path_pat(&["Shape", "Circle"], vec![r]);
                    let body = {
                        let pi_like: f64 = "3.14159".parse().expect("literal parses");
                        let lit = s.float_expr(pi_like);
                        let r1 = s.path_expr(&["r"]);
                        let first = ex_cover(
                            &[lit.span, r1.span],
                            ExprKind::Binary(BinOp::Mul, Box::new(lit), Box::new(r1)),
                        );
                        let r2 = s.path_expr(&["r"]);
                        ex_cover(
                            &[first.span, r2.span],
                            ExprKind::Binary(BinOp::Mul, Box::new(first), Box::new(r2)),
                        )
                    };
                    let span = cover(pat.span, body.span);
                    Arm { pat, body, span }
                };
                let arm2 = {
                    let w = s.binding("w");
                    let h = s.binding("h");
                    let pat = s.path_pat(&["Shape", "Rect"], vec![w, h]);
                    let body = {
                        let w = s.path_expr(&["w"]);
                        let h = s.path_expr(&["h"]);
                        ex_cover(
                            &[w.span, h.span],
                            ExprKind::Binary(BinOp::Mul, Box::new(w), Box::new(h)),
                        )
                    };
                    let span = cover(pat.span, body.span);
                    Arm { pat, body, span }
                };
                ex_cover(
                    &[scrutinee.span, arm1.span, arm2.span],
                    ExprKind::Match(Box::new(scrutinee), vec![arm1, arm2]),
                )
            };
            s.block(Vec::new(), Some(tail))
        };
        items.push(s.item(ItemKind::Fn(FnDecl {
            vis: Visibility::Private,
            generics: None,
            sig,
            body,
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Fn(decl) = &mut last.kind {
            decl.span = cover(decl.sig.span, decl.body.span);
        }
    }

    // fn parse(s: str) -> Result<i64, str> { let n = s.to_int()?; if ... }
    {
        let sig = {
            let name = s.ident("parse");
            let param_name = s.ident("s");
            let param_ty = {
                let path = s.path_type(&["str"], None);
                s.ty(TypeKind::Path(path))
            };
            let ret = {
                let i64_ty = {
                    let path = s.path_type(&["i64"], None);
                    s.ty(TypeKind::Path(path))
                };
                let str_ty = {
                    let path = s.path_type(&["str"], None);
                    s.ty(TypeKind::Path(path))
                };
                let result = s.path_type(&["Result"], Some(vec![i64_ty, str_ty]));
                s.ty(TypeKind::Path(result))
            };
            sig_ex(name, vec![named_param(param_name, param_ty)], Some(ret))
        };
        let body = {
            let let_stmt = {
                let pat = s.binding("n");
                let init = {
                    let s_expr = s.path_expr(&["s"]);
                    let fname = s.ident("to_int");
                    let field_expr = ex_cover(
                        &[s_expr.span, fname.span],
                        ExprKind::Field(Box::new(s_expr), fname),
                    );
                    let call_expr = ex_cover(
                        &[field_expr.span],
                        ExprKind::Call(Box::new(field_expr), Vec::new()),
                    );
                    ex_cover(&[call_expr.span], ExprKind::Try(Box::new(call_expr)))
                };
                let span = cover(pat.span, init.span);
                s.stmt(StmtKind::Let(LetStmt {
                    mutable: false,
                    pat,
                    ty: None,
                    init,
                    span,
                }))
            };
            let tail = {
                let cond = {
                    let n = s.path_expr(&["n"]);
                    let zero = s.int_expr(0, Radix::Dec);
                    ex_cover(
                        &[n.span, zero.span],
                        ExprKind::Binary(BinOp::Lt, Box::new(n), Box::new(zero)),
                    )
                };
                let then = {
                    let callee = s.path_expr(&["Err"]);
                    let arg = s.string_expr("negativo");
                    let call_expr = ex_cover(
                        &[callee.span, arg.span],
                        ExprKind::Call(Box::new(callee), vec![arg]),
                    );
                    s.block(Vec::new(), Some(call_expr))
                };
                let else_branch = {
                    let block = {
                        let callee = s.path_expr(&["Ok"]);
                        let arg = s.path_expr(&["n"]);
                        let call_expr = ex_cover(
                            &[callee.span, arg.span],
                            ExprKind::Call(Box::new(callee), vec![arg]),
                        );
                        s.block(Vec::new(), Some(call_expr))
                    };
                    let span = block.span;
                    ex(span, ExprKind::Block(block))
                };
                ex_cover(
                    &[cond.span, then.span, else_branch.span],
                    ExprKind::If(Box::new(cond), then, Some(Box::new(else_branch))),
                )
            };
            s.block(vec![let_stmt], Some(tail))
        };
        items.push(s.item(ItemKind::Fn(FnDecl {
            vis: Visibility::Private,
            generics: None,
            sig,
            body,
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Fn(decl) = &mut last.kind {
            decl.span = cover(decl.sig.span, decl.body.span);
        }
    }

    // fn main() { let mut total = 0.0; for ... { ... } io.println(...); }
    {
        let sig = {
            let name = s.ident("main");
            sig_ex(name, Vec::new(), None)
        };
        let body = {
            let let_stmt = {
                let pat = s.binding("total");
                let init = s.float_expr(0.0);
                let span = cover(pat.span, init.span);
                s.stmt(StmtKind::Let(LetStmt {
                    mutable: true,
                    pat,
                    ty: None,
                    init,
                    span,
                }))
            };
            let for_stmt = {
                let pat = s.binding("i");
                let iter = {
                    let start = s.int_expr(0, Radix::Dec);
                    let end = s.int_expr(10, Radix::Dec);
                    ex_cover(
                        &[start.span, end.span],
                        ExprKind::Range(Box::new(start), Box::new(end)),
                    )
                };
                let for_body = {
                    let assign_stmt = {
                        let target = s.path_expr(&["total"]);
                        let value = {
                            let i = s.path_expr(&["i"]);
                            let ty = {
                                let path = s.path_type(&["f64"], None);
                                s.ty(TypeKind::Path(path))
                            };
                            let span = cover(i.span, ty.span);
                            ex(span, ExprKind::Cast(Box::new(i), ty))
                        };
                        let assign = ex_cover(
                            &[target.span, value.span],
                            ExprKind::Assign(
                                AssignOp::AddAssign,
                                Box::new(target),
                                Box::new(value),
                            ),
                        );
                        s.stmt(StmtKind::Expr(assign))
                    };
                    s.block(vec![assign_stmt], None)
                };
                let for_expr = ex_cover(
                    &[pat.span, iter.span, for_body.span],
                    ExprKind::For(Box::new(pat), Box::new(iter), for_body),
                );
                s.stmt(StmtKind::Expr(for_expr))
            };
            let println_stmt = {
                let callee = {
                    let base = s.path_expr(&["io"]);
                    let fname = s.ident("println");
                    ex_cover(
                        &[base.span, fname.span],
                        ExprKind::Field(Box::new(base), fname),
                    )
                };
                let text_span = s.leaf(8);
                let interp_expr = s.path_expr(&["total"]);
                let lit_span = cover(text_span, interp_expr.span);
                let arg = ex(
                    lit_span,
                    ExprKind::Lit(Lit::Str(StringLit {
                        parts: vec![
                            StringPart::Text(TextSegment {
                                text: "total = ".to_string(),
                                span: text_span,
                            }),
                            StringPart::Expr(interp_expr),
                        ],
                        span: lit_span,
                    })),
                );
                let call_expr = ex_cover(
                    &[callee.span, arg.span],
                    ExprKind::Call(Box::new(callee), vec![arg]),
                );
                s.stmt(StmtKind::Expr(call_expr))
            };
            s.block(vec![let_stmt, for_stmt, println_stmt], None)
        };
        items.push(s.item(ItemKind::Fn(FnDecl {
            vis: Visibility::Private,
            generics: None,
            sig,
            body,
            span: fspan(0, 0),
        })));
        let last = items.last_mut().expect("just pushed");
        if let ItemKind::Fn(decl) = &mut last.kind {
            decl.span = cover(decl.sig.span, decl.body.span);
        }
    }

    items
}

/// Returns the span of a visibility marker.
pub fn vis_span(vis: &Visibility) -> Span {
    match vis {
        Visibility::Private => fspan(0, 0),
        Visibility::Public(span) => *span,
    }
}

// ------------------------------------------------------------------
// Miscellaneous builders used by multiple suites
// ------------------------------------------------------------------

/// Wraps a function kind as an item with a derived span.
pub fn fn_item(name: &str, params: Vec<Param>, ret: Option<Type>, body: Block) -> Item {
    let name = ident(name, 0);
    let sig = sig_ex(name, params, ret);
    let span = cover(sig.span, body.span);
    Item {
        id: ItemId::from_index(span.lo),
        kind: ItemKind::Fn(FnDecl {
            vis: Visibility::Private,
            generics: None,
            sig,
            body,
            span,
        }),
        span,
    }
}

/// Trait declaration convenience builder.
pub fn trait_item_with(name: &str, sig: FnSig) -> TraitDecl {
    let name_ident = ident(name, 0);
    let span = cover(name_ident.span, sig.span);
    TraitDecl {
        vis: Visibility::Private,
        name: name_ident,
        generics: None,
        items: vec![xl_ast::TraitItem {
            span: sig.span,
            sig,
        }],
        span,
    }
}
