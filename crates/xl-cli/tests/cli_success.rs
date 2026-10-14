//! Success paths: the specification example end-to-end, the accepted
//! Phase 1 corpus, representative feature inputs, determinism, and
//! round-trip interoperability.

mod common;

use common::{EXIT_SUCCESS, TempDir, assert_status, parse_bytes, run_xc, stdout_text};

/// One program exercising every representative input category required for
/// the CLI: Unicode identifiers, numeric literals, compound assignments,
/// functions and methods, structs and enums, generic types, pattern
/// matching, control-flow expressions, and interpolated strings.
const FEATURES: &str = "\
import std.io;

pub fn g<T: Ord, U>(a: T, b: U) -> T { a }

pub struct S<T> { pub x: i64, y: [i64; 4], z: Vec<T>, }

enum E { A, B(i64), C(i64, bool) }

impl P { pub fn m(self, o: Point, x: i64) -> i64 { x } }

struct W<T> { v: T }
impl<T> W<T> { fn get(self) -> T { self.v } }

fn f() {
    let café = 1;
    let δ = café + 1;
    let a = 0xFFFF;
    let b = 0b1010;
    let c = 1_000;
    let d = 1e300;
    let e = 0.5;
    let ch = 'x';
    total += i as f64;
    while a { b(); }
    loop { break; }
    let v = 0..10;
    return;
}

