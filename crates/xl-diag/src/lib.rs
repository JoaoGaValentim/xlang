#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Structured diagnostics for the XLANG compiler.
//!
//! This crate defines severity levels, labels, diagnostics, and source-aware
//! rendering on top of [`xl_span`].
