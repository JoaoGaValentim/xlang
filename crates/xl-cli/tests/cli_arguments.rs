//! Argument handling: help, version, and every documented usage error.
//!
//! The exact help and usage texts are pinned here from outside the binary
//! crate so that accidental format drift fails loudly.

mod common;

use common::{
    EXIT_OPERATIONAL, EXIT_SUCCESS, TempDir, assert_status, run_xc, stderr_text, stdout_text,
};

/// The exact `xc --help` stdout.
const HELP: &str = "\
xc - XLANG compiler frontend (Phase 1)

Usage:
  xc --help
  xc --version
  xc parse <file.x>

Commands:
  parse <file.x>  Lex and parse a source file, then print its canonical AST
                  on stdout. Diagnostics are rendered on stderr.

Options:
  -h, --help      Print this help and exit.
  -V, --version   Print the version and exit.

Exit codes:
  0  success (no error-level diagnostics)
  1  source errors (lexical or syntactic diagnostics; no AST is printed)
  2  usage, I/O, or internal errors

The command never executes an XLANG program: the printed output is
normalized XLANG syntax for the parsed tree, not compiled code or runtime
output.
";

/// The usage summary appended to every usage error.
const USAGE_SUMMARY: &str = "\
usage: xc --help
       xc --version
       xc parse <file.x>
Try 'xc --help' for more information.
";

/// Builds the expected stderr for a usage error message.
fn usage_error(message: &str) -> String {
    format!("error: {message}\n{USAGE_SUMMARY}")
}

#[test]
fn help_prints_usage_to_stdout_and_exits_zero() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["--help"]);
    assert_status(&output, EXIT_SUCCESS);
    assert_eq!(stdout_text(&output), HELP);
    assert!(output.stderr.is_empty(), "help must not write to stderr");
}

#[test]
fn short_help_alias_behaves_identically() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["-h"]);
    assert_status(&output, EXIT_SUCCESS);
    assert_eq!(stdout_text(&output), HELP);
    assert!(output.stderr.is_empty());
}

#[test]
fn version_prints_name_and_version_and_exits_zero() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["--version"]);
    assert_status(&output, EXIT_SUCCESS);
    assert_eq!(
        stdout_text(&output),
        format!("xc {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn short_version_alias_behaves_identically() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["-V"]);
    assert_status(&output, EXIT_SUCCESS);
    assert_eq!(
        stdout_text(&output),
        format!("xc {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn help_and_version_reject_extra_arguments() {
    let dir = TempDir::new();
    for args in [["--help", "extra"], ["--version", "extra"]] {
        let output = run_xc(dir.path(), &args);
        assert_status(&output, EXIT_OPERATIONAL);
        assert!(output.stdout.is_empty());
        assert_eq!(
            stderr_text(&output),
            usage_error("unexpected argument 'extra'")
        );
    }
}

#[test]
fn missing_command_reports_a_usage_error() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &[]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), usage_error("missing command"));
}

#[test]
fn unknown_command_reports_a_usage_error() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["frobnicate"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr_text(&output),
        usage_error("unknown command 'frobnicate'")
    );
}

#[test]
fn unknown_top_level_flag_reports_a_usage_error() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["--tokens"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), usage_error("unknown flag '--tokens'"));
}

#[test]
fn parse_without_a_file_argument_reports_a_usage_error() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["parse"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), usage_error("missing file argument"));
}

#[test]
fn parse_rejects_a_flag_like_file_argument() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["parse", "--tokens"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), usage_error("unknown flag '--tokens'"));
}

#[test]
fn parse_rejects_help_because_help_is_top_level_only() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["parse", "--help"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), usage_error("unknown flag '--help'"));
}

#[test]
fn parse_rejects_excessive_positional_arguments() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["parse", "a.x", "b.x"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr_text(&output),
        usage_error("unexpected argument 'b.x'")
    );
}

#[test]
fn parse_reports_an_unknown_flag_before_an_extra_argument() {
    let dir = TempDir::new();
    let output = run_xc(dir.path(), &["parse", "a.x", "--json"]);
    assert_status(&output, EXIT_OPERATIONAL);
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_text(&output), usage_error("unknown flag '--json'"));
}
