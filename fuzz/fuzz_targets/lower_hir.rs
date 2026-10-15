#![no_main]

//! Lexer + parser + structural HIR lowering fuzz target (Directive 011 §15).
//!
//! Pipeline: arbitrary bytes → lex → parse → structural lowering → validate.
//!
//! Properties checked on every input (no panic and no stack overflow are
//! implicit — libFuzzer catches both):
//! - a completed lowering always passes the structural validator (unique
//!   occurrence keys, valid arena references, origin correspondence, ...);
//! - resource failures are explicit controlled aborts, never panics, and an
//!   aborted output is never claimed complete;
//! - lowering is deterministic: identical inputs produce identical modules,
//!   fates, and diagnostics.

use libfuzzer_sys::fuzz_target;
use xl_hir::{lower_structure, validate, validate_resolutions, ModuleId, PackageId, StageFate};
use xl_span::SourceMap;

fuzz_target!(|data: &[u8]| {
    // The lexer requires UTF-8. Replace malformed byte sequences to retain
    // coverage from arbitrary libFuzzer inputs; the CLI separately rejects
    // invalid UTF-8 rather than interpreting this replacement as source.
    let decoded = String::from_utf8_lossy(data);
    let text = decoded.as_ref();
    let mut sources = SourceMap::new();
    let Ok(file) = sources.add_file("fuzz.x", text) else {
        return;
    };
    let Ok(source) = sources.file(file) else {
        return;
    };

    let parsed = xl_parser::parse(source, file);
    let module = ModuleId::from_index(0);
    let package = PackageId::from_index(0);

    let first = lower_structure(&parsed.items, file, module, package);

    // A completed lowering must satisfy every structural invariant; an
    // aborted one must carry an explicit reason (never a silent failure).
    match first.fate {
        StageFate::Complete => {
            let violations = validate(&first.module);
            assert!(
                violations.is_empty(),
                "completed lowering must validate: {violations:?}"
            );
            let resolutions = validate_resolutions(&first.module, &first.resolutions);
            assert!(resolutions.is_empty(), "resolution side table must validate: {resolutions:?}");
        }
        StageFate::Aborted { .. } => {}
    }

    // Determinism: identical inputs produce identical outputs.
    let second = lower_structure(&parsed.items, file, module, package);
    assert_eq!(first.module, second.module, "lowering must be deterministic");
    assert_eq!(first.fate, second.fate, "fate must be deterministic");
    assert_eq!(first.resolutions, second.resolutions, "resolutions must be deterministic");
    assert_eq!(
        first.diagnostics, second.diagnostics,
        "diagnostics must be deterministic"
    );
});
