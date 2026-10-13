//! Suite E: structured strings and interpolation.

mod common;

use common::{clean, fn_body, parse_source};
use xl_ast::{ExprKind, Lit, StmtKind, StringPart};

/// Extracts the string literal from `let s = <literal>;`.
fn string_lit(source: &str) -> (common::Parsed, xl_ast::StringLit) {
    let full = format!("fn f() {{ let s = {source}; }}");
    let parsed = clean(&full);
    let string = {
        let body = fn_body(&parsed.output.items, "f");
        let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
            panic!("let");
        };
        let ExprKind::Lit(Lit::Str(string)) = &let_stmt.init.kind else {
            panic!("string literal expected");
        };
        string.clone()
    };
    (parsed, string)
}

#[test]
fn text_only_string() {
    let (_, string) = string_lit("\"a\\nb\"");
    assert_eq!(string.parts.len(), 1);
    match &string.parts[0] {
        StringPart::Text(text) => assert_eq!(text.text, "a\nb"),
        other => panic!("text part expected, got {other:?}"),
    }
}

#[test]
fn escaped_braces_stay_text() {
    let (_, string) = string_lit("\"literal \\{brace\\}\"");
    assert_eq!(string.parts.len(), 1);
    match &string.parts[0] {
        StringPart::Text(text) => assert_eq!(text.text, "literal {brace}"),
        other => panic!("text part expected, got {other:?}"),
    }
}

#[test]
fn simple_interpolation_parts() {
    let (_, string) = string_lit("\"total = {total}\"");
    assert_eq!(string.parts.len(), 2);
    match &string.parts[0] {
        StringPart::Text(text) => assert_eq!(text.text, "total = "),
        other => panic!("text, got {other:?}"),
    }
    match &string.parts[1] {
        StringPart::Expr(expr) => assert!(matches!(expr.kind, ExprKind::Path(_))),
        other => panic!("expr, got {other:?}"),
    }
}

#[test]
fn interpolation_expression_parses_full_grammar() {
    let (_, string) = string_lit("\"{compute(1 + 2)}\"");
    match &string.parts[0] {
        StringPart::Expr(expr) => match &expr.kind {
            ExprKind::Call(callee, args) => {
                assert!(matches!(callee.kind, ExprKind::Path(_)));
                assert_eq!(args.len(), 1);
                assert!(matches!(
                    common::binary_op(&args[0]),
                    Some(xl_ast::BinOp::Add)
                ));
            }
            other => panic!("call expected, got {other:?}"),
        },
        other => panic!("expr, got {other:?}"),
    }
}

#[test]
fn interpolation_with_blocks_and_if() {
    let (_, string) = string_lit("\"{ if x { 1 } else { 2 } }\"");
    match &string.parts[0] {
        StringPart::Expr(expr) => assert!(matches!(expr.kind, ExprKind::If(..))),
        other => panic!("expr, got {other:?}"),
    }
}

#[test]
fn nested_interpolation_through_inner_string() {
    let (_, string) = string_lit("\"a { \"b {c} d\" } e\"");
    assert_eq!(string.parts.len(), 3);
    match &string.parts[1] {
        StringPart::Expr(expr) => {
            let ExprKind::Lit(Lit::Str(inner)) = &expr.kind else {
                panic!("inner string expected");
            };
            assert_eq!(inner.parts.len(), 3);
            match &inner.parts[1] {
                StringPart::Expr(nested) => {
                    assert!(matches!(nested.kind, ExprKind::Path(_)));
                }
                other => panic!("nested interpolation, got {other:?}"),
            }
        }
        other => panic!("expr, got {other:?}"),
    }
}

#[test]
fn brace_inside_comment_does_not_close_interpolation() {
    let (_, string) = string_lit("\"{ f() /* } */ }\"");
    assert_eq!(string.parts.len(), 1);
    match &string.parts[0] {
        StringPart::Expr(expr) => assert!(matches!(expr.kind, ExprKind::Call(..))),
        other => panic!("expr, got {other:?}"),
    }
}

#[test]
fn interpolation_spans_are_absolute_and_contained() {
    let source = "fn f() { let s = \"total = {total}\"; }";
    let parsed = clean(source);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    let ExprKind::Lit(Lit::Str(string)) = &let_stmt.init.kind else {
        panic!("string");
    };
    let StringPart::Expr(expr) = &string.parts[1] else {
        panic!("expr part");
    };
    assert_eq!(expr.span.file, parsed.file);
    assert!(expr.span.lo >= string.span.lo && expr.span.hi <= string.span.hi);
    assert_eq!(
        parsed.sources.span_slice(expr.span).expect("resolvable"),
        "total"
    );
}

