#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Structured diagnostics for the XLANG compiler.
//!
//! This crate provides the diagnostic data model — severities, labels, and
//! human-readable messages — and deterministic, source-aware plain-text
//! rendering on top of [`xl_span`]. It does not depend on the CLI, the
//! parser, or any later compiler stage; rendering is a pure function from a
//! [`Diagnostic`] and a [`SourceMap`] to a [`String`].
//!
//! # Rendering
//!
//! Rendered output is deterministic for identical inputs and contains the
//! file location, the relevant source line, the highlighted span, the
//! associated label, and the diagnostic message:
//!
//! ```text
//! error: mismatched types
//!  --> src/main.x:1:13
//!   |
//! 1 | let alpha = beta + 1;
//!   |             ^^^^ expected `i64`, found `str`
//! ```
//!
//! Labels whose spans cannot be resolved against the source map (unknown
//! file, out-of-range or misaligned offsets) are rendered with an explicit
//! placeholder instead of causing a panic:
//!
//! ```text
//! error: example message
//!  --> <unknown file: 9>
//!   = example label
//! ```
//!
//! Multi-line spans are rendered on their first line only: the highlight
//! covers the span's start line from its start column to the end of that
//! line, and continuation lines are not displayed. This Phase 0 behavior is
//! pinned by tests.
//!
//! Columns count Unicode scalar values, matching [`xl_span`]; alignment in
//! terminals therefore assumes one display column per scalar value. The
//! open decision on wide-character display alignment is tracked in
//! `docs/pending-decisions.md` in the repository.

use std::fmt::Write;

use xl_span::{SourceMap, Span};

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// A rule violation that makes the program invalid.
    Error,
    /// A rule violation that does not make the program invalid.
    Warning,
    /// Additional context attached to another diagnostic.
    Note,
}

impl Severity {
    /// Returns the lowercase name used in rendered output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// A source span paired with the human-readable text shown next to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The source range this label refers to.
    pub span: Span,
    /// The text displayed next to the span. May be empty.
    pub message: String,
}

/// A structured compiler diagnostic.
///
/// A diagnostic always has a severity, a human-readable message, and a
/// primary label; secondary labels are optional additional context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: Severity,
    message: String,
    primary: Label,
    secondary: Vec<Label>,
}

impl Diagnostic {
    /// Creates a diagnostic with a primary span and no label text.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>, primary: Span) -> Self {
        Self {
            severity,
            message: message.into(),
            primary: Label {
                span: primary,
                message: String::new(),
            },
            secondary: Vec::new(),
        }
    }

    /// Creates an [`Severity::Error`] diagnostic with the given primary span.
    #[must_use]
    pub fn error(message: impl Into<String>, primary: Span) -> Self {
        Self::new(Severity::Error, message, primary)
    }

    /// Creates a [`Severity::Warning`] diagnostic with the given primary span.
    #[must_use]
    pub fn warning(message: impl Into<String>, primary: Span) -> Self {
        Self::new(Severity::Warning, message, primary)
    }

    /// Creates a [`Severity::Note`] diagnostic with the given primary span.
    #[must_use]
    pub fn note(message: impl Into<String>, primary: Span) -> Self {
        Self::new(Severity::Note, message, primary)
    }

    /// Returns this diagnostic with the text rendered next to the primary span.
    #[must_use]
    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        self.primary.message = message.into();
        self
    }

    /// Returns this diagnostic with an additional secondary label.
    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.secondary.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    /// Returns the severity.
    #[must_use]
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the primary label.
    #[must_use]
    pub fn primary(&self) -> &Label {
        &self.primary
    }

    /// Returns the secondary labels in insertion order.
    #[must_use]
    pub fn secondary(&self) -> &[Label] {
        &self.secondary
    }
}

/// Renders `diagnostic` against `sources` as deterministic plain text.
///
/// The output contains, in order: the severity and message, the primary
/// label, and then every secondary label in insertion order. Each label is
/// rendered with its file location, the relevant source line, the
/// highlighted span, and the label text. Labels whose spans cannot be
/// resolved are rendered with an explicit placeholder
/// (` --> <unknown file: N>` or ` --> <name>: <invalid span lo..hi>`).
#[must_use]
pub fn render(diagnostic: &Diagnostic, sources: &SourceMap) -> String {
    let mut out = String::new();
    out.push_str(diagnostic.severity().as_str());
    out.push_str(": ");
    out.push_str(diagnostic.message());
    out.push('\n');
    push_label(&mut out, sources, diagnostic.primary(), '^');
    for label in diagnostic.secondary() {
        push_label(&mut out, sources, label, '-');
    }
    out
}

/// Appends one rendered label block using `marker` for the span highlight.
fn push_label(out: &mut String, sources: &SourceMap, label: &Label, marker: char) {
    let span = label.span;
    let file = match sources.file(span.file) {
        Ok(file) => file,
        Err(_) => {
            let _ = writeln!(out, " --> <unknown file: {}>", span.file.index());
            push_fallback_message(out, label);
            return;
        }
    };
    let (start, end) = match sources.line_col_range(span) {
        Ok(range) => range,
        Err(_) => {
            let _ = writeln!(
                out,
                " --> {}: <invalid span {}..{}>",
                file.name(),
                span.lo,
                span.hi
            );
            push_fallback_message(out, label);
            return;
        }
    };
    let line_text = sources.line_text(span.file, start.line).unwrap_or_default();
    let gutter = start.line.to_string().len();
    let _ = writeln!(out, " --> {}:{}:{}", file.name(), start.line, start.col);
    let _ = writeln!(out, "{:>gutter$} |", "", gutter = gutter);
    let _ = writeln!(
        out,
        "{:>gutter$} | {}",
        start.line,
        line_text,
        gutter = gutter
    );
    let pad = start.col.saturating_sub(1) as usize;
    let line_chars = line_text.chars().count();
    let available = line_chars.saturating_sub(pad);
    let width = if start.line == end.line {
        (end.col.saturating_sub(start.col) as usize).min(available)
    } else {
        available
    };
    let width = width.max(1);
    let padding = " ".repeat(pad);
    let carets = marker.to_string().repeat(width);
    let suffix = if label.message.is_empty() {
        String::new()
    } else {
        format!(" {}", label.message)
    };
    let _ = writeln!(
        out,
        "{:>gutter$} | {padding}{carets}{suffix}",
        "",
        gutter = gutter
    );
}

/// Appends the label text as a fallback line when no source block is shown.
fn push_fallback_message(out: &mut String, label: &Label) {
    if !label.message.is_empty() {
        let _ = writeln!(out, "  = {}", label.message);
    }
}

#[cfg(test)]
mod tests {
    use xl_span::FileId;

    use super::*;

    #[test]
    fn severity_names_are_stable() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Note.as_str(), "note");
    }

    #[test]
    fn accessors_reflect_construction() {
        let span = Span::new(FileId::from_index(0), 0, 1).unwrap();
        let diagnostic =
            Diagnostic::new(Severity::Warning, "msg", span).with_primary_label("primary text");

        assert_eq!(diagnostic.severity(), Severity::Warning);
        assert_eq!(diagnostic.message(), "msg");
        assert_eq!(diagnostic.primary().span, span);
        assert_eq!(diagnostic.primary().message, "primary text");
        assert!(diagnostic.secondary().is_empty());
    }
}
