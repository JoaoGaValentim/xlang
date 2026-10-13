//! The XLANG syntax tree data model.
//!
//! The node shapes implement the accepted Phase 1 parser/AST contract
//! (`docs/proposals/phase-1/parser-ast-contract.md` §2.2, ratified by
//! Directive 006; ADR-0004). The tree represents **syntax only**: no name
//! resolution, typing, or desugaring is encoded here, and every node keeps
//! its source [`Span`].
//!
//! Ownership is explicit and tree-shaped: `Box` for recursive single
//! children, `Vec` for ordered child lists, owned `String`s for names and
//! decoded text. There are no reference-counted graphs, cycles, or global
//! interners; a session-level symbol interner can replace the `String`
//! payloads later without changing the observable model.

use xl_span::Span;

use crate::id::{ExprId, ItemId};

/// An identifier occurrence: its name and the exact source span.
///
/// The name is stored as owned text (interning deferred; no global state).
/// Recovered nodes should still carry a non-empty name (see
/// [`crate::validate`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    /// The identifier text.
    pub name: String,
    /// The span of the identifier in the source file.
    pub span: Span,
}

/// Declared visibility of an item, field, or method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// No `pub` keyword was written (grammar default; semantic rules are
    /// Phase 2).
    Private,
    /// The span of the `pub` keyword.
    Public(Span),
}

/// Radix of an integer literal, preserved from the accepted lexical
/// contract (amendment B-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radix {
    /// Decimal (`0`, `1_000`).
    Dec,
    /// Hexadecimal (`0xFF`).
    Hex,
    /// Binary (`0b1010`).
    Bin,
}

/// Integer literal payload: unsigned magnitude plus radix.
///
/// Signed and target-type range checks are the type-system phase's
/// responsibility; the syntax tree preserves the magnitude exactly as the
/// lexer produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntLit {
    /// The literal's magnitude (`_` separators already removed).
    pub magnitude: u64,
    /// The radix the literal was written in.
    pub radix: Radix,
}

/// A decoded literal-text segment of a string literal.
///
/// `text` holds the decoded value (escapes resolved); `span` covers the raw
/// source bytes of the segment, including any escape sequences. (The
/// accepted contract lists `Text(String)` indicatively; per Directives
/// 006/007 the segment span is retained so interpolation ordering can be
/// validated and printed faithfully.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSegment {
    /// Decoded text.
    pub text: String,
    /// Raw source span of the segment.
    pub span: Span,
}

/// One part of a structured string literal, in source order.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Decoded literal text.
    Text(TextSegment),
    /// An interpolated expression.
    Expr(Expr),
}

/// Structured string-literal payload.
#[derive(Debug, Clone, PartialEq)]
pub struct StringLit {
    /// The ordered parts of the literal (empty for `""`).
    pub parts: Vec<StringPart>,
    /// Span of the whole literal, including both quotes.
    pub span: Span,
}

/// Literal payload of an expression or pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// Integer literal.
    Int(IntLit),
    /// Finite floating-point literal (non-finite values cannot be printed;
    /// see [`crate::print`]).
    Float(f64),
    /// Boolean literal (`true`/`false`).
    Bool(bool),
    /// Character literal.
    Char(char),
    /// Structured string literal.
    Str(StringLit),
}

/// A `::`-separated path used in expressions (`io::println`, `self`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// The path segments in source order (at least one after a successful
    /// parse; empty paths are invalid for printing).
    pub segments: Vec<Ident>,
    /// Span covering the whole path.
    pub span: Span,
}

/// A `::`-separated type path with optional generic arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct PathType {
    /// The path segments in source order.
    pub segments: Vec<Ident>,
    /// Generic arguments (`Result<i64, str>`), when written.
    pub generics: Option<Vec<Type>>,
    /// Span covering the whole type path.
    pub span: Span,
}

/// Expression node with identity, kind, and exact source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Parser-assigned identifier (instance-scoped).
    pub id: ExprId,
    /// The expression's shape.
    pub kind: ExprKind,
    /// Span covering the whole expression.
    pub span: Span,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Arithmetic negation (`-e`).
    Neg,
    /// Logical negation (`!e`).
    Not,
}

/// Binary operators of the left-associative and comparison tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
}

/// Assignment operators (tier 1, syntactically non-associative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubAssign,
    /// `*=`
    MulAssign,
    /// `/=`
    DivAssign,
    /// `%=`
    RemAssign,
}

