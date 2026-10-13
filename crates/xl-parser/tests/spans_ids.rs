//! Suite G: deterministic identifiers and source-span integrity.

mod common;

use common::{SPEC_EXAMPLE, clean, parse_source};
use xl_ast::{
    Arm, Block, Expr, ExprKind, ImplItem, Item, ItemKind, Param, Pat, PatKind, Stmt, StmtKind,
    StringPart, Type, TypeKind,
};
use xl_span::Span;

/// Collects every expression identifier in an item list (construction
/// order is not reconstructible by walking, so the result is checked by
/// set properties instead).
fn collect_expr_ids(items: &[Item]) -> Vec<u32> {
    let mut ids = Vec::new();
    for item in items {
        walk_item_exprs(item, &mut |expr| ids.push(expr.id.index()));
    }
    ids
}

/// Collects every span of every node in an item list.
fn collect_spans(items: &[Item]) -> Vec<Span> {
    let mut spans = Vec::new();
    for item in items {
        walk_item_spans(item, &mut |span| spans.push(span));
    }
    spans
}

trait SpanSink {
    fn push(&mut self, span: Span);
}

impl<F: FnMut(Span)> SpanSink for F {
    fn push(&mut self, span: Span) {
        self(span);
    }
}

fn walk_item_spans(item: &Item, sink: &mut impl SpanSink) {
    sink.push(item.span);
    match &item.kind {
        ItemKind::Fn(decl) => {
            sink.push(decl.span);
            sink.push(decl.sig.span);
            sink.push(decl.sig.name.span);
            for param in &decl.sig.params {
                match param {
                    Param::Receiver(span) => sink.push(*span),
                    Param::Named { name, ty, span } => {
                        sink.push(*span);
                        sink.push(name.span);
                        walk_type_spans(ty, sink);
                    }
                }
            }
            if let Some(ret) = &decl.sig.ret {
                walk_type_spans(ret, sink);
            }
            walk_block_spans(&decl.body, sink);
        }
        ItemKind::Struct(decl) => {
            sink.push(decl.span);
            sink.push(decl.name.span);
            for field in &decl.fields {
                sink.push(field.span);
                sink.push(field.name.span);
                walk_type_spans(&field.ty, sink);
            }
        }
        ItemKind::Enum(decl) => {
            sink.push(decl.span);
            for variant in &decl.variants {
                sink.push(variant.span);
                sink.push(variant.name.span);
                for ty in &variant.payload {
                    walk_type_spans(ty, sink);
                }
            }
        }
        ItemKind::Impl(decl) => {
            sink.push(decl.span);
            for ImplItem::Fn(func) in &decl.items {
                walk_fn_spans(func, sink);
            }
        }
        ItemKind::Trait(decl) => {
            sink.push(decl.span);
            for item in &decl.items {
                sink.push(item.span);
            }
        }
        ItemKind::Import(decl) => {
            sink.push(decl.span);
            for ident in &decl.path {
                sink.push(ident.span);
            }
        }
    }
}

fn walk_fn_spans(decl: &xl_ast::FnDecl, sink: &mut impl SpanSink) {
    sink.push(decl.span);
    sink.push(decl.sig.span);
    walk_block_spans(&decl.body, sink);
}

fn walk_block_spans(block: &Block, sink: &mut impl SpanSink) {
    sink.push(block.span);
    for stmt in &block.stmts {
        walk_stmt_spans(stmt, sink);
    }
    if let Some(tail) = &block.tail {
        walk_expr_spans(tail, sink);
    }
}

fn walk_stmt_spans(stmt: &Stmt, sink: &mut impl SpanSink) {
    sink.push(stmt.span);
    match &stmt.kind {
        StmtKind::Let(let_stmt) => {
            sink.push(let_stmt.span);
            walk_pat_spans(&let_stmt.pat, sink);
            if let Some(ty) = &let_stmt.ty {
                walk_type_spans(ty, sink);
            }
            walk_expr_spans(&let_stmt.init, sink);
        }
        StmtKind::Expr(expr) => walk_expr_spans(expr, sink),
        StmtKind::Item(item) => walk_item_spans(item, sink),
    }
}

