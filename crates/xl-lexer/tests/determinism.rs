//! Suite H — determinism: repeated lexing yields identical results.

mod common;

use common::{describe, lex};

const CORPUS: &[&str] = &[
    "",
    "fn main() { let mut total = 0.0; for i in 0..10 { total += i as f64; } }",
    "import std.io; struct Point { x: f64, y: f64 }",
    "\"total = {total}\"",
    "\"nested {if flag { 1 } else { 2 }}\"",
    "0xFF 0b1010 1_000 3.14159 1e5 'x' '\\n'",
    "/* a /* b */ c */ // line",
    "~ ~ ~ unterminated \"",
    "a>=b a+=b a==b a=>b 1..2",
];

#[test]
fn repeated_lexing_is_identical() {
    for source in CORPUS {
        let (_, first) = lex(source);
        let (_, second) = lex(source);
        assert_eq!(
            first, second,
            "source {source:?} must lex deterministically"
        );
    }
}

#[test]
fn token_renderings_are_stable() {
    for source in CORPUS {
        let (_, first) = lex(source);
        let (_, second) = lex(source);
        let first_render: Vec<String> = first.tokens.iter().map(|t| describe(&t.kind)).collect();
        let second_render: Vec<String> = second.tokens.iter().map(|t| describe(&t.kind)).collect();
        assert_eq!(first_render, second_render, "source {source:?}");
    }
}