/// The shape of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Literal.
    Lit(Lit),
    /// Path reference.
    Path(Path),
    /// Unary operation.
    Unary(UnOp, Box<Expr>),
    /// Binary operation.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Function call.
    Call(Box<Expr>, Vec<Expr>),
    /// Field access (`base.name`).
    Field(Box<Expr>, Ident),
    /// Indexing (`base[index]`).
    Index(Box<Expr>, Box<Expr>),
    /// Assignment (target, operator, value).
    Assign(AssignOp, Box<Expr>, Box<Expr>),
    /// Exclusive range (`a..b`).
    Range(Box<Expr>, Box<Expr>),
    /// Type cast (`expr as type`).
    Cast(Box<Expr>, Type),
    /// Error propagation (`expr?`).
    Try(Box<Expr>),
    /// `if cond { .. } else { .. }` (`else` optional in the tree).
    If(Box<Expr>, Block, Option<Box<Expr>>),
    /// `match scrutinee { arms }`.
    Match(Box<Expr>, Vec<Arm>),
    /// Block expression.
    Block(Block),
    /// `while cond { .. }`.
    While(Box<Expr>, Block),
    /// `for pattern in iterable { .. }`.
    For(Box<Pat>, Box<Expr>, Block),
    /// `loop { .. }`.
    Loop(Block),
    /// `return` with an optional operand (operand extends maximally).
    Return(Option<Box<Expr>>),
    /// `break` (no operand in the accepted grammar).
    Break,
    /// `continue`.
    Continue,
    /// Tuple/unit expression (`()`, `(a,)`, `(a, b)`).
    Tuple(Vec<Expr>),
    /// Array literal (`[a, b]`; empty allowed syntactically).
    Array(Vec<Expr>),
}

/// A `match` arm.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    /// The pattern.
    pub pat: Pat,
    /// The arm body expression.
    pub body: Expr,
    /// Span covering pattern through body (delimiters excluded).
    pub span: Span,
}

/// The shape of a pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    /// `_`
    Wildcard,
    /// A binding identifier (syntactically an identifier; resolution is
    /// Phase 2).
    Binding(Ident),
    /// A literal pattern.
    Lit(Lit),
    /// A path pattern with optional tuple-style payload patterns
    /// (`Shape::Circle(r)`, `Shape::Circle`).
    Path {
        /// The constructor path.
        path: Path,
        /// Payload patterns, present when parentheses were written.
        args: Option<Vec<Pat>>,
    },
    /// Tuple pattern.
    Tuple(Vec<Pat>),
}

/// Pattern node with exact span.
#[derive(Debug, Clone, PartialEq)]
pub struct Pat {
    /// The pattern's shape.
    pub kind: PatKind,
    /// Span covering the whole pattern.
    pub span: Span,
}

/// The shape of a type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// Named type with optional generic arguments.
    Path(PathType),
    /// Tuple type; `[]` is the unit type `()` and a single element requires
    /// a trailing comma when printed (`(T,)`).
    Tuple(Vec<Type>),
    /// Slice type (`[T]`).
    Slice(Box<Type>),
    /// Array type (`[T; N]`), with the length expression.
    Array(Box<Type>, Box<Expr>),
    /// Function type (`fn(A, B) -> R`; return optional).
    Fn {
        /// Parameter types.
        params: Vec<Type>,
        /// Return type, when written.
        ret: Option<Box<Type>>,
    },
}

/// Type node with exact span.
#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    /// The type's shape.
    pub kind: TypeKind,
    /// Span covering the whole type.
    pub span: Span,
}

/// Generic parameter list (`<T: Ord, U>`).
#[derive(Debug, Clone, PartialEq)]
pub struct Generics {
    /// The parameters (at least one after a successful parse).
    pub params: Vec<GenericParam>,
    /// Span covering `<` through `>`.
    pub span: Span,
}

/// A single generic parameter with an optional bound.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    /// The parameter name.
    pub name: Ident,
    /// The bound path, when written (`T: Ord`).
    pub bound: Option<PathType>,
    /// Span covering the parameter.
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    /// The `self` method receiver; carries the span of `self`.
    Receiver(Span),
    /// A named parameter (`o: Point`).
    Named {
        /// The parameter name.
        name: Ident,
        /// The declared type.
        ty: Type,
        /// Span covering the parameter.
        span: Span,
    },
}

/// A function signature.
#[derive(Debug, Clone, PartialEq)]
pub struct FnSig {
    /// The function name.
    pub name: Ident,
    /// The parameters in order.
    pub params: Vec<Param>,
    /// The declared return type, when written.
    pub ret: Option<Type>,
    /// Span covering the signature (name through return type).
    pub span: Span,
}