fn walk_expr_spans(expr: &Expr, sink: &mut impl SpanSink) {
    sink.push(expr.span);
    match &expr.kind {
        ExprKind::Lit(lit) => {
            if let xl_ast::Lit::Str(string) = lit {
                sink.push(string.span);
                for part in &string.parts {
                    match part {
                        StringPart::Text(text) => sink.push(text.span),
                        StringPart::Expr(inner) => walk_expr_spans(inner, sink),
                    }
                }
            }
        }
        ExprKind::Path(path) => {
            sink.push(path.span);
            for ident in &path.segments {
                sink.push(ident.span);
            }
        }
        ExprKind::Unary(_, inner) | ExprKind::Try(inner) => walk_expr_spans(inner, sink),
        ExprKind::Binary(_, lhs, rhs) | ExprKind::Range(lhs, rhs) | ExprKind::Index(lhs, rhs) => {
            walk_expr_spans(lhs, sink);
            walk_expr_spans(rhs, sink);
        }
        ExprKind::Assign(_, lhs, rhs) => {
            walk_expr_spans(lhs, sink);
            walk_expr_spans(rhs, sink);
        }
        ExprKind::Call(callee, args) => {
            walk_expr_spans(callee, sink);
            for arg in args {
                walk_expr_spans(arg, sink);
            }
        }
        ExprKind::Field(base, name) => {
            walk_expr_spans(base, sink);
            sink.push(name.span);
        }
        ExprKind::Cast(inner, ty) => {
            walk_expr_spans(inner, sink);
            walk_type_spans(ty, sink);
        }
        ExprKind::If(cond, block, else_) => {
            walk_expr_spans(cond, sink);
            walk_block_spans(block, sink);
            if let Some(else_expr) = else_ {
                walk_expr_spans(else_expr, sink);
            }
        }
        ExprKind::Match(scrutinee, arms) => {
            walk_expr_spans(scrutinee, sink);
            for Arm { pat, body, span } in arms {
                sink.push(*span);
                walk_pat_spans(pat, sink);
                walk_expr_spans(body, sink);
            }
        }
        ExprKind::Block(block) => walk_block_spans(block, sink),
        ExprKind::While(cond, block) => {
            walk_expr_spans(cond, sink);
            walk_block_spans(block, sink);
        }
        ExprKind::For(pat, iterable, block) => {
            walk_pat_spans(pat, sink);
            walk_expr_spans(iterable, sink);
            walk_block_spans(block, sink);
        }
        ExprKind::Loop(block) => walk_block_spans(block, sink),
        ExprKind::Return(operand) => {
            if let Some(operand) = operand {
                walk_expr_spans(operand, sink);
            }
        }
        ExprKind::Tuple(items) | ExprKind::Array(items) => {
            for item in items {
                walk_expr_spans(item, sink);
            }
        }
        ExprKind::Break | ExprKind::Continue => {}
    }
}

fn walk_pat_spans(pat: &Pat, sink: &mut impl SpanSink) {
    sink.push(pat.span);
    match &pat.kind {
        PatKind::Wildcard | PatKind::Lit(_) => {}
        PatKind::Binding(ident) => sink.push(ident.span),
        PatKind::Path { path, args } => {
            sink.push(path.span);
            if let Some(args) = args {
                for arg in args {
                    walk_pat_spans(arg, sink);
                }
            }
        }
        PatKind::Tuple(items) => {
            for item in items {
                walk_pat_spans(item, sink);
            }
        }
    }
}

