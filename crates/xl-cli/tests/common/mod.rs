//! Shared helpers for the `xl-cli` end-to-end test suites.
//!
//! Every suite invokes the real `xc` binary built by Cargo for this crate
//! (`CARGO_BIN_EXE_xc`), writes real `.x` files into per-test temporary
//! directories, and compares captured stdout/stderr/exit status
//! byte-for-byte. No file is ever produced outside the operating system's
//! temporary directory, and no external test dependency is used.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// Exit status for a successful parse with no error-level diagnostics.
pub const EXIT_SUCCESS: i32 = 0;
/// Exit status for lexical or syntactic source errors.
pub const EXIT_SOURCE_ERRORS: i32 = 1;
/// Exit status for usage, I/O, and internal errors (CLI policy).
pub const EXIT_OPERATIONAL: i32 = 2;

/// The complete original specification example: the exact bytes of
/// `examples/program.x`. The conformance suites require these bytes to
/// appear unmodified inside `docs/spec-v0.1-original.txt` §3, so the
/// example cannot be substituted or shortened silently.
///
/// Git may check text files out with CRLF on Windows even though the
/// committed blobs use LF; use [`spec_example`] for byte-level comparisons
/// against CLI output.
pub const SPEC_EXAMPLE: &str = include_str!("../../../../examples/program.x");

/// The canonical printer output for [`SPEC_EXAMPLE`], byte-for-byte; the
/// fixture is identical to the accepted `xl-parser` canonical snapshot
/// (`snapshots__spec_example_canonical.snap`).
///
/// The same checkout note as [`SPEC_EXAMPLE`] applies; use
/// [`spec_example_canonical`] for byte-level comparisons against CLI
/// output.
pub const SPEC_EXAMPLE_CANONICAL: &str = include_str!("../fixtures/spec_example_canonical.x");

/// Normalizes checkout line endings to `\n`.
///
/// File content embedded with `include_str!` and the archived
/// specification are read from the working tree, which Git may materialize
/// with CRLF line endings on Windows; the committed blobs use LF. CLI
/// output, by contrast, must use LF on every platform, which the suites
/// assert explicitly before comparing normalized content.
pub fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// The specification example with checkout line endings normalized to
/// `\n`.
pub fn spec_example() -> String {
    normalized(SPEC_EXAMPLE)
}

/// The canonical fixture with checkout line endings normalized to `\n`.
pub fn spec_example_canonical() -> String {
    normalized(SPEC_EXAMPLE_CANONICAL)
}

/// Accepted Phase 1 inputs exercised end-to-end through the real binary:
/// every representative category required for the CLI (Unicode
/// identifiers, numeric literals, compound assignments, functions and
/// methods, structs and enums, generic types, pattern matching,
/// control-flow expressions, interpolated strings) plus the accepted
/// parser round-trip corpus subset.
pub const CORPUS: &[&str] = &[
    // Unicode identifiers.
    "fn f() { let café = 1; let δ = café + 1; }\n",
    // Integer literals (radix and separators) and boundary values.
    "fn f() { let a = 0xFFFF; let b = 0b1010; let c = 1_000; let d = 9223372036854775808; let e = 18446744073709551615; }\n",
    // Float literals.
    "fn f() { let a = 1e300; let b = 0.5; let c = -0.0; let d = 1e-9999; }\n",
    // Compound assignments.
    "fn f() { x += 1; x -= 1; x *= 2; x /= 2; x %= 2; }\n",
    // Functions and methods.
    "impl P { pub fn m(self, o: Point, x: i64) -> i64 { x } }\n",
    "fn outer() { fn inner() -> i64 { 1 } struct S { x: i64 } }\n",
    // Structs and enums.
    "pub struct S<T> { pub x: i64, y: [i64; 4], z: Vec<T>, }\n",
    "enum E { A, B(i64), C(i64, bool) }\n",
    // Generic types and bounds.
    "fn f(x: Vec<Vec<i64>>) -> Result<i64, str> { x }\n",
    "pub fn g<T: Ord, U>(a: T, b: U) -> T { a }\n",
    "struct W<T> { v: T }\nimpl<T> W<T> { fn get(self) -> T { self.v } }\n",
    // Pattern matching.
    "fn f() -> i64 { match v { 0 => 10, _ => 20, } }\n",
    "fn f() { let (a, b) = pair; }\n",
    // Control-flow expressions.
    "fn f() -> i64 { if a { 1 } else if b { 2 } else { 3 } }\n",
    "fn f() { while a { b(); } loop { break; } continue; }\n",
    "fn f() { for i in 0..10 { p(i); } finish(); }\n",
    "fn f() { return; } fn g() -> i64 { return n < 0 }\n",
    // Interpolated strings, escapes, and nested interpolation.
    "fn f() { let s = \"value = {compute(1 + 2)}\"; let t = \"esc \\{brace\\}\"; }\n",
    "fn f() { let s = \"a { \"b {c} d\" } e\"; }\n",
    // Traits and impls.
    "trait Show { fn show(self) -> str; } impl Show for Point { pub fn show(self) -> str { \"s\" } }\n",
    // Postfix chains, casts, and paths.
    "fn f() { let v = s.to_int()? as f64; }\n",
    "fn f() { let v = a::b::c; }\n",
    "fn f() { let x = (a < b) < c; }\n",
];