/// A function declaration (with body).
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    /// Declared visibility.
    pub vis: Visibility,
    /// Generic parameters, when written.
    pub generics: Option<Generics>,
    /// The signature.
    pub sig: FnSig,
    /// The body block.
    pub body: Block,
    /// Span covering the whole declaration.
    pub span: Span,
}

/// A struct field declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Declared visibility.
    pub vis: Visibility,
    /// The field name.
    pub name: Ident,
    /// The field type.
    pub ty: Type,
    /// Span covering the field.
    pub span: Span,
}

/// A struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    /// Declared visibility.
    pub vis: Visibility,
    /// The struct name.
    pub name: Ident,
    /// Generic parameters, when written.
    pub generics: Option<Generics>,
    /// The fields in order (possibly empty).
    pub fields: Vec<Field>,
    /// Span covering the whole declaration.
    pub span: Span,
}

/// An enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// The variant name.
    pub name: Ident,
    /// Payload types for tuple variants; empty for unit variants.
    pub payload: Vec<Type>,
    /// Span covering the variant.
    pub span: Span,
}

/// An enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    /// Declared visibility.
    pub vis: Visibility,
    /// The enum name.
    pub name: Ident,
    /// Generic parameters, when written.
    pub generics: Option<Generics>,
    /// The variants in order (possibly empty).
    pub variants: Vec<Variant>,
    /// Span covering the whole declaration.
    pub span: Span,
}

/// An item inside an `impl` block.
#[derive(Debug, Clone, PartialEq)]
pub enum ImplItem {
    /// A method (its own `vis` applies inside the impl block).
    Fn(FnDecl),
}

/// An `impl` block (inherent or trait implementation).
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    /// Generic parameters, when written (`impl<T> ...`).
    pub generics: Option<Generics>,
    /// The implemented trait, when written (`impl Show for Point`).
    pub trait_: Option<PathType>,
    /// The self type.
    pub self_ty: PathType,
    /// The items in order (possibly empty).
    pub items: Vec<ImplItem>,
    /// Span covering the whole block.
    pub span: Span,
}

/// An item inside a trait block.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitItem {
    /// The signature (bodies are not part of the accepted trait syntax).
    pub sig: FnSig,
    /// Span covering the item.
    pub span: Span,
}

/// A trait declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    /// Declared visibility.
    pub vis: Visibility,
    /// The trait name.
    pub name: Ident,
    /// Generic parameters, when written.
    pub generics: Option<Generics>,
    /// The items in order (possibly empty).
    pub items: Vec<TraitItem>,
    /// Span covering the whole declaration.
    pub span: Span,
}

/// An import declaration (`import std.io as io;`).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    /// The dotted module path segments (at least one after a successful
    /// parse).
    pub path: Vec<Ident>,
    /// The alias, when written.
    pub alias: Option<Ident>,
    /// Span covering the whole declaration.
    pub span: Span,
}

/// The shape of an item.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// A function.
    Fn(FnDecl),
    /// A struct.
    Struct(StructDecl),
    /// An enum.
    Enum(EnumDecl),
    /// An `impl` block.
    Impl(ImplDecl),
    /// A trait declaration.
    Trait(TraitDecl),
    /// An import.
    Import(ImportDecl),
}

/// Top-level item node (the root of a parsed file is a sequence of items).
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Parser-assigned identifier (instance-scoped).
    pub id: ItemId,
    /// The item's shape.
    pub kind: ItemKind,
    /// Span covering the whole item.
    pub span: Span,
}

/// A `let` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    /// Whether `mut` was written.
    pub mutable: bool,
    /// The bound pattern.
    pub pat: Pat,
    /// The type annotation, when written.
    pub ty: Option<Type>,
    /// The initializer (required by the accepted grammar).
    pub init: Expr,
    /// Span covering the statement.
    pub span: Span,
}

/// The shape of a statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// A `let` declaration.
    Let(LetStmt),
    /// An expression statement.
    Expr(Expr),
    /// A nested item.
    Item(Item),
}

/// A statement node with exact span.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// The statement's shape.
    pub kind: StmtKind,
    /// Span covering the statement (including a terminating `;` when the
    /// parser includes it).
    pub span: Span,
}

/// A brace-delimited block: statements plus an optional tail expression.
///
/// The tail expression is a separate field by construction: a block's value
/// is exactly `tail`, and statements (including semicolon-terminated
/// block-like statements) never double as the tail. The canonical printer
/// preserves this distinction (see [`crate::print`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The statements in order.
    pub stmts: Vec<Stmt>,
    /// The tail (value) expression, when present.
    pub tail: Option<Box<Expr>>,
    /// Span covering the braces and contents.
    pub span: Span,
}
