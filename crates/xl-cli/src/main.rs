#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! The `xc` command-line interface for the XLANG compiler frontend.
//!
//! Phase 1 (Directive 009) authorizes exactly one command:
//!
//! ```text
//! xc parse <file.x>   lex and parse a source file, print its canonical AST
//!                     to stdout, and render diagnostics to stderr
//! xc --help           print usage
//! xc --version        print the version
//! ```
//!
//! The command is a syntax-only frontend: it never executes an XLANG
//! program and prints normalized XLANG syntax for the parsed tree, not
//! compiled code or runtime output. Exit codes are CLI policy (see
//! [`run`]): `0` success, `1` source errors, `2` usage, I/O, or internal
//! errors.

mod args;
mod run;

use std::io::Write;
use std::process::ExitCode;

/// Runs `xc` with the process arguments and maps the status to an
/// [`ExitCode`].
fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();
    let status = run::run(&args, &mut stdout, &mut stderr);
    let _ = stdout.flush();
    ExitCode::from(status)
}