fn walk_type_spans(ty: &Type, sink: &mut impl SpanSink) {
    sink.push(ty.span);
    match &ty.kind {
        TypeKind::Path(path) => {
            sink.push(path.span);
            for ident in &path.segments {
                sink.push(ident.span);
            }
            if let Some(args) = &path.generics {
                for arg in args {
                    walk_type_spans(arg, sink);
                }
            }
        }
        TypeKind::Tuple(items) => {
            for item in items {
                walk_type_spans(item, sink);
            }
        }
        TypeKind::Slice(inner) => walk_type_spans(inner, sink),
        TypeKind::Array(inner, length) => {
            walk_type_spans(inner, sink);
            walk_expr_spans(length, sink);
        }
        TypeKind::Fn { params, ret } => {
            for param in params {
                walk_type_spans(param, sink);
            }
            if let Some(ret) = ret {
                walk_type_spans(ret, sink);
            }
        }
    }
}

fn walk_item_exprs(item: &Item, sink: &mut impl FnMut(&Expr)) {
    match &item.kind {
        ItemKind::Fn(decl) => walk_block_exprs(&decl.body, sink),
        ItemKind::Impl(decl) => {
            for ImplItem::Fn(func) in &decl.items {
                walk_block_exprs(&func.body, sink);
            }
        }
        _ => {}
    }
}

fn walk_block_exprs(block: &Block, sink: &mut impl FnMut(&Expr)) {
    for stmt in &block.stmts {
        match &stmt.kind {
            StmtKind::Let(let_stmt) => walk_exprs(&let_stmt.init, sink),
            StmtKind::Expr(expr) => walk_exprs(expr, sink),
            StmtKind::Item(item) => walk_item_exprs(item, sink),
        }
    }
    if let Some(tail) = &block.tail {
        walk_exprs(tail, sink);
    }
}

fn walk_exprs(expr: &Expr, sink: &mut impl FnMut(&Expr)) {
    sink(expr);
    match &expr.kind {
        ExprKind::Lit(xl_ast::Lit::Str(string)) => {
            for part in &string.parts {
                if let StringPart::Expr(inner) = part {
                    walk_exprs(inner, sink);
                }
            }
        }
        ExprKind::Unary(_, inner) | ExprKind::Try(inner) => walk_exprs(inner, sink),
        ExprKind::Binary(_, lhs, rhs) | ExprKind::Range(lhs, rhs) | ExprKind::Index(lhs, rhs) => {
            walk_exprs(lhs, sink);
            walk_exprs(rhs, sink);
        }
        ExprKind::Assign(_, lhs, rhs) => {
            walk_exprs(lhs, sink);
            walk_exprs(rhs, sink);
        }
        ExprKind::Call(callee, args) => {
            walk_exprs(callee, sink);
            for arg in args {
                walk_exprs(arg, sink);
            }
        }
        ExprKind::Field(base, _) => walk_exprs(base, sink),
        ExprKind::Cast(inner, _) => walk_exprs(inner, sink),
        ExprKind::If(cond, block, else_) => {
            walk_exprs(cond, sink);
            walk_block_exprs(block, sink);
            if let Some(else_expr) = else_ {
                walk_exprs(else_expr, sink);
            }
        }
        ExprKind::Match(scrutinee, arms) => {
            walk_exprs(scrutinee, sink);
            for arm in arms {
                walk_exprs(&arm.body, sink);
            }
        }
        ExprKind::Block(block) => walk_block_exprs(block, sink),
        ExprKind::While(cond, block) => {
            walk_exprs(cond, sink);
            walk_block_exprs(block, sink);
        }
        ExprKind::For(_, iterable, block) => {
            walk_exprs(iterable, sink);
            walk_block_exprs(block, sink);
        }
        ExprKind::Loop(block) => walk_block_exprs(block, sink),
        ExprKind::Return(Some(operand)) => walk_exprs(operand, sink),
        ExprKind::Tuple(items) | ExprKind::Array(items) => {
            for item in items {
                walk_exprs(item, sink);
            }
        }
        _ => {}
    }
}

#[test]
fn item_ids_are_sequential_from_zero() {
    let parsed = clean(SPEC_EXAMPLE);
    for (index, item) in parsed.output.items.iter().enumerate() {
        assert_eq!(
            item.id.index(),
            index as u32,
            "item identifiers are instance-scoped allocation order"
        );
    }
}