/// The path of the `xc` binary under test.
pub fn xc_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xc")
}

/// The repository root (this crate lives at `<root>/crates/xl-cli`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives under crates/")
        .to_path_buf()
}

/// Reads the archived original specification (`docs/spec-v0.1-original.txt`).
pub fn archived_spec() -> String {
    let path = repo_root().join("docs/spec-v0.1-original.txt");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// A self-cleaning temporary directory with a unique name.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates a fresh directory under the system temporary directory.
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "xlang-cli-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("create temporary directory");
        Self { path }
    }

    /// Returns the directory path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `bytes` to `name` inside the directory and returns its path.
    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, bytes).expect("write temporary file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Runs `xc` with `args` (relative to `cwd`) and captures the result.
pub fn run_xc(cwd: &Path, args: &[&str]) -> Output {
    Command::new(xc_bin())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run the xc binary")
}

/// Writes `bytes` as `name` in a fresh directory and parses it with `xc`.
pub fn parse_bytes(name: &str, bytes: &[u8]) -> (TempDir, Output) {
    let dir = TempDir::new();
    dir.write(name, bytes);
    let output = run_xc(dir.path(), &["parse", name]);
    (dir, output)
}

/// Parses a UTF-8 source expected to succeed and returns its canonical
/// stdout. Asserts exit status `0` and an empty stderr.
pub fn parse_clean(name: &str, source: &str) -> String {
    let (_dir, output) = parse_bytes(name, source.as_bytes());
    assert_status(&output, EXIT_SUCCESS);
    assert!(
        output.stderr.is_empty(),
        "a clean parse must not write to stderr, got:\n{}",
        stderr_text(&output)
    );
    stdout_text(&output)
}

/// Asserts both CLI round-trip properties for `source`: a clean parse, and
/// canonical output that re-parses to byte-identical canonical output.
pub fn assert_roundtrip(name: &str, source: &str) {
    let first = parse_clean(name, source);
    assert!(
        !first.is_empty(),
        "canonical output must not be empty for a non-empty program"
    );
    let second = parse_clean("canonical.x", &first);
    assert_eq!(
        first, second,
        "canonical printing must be idempotent through the CLI for:\n{source}"
    );
}

/// Asserts the process exit status, showing the captured streams on
/// mismatch.
pub fn assert_status(output: &Output, expected: i32) {
    let code = output.status.code();
    assert_eq!(
        code,
        Some(expected),
        "expected exit status {expected}, got {code:?}\nstdout:\n{}\nstderr:\n{}",
        stdout_text_lossy(output),
        stderr_text_lossy(output),
    );
}

/// Returns stdout as UTF-8 text (panics if the binary emitted non-UTF-8).
pub fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

/// Returns stderr as UTF-8 text (panics if the binary emitted non-UTF-8).
pub fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

/// Returns stdout as lossy text for assertion messages.
pub fn stdout_text_lossy(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Returns stderr as lossy text for assertion messages.
pub fn stderr_text_lossy(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
