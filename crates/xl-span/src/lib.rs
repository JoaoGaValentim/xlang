#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Source location infrastructure for the XLANG compiler.
//!
//! This crate defines the compiler-wide contracts for source files, byte
//! offsets, and spans and is the foundation for all later compiler phases.
//!
//! # Position conventions
//!
//! - [`Span`] offsets are UTF-8 **byte** positions into the source text,
//!   stored as `u32`; spans are half-open ranges `[lo, hi)`.
//! - User-facing line numbers are **1-based**. A file always has at least one
//!   line, and a file ending with `\n` has a final empty line.
//! - User-facing columns are **1-based** and count Unicode scalar values
//!   (`char`s) from the start of the line. They do not account for the
//!   display width of wide characters; the open decision on terminal display
//!   alignment is tracked in `docs/pending-decisions.md` in the repository.
//! - Lines are split on `\n`; a single trailing `\r` (as in CRLF line
//!   endings) is stripped from line text returned for display.
//! - Every operation on potentially invalid positions returns [`SpanError`]
//!   and never panics.
//!
//! # Determinism
//!
//! All operations are pure functions of the registered sources. The crate
//! holds no global mutable state; [`SourceMap`] instances are owned by the
//! caller and passed explicitly to the components that need them.

use std::fmt;

/// Identifier for a source file registered in a [`SourceMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(u32);

impl FileId {
    /// Returns the raw index of this identifier.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Creates a `FileId` from a raw index.
    ///
    /// The returned identifier only resolves if the index was produced by
    /// [`SourceMap::add_file`] on the same map instance. All lookups validate
    /// the identifier and return [`SpanError::UnknownFile`] otherwise.
    #[must_use]
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A half-open byte range `[lo, hi)` within a source file.
///
/// Spans are cheap value types whose fields are public by contract. Because
/// the fields are public, a span can be built with a struct literal that
/// violates the `lo <= hi` invariant; every [`SourceMap`] lookup validates
/// spans and reports violations as [`SpanError`] instead of panicking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// The file this span refers to.
    pub file: FileId,
    /// Byte offset of the first byte of the span.
    pub lo: u32,
    /// Byte offset one past the last byte of the span.
    pub hi: u32,
}

impl Span {
    /// Creates a span, checking that `lo <= hi`.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::Inverted`] if `lo > hi`. Offsets are not checked
    /// against any file here; [`SourceMap`] lookups perform that validation.
    pub const fn new(file: FileId, lo: u32, hi: u32) -> Result<Self, SpanError> {
        if lo <= hi {
            Ok(Self { file, lo, hi })
        } else {
            Err(SpanError::Inverted { lo, hi })
        }
    }

    /// Returns the length of the span in bytes.
    ///
    /// For spans built through unchecked struct literals with `lo > hi`, the
    /// length saturates to `0` rather than underflowing.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.hi.saturating_sub(self.lo)
    }

    /// Returns `true` if the span is empty (`lo == hi`).
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.lo == self.hi
    }
}

/// Errors produced when source positions or spans cannot be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    /// A span was constructed with `lo > hi`.
    Inverted {
        /// The offending start offset.
        lo: u32,
        /// The offending end offset.
        hi: u32,
    },
    /// No file with the given identifier is registered in the map.
    UnknownFile(FileId),
    /// An offset lies beyond the end of the file's source text.
    OutOfBounds {
        /// The offending offset.
        offset: u32,
        /// The length of the source text in bytes.
        len: u32,
    },
    /// An offset does not lie on a UTF-8 character boundary.
    NotCharBoundary {
        /// The offending offset.
        offset: u32,
    },
    /// A requested line number is outside the file's line count.
    LineOutOfBounds {
        /// The requested line number.
        line: u32,
        /// The number of lines in the file.
        lines: u32,
    },
    /// The source text is too large to be addressed with `u32` byte offsets.
    SourceTooLarge {
        /// The size of the source text in bytes.
        len: usize,
    },
    /// The source map holds more files than `u32` identifiers can address.
    TooManyFiles,
}

