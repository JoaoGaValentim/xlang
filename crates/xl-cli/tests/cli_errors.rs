//! Diagnostic and file-error behavior of `xc parse`.
//!
//! Rendered diagnostics are asserted exactly (house testing rule), so
//! accidental format drift fails loudly. Operational messages that embed
//! platform-specific OS error text are asserted by prefix only.

mod common;

use common::{
    EXIT_OPERATIONAL, EXIT_SOURCE_ERRORS, TempDir, assert_status, parse_bytes, run_xc, stderr_text,
};

/// Exact stderr for `fn main() {\n    let x = ;\n}\n`.
const SYNTAX_ERROR_STDERR: &str = concat!(
    "error: expected expression\n",
    " --> syntax.x:2:13\n",
    "  |\n",
    "2 |     let x = ;\n",
    "  |             ^\n",
);

/// Exact stderr for `fn main() {\n    let x = @;\n}\n` (lexical error plus
/// the parser's cascaded recovery diagnostic).
const LEXICAL_ERROR_STDERR: &str = concat!(
    "error: unexpected character '@'\n",
    " --> lexical.x:2:13\n",
    "  |\n",
    "2 |     let x = @;\n",
    "  |             ^\n",
    "error: expected expression\n",
    " --> lexical.x:2:14\n",
    "  |\n",
    "2 |     let x = @;\n",
    "  |              ^\n",
);

/// Exact stderr for `fn main() {\n    let s = "abc;\n}\n`.
const UNTERMINATED_STRING_STDERR: &str = concat!(
    "error: unterminated string literal\n",
    " --> unterminated.x:2:13\n",
    "  |\n",
    "2 |     let s = \"abc;\n",
    "  |             ^^^^^\n",
    "error: expected expression\n",
    " --> unterminated.x:3:1\n",
    "  |\n",
    "3 | }\n",
    "  | ^\n",
);

/// Exact stderr for two independent `let` errors on separate lines.
const MULTI_ERROR_STDERR: &str = concat!(
    "error: expected expression\n",
    " --> multi.x:2:13\n",
    "  |\n",
    "2 |     let x = ;\n",
    "  |             ^\n",
    "error: expected expression\n",
    " --> multi.x:5:13\n",
    "  |\n",
    "5 |     let y = ;\n",
    "  |             ^\n",
);

#[test]
fn syntax_error_renders_diagnostics_on_stderr_only() {
    let (_dir, output) = parse_bytes("syntax.x", b"fn main() {\n    let x = ;\n}\n");
    assert_status(&output, EXIT_SOURCE_ERRORS);
    assert!(
        output.stdout.is_empty(),
        "a recovered tree must never be printed as valid source"
    );
    assert_eq!(stderr_text(&output), SYNTAX_ERROR_STDERR);
}

#[test]
fn lexical_error_renders_diagnostics_on_stderr_only() {
    let (_dir, output) = parse_bytes("lexical.x", b"fn main() {\n    let x = @;\n}\n");
    assert_status(&output, EXIT_SOURCE_ERRORS);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), LEXICAL_ERROR_STDERR);
}

#[test]
fn unterminated_string_reports_the_lexical_diagnostic() {
    let (_dir, output) = parse_bytes("unterminated.x", b"fn main() {\n    let s = \"abc;\n}\n");
    assert_status(&output, EXIT_SOURCE_ERRORS);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), UNTERMINATED_STRING_STDERR);
}

#[test]
fn multiple_independent_errors_are_reported_in_order() {
    let source = b"fn a() {\n    let x = ;\n}\nfn b() {\n    let y = ;\n}\n";
    let (_dir, output) = parse_bytes("multi.x", source);
    assert_status(&output, EXIT_SOURCE_ERRORS);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), MULTI_ERROR_STDERR);
    // A second run is byte-identical (determinism of diagnostics).
    let (_dir2, second) = parse_bytes("multi.x", source);
    assert_eq!(output.stderr, second.stderr);
    assert_eq!(output.status.code(), second.status.code());
}

#[test]
fn many_independent_errors_are_all_reported() {
    let mut source = String::new();
    for index in 0..30 {
        source.push_str(&format!("fn f{index}() {{\n    let x = ;\n}}\n"));
    }
    let (_dir, output) = parse_bytes("many.x", source.as_bytes());
    assert_status(&output, EXIT_SOURCE_ERRORS);
    assert!(output.stdout.is_empty());
    let stderr = stderr_text(&output);
    let errors = stderr
        .lines()
        .filter(|line| line.starts_with("error: "))
        .count();
    assert_eq!(errors, 30, "every independent error must be reported");
}

#[test]
fn deep_nesting_is_reported_without_crashing() {
    let mut source = String::from("fn f() { let x = ");
    source.push_str(&"(".repeat(200));
    source.push('1');
    source.push_str(&")".repeat(200));
    source.push_str("; }\n");
    let (_dir, output) = parse_bytes("deep.x", source.as_bytes());
    assert_status(&output, EXIT_SOURCE_ERRORS);
    assert!(output.stdout.is_empty());
    assert!(
        stderr_text(&output).contains("expression nesting limit (64) exceeded"),
        "the depth limit must produce the controlled diagnostic, got:\n{}",
        stderr_text(&output)
    );
}

#[test]
fn missing_file_is_an_operational_error() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["parse", "missing.x"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    let stderr = stderr_text(&output);
    assert!(
        stderr.starts_with("error: cannot read 'missing.x': "),
        "unexpected message: {stderr}"
    );
}

#[test]
fn invalid_utf8_is_an_operational_error() {
    let (_dir, output) = parse_bytes("utf8.x", b"\xff\xfe\x00abc");
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr_text(&output),
        "error: 'utf8.x' is not valid UTF-8 (invalid byte sequence at offset 0)\n"
    );
}

#[test]
fn directory_input_is_an_operational_error() {
    let dir = TempDir::new();
    std::fs::create_dir(dir.path().join("adir")).expect("create directory input");
    let output = run_xc(dir.path(), &["parse", "adir"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert!(
        stderr_text(&output).starts_with("error: cannot read 'adir': "),
        "unexpected message: {}",
        stderr_text(&output)
    );
}

#[cfg(unix)]
#[test]
fn source_beyond_the_u32_offset_limit_is_rejected_before_reading() {
    // A sparse file makes the boundary condition cheap to reproduce on
    // Unix filesystems: the file is reported as >4 GiB without allocating
    // storage, and the CLI must reject it from metadata alone.
    let dir = TempDir::new();
    let path = dir.path().join("huge.x");
    let file = std::fs::File::create(&path).expect("create sparse file");
    file.set_len(u64::from(u32::MAX) + 1)
        .expect("set sparse file length");
    drop(file);

    let output = run_xc(dir.path(), &["parse", "huge.x"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr_text(&output),
        format!(
            "error: cannot load 'huge.x': source of {} bytes exceeds the u32 offset limit\n",
            u64::from(u32::MAX) + 1
        )
    );
}

#[cfg(unix)]
#[test]
fn unreadable_file_is_an_operational_error_when_reproducible() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new();
    let path = dir.write("locked.x", b"fn main() { }\n");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod 0o000");

    // Running as root bypasses permission checks; the condition is then not
    // reproducible and the assertions are skipped (documented constraint).
    if std::fs::read(&path).is_ok() {
        eprintln!("skipping unreadable-file assertions: process can read 0o000 files");
        return;
    }

    let output = run_xc(dir.path(), &["parse", "locked.x"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert!(
        stderr_text(&output).starts_with("error: cannot read 'locked.x': "),
        "unexpected message: {}",
        stderr_text(&output)
    );
}
