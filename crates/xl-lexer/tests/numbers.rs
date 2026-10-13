//! Suite C — numeric literals: valid forms, boundaries, and munch rules.

mod common;

use common::{kinds, lex};
use xl_lexer::{IntLit, Radix, TokenKind};

fn int(magnitude: u64, radix: Radix) -> TokenKind {
    TokenKind::Int(IntLit { magnitude, radix })
}

#[test]
fn mandated_integer_cases() {
    assert_eq!(kinds("0"), [int(0, Radix::Dec), TokenKind::Eof]);
    assert_eq!(kinds("1_000"), [int(1000, Radix::Dec), TokenKind::Eof]);
    assert_eq!(kinds("0xFF"), [int(255, Radix::Hex), TokenKind::Eof]);
    assert_eq!(kinds("0b1010"), [int(10, Radix::Bin), TokenKind::Eof]);
    // i64::MIN's magnitude fits u64 and must stay lexable (amendment B-2).
    assert_eq!(
        kinds("9223372036854775808"),
        [int(9_223_372_036_854_775_808, Radix::Dec), TokenKind::Eof]
    );
    assert_eq!(
        kinds("18446744073709551615"),
        [int(u64::MAX, Radix::Dec), TokenKind::Eof]
    );
}

#[test]
fn magnitude_above_u64_is_rejected() {
    let (_, output) = lex("18446744073709551616");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(
        output.diagnostics[0].message(),
        "integer literal is too large"
    );
    let span = output.diagnostics[0].primary().span;
    assert_eq!((span.lo, span.hi), (0, 20));
}

#[test]
fn leading_zeros_are_allowed() {
    assert_eq!(kinds("007"), [int(7, Radix::Dec), TokenKind::Eof]);
}

#[test]
fn underscores_must_appear_between_digits() {
    for source in ["1_", "1__0", "0x_1", "0b1_"] {
        let (_, output) = lex(source);
        assert_eq!(
            output.tokens[0].kind,
            TokenKind::Error,
            "source {source:?} must be rejected"
        );
        assert_eq!(output.diagnostics.len(), 1, "source {source:?}");
        assert_eq!(
            output.diagnostics[0].message(),
            "'_' must appear between digits",
            "source {source:?}"
        );
        // The primary span points at the first offending underscore.
        let span = output.diagnostics[0].primary().span;
        assert_eq!(span.hi - span.lo, 1, "source {source:?}");
        assert_eq!(
            &source[span.lo as usize..span.hi as usize],
            "_",
            "source {source:?}"
        );
    }
}

#[test]
fn underscore_separated_radix_digits() {
    assert_eq!(kinds("0xFF_FF"), [int(65535, Radix::Hex), TokenKind::Eof]);
    assert_eq!(kinds("0b1010_0101"), [int(165, Radix::Bin), TokenKind::Eof]);
}

#[test]
fn malformed_radix_prefixes() {
    let (_, output) = lex("0x");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(
        output.diagnostics[0].message(),
        "expected digits after '0x'"
    );

    let (_, output) = lex("0b2");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(
        output.diagnostics[0].message(),
        "expected digits after '0b'"
    );
    // The literal `2` after the failed prefix is still lexed.
    assert_eq!(output.tokens[1].kind, int(2, Radix::Dec));
}

#[test]
fn mandated_float_cases() {
    assert_eq!(kinds("0.0"), [TokenKind::Float(0.0), TokenKind::Eof]);
    assert_eq!(
        kinds("3.14159"),
        [
            TokenKind::Float("3.14159".parse::<f64>().expect("literal parses")),
            TokenKind::Eof,
        ]
    );
    assert_eq!(kinds("1.5e3"), [TokenKind::Float(1500.0), TokenKind::Eof]);
    assert_eq!(kinds("1e5"), [TokenKind::Float(100000.0), TokenKind::Eof]);
    assert_eq!(kinds("1e300"), [TokenKind::Float(1e300), TokenKind::Eof]);
}

#[test]
fn float_overflow_is_rejected_and_underflow_rounds_to_zero() {
    let (_, output) = lex("1e9999");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(
        output.diagnostics[0].message(),
        "floating-point literal is not representable"
    );

    assert_eq!(kinds("1e-9999"), [TokenKind::Float(0.0), TokenKind::Eof]);
    let (_, output) = lex("1e-9999");
    assert!(output.diagnostics.is_empty());
}

#[test]
fn exponent_requires_digits() {
    assert_eq!(
        kinds("1e"),
        [
            int(1, Radix::Dec),
            TokenKind::Ident("e".to_string()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("1e+"),
        [
            int(1, Radix::Dec),
            TokenKind::Ident("e".to_string()),
            TokenKind::Plus,
            TokenKind::Eof,
        ]
    );
    assert_eq!(kinds("1e+5"), [TokenKind::Float(100000.0), TokenKind::Eof]);
    assert_eq!(kinds("1E5"), [TokenKind::Float(100000.0), TokenKind::Eof]);
}

#[test]
fn munch_rules_for_dot_range_and_field_access() {
    // `1..2` is a range, never a float.
    assert_eq!(
        kinds("1..2"),
        [
            int(1, Radix::Dec),
            TokenKind::DotDot,
            int(2, Radix::Dec),
            TokenKind::Eof,
        ]
    );
    // `1.5` is a float.
    assert_eq!(kinds("1.5"), [TokenKind::Float(1.5), TokenKind::Eof]);
    // `1.foo` is `1`, `.`, `foo` (field access on a literal).
    assert_eq!(
        kinds("1.foo"),
        [
            int(1, Radix::Dec),
            TokenKind::Dot,
            TokenKind::Ident("foo".to_string()),
            TokenKind::Eof,
        ]
    );
    // `x.0` is `x`, `.`, `0`.
    assert_eq!(
        kinds("x.0"),
        [
            TokenKind::Ident("x".to_string()),
            TokenKind::Dot,
            int(0, Radix::Dec),
            TokenKind::Eof,
        ]
    );
    // `1.e3` is not a float: field access `e3` on `1`.
    assert_eq!(
        kinds("1.e3"),
        [
            int(1, Radix::Dec),
            TokenKind::Dot,
            TokenKind::Ident("e3".to_string()),
            TokenKind::Eof,
        ]
    );
    // `.5` never starts a float.
    assert_eq!(
        kinds(".5"),
        [TokenKind::Dot, int(5, Radix::Dec), TokenKind::Eof]
    );
    // `1.` is an int followed by a dot.
    assert_eq!(
        kinds("1."),
        [int(1, Radix::Dec), TokenKind::Dot, TokenKind::Eof]
    );
}

#[test]
fn number_with_trailing_ident_splits_into_two_tokens() {
    // No suffix support: `1u32` is an int followed by an identifier.
    assert_eq!(
        kinds("1u32"),
        [
            int(1, Radix::Dec),
            TokenKind::Ident("u32".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn float_with_misplaced_underscore_is_rejected() {
    let (_, output) = lex("1.0e1_");
    assert_eq!(output.tokens[0].kind, TokenKind::Error);
    assert_eq!(
        output.diagnostics[0].message(),
        "'_' must appear between digits"
    );
}
