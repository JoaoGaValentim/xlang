//! Suite G — patterns and types.

mod common;

use common::*;
use xl_ast::*;

#[test]
fn wildcard_and_binding_patterns_print() {
    let wildcard = Pat {
        kind: PatKind::Wildcard,
        span: fspan(0, 1),
    };
    assert_eq!(print_pattern(&wildcard).expect("prints"), "_");
    assert_eq!(print_pattern(&binding_pat("r")).expect("prints"), "r");
}

#[test]
fn literal_patterns_print() {
    let int_pat = Pat {
        kind: PatKind::Lit(Lit::Int(IntLit {
            magnitude: 0,
            radix: Radix::Dec,
        })),
        span: fspan(0, 1),
    };
    assert_eq!(print_pattern(&int_pat).expect("prints"), "0");
    let bool_pat = Pat {
        kind: PatKind::Lit(Lit::Bool(true)),
        span: fspan(0, 4),
    };
    assert_eq!(print_pattern(&bool_pat).expect("prints"), "true");
    let text_pat = Pat {
        kind: PatKind::Lit(Lit::Str(StringLit {
            parts: vec![StringPart::Text(TextSegment {
                text: "a".to_string(),
                span: fspan(0, 3),
            })],
            span: fspan(0, 3),
        })),
        span: fspan(0, 3),
    };
    assert_eq!(print_pattern(&text_pat).expect("prints"), "\"a\"");
}

#[test]
fn path_patterns_print_with_and_without_payloads() {
    let mut s = Spans::new();
    let with_args = s.path_pat(&["Shape", "Circle"], vec![binding_pat("r")]);
    assert_eq!(
        print_pattern(&with_args).expect("prints"),
        "Shape::Circle(r)"
    );
    let without_args = s.path_pat(&["None"], Vec::new());
    assert_eq!(print_pattern(&without_args).expect("prints"), "None");
    let two_args = s.path_pat(&["Shape", "Rect"], vec![binding_pat("w"), binding_pat("h")]);
    assert_eq!(
        print_pattern(&two_args).expect("prints"),
        "Shape::Rect(w, h)"
    );
}

#[test]
fn tuple_patterns_print_including_one_element_form() {
    let mut s = Spans::new();
    let pair = {
        let a = s.binding("a");
        let b = s.binding("b");
        let span = cover(a.span, b.span);
        Pat {
            kind: PatKind::Tuple(vec![a, b]),
            span,
        }
    };
    assert_eq!(print_pattern(&pair).expect("prints"), "(a, b)");
    let single = {
        let a = s.binding("a");
        Pat {
            span: a.span,
            kind: PatKind::Tuple(vec![a]),
        }
    };
    assert_eq!(print_pattern(&single).expect("prints"), "(a,)");
    let unit = Pat {
        kind: PatKind::Tuple(Vec::new()),
        span: fspan(0, 2),
    };
    assert_eq!(print_pattern(&unit).expect("prints"), "()");
}

#[test]
fn path_types_with_nested_generics_print() {
    let mut s = Spans::new();
    let inner = {
        let path = s.path_type(&["i64"], None);
        s.ty(TypeKind::Path(path))
    };
    let nested = {
        let path = s.path_type(&["Vec"], Some(vec![inner]));
        s.ty(TypeKind::Path(path))
    };
    let outer = {
        let path = s.path_type(&["Vec"], Some(vec![nested]));
        s.ty(TypeKind::Path(path))
    };
    assert_eq!(print_type(&outer).expect("prints"), "Vec<Vec<i64>>");
}

#[test]
fn function_types_print_with_optional_pieces() {
    let plain = Type {
        kind: TypeKind::Fn {
            params: Vec::new(),
            ret: None,
        },
        span: fspan(0, 4),
    };
    assert_eq!(print_type(&plain).expect("prints"), "fn()");
    let with_params = Type {
        kind: TypeKind::Fn {
            params: vec![ty_path("i64")],
            ret: None,
        },
        span: fspan(0, 7),
    };
    assert_eq!(print_type(&with_params).expect("prints"), "fn(i64)");
    let with_ret = Type {
        kind: TypeKind::Fn {
            params: vec![ty_path("i64")],
            ret: Some(Box::new(ty_path("i64"))),
        },
        span: fspan(0, 14),
    };
    assert_eq!(print_type(&with_ret).expect("prints"), "fn(i64) -> i64");
}

#[test]
fn array_types_print_with_length_expression() {
    let array = Type {
        kind: TypeKind::Array(Box::new(ty_path("i64")), Box::new(lit_int(4))),
        span: fspan(0, 7),
    };
    assert_eq!(print_type(&array).expect("prints"), "[i64; 4]");
}

#[test]
fn print_pattern_roundtrips_via_lexer() {
    let mut s = Spans::new();
    let pat = s.path_pat(&["Shape", "Circle"], vec![binding_pat("r")]);
    let printed = print_pattern(&pat).expect("prints");
    let mut sources = xl_span::SourceMap::new();
    let file = sources
        .add_file("p.x", &printed)
        .expect("registration succeeds");
    let output = xl_lexer::lex_file(&sources, file).expect("registered");
    assert!(output.diagnostics.is_empty());
    assert!(output.tokens.len() > 2);
}
