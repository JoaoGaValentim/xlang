//! Suite J — the specification example program: construction, canonical
//! output, lexical round-trip, and shape equivalence with the original.
//!
//! Parser-based `parse(print(ast))` round-tripping cannot be executed yet
//! (no `xl-parser` exists); it is a mandatory future integration gate.
//! Everything provable today is executed here: the printed source must
//! lex with zero diagnostics and preserve the original example's token
//! structure (modulo canonical trailing-comma elision).

mod common;

use common::*;
use xl_ast::*;
use xl_span::SourceMap;

/// The specification example, verbatim.
const ORIGINAL: &str = "\
import std.io;

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
    if n < 0 { Err(\"negativo\") } else { Ok(n) }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 { total += i as f64; }
    io.println(\"total = {total}\");
}
";

/// The expected canonical printing of the example AST.
const CANONICAL: &str = "\
import std.io;

struct Point {
    x: f64,
    y: f64
}

impl Point {
    pub fn dist(self, o: Point) -> f64 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape {
    Circle(f64),
    Rect(f64, f64)
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rect(w, h) => w * h
    }
}

fn parse(s: str) -> Result<i64, str> {
    let n = s.to_int()?;
    if n < 0 {
        Err(\"negativo\")
    } else {
        Ok(n)
    }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 {
        total += i as f64;
    }
    io.println(\"total = {total}\");
}";

fn lex(source: &str) -> xl_lexer::LexOutput {
    let mut sources = SourceMap::new();
    let file = sources
        .add_file("example.x", source)
        .expect("registration succeeds");
    xl_lexer::lex_file(&sources, file).expect("registered")
}

/// Span-free token shape used for structural comparison.
fn shape(kind: &xl_lexer::TokenKind) -> String {
    use xl_lexer::TokenKind as K;
    match kind {
        K::Ident(text) => format!("Ident({text})"),
        K::Int(int) => format!("Int({},{:?})", int.magnitude, int.radix),
        K::Float(value) => format!("Float({value:?})"),
        K::Char(value) => format!("Char({value:?})"),
        K::Str(lit) => {
            let parts: Vec<String> = lit
                .parts
                .iter()
                .map(|part| match part {
                    xl_lexer::StrPart::Text(text) => format!("Text({:?})", text.text),
                    xl_lexer::StrPart::Interp(interp) => {
                        let inner: Vec<String> =
                            interp.tokens.iter().map(|t| shape(&t.kind)).collect();
                        format!("Expr[{}]", inner.join(","))
                    }
                })
                .collect();
            format!("Str({})", parts.join("|"))
        }
        other => format!("{other:?}"),
    }
}

fn shapes(source: &str) -> Vec<String> {
    lex(source).tokens.iter().map(|t| shape(&t.kind)).collect()
}

/// Removes `,` shapes that immediately precede a closing delimiter
/// (canonical printing elides trailing commas).
fn without_trailing_commas(shapes: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < shapes.len() {
        let item = shapes[index].as_str();
        let next = shapes.get(index + 1).map(String::as_str);
        let comma_before_closer =
            item == "Comma" && matches!(next, Some("RBrace" | "RParen" | "RBracket"));
        if !comma_before_closer {
            out.push(shapes[index].clone());
        }
        index += 1;
    }
    out
}

#[test]
fn example_prints_to_the_canonical_source() {
    let items = example_program();
    assert_eq!(print_items(&items).expect("prints"), CANONICAL);
}

#[test]
fn canonical_output_has_no_lexical_diagnostics() {
    let output = lex(CANONICAL);
    let messages: Vec<&str> = output.diagnostics.iter().map(|d| d.message()).collect();
    assert!(messages.is_empty(), "unexpected diagnostics: {messages:?}");
}

#[test]
fn canonical_output_preserves_the_original_token_structure() {
    let original = without_trailing_commas(&shapes(ORIGINAL));
    let printed = without_trailing_commas(&shapes(CANONICAL));
    assert_eq!(original, printed);
}

#[test]
fn the_original_source_and_its_ast_have_matching_program_structure() {
    // The fixture has exactly the seven items of the original example.
    let items = example_program();
    assert_eq!(items.len(), 7);
    assert!(matches!(items[0].kind, ItemKind::Import(_)));
    assert!(matches!(items[1].kind, ItemKind::Struct(_)));
    assert!(matches!(items[2].kind, ItemKind::Impl(_)));
    assert!(matches!(items[3].kind, ItemKind::Enum(_)));
    assert!(matches!(items[4].kind, ItemKind::Fn(_)));
    assert!(matches!(items[5].kind, ItemKind::Fn(_)));
    assert!(matches!(items[6].kind, ItemKind::Fn(_)));
}

#[test]
fn printed_expression_fragments_lex_cleanly() {
    // A few parse-independent fragments from the example, printed
    // standalone (not valid statements, but valid token sequences).
    let fragments = [
        bin(BinOp::Add, var("a"), bin(BinOp::Mul, var("b"), var("c"))),
        try_expr(call(field(var("s"), "to_int"), Vec::new())),
        assign(
            AssignOp::AddAssign,
            var("total"),
            cast(var("i"), ty_path("f64")),
        ),
        range(lit_int(0), lit_int(10)),
    ];
    for expr in fragments {
        let printed = print_expr(&expr).expect("prints");
        let output = lex(&printed);
        assert!(
            output.diagnostics.is_empty(),
            "fragment {printed:?} must lex cleanly"
        );
    }
}
