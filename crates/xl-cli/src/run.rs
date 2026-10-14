//! Execution of `xc` invocations: file loading, parsing, diagnostics, and
//! canonical printing.
//!
//! # Command contract (CLI policy)
//!
//! - Successful parse (no error-level diagnostics): canonical AST on
//!   stdout, exit [`EXIT_SUCCESS`] (`0`). Warnings and notes alone do not
//!   fail; any diagnostics are rendered on stderr.
//! - Lexical or syntactic errors: diagnostics on stderr, exit
//!   [`EXIT_SOURCE_ERRORS`] (`1`); the recovered tree is never printed on
//!   stdout.
//! - Usage, I/O, and internal errors: message on stderr, exit
//!   [`EXIT_OPERATIONAL`] (`2`).
//!
//! Exit codes `0`/`1` follow the accepted `xc parse` contract; `2` is CLI
//! policy for operational conditions, not a language semantic. The command
//! never executes an XLANG program.
//!
//! # Determinism
//!
//! Identical inputs produce byte-identical stdout, stderr, and exit
//! status: output is a pure function of the argument list and the named
//! file's bytes. No timestamps, environment data, or locale affect it.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;

use xl_parser::ParseOutput;

use crate::args::{Invocation, UsageError};

/// Exit status: parsing succeeded with no error-level diagnostics.
pub const EXIT_SUCCESS: u8 = 0;
/// Exit status: the source has lexical or syntactic errors.
pub const EXIT_SOURCE_ERRORS: u8 = 1;
/// Exit status: usage, I/O, or internal error (CLI policy).
pub const EXIT_OPERATIONAL: u8 = 2;

/// The `xc --help` text.
pub const HELP: &str = "\
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

/// The `xc --version` text.
pub const VERSION: &str = concat!("xc ", env!("CARGO_PKG_VERSION"), "\n");

/// Dispatches one invocation and returns the process exit status.
///
/// `args` are the process arguments after the program name. All output is
/// written through the provided streams so the full command behavior is
/// testable without spawning a process.
pub fn run(args: &[OsString], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    match crate::args::parse(args) {
        Ok(Invocation::Help) => write_stdout(stdout, stderr, HELP),
        Ok(Invocation::Version) => write_stdout(stdout, stderr, VERSION),
        Ok(Invocation::Parse(path)) => parse_file(&path, stdout, stderr),
        Err(error) => {
            write_stderr(stderr, &usage_error_text(&error));
            EXIT_OPERATIONAL
        }
    }
}

/// Reads, decodes, and parses one source file, then reports the result.
///
/// Every failure mode is explicit and recoverable: unreadable paths,
/// invalid UTF-8, sources beyond the `u32` span-addressing limit, and
/// unexpected internal failures all exit with [`EXIT_OPERATIONAL`]; source
/// diagnostics exit with [`EXIT_SOURCE_ERRORS`]; a clean parse prints the
/// canonical AST and exits with [`EXIT_SUCCESS`]. A byte-empty file is a
/// valid empty module (`module = { item }`): it parses to zero items with
/// no diagnostics, prints nothing, and exits with [`EXIT_SUCCESS`], exactly
/// like whitespace-only or comment-only input.
fn parse_file(path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let display = path.display().to_string();
    // Reject regular files beyond the accepted `u32` span-addressing limit
    // before reading them, so an oversized source is never buffered in
    // memory just to fail registration afterwards. The post-read
    // registration still enforces the same limit for anything that changes
    // size concurrently.
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_file() && metadata.len() > u64::from(u32::MAX) {
                let size = metadata.len();
                let _ = writeln!(
                    stderr,
                    "error: cannot load '{display}': source of {size} bytes exceeds the u32 offset limit"
                );
                return EXIT_OPERATIONAL;
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "error: cannot read '{display}': {error}");
            return EXIT_OPERATIONAL;
        }
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "error: cannot read '{display}': {error}");
            return EXIT_OPERATIONAL;
        }
    };
    let source = match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(error) => {
            let offset = error.utf8_error().valid_up_to();
            let _ = writeln!(
                stderr,
                "error: '{display}' is not valid UTF-8 (invalid byte sequence at offset {offset})"
            );
            return EXIT_OPERATIONAL;
        }
    };
    let mut sources = xl_span::SourceMap::new();
    let file = match sources.add_file(display.clone(), source) {
        Ok(file) => file,
        Err(error) => {
            let _ = writeln!(stderr, "error: cannot load '{display}': {error}");
            return EXIT_OPERATIONAL;
        }
    };
    let output = match xl_parser::parse_file(&sources, file) {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(stderr, "error: internal error: {error}");
            return EXIT_OPERATIONAL;
        }
    };
    if !output.diagnostics.is_empty() {
        let mut rendered = String::new();
        for diagnostic in &output.diagnostics {
            rendered.push_str(&xl_diag::render(diagnostic, &sources));
        }
        write_stderr(stderr, &rendered);
    }
    let status = source_exit_status(&output);
    if status != EXIT_SUCCESS {
        return status;
    }
    let printed = match xl_ast::print_items(&output.items) {
        Ok(printed) => printed,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "error: internal error: canonical printing failed: {error}"
            );
            return EXIT_OPERATIONAL;
        }
    };
    if printed.is_empty() {
        return EXIT_SUCCESS;
    }
    let mut text = printed;
    text.push('\n');
    write_stdout(stdout, stderr, &text)
}