#[test]
fn embedded_expression_carries_its_own_spans() {
    let source = "fn f() { let s = \"{a + b}\"; }";
    let parsed = clean(source);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    let ExprKind::Lit(Lit::Str(string)) = &let_stmt.init.kind else {
        panic!("string");
    };
    let StringPart::Expr(expr) = &string.parts[0] else {
        panic!("expr part");
    };
    assert_eq!(
        parsed.sources.span_slice(expr.span).expect("resolvable"),
        "a + b"
    );
}

#[test]
fn empty_interpolation_is_reported() {
    let full = "fn f() { let s = \"{}\"; }";
    let parsed = parse_source(full);
    assert_eq!(parsed.messages(), vec!["empty interpolation"]);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    let ExprKind::Lit(Lit::Str(string)) = &let_stmt.init.kind else {
        panic!("string");
    };
    let StringPart::Expr(expr) = &string.parts[0] else {
        panic!("placeholder expr part");
    };
    assert!(expr.span.is_empty(), "placeholder span is zero-width");
}

#[test]
fn empty_interpolation_diagnostic_span_covers_the_segment() {
    let source = "fn f() { let s = \"{}\"; }";
    let parsed = parse_source(source);
    let diagnostic = &parsed.output.diagnostics[0];
    assert_eq!(
        parsed
            .sources
            .span_slice(diagnostic.primary().span)
            .expect("resolvable"),
        "{}"
    );
}

#[test]
fn unterminated_interpolation_reports_lexically_with_structural_recovery() {
    // `"a { b"` — the quote inside the interpolation opens an embedded
    // string (`lexer-contract.md` §5.3), which is then unterminated and
    // swallows the rest of the line, including the block's closing brace.
    // The lexical reports are joined by the structural `expected '}'` for
    // that genuinely missing brace; the parser adds no interpolation-level
    // cascade.
    let full = "fn f() { let s = \"a { b\"; }";
    let parsed = parse_source(full);
    assert_eq!(
        parsed.messages(),
        vec![
            "unterminated string literal",
            "unterminated interpolation in string literal",
            "expected '}'"
        ]
    );
    // The string literal survives with structured parts.
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(matches!(let_stmt.init.kind, ExprKind::Lit(Lit::Str(_))));
}

#[test]
fn unterminated_interpolation_with_expression_reports_lexically_only() {
    // EOF inside the interpolation with a complete expression present:
    // the parser consumes the expression, sees the abnormal-termination
    // marker, and adds nothing.
    let full = "fn f() { let s = \"a { b + c;";
    let parsed = parse_source(full);
    assert_eq!(
        parsed.messages(),
        vec![
            "unterminated interpolation in string literal",
            "expected '}'"
        ]
    );
}

#[test]
fn empty_unterminated_interpolation_reports_lexically_with_structural_recovery() {
    let full = "fn f() { let s = \"{\"; }";
    let parsed = parse_source(full);
    assert_eq!(
        parsed.messages(),
        vec![
            "unterminated string literal",
            "unterminated interpolation in string literal",
            "expected '}'"
        ]
    );
    // No `empty interpolation` cascade: the lexer already reported the
    // abnormal termination.
    assert!(
        !parsed
            .messages()
            .iter()
            .any(|message| message == "empty interpolation")
    );
}

#[test]
fn interpolation_leftover_tokens_reported() {
    let full = "fn f() { let s = \"{1 2}\"; }";
    let parsed = parse_source(full);
    assert_eq!(parsed.messages(), vec!["expected '}'"]);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    let ExprKind::Lit(Lit::Str(string)) = &let_stmt.init.kind else {
        panic!("string survives interpolation recovery");
    };
    assert_eq!(string.parts.len(), 1);
}

#[test]
fn multiple_interpolations_in_one_string() {
    let (_, string) = string_lit("\"{a} and {b}\"");
    assert_eq!(string.parts.len(), 3);
    assert!(matches!(string.parts[1], StringPart::Text(_)));
    assert!(matches!(string.parts[0], StringPart::Expr(_)));
    assert!(matches!(string.parts[2], StringPart::Expr(_)));
}

#[test]
fn interpolation_with_try_and_cast() {
    let (_, string) = string_lit("\"{x.to_int()? as f64}\"");
    match &string.parts[0] {
        StringPart::Expr(expr) => assert!(matches!(expr.kind, ExprKind::Cast(..))),
        other => panic!("expr, got {other:?}"),
    }
}

#[test]
fn string_in_pattern_position() {
    let parsed = clean("fn f() { match s { \"a\" => 1, _ => 2, } }");
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("match");
    };
    assert!(matches!(
        arms[0].pat.kind,
        xl_ast::PatKind::Lit(Lit::Str(_))
    ));
}
