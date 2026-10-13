//! Suite J: determinism — the same source always yields the same items and
//! diagnostics.

mod common;

use common::{SPEC_EXAMPLE, clean, parse_source};

#[test]
fn identical_sources_yield_identical_ast_and_diagnostics() {
    for source in [
        SPEC_EXAMPLE,
        "fn f() { let x = ; let y = &; }",
        "fn f( { } fn g() -> i64 { 1 }",
        "match nope",
        "fn f() { let s = \"a { b\"; }",
    ] {
        let first = parse_source(source);
        let second = parse_source(source);
        assert_eq!(
            first.output.items, second.output.items,
            "ASTs must be deterministic for {source:?}"
        );
        assert_eq!(
            first.output.diagnostics, second.output.diagnostics,
            "diagnostics must be deterministic for {source:?}"
        );
    }
}

#[test]
fn canonical_printing_is_deterministic() {
    let source = SPEC_EXAMPLE;
    let a = clean(source).printed();
    let b = clean(source).printed();
    assert_eq!(a, b);
}

#[test]
fn unicode_identifiers_are_deterministic() {
    let source = "fn f() { let café = 1; let δ = café + 1; }";
    let first = parse_source(source);
    let second = parse_source(source);
    assert_eq!(first.output.items, second.output.items);
    assert!(first.output.diagnostics.is_empty());
}

#[test]
fn diagnostic_rendering_is_deterministic() {
    let source = "fn f() { let x = ; }";
    let a = parse_source(source);
    let b = parse_source(source);
    assert_eq!(a.rendered(), b.rendered());
    assert!(!a.rendered().is_empty());
}

#[test]
fn whitespace_and_comment_trivia_do_not_affect_structure() {
    let plain = clean("fn f(){let x=1;}");
    let spaced = clean("fn f() {\n    // comment\n    let x = 1; /* tail */\n}\n");
    // Spans necessarily differ; the structure must not.
    common::items_equivalent(&plain.output.items, &spaced.output.items)
        .expect("trivia must not affect structure");
}
