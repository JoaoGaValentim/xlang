#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Source location infrastructure for the XLANG compiler.
//!
//! This crate defines the compiler-wide contracts for source files, byte
//! offsets, and spans and is the foundation for all later compiler phases.
