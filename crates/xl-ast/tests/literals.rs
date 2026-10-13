//! Suite C — literal representation: integers, floats, characters, and
//! structured strings, including lexical round-trips of printed output.

mod common;

use common::*;
use xl_ast::*;
use xl_span::SourceMap;

/// Lexes `source`, returning (kinds, diagnostic count).
fn lex_info(source: &str) -> (Vec<xl_lexer::TokenKind>, usize) {
    let mut sources = SourceMap::new();
    let file = sources
        .add_file("printed.x", source)
        .expect("registration succeeds");
    let output = xl_lexer::lex_file(&sources, file).expect("registered");
    (
        output.tokens.into_iter().map(|t| t.kind).collect(),
        output.diagnostics.len(),
    )
}

fn int_expr(magnitude: u64, radix: Radix) -> Expr {
    ex(
        fspan(0, 1),
        ExprKind::Lit(Lit::Int(IntLit { magnitude, radix })),
    )
}

#[test]
fn integer_magnitudes_print_and_round_trip() {
    let cases = [
        (0_u64, Radix::Dec, "0"),
        (1, Radix::Dec, "1"),
        (1_000, Radix::Dec, "1000"),
        (9_223_372_036_854_775_808, Radix::Dec, "9223372036854775808"),
        (u64::MAX, Radix::Dec, "18446744073709551615"),
        (255, Radix::Hex, "0xff"),
        (65_535, Radix::Hex, "0xffff"),
        (10, Radix::Bin, "0b1010"),
    ];
    for (magnitude, radix, expected) in cases {
        let printed = print_expr(&int_expr(magnitude, radix)).expect("prints");
        assert_eq!(printed, expected);
        let (kinds, diagnostics) = lex_info(&printed);
        assert_eq!(diagnostics, 0, "printed {printed:?} must lex cleanly");
        assert_eq!(
            kinds,
            vec![
                xl_lexer::TokenKind::Int(xl_lexer::IntLit {
                    magnitude,
                    radix: lex_radix(radix)
                }),
                xl_lexer::TokenKind::Eof,
            ]
        );
    }
}

fn lex_radix(radix: Radix) -> xl_lexer::Radix {
    match radix {
        Radix::Dec => xl_lexer::Radix::Dec,
        Radix::Hex => xl_lexer::Radix::Hex,
        Radix::Bin => xl_lexer::Radix::Bin,
    }
}

#[test]
fn finite_floats_print_in_lexer_valid_notation() {
    let pi_like: f64 = "3.14159".parse().expect("literal parses");
    let cases = [
        (0.0_f64, "0.0"),
        (-0.0_f64, "-0.0"),
        (1.5, "1.5"),
        (0.1, "0.1"),
        (pi_like, "3.14159"),
        (1e300, "1e300"),
        (1e-10, "1e-10"),
    ];
    for (value, expected) in cases {
        let printed = print_expr(&lit_float(value)).expect("prints");
        assert_eq!(printed, expected);
        let (kinds, diagnostics) = lex_info(&printed);
        assert_eq!(diagnostics, 0, "printed {printed:?} must lex cleanly");
        if value == 0.0 && value.is_sign_negative() {
            // `-0.0` lexes as unary minus applied to `0.0`.
            assert!(matches!(kinds.first(), Some(xl_lexer::TokenKind::Minus)));
        } else {
            assert!(
                matches!(kinds.first(), Some(xl_lexer::TokenKind::Float(v)) if *v == value),
                "printed {printed:?} must lex back to the same value"
            );
        }
    }
}

#[test]
fn non_finite_floats_are_structured_errors() {
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let err = print_expr(&lit_float(value)).expect_err("must not print");
        assert_eq!(err.kind(), PrintErrorKind::NonFiniteFloat);
        assert!(!err.message().is_empty());
    }
}

#[test]
fn characters_print_with_escapes() {
    let cases = [
        ('a', "'a'"),
        ('é', "'é'"),
        ('\n', "'\\n'"),
        ('\t', "'\\t'"),
        ('\'', "'\\''"),
        ('\\', "'\\\\'"),
        ('\0', "'\\0'"),
        ('\u{7}', "'\\u{7}'"),
    ];
    for (value, expected) in cases {
        let expr = ex(fspan(0, 3), ExprKind::Lit(Lit::Char(value)));
        let printed = print_expr(&expr).expect("prints");
        assert_eq!(printed, expected);
        let char_literal = printed
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .expect("quoted");
        let (kinds, diagnostics) = lex_info(&printed);
        assert_eq!(diagnostics, 0, "printed {printed:?} must lex cleanly");
        assert!(
            matches!(kinds.first(), Some(xl_lexer::TokenKind::Char(c)) if *c == value),
            "printed {char_literal} must lex back to {value:?}"
        );
    }
}

fn string_lit(parts: Vec<(String, xl_span::Span)>, span: xl_span::Span) -> Expr {
    ex(
        span,
        ExprKind::Lit(Lit::Str(StringLit {
            parts: parts
                .into_iter()
                .map(|(text, span)| StringPart::Text(TextSegment { text, span }))
                .collect(),
            span,
        })),
    )
}

#[test]
fn string_text_is_re_escaped_so_braces_never_become_interpolation() {
    let expr = string_lit(
        vec![("a {b} \"c\" \\ d\n".to_string(), fspan(0, 1))],
        fspan(0, 1),
    );
    let printed = print_expr(&expr).expect("prints");
    assert_eq!(printed, "\"a \\{b\\} \\\"c\\\" \\\\ d\\n\"");
    let (kinds, diagnostics) = lex_info(&printed);
    assert_eq!(diagnostics, 0);
    let xl_lexer::TokenKind::Str(lit) = &kinds[0] else {
        panic!("expected a string token");
    };
    // One decoded text part equal to the original text: no interpolation
    // was created by printing.
    assert_eq!(lit.parts.len(), 1);
    let xl_lexer::StrPart::Text(segment) = &lit.parts[0] else {
        panic!("expected a text segment");
    };
    assert_eq!(segment.text, "a {b} \"c\" \\ d\n");
}

#[test]
fn interpolation_parts_print_and_round_trip() {
    let mut s = Spans::new();
    let inner = s.path_expr(&["total"]);
    let expr = s.string_interp("total = ", inner);
    let printed = print_expr(&expr).expect("prints");
    assert_eq!(printed, "\"total = {total}\"");
    let (kinds, diagnostics) = lex_info(&printed);
    assert_eq!(diagnostics, 0);
    let xl_lexer::TokenKind::Str(lit) = &kinds[0] else {
        panic!("expected a string token");
    };
    assert_eq!(lit.parts.len(), 2);
}

#[test]
fn nested_interpolation_strings_print() {
    let mut s = Spans::new();
    let inner_struct = s.string_expr("");
    let nested = s.string_interp("n: ", inner_struct);
    let outer = s.string_interp("v = ", nested);
    let printed = print_expr(&outer).expect("prints");
    // The nested string is printed as an embedded expression; its quotes
    // are tokens of the interpolation, not delimiters of the outer string.
    assert_eq!(printed, "\"v = {\"n: {\"\"}\"}\"");
    let (_, diagnostics) = lex_info(&printed);
    assert_eq!(diagnostics, 0);
}

#[test]
fn empty_string_prints_as_two_quotes() {
    let expr = StringLit {
        parts: Vec::new(),
        span: fspan(0, 2),
    };
    let expr = ex(fspan(0, 2), ExprKind::Lit(Lit::Str(expr)));
    assert_eq!(print_expr(&expr).expect("prints"), "\"\"");
}