#[test]
fn expression_ids_are_unique_and_dense_from_zero() {
    let parsed = clean(SPEC_EXAMPLE);
    let mut ids = collect_expr_ids(&parsed.output.items);
    assert!(!ids.is_empty());
    ids.sort_unstable();
    let unique = {
        let mut deduped = ids.clone();
        deduped.dedup();
        deduped
    };
    assert_eq!(ids.len(), unique.len(), "identifiers must be unique");
    assert_eq!(
        unique.first().copied(),
        Some(0),
        "identifiers start at zero"
    );
    let expected: Vec<u32> = (0..unique.len() as u32).collect();
    assert_eq!(unique, expected, "identifiers are dense");
}

#[test]
fn identifiers_restart_per_ast_instance() {
    let first = clean("fn a() { let x = 1; }");
    let second = clean("fn b() -> i64 { 2 }");
    let first_ids = collect_expr_ids(&first.output.items);
    let second_ids = collect_expr_ids(&second.output.items);
    assert_eq!(first_ids.first().copied(), Some(0));
    assert_eq!(second_ids.first().copied(), Some(0));
}

#[test]
fn every_span_is_ordered_and_resolvable() {
    let parsed = clean(SPEC_EXAMPLE);
    let len = SPEC_EXAMPLE.len() as u32;
    for span in collect_spans(&parsed.output.items) {
        assert_eq!(span.file, parsed.file, "single-file parse");
        assert!(span.lo <= span.hi, "not inverted: {span:?}");
        assert!(span.hi <= len, "within file: {span:?}");
        parsed
            .sources
            .span_slice(span)
            .unwrap_or_else(|error| panic!("span {span:?} must resolve: {error}"));
    }
}

#[test]
fn spans_are_utf8_boundaries_with_unicode_identifiers() {
    let source = "fn f() { let café = 1; let δ = café + 1; }";
    let parsed = clean(source);
    let len = source.len() as u32;
    for span in collect_spans(&parsed.output.items) {
        assert!(span.hi <= len);
        parsed.sources.span_slice(span).expect("char-boundary span");
    }
    // The δ path span slices exactly to the identifier text.
    let mut found = false;
    let mut sink = |span: Span| {
        if parsed.sources.span_slice(span) == Ok("δ") {
            found = true;
        }
    };
    for item in &parsed.output.items {
        walk_item_spans(item, &mut sink);
    }
    assert!(found, "the δ identifier must have an exact span");
}

#[test]
fn nested_expression_spans_are_contained() {
    let source = "fn f() { let v = (a + b) * c; }";
    let parsed = clean(source);
    let spans = collect_spans(&parsed.output.items);
    let top = spans[0];
    for span in &spans[1..] {
        assert!(
            span.lo >= top.lo && span.hi <= top.hi,
            "{span:?} must be inside {top:?}"
        );
    }
}

#[test]
fn placeholder_spans_are_zero_width_at_insertion_points() {
    let parsed = parse_source("fn f() { let x = ; }");
    let body = common::fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(let_stmt.init.span.is_empty());
    assert_eq!(let_stmt.init.span.file, parsed.file);
    // The insertion point sits at the token that followed the `=`.
    assert_eq!(
        parsed
            .sources
            .span_slice(xl_span::Span {
                file: parsed.file,
                lo: let_stmt.init.span.lo,
                hi: let_stmt.init.span.lo + 1
            })
            .expect("resolvable"),
        ";"
    );
}

#[test]
fn ids_do_not_depend_on_memory_addresses() {
    // Two parses of equal-shaped different sources produce identical ID
    // sequences (addresses and allocation history are irrelevant).
    let a = clean("fn f() { let x = 1; }");
    let b = clean("fn g() { let y = 2; }");
    assert_eq!(
        collect_expr_ids(&a.output.items),
        collect_expr_ids(&b.output.items)
    );
}
