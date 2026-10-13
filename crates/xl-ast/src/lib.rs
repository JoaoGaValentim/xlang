#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! The XLANG syntax tree: data model, canonical printing, and structural
//! validation.
//!
//! This crate implements the AST component of Phase 1 as established by the
//! accepted parser/AST contract
//! (`docs/proposals/phase-1/parser-ast-contract.md`, ratified by Directive
//! 006; ADR-0004) under Engineering Directive 007. It contains **no
//! parser**: the future `xl-parser` assigns node identifiers and source
//! spans and translates lexical payloads at its own boundary.
//!
//! # Model
//!
//! - [`Item`] is the root unit (a parsed file is a sequence of items; see
//!   [`print_items`]).
//! - Every node carries its exact [`xl_span::Span`] (UTF-8 byte offsets,
//!   half-open, ADR-0002). Spans are taken from the lexer/parser unchanged;
//!   this crate never invents positions.
//! - Node identifiers ([`ItemId`], [`ExprId`]) are parser-assigned and
//!   **instance-scoped** (unique per parsed file/AST instance, not
//!   globally). `xl-ast` performs no allocation.
//! - Ownership is explicit: `Box` for recursive single children, `Vec` for
//!   ordered lists, owned `String`s for names and decoded text. No `Rc`,
//!   no cycles, no global state, no interning.
//! - The tree encodes syntax only. Name resolution, typing, mutability,
//!   exhaustiveness, and desugaring are later phases and are deliberately
//!   absent.
//!
//! # Printing
//!
//! [`print_item`] (and friends) produce deterministic canonical source
//! that preserves the tree structure under the accepted precedence table
//! and statement/tail classification rules; the printer never panics and
//! returns a structured [`PrintError`] for malformed or over-deep trees.
//! It is a debugging/validation tool for `xc parse`, not the Phase 8
//! formatter.
//!
//! # Validation
//!
//! [`validate_item`] (and friends) report **structural** violations only
//! ([`Violation`]): span sanity/containment, file consistency, required
//! names/paths/generic lists, block tail separation, interpolation
//! ordering, and traversal depth. Source-map validity and semantic
//! validity are separate concerns by design.
//!
//! # Depth and stack safety
//!
//! Printing and validation are bounded by [`print::MAX_PRINT_DEPTH`] and
//! [`validate::MAX_VALIDATE_DEPTH`] (128 node visits), far above the
//! parser's own accepted limit (64, amendment B-4), and fail with
//! structured errors instead of unbounded recursion. **Drop behavior is
//! not depth-bounded**: like any recursive Rust structure, dropping a
//! manually constructed tree deeper than the native stack allows will
//! overflow — the accepted parser cannot produce such trees, and this
//! crate does not claim a global stack-safety guarantee for hand-built
//! input.
//!
//! # Dependency direction
//!
//! `xl-ast → xl-span` only. The future `xl-parser` depends on `xl-ast`
//! (and `xl-lexer`), never the reverse; a dev-only dependency on
//! `xl-lexer` exists for tests that re-lex printed output.

pub mod ast;
pub mod id;
pub mod print;
pub mod validate;

pub use ast::{
    Arm, AssignOp, BinOp, Block, EnumDecl, Expr, ExprKind, Field, FnDecl, FnSig, GenericParam,
    Generics, Ident, ImplDecl, ImplItem, ImportDecl, IntLit, Item, ItemKind, LetStmt, Lit, Param,
    Pat, PatKind, Path, PathType, Radix, Stmt, StmtKind, StringLit, StringPart, StructDecl,
    TextSegment, TraitDecl, TraitItem, Type, TypeKind, UnOp, Variant, Visibility,
};
pub use id::{ExprId, ItemId};
pub use print::{
    MAX_PRINT_DEPTH, PrintError, PrintErrorKind, print_block, print_expr, print_item, print_items,
    print_pattern, print_type,
};
pub use validate::{
    MAX_VALIDATE_DEPTH, Violation, ViolationKind, validate_expr, validate_item, validate_pattern,
    validate_type,
};