impl fmt::Display for SpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inverted { lo, hi } => {
                write!(f, "span start {lo} is greater than span end {hi}")
            }
            Self::UnknownFile(id) => {
                write!(f, "file id {id} is not registered in this source map")
            }
            Self::OutOfBounds { offset, len } => {
                write!(
                    f,
                    "offset {offset} is out of bounds for source of length {len}"
                )
            }
            Self::NotCharBoundary { offset } => {
                write!(f, "offset {offset} is not on a UTF-8 character boundary")
            }
            Self::LineOutOfBounds { line, lines } => {
                write!(
                    f,
                    "line {line} is out of bounds for source with {lines} line(s)"
                )
            }
            Self::SourceTooLarge { len } => {
                write!(f, "source of {len} bytes exceeds the u32 offset limit")
            }
            Self::TooManyFiles => {
                f.write_str("source map holds more files than u32 identifiers can address")
            }
        }
    }
}

impl std::error::Error for SpanError {}

/// A 1-based line/column position in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column, counted in Unicode scalar values from the line start.
    pub col: u32,
}

/// A single source file stored in a [`SourceMap`].
#[derive(Debug, Clone)]
pub struct SourceFile {
    name: String,
    source: String,
    line_starts: Vec<u32>,
    line_count: u32,
    byte_len: u32,
}

impl SourceFile {
    fn new(name: String, source: String) -> Result<Self, SpanError> {
        let byte_len = u32::try_from(source.len())
            .map_err(|_| SpanError::SourceTooLarge { len: source.len() })?;
        let line_starts = compute_line_starts(&source);
        let line_count = u32::try_from(line_starts.len())
            .map_err(|_| SpanError::SourceTooLarge { len: source.len() })?;
        Ok(Self {
            name,
            source,
            line_starts,
            line_count,
            byte_len,
        })
    }

    /// Returns the display name of the file (typically its path).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the full source text of the file.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the length of the source text in bytes.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.byte_len
    }

    /// Returns `true` if the source text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Returns the number of lines in the file (always at least 1).
    #[must_use]
    pub fn line_count(&self) -> u32 {
        self.line_count
    }
}

/// Computes the byte offset of the start of every line.
///
/// The first line always starts at offset `0`, and a `\n` at the end of the
/// source introduces a final empty line. Precondition: `source.len()` fits in
/// `u32`, which is enforced by [`SourceFile::new`] before this is called.
fn compute_line_starts(source: &str) -> Vec<u32> {
    debug_assert!(u32::try_from(source.len()).is_ok());
    let mut starts = Vec::new();
    starts.push(0);
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push((index + 1) as u32);
        }
    }
    starts
}

/// Validates a span against a file: ordering, bounds, and UTF-8 boundaries.
fn check_span(file: &SourceFile, span: Span) -> Result<(), SpanError> {
    if span.lo > span.hi {
        return Err(SpanError::Inverted {
            lo: span.lo,
            hi: span.hi,
        });
    }
    if span.hi > file.byte_len {
        return Err(SpanError::OutOfBounds {
            offset: span.hi,
            len: file.byte_len,
        });
    }
    if !file.source.is_char_boundary(span.lo as usize) {
        return Err(SpanError::NotCharBoundary { offset: span.lo });
    }
    if !file.source.is_char_boundary(span.hi as usize) {
        return Err(SpanError::NotCharBoundary { offset: span.hi });
    }
    Ok(())
}

