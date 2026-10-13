//! Suite F: syntactic error recovery — multiple diagnostics, missing
//! delimiters, truncated input, and invalid lexical tokens.

mod common;

use common::{clean, find_fn, fn_body, parse_source};
use xl_ast::{ExprKind, ItemKind, StmtKind};

#[test]
fn multiple_independent_errors_are_reported() {
    let parsed = parse_source("fn a( {} fn b() { let x = ; }");
    let messages = parsed.messages();
    assert!(
        messages.len() >= 2,
        "both items should contribute diagnostics: {messages:?}"
    );
    assert!(messages.iter().any(|m| m == "expected parameter"));
    assert!(messages.iter().any(|m| m == "expected expression"));
    assert_eq!(parsed.output.items.len(), 2, "both items must survive");
}

#[test]
fn error_in_one_item_does_not_kill_later_items() {
    let parsed = parse_source("fn a( {} fn b() -> i64 { 1 } fn c() {}");
    assert_eq!(parsed.output.items.len(), 3);
    let b = find_fn(&parsed.output.items, "b").expect("b parses");
    assert!(b.sig.ret.is_some());
}

#[test]
fn missing_parameter_reports_once_and_reaches_body() {
    let parsed = parse_source("fn f( { }");
    assert_eq!(parsed.messages(), vec!["expected parameter"]);
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn stray_comma_in_parameters() {
    let parsed = parse_source("fn f(,) {}");
    assert_eq!(parsed.messages(), vec!["expected parameter"]);
}

#[test]
fn struct_missing_name_reports_identifier() {
    let parsed = parse_source("struct { }");
    assert_eq!(parsed.messages(), vec!["expected identifier"]);
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn truncated_function_reports_missing_close() {
    let parsed = parse_source("fn f() {");
    assert_eq!(parsed.messages(), vec!["expected '}'"]);
    assert_eq!(parsed.output.items.len(), 1, "item still built");
}

#[test]
fn truncated_expression_reports_expected_expression() {
    let parsed = parse_source("fn f() { let x = ; }");
    assert_eq!(parsed.messages(), vec!["expected expression"]);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(let_stmt.init.span.is_empty(), "placeholder is zero-width");
}

#[test]
fn match_with_empty_arms_parses() {
    let parsed = clean("fn f() { match s { } }");
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("match");
    };
    assert!(arms.is_empty());
}

#[test]
fn match_arm_missing_body_reports_expression() {
    let parsed = parse_source("fn f() { match s { A => } }");
    assert_eq!(parsed.messages(), vec!["expected expression"]);
    let body = fn_body(&parsed.output.items, "f");
    let tail = body.tail.as_ref().expect("tail");
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("match");
    };
    assert_eq!(arms.len(), 1, "arm survives with placeholder body");
}

#[test]
fn illegal_lexical_token_is_preserved_and_skipped() {
    let parsed = parse_source("fn f() { let x = &y; }");
    // The lexer owns the character diagnostic; the parser skips the Error
    // token and parses the following identifier silently.
    assert_eq!(parsed.messages(), vec!["unexpected character '&'"]);
    let body = fn_body(&parsed.output.items, "f");
    let StmtKind::Let(let_stmt) = &body.stmts[0].kind else {
        panic!("let");
    };
    assert!(
        matches!(let_stmt.init.kind, ExprKind::Path(_)),
        "recovered initializer is the surviving identifier"
    );
}

#[test]
fn lexical_and_syntactic_diagnostics_are_both_reported() {
    let parsed = parse_source("fn f() { let x = @; let y = ; }");
    let messages = parsed.messages();
    assert_eq!(messages[0], "unexpected character '@'", "lexical first");
    assert!(messages.contains(&"expected expression".to_string()));
}

#[test]
fn unclosed_expression_recovers_at_delimiter() {
    let parsed = parse_source("fn f() { g(1, ; }");
    let body = fn_body(&parsed.output.items, "f");
    // Recovery leaves the block's `}`, so the recovered call becomes the
    // block tail.
    let expr = match (&body.tail, body.stmts.first()) {
        (Some(tail), _) => tail,
        (None, Some(stmt)) => match &stmt.kind {
            StmtKind::Expr(expr) => expr,
            other => panic!("call expression expected, got {other:?}"),
        },
        other => panic!("call expression expected, got {other:?}"),
    };
    assert!(matches!(expr.kind, ExprKind::Call(..)));
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == "expected expression"),
        "{:?}",
        parsed.messages()
    );
}

#[test]
fn missing_call_closer_reports_once() {
    let parsed = parse_source("fn f() { g(1 }");
    let messages = parsed.messages();
    assert_eq!(messages, vec!["expected ',' or ')'"]);
    // The block's brace survives the argument-list recovery.
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn truncated_struct_body() {
    let parsed = parse_source("struct S { x: i64, ");
    assert_eq!(parsed.messages(), vec!["expected '}'"]);
    let ItemKind::Struct(decl) = &parsed.output.items[0].kind else {
        panic!("struct");
    };
    assert_eq!(decl.fields.len(), 1);
}

#[test]
fn unmatched_close_brace_at_top_level() {
    let parsed = parse_source("} fn f() {}");
    let messages = parsed.messages();
    assert_eq!(messages, vec!["expected item"]);
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn truncated_enum_and_trait() {
    let parsed = parse_source("enum E { A, B(");
    assert!(
        parsed.messages().iter().any(|m| m == "expected type"),
        "{:?}",
        parsed.messages()
    );
    let parsed = parse_source("trait T { fn f(self) }");
    assert_eq!(parsed.messages(), vec!["expected ';'"]);
}

#[test]
fn top_level_let_recovers_to_the_next_item() {
    let parsed = parse_source("let x = 1; fn f() {}");
    let messages = parsed.messages();
    assert_eq!(messages, vec!["expected item"]);
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn recovery_never_fabricates_tail_expressions() {
    let parsed = parse_source("fn f() { g() h() }");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.tail.is_none());
}

#[test]
fn multiple_statement_errors_in_one_block() {
    let parsed = parse_source("fn f() { g() h(); let x = ; i() }");
    let messages = parsed.messages();
    assert!(messages.len() >= 2, "{messages:?}");
    let body = fn_body(&parsed.output.items, "f");
    assert!(body.stmts.len() >= 2);
}

#[test]
fn error_tokens_are_not_expressions() {
    let parsed = parse_source("fn f() { let x = $; }");
    // The lexer owns the character diagnostic; the parser skips the Error
    // token and then reports the genuinely missing expression at `;`.
    assert_eq!(
        parsed.messages(),
        vec!["unexpected character '$'", "expected expression"]
    );
}

#[test]
fn root_level_expression_recovers_to_next_item() {
    let parsed = parse_source("1 + 2 fn f() {}");
    let messages = parsed.messages();
    assert!(
        messages.iter().any(|m| m == "expected item"),
        "{messages:?}"
    );
    assert_eq!(parsed.output.items.len(), 1);
    assert!(matches!(parsed.output.items[0].kind, ItemKind::Fn(_)));
}
