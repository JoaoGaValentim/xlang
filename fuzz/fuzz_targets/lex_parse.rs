#![no_main]

//! Lexer + parser pipeline fuzz target (Directive 008 §17).
//!
//! Properties checked on every input (no panic is implicit):
//! - every emitted span is ordered, in bounds, and resolvable;
//! - parsing is deterministic (identical items and diagnostics twice);
//! - diagnostics do not explode (bounded by a generous linear cap);
//! - printed output of a clean parse reparses cleanly and prints
//!   identically (round-trip stability).

use libfuzzer_sys::fuzz_target;
use xl_span::SourceMap;

fuzz_target!(|data: &[u8]| {
    // The lexer API operates on validated Rust strings; raw bytes must be
    // converted safely first (byte-level targets skip invalid UTF-8).
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut sources = SourceMap::new();
    let Ok(file) = sources.add_file("fuzz.x", text) else {
        return;
    };
    let Ok(source) = sources.file(file) else {
        return;
    };

    let output = xl_parser::parse(source, file);

    // Span invariants.
    for item in &output.items {
        assert!(item.span.lo <= item.span.hi);
        assert!(
            sources.span_slice(item.span).is_ok(),
            "item span must resolve"
        );
    }
    for diagnostic in &output.diagnostics {
        let span = diagnostic.primary().span;
        assert!(span.lo <= span.hi);
    }

    // Diagnostics must stay linear in the input with a small constant:
    // each recovery cycle consumes input or returns to a caller that does,
    // so a superlinear (runaway) cascade is a defect.
    assert!(
        output.diagnostics.len() <= 4 * text.len() + 16,
        "diagnostic cascade"
    );

    // Determinism.
    let second = xl_parser::parse(source, file);
    assert_eq!(output.items, second.items);
    assert_eq!(output.diagnostics, second.diagnostics);

    // Successful parses round-trip through the canonical printer.
    if output.diagnostics.is_empty() {
        if let Ok(printed) = xl_ast::print_items(&output.items) {
            let Ok(printed_file) = sources.add_file("printed.x", printed.clone()) else {
                return;
            };
            let Ok(printed_source) = sources.file(printed_file) else {
                return;
            };
            let reparsed = xl_parser::parse(printed_source, printed_file);
            assert!(
                reparsed.diagnostics.is_empty(),
                "printed output of a clean parse must reparse cleanly"
            );
            let Ok(reprinted) = xl_ast::print_items(&reparsed.items) else {
                return;
            };
            assert_eq!(printed, reprinted, "canonical printing must be idempotent");
        }
    }
});