/// A collection of registered source files.
///
/// The map owns all registered sources; identifiers are stable for the
/// lifetime of the map. The map is not internally synchronized and holds no
/// global state: callers own it and pass it explicitly to the components that
/// need source access.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Creates an empty source map.
    #[must_use]
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Registers a source file and returns its stable identifier.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::SourceTooLarge`] if the source text cannot be
    /// addressed with `u32` byte offsets, or [`SpanError::TooManyFiles`] if
    /// the map has no room for another file identifier.
    pub fn add_file(
        &mut self,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<FileId, SpanError> {
        let index = u32::try_from(self.files.len()).map_err(|_| SpanError::TooManyFiles)?;
        let file = SourceFile::new(name.into(), source.into())?;
        self.files.push(file);
        Ok(FileId(index))
    }

    /// Returns the registered file for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::UnknownFile`] if `id` was not produced by this
    /// map instance.
    pub fn file(&self, id: FileId) -> Result<&SourceFile, SpanError> {
        self.files
            .get(id.0 as usize)
            .ok_or(SpanError::UnknownFile(id))
    }

    /// Returns the source text registered for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::UnknownFile`] if `id` is not registered.
    pub fn source(&self, id: FileId) -> Result<&str, SpanError> {
        Ok(self.file(id)?.source())
    }

    /// Returns the exact source text covered by `span`.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError`] when the span is inverted, refers to an
    /// unregistered file, exceeds the file bounds, or is not aligned to
    /// UTF-8 character boundaries.
    pub fn span_slice(&self, span: Span) -> Result<&str, SpanError> {
        let file = self.file(span.file)?;
        check_span(file, span)?;
        Ok(&file.source[span.lo as usize..span.hi as usize])
    }

    /// Maps a byte offset to a 1-based line/column position.
    ///
    /// An offset may equal the file length (the end-of-file position).
    /// Columns count Unicode scalar values from the start of the line and
    /// saturate at `u32::MAX` only for pathological single lines that exceed
    /// the representable column count.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::UnknownFile`] for unregistered files,
    /// [`SpanError::OutOfBounds`] for offsets beyond the end of the file, and
    /// [`SpanError::NotCharBoundary`] for offsets inside a multi-byte
    /// character.
    pub fn line_col(&self, file: FileId, offset: u32) -> Result<LineCol, SpanError> {
        let file = self.file(file)?;
        if offset > file.byte_len {
            return Err(SpanError::OutOfBounds {
                offset,
                len: file.byte_len,
            });
        }
        if !file.source.is_char_boundary(offset as usize) {
            return Err(SpanError::NotCharBoundary { offset });
        }
        let line_index = file
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = file.line_starts[line_index] as usize;
        let col = file.source[line_start..offset as usize].chars().count();
        let col = u32::try_from(col).map_err(|_| SpanError::SourceTooLarge {
            len: file.source.len(),
        })?;
        Ok(LineCol {
            line: line_index as u32 + 1,
            col: col.saturating_add(1),
        })
    }

    /// Maps both ends of `span` to 1-based positions.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`SourceMap::span_slice`].
    pub fn line_col_range(&self, span: Span) -> Result<(LineCol, LineCol), SpanError> {
        let file = self.file(span.file)?;
        check_span(file, span)?;
        let start = self.line_col(span.file, span.lo)?;
        let end = self.line_col(span.file, span.hi)?;
        Ok((start, end))
    }

    /// Returns the display text of `line` (1-based) without line endings.
    ///
    /// A trailing `\n` and a single trailing `\r` (as in CRLF line endings)
    /// are removed. The final line of a file ending with `\n` is empty.
    ///
    /// # Errors
    ///
    /// Returns [`SpanError::UnknownFile`] for unregistered files or
    /// [`SpanError::LineOutOfBounds`] if `line` is `0` or exceeds the file's
    /// line count.
    pub fn line_text(&self, file: FileId, line: u32) -> Result<&str, SpanError> {
        let file = self.file(file)?;
        if line == 0 || line > file.line_count {
            return Err(SpanError::LineOutOfBounds {
                line,
                lines: file.line_count,
            });
        }
        let start = file.line_starts[line as usize - 1] as usize;
        let end = file
            .line_starts
            .get(line as usize)
            .map_or(file.source.len(), |&next| next as usize);
        let raw = &file.source[start..end];
        let raw = raw.strip_suffix('\n').unwrap_or(raw);
        Ok(raw.strip_suffix('\r').unwrap_or(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts_of_empty_source() {
        assert_eq!(compute_line_starts(""), vec![0]);
    }

    #[test]
    fn line_starts_of_newline_only_source() {
        assert_eq!(compute_line_starts("\n\n"), vec![0, 1, 2]);
    }

    #[test]
    fn line_starts_after_crlf() {
        assert_eq!(compute_line_starts("a\r\nb"), vec![0, 3]);
    }

    #[test]
    fn line_starts_without_trailing_newline() {
        assert_eq!(compute_line_starts("ab\ncd"), vec![0, 3]);
    }
}
