//! Property-based tests: termination, span validity, ordering, EOF
//! uniqueness, determinism, and structured-string invariants.

mod common;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use xl_lexer::{StrLit, TokenKind};

fn arb_source() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..48).prop_map(|chars| chars.into_iter().collect())
}

/// Checks the structured-string span invariants (lexer-contract §5.5) for
/// one literal. Returns `Some(violation)` when a rule is broken.
fn check_string_invariants(
    lit: &StrLit,
    token_lo: u32,
    token_hi: u32,
    src: &str,
) -> Option<String> {
    let mut cursor = token_lo;
    for part in &lit.parts {
        match part {
            xl_lexer::StrPart::Text(text) => {
                if text.span.lo < cursor || text.span.hi < text.span.lo || text.span.hi > token_hi {
                    return Some(format!("text segment out of order/range: {text:?}"));
                }
                cursor = text.span.hi;
            }
            xl_lexer::StrPart::Interp(interp) => {
                if interp.open.lo < cursor
                    || interp.open.hi > token_hi
                    || interp.close.lo < interp.open.hi
                    || interp.close.hi > token_hi
                    || interp.close.lo > interp.close.hi
                {
                    return Some(format!("interpolation delimiters invalid: {interp:?}"));
                }
                let mut prev_hi = interp.open.hi;
                for token in &interp.tokens {
                    if matches!(token.kind, TokenKind::Eof) {
                        return Some("embedded stream must not contain Eof".to_string());
                    }
                    if token.span.lo < prev_hi || token.span.hi > interp.close.lo {
                        return Some(format!("embedded token outside interior: {:?}", token.span));
                    }
                    if !src.is_char_boundary(token.span.lo as usize)
                        || !src.is_char_boundary(token.span.hi as usize)
                    {
                        return Some(format!("embedded token off boundary: {:?}", token.span));
                    }
                    prev_hi = token.span.hi;
                }
                cursor = interp.close.hi;
            }
        }
    }
    None
}

proptest! {
    #[test]
    fn terminates_with_valid_spans_and_unique_eof(source in arb_source()) {
        let (_, output) = common::lex(&source);
        let len = source.len() as u32;
        prop_assert!(!output.tokens.is_empty());
        let eofs = output
            .tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Eof))
            .count();
        prop_assert_eq!(eofs, 1);
        let last = output.tokens.last().expect("non-empty");
        prop_assert!(matches!(last.kind, TokenKind::Eof));
        prop_assert_eq!(last.span.lo, len);
        prop_assert_eq!(last.span.hi, len);
        for token in &output.tokens {
            prop_assert!(token.span.lo <= token.span.hi);
            prop_assert!(token.span.hi <= len);
            prop_assert!(source.is_char_boundary(token.span.lo as usize));
            prop_assert!(source.is_char_boundary(token.span.hi as usize));
        }
    }

    #[test]
    fn top_level_spans_are_nonoverlapping_and_monotonic(source in arb_source()) {
        let (_, output) = common::lex(&source);
        let mut prev_hi = 0u32;
        for token in &output.tokens {
            prop_assert!(
                token.span.lo >= prev_hi,
                "overlapping token {:?} after {prev_hi}",
                token.span
            );
            prev_hi = token.span.hi;
        }
    }

    #[test]
    fn lexing_is_deterministic(source in arb_source()) {
        let (_, first) = common::lex(&source);
        let (_, second) = common::lex(&source);
        prop_assert_eq!(first.tokens, second.tokens);
        prop_assert_eq!(first.diagnostics, second.diagnostics);
    }

    #[test]
    fn structured_string_invariants_hold(source in arb_source()) {
        let (_, output) = common::lex(&source);
        for token in &output.tokens {
            if let TokenKind::Str(lit) = &token.kind
                && let Some(violation) =
                    check_string_invariants(lit, token.span.lo, token.span.hi, &source)
            {
                return Err(TestCaseError::fail(violation));
            }
        }
    }
}
