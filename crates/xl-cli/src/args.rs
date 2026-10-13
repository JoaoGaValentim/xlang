//! Command-line argument parsing for `xc`.
//!
//! The Phase 1 surface is intentionally minimal and hand-written; no
//! external argument-parsing dependency is used. Recognized forms:
//!
//! ```text
//! xc --help | -h
//! xc --version | -V
//! xc parse <file.x>
//! ```
//!
//! Everything else is a [`UsageError`], reported by the caller with the
//! CLI's documented exit status for operational errors. This module is a
//! pure function over the argument list: it performs no I/O and never
//! panics, including for arguments that are not valid UTF-8.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

/// A recognized `xc` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// `xc --help` or `xc -h`: print help on stdout.
    Help,
    /// `xc --version` or `xc -V`: print the version on stdout.
    Version,
    /// `xc parse <file>`: parse the named source file.
    Parse(PathBuf),
}

/// A command-line usage error.
///
/// Usage errors are operational conditions (CLI policy), not XLANG source
/// diagnostics; the caller renders them on stderr and exits with the
/// operational status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    /// No command was given.
    MissingCommand,
    /// The command is not recognized.
    UnknownCommand(String),
    /// A flag is not recognized.
    UnknownFlag(String),
    /// `parse` was invoked without a file argument.
    MissingFileArgument,
    /// More positional arguments were given than the command accepts.
    UnexpectedArgument(String),
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => f.write_str("missing command"),
            Self::UnknownCommand(command) => write!(f, "unknown command '{command}'"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag '{flag}'"),
            Self::MissingFileArgument => f.write_str("missing file argument"),
            Self::UnexpectedArgument(argument) => write!(f, "unexpected argument '{argument}'"),
        }
    }
}

impl std::error::Error for UsageError {}

/// Parses the process arguments after the program name.
///
/// # Errors
///
/// Returns the first [`UsageError`] encountered: a missing or unknown
/// command, an unknown flag, or missing/excessive arguments. Help and
/// version take no additional arguments.
pub fn parse(args: &[OsString]) -> Result<Invocation, UsageError> {
    let Some((first, rest)) = args.split_first() else {
        return Err(UsageError::MissingCommand);
    };
    match first.to_str() {
        Some("--help" | "-h") => match rest {
            [] => Ok(Invocation::Help),
            [extra, ..] => Err(UsageError::UnexpectedArgument(display(extra))),
        },
        Some("--version" | "-V") => match rest {
            [] => Ok(Invocation::Version),
            [extra, ..] => Err(UsageError::UnexpectedArgument(display(extra))),
        },
        Some("parse") => parse_command(rest),
        Some(flag) if flag.starts_with('-') => Err(UsageError::UnknownFlag(flag.to_string())),
        Some(command) => Err(UsageError::UnknownCommand(command.to_string())),
        None => Err(UsageError::UnknownCommand(display(first))),
    }
}

/// Parses the arguments of the `parse` command.
fn parse_command(rest: &[OsString]) -> Result<Invocation, UsageError> {
    let Some((file, extra)) = rest.split_first() else {
        return Err(UsageError::MissingFileArgument);
    };
    if is_flag(file) {
        return Err(UsageError::UnknownFlag(display(file)));
    }
    if let Some(extra) = extra.first() {
        return Err(if is_flag(extra) {
            UsageError::UnknownFlag(display(extra))
        } else {
            UsageError::UnexpectedArgument(display(extra))
        });
    }
    Ok(Invocation::Parse(PathBuf::from(file)))
}

/// Returns `true` when `arg` looks like a flag (starts with `-`).
///
/// Arguments that are not valid UTF-8 cannot match ASCII flags and are
/// treated as file paths.
fn is_flag(arg: &OsStr) -> bool {
    arg.to_str().is_some_and(|token| token.starts_with('-'))
}

/// Renders an argument for display, lossily for non-UTF-8 input.
fn display(arg: &OsStr) -> String {
    arg.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_arguments_is_a_missing_command() {
        assert_eq!(parse(&[]), Err(UsageError::MissingCommand));
    }

    #[test]
    fn help_and_version_are_recognized_alone() {
        assert_eq!(parse(&args(&["--help"])), Ok(Invocation::Help));
        assert_eq!(parse(&args(&["-h"])), Ok(Invocation::Help));
        assert_eq!(parse(&args(&["--version"])), Ok(Invocation::Version));
        assert_eq!(parse(&args(&["-V"])), Ok(Invocation::Version));
    }

    #[test]
    fn help_and_version_reject_extra_arguments() {
        assert_eq!(
            parse(&args(&["--help", "extra"])),
            Err(UsageError::UnexpectedArgument("extra".to_string()))
        );
        assert_eq!(
            parse(&args(&["--version", "extra"])),
            Err(UsageError::UnexpectedArgument("extra".to_string()))
        );
    }

    #[test]
    fn parse_requires_a_file_argument() {
        assert_eq!(
            parse(&args(&["parse"])),
            Err(UsageError::MissingFileArgument)
        );
    }

    #[test]
    fn parse_accepts_one_file_including_non_x_extensions() {
        assert_eq!(
            parse(&args(&["parse", "example.x"])),
            Ok(Invocation::Parse(PathBuf::from("example.x")))
        );
        assert_eq!(
            parse(&args(&["parse", "notes.txt"])),
            Ok(Invocation::Parse(PathBuf::from("notes.txt")))
        );
    }

    #[test]
    fn parse_rejects_flag_like_file_arguments() {
        assert_eq!(
            parse(&args(&["parse", "--tokens"])),
            Err(UsageError::UnknownFlag("--tokens".to_string()))
        );
        assert_eq!(
            parse(&args(&["parse", "-x"])),
            Err(UsageError::UnknownFlag("-x".to_string()))
        );
    }

    #[test]
    fn parse_rejects_excessive_positional_arguments() {
        assert_eq!(
            parse(&args(&["parse", "a.x", "b.x"])),
            Err(UsageError::UnexpectedArgument("b.x".to_string()))
        );
    }

    #[test]
    fn parse_reports_an_unknown_flag_before_an_extra_argument() {
        assert_eq!(
            parse(&args(&["parse", "a.x", "--json"])),
            Err(UsageError::UnknownFlag("--json".to_string()))
        );
    }

    #[test]
    fn unknown_commands_and_flags_are_reported() {
        assert_eq!(
            parse(&args(&["check"])),
            Err(UsageError::UnknownCommand("check".to_string()))
        );
        assert_eq!(
            parse(&args(&["--json"])),
            Err(UsageError::UnknownFlag("--json".to_string()))
        );
    }

    #[test]
    fn usage_error_messages_are_stable() {
        assert_eq!(UsageError::MissingCommand.to_string(), "missing command");
        assert_eq!(
            UsageError::UnknownCommand("check".to_string()).to_string(),
            "unknown command 'check'"
        );
        assert_eq!(
            UsageError::UnknownFlag("--json".to_string()).to_string(),
            "unknown flag '--json'"
        );
        assert_eq!(
            UsageError::MissingFileArgument.to_string(),
            "missing file argument"
        );
        assert_eq!(
            UsageError::UnexpectedArgument("b.x".to_string()).to_string(),
            "unexpected argument 'b.x'"
        );
    }
}