fn nested() -> i64 { if a { 1 } else if b { 2 } else { 3 } }
fn matched(v: i64) -> i64 { match v { 0 => 10, _ => 20, } }
fn interp() { let s = \"value = {compute(1 + 2)}\"; let t = \"esc \\{brace\\}\"; }
";

#[test]
fn spec_example_end_to_end_conformance() {
    // The archived original specification must contain the example
    // byte-for-byte (modulo Git checkout line endings); the example cannot
    // be substituted or shortened.
    let spec = common::normalized(&common::archived_spec());
    let example = common::spec_example();
    assert!(
        spec.contains(&example),
        "docs/spec-v0.1-original.txt must contain examples/program.x byte-for-byte"
    );

    // The example is written to a real temporary `.x` file and loaded
    // through the CLI's actual file-reading path.
    let (dir, output) = parse_bytes("spec-example.x", example.as_bytes());
    assert_status(&output, EXIT_SUCCESS);
    assert!(
        output.stderr.is_empty(),
        "a clean parse must not write to stderr, got:\n{}",
        common::stderr_text(&output)
    );

    // Deterministic canonical AST output, byte-identical to the accepted
    // canonical fixture (which equals the parser suite's canonical
    // snapshot); canonical output uses LF on every platform.
    let stdout = stdout_text(&output);
    assert!(
        !stdout.contains('\r'),
        "canonical output must use LF line endings on every platform"
    );
    assert_eq!(stdout, common::spec_example_canonical());

    // Completeness: every top-level item of the original example appears.
    for marker in [
        "import std.io;",
        "struct Point",
        "impl Point",
        "enum Shape",
        "fn area(s: Shape) -> f64",
        "fn parse(s: str) -> Result<i64, str>",
        "fn main()",
    ] {
        assert!(
            stdout.contains(marker),
            "canonical output is missing {marker:?}"
        );
    }

    // Determinism: a repeated run is byte-identical in every stream.
    let second = run_xc(dir.path(), &["parse", "spec-example.x"]);
    assert_eq!(output.status.code(), second.status.code());
    assert_eq!(output.stdout, second.stdout);
    assert_eq!(output.stderr, second.stderr);

    // Interoperability: re-parsing the printed output succeeds and yields
    // byte-identical canonical output (structural equivalence at the CLI
    // boundary).
    let canonical_text = common::spec_example_canonical();
    let (_canon_dir, canonical) = parse_bytes("spec-canonical.x", canonical_text.as_bytes());
    assert_status(&canonical, EXIT_SUCCESS);
    assert!(canonical.stderr.is_empty());
    assert_eq!(stdout_text(&canonical), canonical_text);
}

#[test]
fn checked_in_example_file_parses_through_the_cli() {
    let root = common::repo_root();
    let output = run_xc(&root, &["parse", "examples/program.x"]);
    assert_status(&output, EXIT_SUCCESS);
    assert!(output.stderr.is_empty());
    let stdout = stdout_text(&output);
    assert!(
        !stdout.contains('\r'),
        "canonical output must use LF line endings on every platform"
    );
    assert_eq!(stdout, common::spec_example_canonical());
}

#[test]
fn zero_byte_input_is_a_valid_empty_module() {
    let (_dir, output) = parse_bytes("empty.x", b"");
    assert_status(&output, EXIT_SUCCESS);
    assert!(
        output.stdout.is_empty(),
        "an empty module has no canonical output (zero items)"
    );
    assert!(
        output.stderr.is_empty(),
        "an empty module produces no diagnostics"
    );
}

#[test]
fn whitespace_only_input_is_a_valid_empty_module() {
    let (_dir, output) = parse_bytes("whitespace.x", b"   \t\n\n  \n");
    assert_status(&output, EXIT_SUCCESS);
    assert!(
        output.stdout.is_empty(),
        "an empty module has no canonical output (zero items)"
    );
    assert!(
        output.stderr.is_empty(),
        "an empty module produces no diagnostics"
    );
}

#[test]
fn comment_only_input_is_a_valid_empty_module() {
    let (_dir, output) = parse_bytes("comments.x", b"// only a comment\n/* and another */\n");
    assert_status(&output, EXIT_SUCCESS);
    assert!(
        output.stdout.is_empty(),
        "an empty module has no canonical output (zero items)"
    );
    assert!(
        output.stderr.is_empty(),
        "an empty module produces no diagnostics"
    );
}

#[test]
fn repeated_runs_are_byte_identical() {
    let source = "fn f() { let café = 1; let s = \"n = {café}\"; }\n";
    let (_dir, first) = parse_bytes("determinism.x", source.as_bytes());
    assert_status(&first, EXIT_SUCCESS);
    for _ in 0..2 {
        let (_dir, again) = parse_bytes("determinism.x", source.as_bytes());
        assert_eq!(first.status.code(), again.status.code());
        assert_eq!(first.stdout, again.stdout);
        assert_eq!(first.stderr, again.stderr);
    }
}

#[test]
fn feature_corpus_parses_and_round_trips_through_the_cli() {
    for (index, source) in common::CORPUS.iter().enumerate() {
        common::assert_roundtrip(&format!("corpus-{index}.x"), source);
    }
}

#[test]
fn representative_features_survive_canonical_printing() {
    let stdout = common::parse_clean("features.x", FEATURES);
    for marker in [
        // Unicode identifiers.
        "let café = 1;",
        "let δ = café + 1;",
        // Numeric literals (normalized: lowercase radix, separators removed).
        "let a = 0xffff;",
        "let b = 0b1010;",
        "let c = 1000;",
        "let d = 1e300;",
        // Compound assignment.
        "total += i as f64;",
        // Methods, structs, enums, generics.
        "pub fn m(self, o: Point, x: i64) -> i64",
        "pub struct S<T>",
        "enum E {",
        "pub fn g<T: Ord, U>(a: T, b: U) -> T",
        // Pattern matching and control flow.
        "match v {",
        "while a {",
        "loop {",
        "return;",
        "0..10",
        // Interpolated strings and escaped braces.
        "\"value = {compute(1 + 2)}\"",
        "esc \\{brace\\}",
    ] {
        assert!(
            stdout.contains(marker),
            "canonical output is missing {marker:?}:\n{stdout}"
        );
    }
}

#[test]
fn paths_with_spaces_and_arbitrary_extensions_are_accepted() {
    let dir = TempDir::new();
    dir.write("my program.txt", b"fn main() { }\n");
    let output = run_xc(dir.path(), &["parse", "my program.txt"]);
    assert_status(&output, EXIT_SUCCESS);
    assert!(output.stderr.is_empty());
    assert_eq!(stdout_text(&output), "fn main() {}\n");
}

#[test]
fn large_valid_input_parses_cleanly() {
    let mut source = String::new();
    for index in 0..500 {
        source.push_str(&format!("fn f{index}() -> i64 {{\n    {index}\n}}\n\n"));
    }
    let stdout = common::parse_clean("large.x", &source);
    assert!(stdout.contains("fn f499() -> i64 {"));
    assert!(stdout.len() > 10_000);
}