/// Source-level exit status: [`EXIT_SOURCE_ERRORS`] when any error-level
/// diagnostic exists, otherwise [`EXIT_SUCCESS`]. Warnings and notes alone
/// do not make a successful parse fail.
fn source_exit_status(output: &ParseOutput) -> u8 {
    if output.has_errors() {
        EXIT_SOURCE_ERRORS
    } else {
        EXIT_SUCCESS
    }
}

/// Renders one usage error and the usage summary.
fn usage_error_text(error: &UsageError) -> String {
    format!(
        "error: {error}\n\
         usage: xc --help\n\
         \x20      xc --version\n\
         \x20      xc parse <file.x>\n\
         Try 'xc --help' for more information.\n"
    )
}

/// Writes `text` to stdout, flushing; a write failure is reported on
/// stderr and becomes an operational error.
fn write_stdout(stdout: &mut dyn Write, stderr: &mut dyn Write, text: &str) -> u8 {
    if let Err(error) = stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        let _ = writeln!(stderr, "error: cannot write to stdout: {error}");
        return EXIT_OPERATIONAL;
    }
    EXIT_SUCCESS
}

/// Writes `text` to stderr, ignoring write failures (there is no better
/// channel to report them on; the exit status still reflects the error).
fn write_stderr(stderr: &mut dyn Write, text: &str) {
    let _ = stderr.write_all(text.as_bytes());
    let _ = stderr.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use xl_diag::{Diagnostic, Severity};
    use xl_span::{FileId, Span};

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    fn span() -> Span {
        Span::new(FileId::from_index(0), 0, 1).expect("valid span")
    }

    #[test]
    fn help_goes_to_stdout_with_success() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let status = run(&args(&["--help"]), &mut out, &mut err);
        assert_eq!(status, EXIT_SUCCESS);
        assert_eq!(String::from_utf8(out).expect("utf8"), HELP);
        assert!(err.is_empty());
    }

    #[test]
    fn version_goes_to_stdout_with_success() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let status = run(&args(&["--version"]), &mut out, &mut err);
        assert_eq!(status, EXIT_SUCCESS);
        assert_eq!(String::from_utf8(out).expect("utf8"), VERSION);
        assert!(err.is_empty());
    }

    #[test]
    fn usage_errors_go_to_stderr_with_operational_status() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let status = run(&[], &mut out, &mut err);
        assert_eq!(status, EXIT_OPERATIONAL);
        assert!(out.is_empty());
        let text = String::from_utf8(err).expect("utf8");
        assert!(text.starts_with("error: missing command\n"));
        assert!(text.contains("usage: xc --help"));
        assert!(text.contains("Try 'xc --help' for more information."));
    }

    #[test]
    fn warnings_and_notes_alone_do_not_fail_the_parse() {
        let warning = ParseOutput {
            items: Vec::new(),
            diagnostics: vec![Diagnostic::new(Severity::Warning, "w", span())],
        };
        let note = ParseOutput {
            items: Vec::new(),
            diagnostics: vec![Diagnostic::new(Severity::Note, "n", span())],
        };
        let error = ParseOutput {
            items: Vec::new(),
            diagnostics: vec![Diagnostic::new(Severity::Error, "e", span())],
        };
        let clean = ParseOutput {
            items: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert_eq!(source_exit_status(&warning), EXIT_SUCCESS);
        assert_eq!(source_exit_status(&note), EXIT_SUCCESS);
        assert_eq!(source_exit_status(&clean), EXIT_SUCCESS);
        assert_eq!(source_exit_status(&error), EXIT_SOURCE_ERRORS);
    }

    /// A writer that always fails, to exercise the stdout failure path.
    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated write failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("simulated write failure"))
        }
    }

    #[test]
    fn stdout_write_failures_are_reported_as_operational_errors() {
        let mut out = FailingWriter;
        let mut err = Vec::new();
        let status = run(&args(&["--version"]), &mut out, &mut err);
        assert_eq!(status, EXIT_OPERATIONAL);
        let text = String::from_utf8(err).expect("utf8");
        assert!(text.starts_with("error: cannot write to stdout: "));
    }
}
