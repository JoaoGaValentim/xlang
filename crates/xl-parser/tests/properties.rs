//! Property-based tests: totality, determinism, span validity, and
//! canonical round-tripping of generated programs.

mod common;

use proptest::prelude::*;

/// Arbitrary text strategy (valid UTF-8 `String`s, bounded length).
fn arbitrary_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..250)
        .prop_map(|chars| chars.into_iter().collect::<String>())
}

/// A small structured expression grammar (always syntactically plausible).
fn expression() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        Just("a".to_owned()),
        Just("b".to_owned()),
        Just("42".to_owned()),
        Just("1.5".to_owned()),
        Just("\"s\"".to_owned()),
    ];
    leaf.prop_recursive(4, 64, 4, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(left, right)| format!("({left} + {right})")),
            (inner.clone(), inner.clone()).prop_map(|(left, right)| format!("({left} * {right})")),
            (inner.clone(), inner.clone()).prop_map(|(left, right)| format!("({left} < {right})")),
            (inner.clone(), inner.clone()).prop_map(|(left, right)| format!("({left} && {right})")),
            (inner.clone(), inner.clone()).prop_map(|(left, right)| format!("({left}..{right})")),
            inner.clone().prop_map(|expr| format!("-{expr}")),
            inner.clone().prop_map(|expr| format!("!({expr})")),
            inner.clone().prop_map(|expr| format!("f({expr})")),
            inner.clone().prop_map(|expr| format!("{expr}?")),
            inner.clone().prop_map(|expr| format!("{expr}.x")),
            inner.clone().prop_map(|expr| format!("{expr}[0]")),
            inner.clone().prop_map(|expr| format!("{expr} as i64")),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Parsing arbitrary text is total, deterministic, and produces only
    /// spans inside the file (never inverted, never out of bounds).
    #[test]
    fn arbitrary_text_parses_totally_and_deterministically(source in arbitrary_text()) {
        let first = common::parse_source(&source);
        let second = common::parse_source(&source);
        prop_assert_eq!(&first.output.items, &second.output.items);
        prop_assert_eq!(&first.output.diagnostics, &second.output.diagnostics);

        let len = source.len() as u32;
        for item in &first.output.items {
            prop_assert!(item.span.lo <= item.span.hi, "not inverted");
            prop_assert!(item.span.hi <= len, "within the file");
        }
        for diagnostic in &first.output.diagnostics {
            let span = diagnostic.primary().span;
            prop_assert!(span.lo <= span.hi);
            prop_assert!(span.hi <= len);
        }
    }

    /// Every error-free parse of arbitrary text prints back to a string
    /// that reparses error-free to an equivalent tree.
    #[test]
    fn clean_arbitrary_text_round_trips(source in arbitrary_text()) {
        let first = common::parse_source(&source);
        if first.output.diagnostics.is_empty() {
            let printed = first.printed();
            let second = common::parse_source(&printed);
            prop_assert!(
                second.output.diagnostics.is_empty(),
                "printed output must reparse cleanly: {:?}\nprinted:\n{}",
                second.messages(),
                printed
            );
            common::items_equivalent(&first.output.items, &second.output.items)
                .map_err(TestCaseError::fail)?;
            prop_assert_eq!(printed, second.printed());
        }
    }

    /// Generated expression programs: clean parses round-trip exactly.
    #[test]
    fn generated_expression_programs_round_trip(expr in expression()) {
        let source = format!("fn f() {{ let v = {expr}; }}");
        let first = common::parse_source(&source);
        if first.output.diagnostics.is_empty() {
            let printed = first.printed();
            let second = common::parse_source(&printed);
            prop_assert!(
                second.output.diagnostics.is_empty(),
                "printed output must reparse cleanly: {:?}\nprinted:\n{}",
                second.messages(),
                printed
            );
            common::items_equivalent(&first.output.items, &second.output.items)
                .map_err(TestCaseError::fail)?;
        }
    }

    /// Generated strings with interpolations round-trip their structure.
    #[test]
    fn interpolation_programs_round_trip(expr in expression()) {
        let source = format!("fn f() {{ let s = \"pre {{ {expr} }} post\"; }}");
        let first = common::parse_source(&source);
        if first.output.diagnostics.is_empty() {
            let printed = first.printed();
            let second = common::parse_source(&printed);
            prop_assert!(second.output.diagnostics.is_empty(), "{:?}", second.messages());
            common::items_equivalent(&first.output.items, &second.output.items)
                .map_err(TestCaseError::fail)?;
        }
    }
}
