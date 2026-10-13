//! Suite I: resource safety — nesting limits, bounded recovery, and
//! pathological inputs.

mod common;

use common::{clean, find_fn, fn_body, parse_source};

/// The accepted implementation-safety depth limit (amendment B-4).
const LIMIT_MESSAGE: &str = "expression nesting limit (64) exceeded";

#[test]
fn max_parse_depth_constant_is_64() {
    assert_eq!(xl_parser::MAX_PARSE_DEPTH, 64);
}

#[test]
fn deep_parentheses_are_bounded_and_recover() {
    let depth = 1000;
    let source = format!(
        "fn f() {{ let x = {}1{}; }}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let parsed = parse_source(&source);
    let limit_hits = parsed
        .messages()
        .iter()
        .filter(|message| *message == LIMIT_MESSAGE)
        .count();
    assert_eq!(limit_hits, 1, "{:?}", parsed.messages());
    assert_eq!(parsed.output.items.len(), 1, "parse still terminates");
}

#[test]
fn moderate_nesting_below_the_limit_is_clean() {
    let depth = 40;
    let source = format!(
        "fn f() {{ let x = {}1{}; }}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let parsed = clean(&source);
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn deep_unary_chains_are_bounded() {
    let source = format!("fn f() {{ let x = {}1; }}", "-".repeat(500));
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "{:?}",
        parsed.messages()
    );
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn deep_type_generics_are_bounded() {
    let depth = 200;
    let source = format!(
        "fn f(x: {}i64{}) {{}}",
        "Vec<".repeat(depth),
        ">".repeat(depth)
    );
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "{:?}",
        parsed.messages()
    );
}

#[test]
fn deep_blocks_are_bounded() {
    let depth = 300;
    let source = format!("fn f() {}{}", "{".repeat(depth), "}".repeat(depth));
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "diagnostic count: {}",
        parsed.messages().len()
    );
}

#[test]
fn deep_nested_items_are_bounded() {
    let mut source = String::from("fn f() {");
    let depth = 200;
    for index in 0..depth {
        source.push_str(&format!("fn g{index}() {{"));
    }
    for _ in 0..depth {
        source.push('}');
    }
    source.push('}');
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "diagnostic count: {}",
        parsed.messages().len()
    );
}

#[test]
fn deep_patterns_are_bounded() {
    let depth = 200;
    let source = format!(
        "fn f() {{ let {}a{} = x; }}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "{:?}",
        parsed.messages()
    );
}

#[test]
fn deep_brace_nesting_in_interpolation_is_bounded_by_the_parser() {
    // Plain `{` runs inside a string are block-expression braces inside
    // one interpolation; the parser's depth limit bounds them.
    let depth = 80;
    let source = format!(
        "fn f() {{ let s = \"{}1{}\"; }}",
        "{".repeat(depth),
        "}".repeat(depth)
    );
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "{:?}",
        parsed.messages()
    );
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn deep_embedded_string_nesting_hits_the_lexer_interpolation_limit() {
    // Nesting through embedded strings is what increments the lexer's
    // interpolation depth; > 64 levels produce the lexical limit report.
    let mut inner = String::from("\"1\"");
    for _ in 0..70 {
        inner = format!("\"a {{ {inner} }} b\"");
    }
    let source = format!("fn f() {{ let s = {inner}; }}");
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message.contains("interpolation nesting limit (64) exceeded")),
        "{:?}",
        parsed.messages()
    );
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn long_flat_chains_parse_linearly() {
    let terms = 5000;
    let mut source = String::from("fn f() { let x = a");
    for _ in 0..terms {
        source.push_str(" + a");
    }
    source.push_str("; }");
    let parsed = clean(&source);
    let body = fn_body(&parsed.output.items, "f");
    assert_eq!(body.stmts.len(), 1);
}

#[test]
fn thousands_of_stray_semicolons_terminate() {
    let source = format!("fn f() {{ {} }}", ";".repeat(2000));
    let parsed = parse_source(&source);
    assert_eq!(parsed.output.items.len(), 1);
    assert_eq!(
        parsed
            .messages()
            .iter()
            .filter(|message| *message == "expected expression")
            .count(),
        2000
    );
}

#[test]
fn adversarial_unbalanced_closers_terminate() {
    for source in [
        "fn f() { ".to_string() + &")".repeat(500),
        "fn f() { ".to_string() + &"}".repeat(500),
        "fn f() { ".to_string() + &"]".repeat(500),
        "fn f() { ".to_string() + &"{".repeat(500),
        "fn f() { ".to_string() + &">".repeat(500),
        "fn f( ".to_string() + &",".repeat(500) + " ) {}",
        "".to_string() + &"(".repeat(500),
        "struct S { ".to_string() + &":".repeat(500) + " }",
    ] {
        let parsed = parse_source(&source);
        // Total function: any item count, bounded diagnostics, no panic.
        assert!(
            parsed.output.diagnostics.len() <= 1000,
            "diagnostic explosion for {source:?}"
        );
    }
}

#[test]
fn long_identifier_and_literal_inputs_terminate() {
    let long_ident = "a".repeat(50_000);
    let parsed = parse_source(&format!("fn {long_ident}() {{}}"));
    assert_eq!(parsed.output.items.len(), 1);
    let long_number = format!("fn f() {{ let x = {}1; }}", "0".repeat(10_000));
    let parsed = parse_source(&long_number);
    assert_eq!(parsed.output.items.len(), 1);
}

#[test]
fn deep_match_nesting_is_bounded() {
    let mut source = String::from("fn f() { let v = ");
    for _ in 0..200 {
        source.push_str("match x { _ => ");
    }
    source.push('1');
    for _ in 0..200 {
        source.push_str(" }");
    }
    source.push_str("; }");
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "diagnostic count: {}",
        parsed.messages().len()
    );
}

#[test]
fn deeply_nested_for_expressions_are_bounded() {
    let mut source = String::from("fn f() { ");
    for _ in 0..200 {
        source.push_str("for i in x { ");
    }
    source.push('g');
    for _ in 0..200 {
        source.push_str(" }");
    }
    source.push('}');
    let parsed = parse_source(&source);
    assert!(
        parsed
            .messages()
            .iter()
            .any(|message| message == LIMIT_MESSAGE),
        "diagnostic count: {}",
        parsed.messages().len()
    );
}

#[test]
fn parse_after_recovery_continues_for_later_items() {
    let source = format!(
        "fn broken( {} ) {{}} fn fine() -> i64 {{ 1 }}",
        "(".repeat(300)
    );
    let parsed = parse_source(&source);
    assert!(
        find_fn(&parsed.output.items, "fine").is_some(),
        "later items still parse after deep recovery"
    );
}
