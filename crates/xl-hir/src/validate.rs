//! Structural HIR validation.
//!
//! The validator checks structural invariants only: arena index bounds,
//! entity identity consistency, parent/child kind validity, span sanity
//! within the available context (the module's file), origin presence and
//! well-formedness, unique occurrence keys, block composition, loop-target
//! correctness, import-record consistency, literal safety, nesting depth,
//! and the ratified resource budgets. It performs no name resolution, no
//! typing, and no exhaustiveness analysis; error and placeholder nodes are
//! expected inputs and never fail validation by themselves.
//!
//! Violations are returned as data, never panics; deliberately malformed
//! modules (hand-built values) are rejected. Every module produced by
//! [`crate::lower_structure`] with [`crate::StageFate::Complete`] passes
//! this validator within the ratified budgets.
//!
//! # Depth convention
//!
//! Nesting counts expressions, statements, patterns, and type references.
//! Bodies and item slots are structural glue and are not counted. Error
//! nodes (and the statement wrapper around an error expression) are poison
//! markers and are not counted either, so a module lowered at the exact
//! limit boundary validates.

use std::collections::HashSet;
use std::fmt;

use xl_span::Span;

use crate::hir::{
    Body, BodyId, DefId, DefKind, ExprKind, FieldSel, FnSigId, HirId, HirItemId, HirItemKind,
    HirKind, HirLit, HirModule, PatKind, Resolution, Resolutions, StmtKind, TypeRefId, TypeRefKind,
    UseId,
};
use crate::limits::LowerLimits;
use crate::origin::{AstOrigin, OccurrenceKey};

/// Category of a [`Violation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// A referenced identifier's arena index is out of bounds.
    IndexOutOfBounds,
    /// An entity's stored identity does not match its arena slot.
    EntityIdMismatch,
    /// A reference has the wrong node kind (for example a body where an
    /// expression is required).
    KindMismatch,
    /// An entity has no recorded AST origin (the origin table is shorter
    /// than its arena).
    OriginMissing,
    /// A structural origin key is malformed (an empty child path).
    OriginMalformed,
    /// Two distinct nodes share a source occurrence key.
    DuplicateNodeOccurrence,
    /// Two distinct synthesized entities share a complete occurrence key.
    DuplicateSynthesizedKey,
    /// A synthesized use has no established resolution.
    SynthesizedUsePending,
    /// A span is inverted (`lo > hi`).
    SpanInverted,
    /// A span refers to a different file than the module.
    SpanFileMismatch,
    /// A definition's parent identity or parent kind is inconsistent.
    ParentKindMismatch,
    /// A reserved definition slot was never filled (only possible in an
    /// aborted, unusable module).
    UnfilledDefinition,
    /// A floating-point literal is not finite.
    NonFiniteFloat,
    /// A `break`/`continue` target does not denote an enclosing loop in
    /// the same function body.
    LoopTargetNotEnclosing,
    /// The nesting chain exceeds the ratified depth budget.
    DepthExceeded,
    /// A ratified resource budget is exceeded.
    BudgetExceeded,
    /// An import record does not match its item, or is missing/duplicated.
    ImportRecordMismatch,
    /// A module binding's recorded name does not match its alias or last
    /// path segment.
    ModuleBindingNameMismatch,
    /// A resolution table's length does not match the use table.
    ResolutionLengthMismatch,
    /// A node, body, type, or definition recursively references itself.
    CyclicReference,
    /// An arena entry cannot be reached from any module item or definition.
    UnreachableArenaEntry,
}

/// A structural violation with its location and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The violation category.
    pub kind: ViolationKind,
    /// The offending span, when one applies.
    pub span: Option<Span>,
    /// A human-readable description.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Validates one module with the ratified budgets (`P2D-A7`).
#[must_use]
pub fn validate(module: &HirModule) -> Vec<Violation> {
    validate_with_limits(module, LowerLimits::RATIFIED)
}

/// Validates one module with explicit budgets.
#[must_use]
pub fn validate_with_limits(module: &HirModule, limits: LowerLimits) -> Vec<Violation> {
    let limits = limits.capped();
    // Validate the full ratified nesting budget even when called on a
    // 1-MiB stack (notably Windows test threads). This also covers
    // hand-built malformed HIR independent of the parser's lower bound.
    stacker::maybe_grow(1536 * 1024, 8 * 1024 * 1024, || {
        Validator::new(module, limits).run()
    })
}

/// Validates a resolution side table against its module.
///
/// Checks: one resolution per use, no pending resolution on a synthesized
/// use (internal references are established at creation), and in-bounds
/// definition references.
#[must_use]
pub fn validate_resolutions(module: &HirModule, resolutions: &Resolutions) -> Vec<Violation> {
    let mut violations = Vec::new();
    if resolutions.uses.len() != module.uses.uses.len() {
        violations.push(Violation {
            kind: ViolationKind::ResolutionLengthMismatch,
            span: None,
            message: format!(
                "resolution table has {} entries for {} uses",
                resolutions.uses.len(),
                module.uses.uses.len()
            ),
        });
    }
    for (index, resolution) in resolutions.uses.iter().enumerate() {
        let span = module.uses.uses.get(index).map(|use_| use_.span);
        let synthesized = module
            .origins
            .uses
            .get(index)
            .is_some_and(AstOrigin::is_desugared);
        match resolution {
            Resolution::Unresolved(_) if synthesized => violations.push(Violation {
                kind: ViolationKind::SynthesizedUsePending,
                span,
                message: format!("synthesized use use#{index} has no established resolution"),
            }),
            Resolution::Def(def) => {
                if module.defs.get(*def).is_none() {
                    violations.push(Violation {
                        kind: ViolationKind::IndexOutOfBounds,
                        span,
                        message: format!(
                            "resolution of use#{index} references an unknown definition"
                        ),
                    });
                }
            }
            Resolution::Global(global)
                if global.module == module.module
                    && module.defs.get(global.definition).is_none() =>
            {
                violations.push(Violation {
                    kind: ViolationKind::IndexOutOfBounds,
                    span,
                    message: format!(
                        "resolution of use#{index} references an unknown definition in this module"
                    ),
                });
            }
            _ => {}
        }
    }
    violations
}

/// The expected node role at a reference site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// An expression node.
    Expr,
    /// A statement node.
    Stmt,
    /// A pattern node.
    Pat,
}

struct Validator<'a> {
    module: &'a HirModule,
    limits: LowerLimits,
    violations: Vec<Violation>,
    visited_nodes: Vec<bool>,
    visited_bodies: Vec<bool>,
    visited_types: Vec<bool>,
    visited_sigs: Vec<bool>,
    visited_defs: Vec<bool>,
    active_nodes: Vec<bool>,
    active_bodies: Vec<bool>,
    active_types: Vec<bool>,
    active_defs: Vec<bool>,
}

impl<'a> Validator<'a> {
    fn new(module: &'a HirModule, limits: LowerLimits) -> Self {
        Self {
            module,
            limits,
            violations: Vec::new(),
            visited_nodes: vec![false; module.nodes.nodes.len()],
            visited_bodies: vec![false; module.bodies.bodies.len()],
            visited_types: vec![false; module.types.types.len()],
            visited_sigs: vec![false; module.sigs.sigs.len()],
            visited_defs: vec![false; module.defs.defs.len()],
            active_nodes: vec![false; module.nodes.nodes.len()],
            active_bodies: vec![false; module.bodies.bodies.len()],
            active_types: vec![false; module.types.types.len()],
            active_defs: vec![false; module.defs.defs.len()],
        }
    }

    fn run(mut self) -> Vec<Violation> {
        self.check_resource_budgets();
        self.check_origin_tables();
        self.check_occurrence_keys();
        self.check_table_identities();
        self.check_spans();
        self.check_items();
        self.check_imports();
        self.check_defs();
        self.walk_roots();
        self.violations
    }

    fn violation(&mut self, kind: ViolationKind, span: Option<Span>, message: impl Into<String>) {
        self.violations.push(Violation {
            kind,
            span,
            message: message.into(),
        });
    }

    // ------------------------------------------------------------------
    // Table-level checks
    // ------------------------------------------------------------------

    fn check_resource_budgets(&mut self) {
        if self.module.nodes.nodes.len() > self.limits.max_hir_nodes as usize {
            self.violation(
                ViolationKind::BudgetExceeded,
                None,
                format!(
                    "module holds {} HIR nodes (limit {})",
                    self.module.nodes.nodes.len(),
                    self.limits.max_hir_nodes
                ),
            );
        }
        if self.module.defs.defs.len() > self.limits.max_defs as usize {
            self.violation(
                ViolationKind::BudgetExceeded,
                None,
                format!(
                    "module holds {} definitions (limit {})",
                    self.module.defs.defs.len(),
                    self.limits.max_defs
                ),
            );
        }
        if self.module.uses.uses.len() > self.limits.max_uses as usize {
            self.violation(
                ViolationKind::BudgetExceeded,
                None,
                format!(
                    "module holds {} uses (limit {})",
                    self.module.uses.uses.len(),
                    self.limits.max_uses
                ),
            );
        }
    }

    fn check_origin_tables(&mut self) {
        let table = &self.module.origins;
        let lengths = [
            ("items", table.items.len(), self.module.items.len()),
            ("defs", table.defs.len(), self.module.defs.defs.len()),
            ("uses", table.uses.len(), self.module.uses.uses.len()),
            (
                "bodies",
                table.bodies.len(),
                self.module.bodies.bodies.len(),
            ),
            ("nodes", table.nodes.len(), self.module.nodes.nodes.len()),
            ("types", table.types.len(), self.module.types.types.len()),
        ];
        for (name, origins, entities) in lengths {
            if origins != entities {
                self.violation(
                    ViolationKind::OriginMissing,
                    None,
                    format!(
                        "origin table for {name} holds {origins} entries for {entities} entities"
                    ),
                );
            }
        }
        for (index, origin) in self.module.origins.nodes.iter().enumerate() {
            if let AstOrigin::Structural { path, .. } = origin
                && path.is_empty()
            {
                self.violation(
                    ViolationKind::OriginMalformed,
                    self.module.nodes.nodes.get(index).map(|node| node.span),
                    format!("node hir#{index} has an empty structural path"),
                );
            }
        }
    }

    fn check_occurrence_keys(&mut self) {
        // A depth-poisoned statement is a pair (statement wrapper, inner
        // error expression) that deliberately shares one source construct;
        // both halves are excluded from node-occurrence uniqueness.
        let mut poison_markers = vec![false; self.module.nodes.nodes.len()];
        for (index, node) in self.module.nodes.nodes.iter().enumerate() {
            if let HirKind::Stmt(StmtKind::Expr(inner)) = &node.kind
                && matches!(self.module.nodes.expr(*inner), Some(ExprKind::Error))
            {
                poison_markers[index] = true;
                if let Some(inner) = poison_markers.get_mut(inner.index() as usize) {
                    *inner = true;
                }
            }
        }
        let mut node_keys = HashSet::new();
        for (index, origin) in self.module.origins.nodes.iter().enumerate() {
            if poison_markers.get(index).copied().unwrap_or(false) {
                continue;
            }
            let key = origin.occurrence_key();
            if let OccurrenceKey::Synthesized { .. } = key {
                continue;
            }
            if !node_keys.insert(key) {
                self.violation(
                    ViolationKind::DuplicateNodeOccurrence,
                    self.module.nodes.nodes.get(index).map(|node| node.span),
                    format!("node hir#{index} shares a source occurrence key"),
                );
            }
        }
        // Synthesized occurrence keys must be unique across every arena.
        let mut synthesized: Vec<(OccurrenceKey, Option<Span>, String)> = Vec::new();
        for (index, origin) in self.module.origins.defs.iter().enumerate() {
            synthesized.push((
                origin.occurrence_key(),
                self.module.defs.defs.get(index).map(|def| def.span),
                format!("definition def#{index}"),
            ));
        }
        for (index, origin) in self.module.origins.uses.iter().enumerate() {
            synthesized.push((
                origin.occurrence_key(),
                self.module.uses.uses.get(index).map(|use_| use_.span),
                format!("use use#{index}"),
            ));
        }
        for (index, origin) in self.module.origins.bodies.iter().enumerate() {
            synthesized.push((origin.occurrence_key(), None, format!("body {index}")));
        }
        for (index, origin) in self.module.origins.nodes.iter().enumerate() {
            synthesized.push((
                origin.occurrence_key(),
                self.module.nodes.nodes.get(index).map(|node| node.span),
                format!("node hir#{index}"),
            ));
        }
        for (index, origin) in self.module.origins.types.iter().enumerate() {
            synthesized.push((
                origin.occurrence_key(),
                self.module.types.types.get(index).map(|ty| ty.span),
                format!("type reference type#{index}"),
            ));
        }
        for (index, origin) in self.module.origins.items.iter().enumerate() {
            synthesized.push((
                origin.occurrence_key(),
                self.module.items.get(index).map(|item| item.span),
                format!("item {index}"),
            ));
        }
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();
        for (key, span, what) in synthesized {
            if let OccurrenceKey::Synthesized { .. } = key
                && !seen.insert(key)
            {
                duplicates.push((span, what));
            }
        }
        for (span, what) in duplicates {
            self.violation(
                ViolationKind::DuplicateSynthesizedKey,
                span,
                format!("{what} shares a synthesized occurrence key"),
            );
        }
    }

    fn check_table_identities(&mut self) {
        for (index, item) in self.module.items.iter().enumerate() {
            if item.id.index() as usize != index {
                self.violation(
                    ViolationKind::EntityIdMismatch,
                    Some(item.span),
                    format!("item slot {index} stores id {}", item.id),
                );
            }
        }
        for (index, def) in self.module.defs.defs.iter().enumerate() {
            if def.id.index() as usize != index {
                self.violation(
                    ViolationKind::EntityIdMismatch,
                    Some(def.span),
                    format!("definition slot {index} stores id {}", def.id),
                );
            }
        }
        for (index, node) in self.module.nodes.nodes.iter().enumerate() {
            if node.id.index() as usize != index {
                self.violation(
                    ViolationKind::EntityIdMismatch,
                    Some(node.span),
                    format!("node slot {index} stores id {}", node.id),
                );
            }
        }
        for (index, type_ref) in self.module.types.types.iter().enumerate() {
            if type_ref.id.index() as usize != index {
                self.violation(
                    ViolationKind::EntityIdMismatch,
                    Some(type_ref.span),
                    format!("type slot {index} stores id {}", type_ref.id),
                );
            }
        }
    }

    fn check_spans(&mut self) {
        let module_file = self.module.file;
        let check = |violations: &mut Vec<Violation>, span: Span, what: &str| {
            if span.lo > span.hi {
                violations.push(Violation {
                    kind: ViolationKind::SpanInverted,
                    span: Some(span),
                    message: format!("{what} has an inverted span"),
                });
            }
            if span.file != module_file {
                violations.push(Violation {
                    kind: ViolationKind::SpanFileMismatch,
                    span: Some(span),
                    message: format!("{what} refers to a different file than the module"),
                });
            }
        };
        for (index, node) in self.module.nodes.nodes.iter().enumerate() {
            check(
                &mut self.violations,
                node.span,
                &format!("node hir#{index}"),
            );
        }
        for (index, def) in self.module.defs.defs.iter().enumerate() {
            check(
                &mut self.violations,
                def.span,
                &format!("definition def#{index}"),
            );
        }
        for (index, use_) in self.module.uses.uses.iter().enumerate() {
            check(&mut self.violations, use_.span, &format!("use use#{index}"));
        }
        for (index, item) in self.module.items.iter().enumerate() {
            check(
                &mut self.violations,
                item.span,
                &format!("item slot {index}"),
            );
        }
        for (index, type_ref) in self.module.types.types.iter().enumerate() {
            check(
                &mut self.violations,
                type_ref.span,
                &format!("type reference type#{index}"),
            );
        }
        for (index, sig) in self.module.sigs.sigs.iter().enumerate() {
            check(
                &mut self.violations,
                sig.span,
                &format!("signature sig#{index}"),
            );
        }
    }

    fn check_items(&mut self) {
        for item in &self.module.items {
            let def = self.module.defs.get(item.def);
            let consistent = match (item.kind, def) {
                (_, None) => false,
                (HirItemKind::Fn, Some(def)) => matches!(def.kind, DefKind::Fn { .. }),
                (HirItemKind::Struct, Some(def)) => matches!(def.kind, DefKind::Struct { .. }),
                (HirItemKind::Enum, Some(def)) => matches!(def.kind, DefKind::Enum { .. }),
                (HirItemKind::Trait, Some(def)) => matches!(def.kind, DefKind::Trait { .. }),
                (HirItemKind::Impl, Some(def)) => matches!(def.kind, DefKind::Impl { .. }),
                (HirItemKind::Import, Some(def)) => {
                    matches!(def.kind, DefKind::ModuleBinding(_))
                }
            };
            if !consistent {
                self.violation(
                    ViolationKind::KindMismatch,
                    Some(item.span),
                    format!("item {} does not match its definition", item.id),
                );
            }
        }
    }

    fn check_imports(&mut self) {
        let mut seen: Vec<HirItemId> = Vec::new();
        for record in &self.module.imports {
            let mut consistent = true;
            match self.module.item(record.item) {
                None => consistent = false,
                Some(item) => {
                    if item.def != record.binding || item.kind != HirItemKind::Import {
                        consistent = false;
                    }
                    if item.span != record.decl_span {
                        consistent = false;
                    }
                }
            }
            if seen.contains(&record.item) {
                consistent = false;
            }
            seen.push(record.item);
            match self.module.defs.get(record.binding) {
                None => consistent = false,
                Some(def) => match &def.kind {
                    DefKind::ModuleBinding(binding) => {
                        if binding.span != record.decl_span {
                            consistent = false;
                        }
                        if record.path.len() != binding.path.len()
                            || !record.path.iter().zip(&binding.path).all(|(id, segment)| {
                                self.module
                                    .uses
                                    .uses
                                    .get(id.index() as usize)
                                    .is_some_and(|use_| use_.name == *segment)
                            })
                        {
                            consistent = false;
                        }
                        match (&binding.alias, def.name.as_str(), binding.path.last()) {
                            (Some(alias), name, _) if name != alias => {
                                self.violation(
                                    ViolationKind::ModuleBindingNameMismatch,
                                    Some(def.span),
                                    "module binding name does not match its alias",
                                );
                            }
                            (None, name, Some(last)) if !last.is_empty() && name != last => {
                                self.violation(
                                    ViolationKind::ModuleBindingNameMismatch,
                                    Some(def.span),
                                    "module binding name does not match its last path segment",
                                );
                            }
                            _ => {}
                        }
                    }
                    _ => consistent = false,
                },
            }
            if !consistent {
                self.violation(
                    ViolationKind::ImportRecordMismatch,
                    Some(record.decl_span),
                    "import record does not match a unique import item and binding",
                );
            }
        }
        for item in &self.module.items {
            if item.kind == HirItemKind::Import
                && !self
                    .module
                    .imports
                    .iter()
                    .any(|record| record.item == item.id)
            {
                self.violation(
                    ViolationKind::ImportRecordMismatch,
                    Some(item.span),
                    format!("import item {} has no record", item.id),
                );
            }
        }
    }

    fn check_defs(&mut self) {
        for index in 0..self.module.defs.defs.len() {
            let def = &self.module.defs.defs[index];
            if matches!(def.kind, DefKind::Error) {
                self.violation(
                    ViolationKind::UnfilledDefinition,
                    Some(def.span),
                    format!("definition def#{index} was never completed"),
                );
            }
            if let Some(parent_id) = def.parent
                && self.module.defs.get(parent_id).is_none()
            {
                self.violation(
                    ViolationKind::IndexOutOfBounds,
                    Some(def.span),
                    format!("definition def#{index} has an out-of-bounds parent"),
                );
            }
            let parent_kind = def
                .parent
                .and_then(|parent| self.module.defs.get(parent))
                .map(|parent| &parent.kind);
            let parent_ok = match &def.kind {
                DefKind::Variant { .. } => {
                    matches!(parent_kind, Some(DefKind::Enum { .. }))
                }
                DefKind::Field { .. } => {
                    matches!(parent_kind, Some(DefKind::Struct { .. }))
                }
                DefKind::Method { .. } => {
                    matches!(parent_kind, Some(DefKind::Impl { .. }))
                }
                DefKind::TraitItem { .. } => {
                    matches!(parent_kind, Some(DefKind::Trait { .. }))
                }
                DefKind::GenericParam { .. } | DefKind::Param { .. } | DefKind::Receiver { .. } => {
                    matches!(
                        parent_kind,
                        Some(
                            DefKind::Fn { .. }
                                | DefKind::Method { .. }
                                | DefKind::TraitItem { .. }
                                | DefKind::Struct { .. }
                                | DefKind::Enum { .. }
                                | DefKind::Trait { .. }
                                | DefKind::Impl { .. }
                                | DefKind::Variant { .. }
                        )
                    )
                }
                _ => def.parent.is_none(),
            };
            if !parent_ok {
                self.violation(
                    ViolationKind::ParentKindMismatch,
                    Some(def.span),
                    format!("definition def#{index} has an inconsistent parent"),
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Reference checks
    // ------------------------------------------------------------------

    fn node(&mut self, id: HirId, expect: Expect, at: Option<Span>) -> Option<&'a HirKind> {
        let module = self.module;
        match module.nodes.nodes.get(id.index() as usize) {
            None => {
                self.violation(
                    ViolationKind::IndexOutOfBounds,
                    at,
                    format!("node {id} is out of bounds"),
                );
                None
            }
            Some(node) => {
                let ok = matches!(
                    (expect, &node.kind),
                    (Expect::Expr, HirKind::Expr(_))
                        | (Expect::Stmt, HirKind::Stmt(_))
                        | (Expect::Pat, HirKind::Pat(_))
                );
                if !ok {
                    self.violation(
                        ViolationKind::KindMismatch,
                        at.or(Some(node.span)),
                        format!("node {id} does not have the expected node kind"),
                    );
                    None
                } else {
                    Some(&node.kind)
                }
            }
        }
    }

    fn body(&mut self, id: BodyId, at: Option<Span>) -> Option<&'a Body> {
        let module = self.module;
        match module.bodies.bodies.get(id.index() as usize) {
            None => {
                self.violation(
                    ViolationKind::IndexOutOfBounds,
                    at,
                    format!("body {id} is out of bounds"),
                );
                None
            }
            Some(body) => Some(body),
        }
    }

    fn use_(&mut self, id: UseId, at: Option<Span>) {
        if self.module.uses.uses.get(id.index() as usize).is_none() {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                at,
                format!("use {id} is out of bounds"),
            );
        }
    }

    fn walk_literal(&mut self, lit: &HirLit, at: Option<Span>) {
        if let HirLit::Float(value) = lit
            && !value.is_finite()
        {
            self.violation(
                ViolationKind::NonFiniteFloat,
                at,
                "floating-point literal is not finite",
            );
        }
    }

    // ------------------------------------------------------------------
    // Root walking
    // ------------------------------------------------------------------

    fn walk_roots(&mut self) {
        for item_index in 0..self.module.items.len() {
            let item_id = self.module.items[item_index].id;
            self.walk_item(item_id, 0);
        }
        for def_index in 0..self.module.defs.defs.len() {
            if !self.visited_defs[def_index] {
                self.walk_def(DefId::from_index(def_index as u32), 0);
            }
        }
        // Invalid, disconnected entries still belong to the arena. Check
        // both their reachability and their references rather than silently
        // treating a successful root traversal as complete validation.
        for index in 0..self.module.nodes.nodes.len() {
            if !self.visited_nodes[index] {
                let node = &self.module.nodes.nodes[index];
                self.violation(
                    ViolationKind::UnreachableArenaEntry,
                    Some(node.span),
                    format!("node hir#{index} is not reachable from a module root"),
                );
                let expect = match node.kind {
                    HirKind::Expr(_) => Expect::Expr,
                    HirKind::Stmt(_) => Expect::Stmt,
                    HirKind::Pat(_) => Expect::Pat,
                };
                self.walk_node(HirId::from_index(index as u32), expect, 0, &mut Vec::new());
            }
        }
        for index in 0..self.module.bodies.bodies.len() {
            if !self.visited_bodies[index] {
                self.violation(
                    ViolationKind::UnreachableArenaEntry,
                    None,
                    format!("body {index} is not reachable from a module root"),
                );
                self.walk_body(BodyId::from_index(index as u32), 0, &mut Vec::new());
            }
        }
        for index in 0..self.module.types.types.len() {
            if !self.visited_types[index] {
                self.violation(
                    ViolationKind::UnreachableArenaEntry,
                    Some(self.module.types.types[index].span),
                    format!("type reference type#{index} is not reachable from a module root"),
                );
                self.walk_type(TypeRefId::from_index(index as u32), 0);
            }
        }
        for index in 0..self.module.sigs.sigs.len() {
            if !self.visited_sigs[index] {
                self.violation(
                    ViolationKind::UnreachableArenaEntry,
                    Some(self.module.sigs.sigs[index].span),
                    format!("signature sig#{index} is not reachable from a module root"),
                );
                self.walk_signature(FnSigId::from_index(index as u32), &[], 0);
            }
        }
    }

    fn walk_item(&mut self, item: HirItemId, parent_depth: u32) {
        let Some(item) = self.module.item(item) else {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                None,
                "item reference is out of bounds",
            );
            return;
        };
        let def = item.def;
        self.walk_def(def, parent_depth);
    }

    /// Walks one definition. Items and definitions are structural glue and
    /// do not contribute to the nesting chain.
    fn walk_def(&mut self, id: DefId, parent_depth: u32) {
        let index = id.index() as usize;
        if index >= self.module.defs.defs.len() {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                None,
                format!("definition {id} is out of bounds"),
            );
            return;
        }
        if self.active_defs[index] {
            self.violation(
                ViolationKind::CyclicReference,
                None,
                format!("definition {id} forms a cycle"),
            );
            return;
        }
        if self.visited_defs[index] {
            return;
        }
        self.visited_defs[index] = true;
        self.active_defs[index] = true;
        let kind = self.module.defs.defs[index].kind.clone();
        let span = Some(self.module.defs.defs[index].span);
        match kind {
            DefKind::Fn {
                generics,
                sig,
                body,
            }
            | DefKind::Method {
                generics,
                sig,
                body,
            } => {
                self.walk_generics(&generics, parent_depth);
                self.walk_signature(sig, &generics, parent_depth);
                if let Some(body) = body {
                    let mut loops = Vec::new();
                    self.walk_body(body, parent_depth, &mut loops);
                }
            }
            DefKind::Struct { generics, fields } => {
                self.walk_generics(&generics, parent_depth);
                for field in fields {
                    self.walk_def(field, parent_depth);
                }
            }
            DefKind::Enum { generics, variants } => {
                self.walk_generics(&generics, parent_depth);
                for variant in variants {
                    self.walk_def(variant, parent_depth);
                }
            }
            DefKind::Trait { generics, items } => {
                self.walk_generics(&generics, parent_depth);
                for item in items {
                    self.walk_def(item, parent_depth);
                }
            }
            DefKind::Impl {
                generics,
                trait_ref,
                self_ty,
                ..
            } => {
                self.walk_generics(&generics, parent_depth);
                if let Some(trait_ref) = trait_ref {
                    self.walk_type(trait_ref, parent_depth);
                }
                self.walk_type(self_ty, parent_depth);
            }
            DefKind::Variant { payload } => {
                for ty in payload {
                    self.walk_type(ty, parent_depth);
                }
            }
            DefKind::Field { ty } => {
                self.walk_type(ty, parent_depth);
            }
            DefKind::TraitItem { sig } => {
                self.walk_signature(sig, &[], parent_depth);
            }
            DefKind::GenericParam { bound } => {
                if let Some(bound) = bound {
                    self.walk_type(bound, parent_depth);
                }
            }
            DefKind::Param { ty, .. } => {
                self.walk_type(ty, parent_depth);
            }
            DefKind::Receiver { .. } | DefKind::ModuleBinding(_) | DefKind::Error => {}
            DefKind::Local { ty, body, .. } => {
                if let Some(ty) = ty {
                    self.walk_type(ty, parent_depth);
                }
                self.body(body, span);
            }
        }
        self.active_defs[index] = false;
    }

    fn walk_generics(&mut self, generics: &[DefId], parent_depth: u32) {
        for generic in generics {
            self.walk_def(*generic, parent_depth);
        }
    }

    fn walk_signature(&mut self, sig: FnSigId, generics: &[DefId], parent_depth: u32) {
        let _ = generics;
        let index = sig.index() as usize;
        let Some(signature) = self.module.sig(sig) else {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                None,
                format!("signature {sig} is out of bounds"),
            );
            return;
        };
        if self.visited_sigs[index] {
            return;
        }
        self.visited_sigs[index] = true;
        if self.module.defs.get(signature.name).is_none() {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                Some(signature.span),
                format!("signature {sig} references an unknown owner"),
            );
        }
        let params = signature.params.clone();
        let ret = signature.ret;
        for param in params {
            self.walk_def(param, parent_depth);
        }
        if let Some(ret) = ret {
            self.walk_type(ret, parent_depth);
        }
    }

    fn walk_body(&mut self, id: BodyId, parent_depth: u32, loops: &mut Vec<HirId>) {
        let index = id.index() as usize;
        if index >= self.module.bodies.bodies.len() {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                None,
                format!("body {id} is out of bounds"),
            );
            return;
        }
        if self.active_bodies[index] {
            self.violation(
                ViolationKind::CyclicReference,
                None,
                format!("body {id} forms a cycle"),
            );
            return;
        }
        if self.visited_bodies[index] {
            return;
        }
        self.visited_bodies[index] = true;
        self.active_bodies[index] = true;
        let module = self.module;
        let body = &module.bodies.bodies[index];
        for stmt in &body.stmts {
            self.walk_node(*stmt, Expect::Stmt, parent_depth, loops);
        }
        if let Some(tail) = body.tail {
            self.walk_node(tail, Expect::Expr, parent_depth, loops);
        }
        self.active_bodies[index] = false;
    }

    /// Walks one node. `parent_depth` is the depth of the enclosing
    /// counted chain; the node itself occupies `parent_depth + 1`.
    fn walk_node(&mut self, id: HirId, expect: Expect, parent_depth: u32, loops: &mut Vec<HirId>) {
        let Some(kind) = self.node(id, expect, None) else {
            return;
        };
        // Poison markers (error nodes and statement wrappers around them)
        // are leaves and do not contribute to the nesting chain.
        let poison = match kind {
            HirKind::Expr(ExprKind::Error) | HirKind::Pat(PatKind::Error) => true,
            HirKind::Stmt(StmtKind::Expr(inner)) => {
                matches!(self.module.nodes.expr(*inner), Some(ExprKind::Error) | None)
            }
            _ => false,
        };
        if poison {
            self.visited_nodes[id.index() as usize] = true;
            if let HirKind::Stmt(StmtKind::Expr(inner)) = kind {
                self.walk_node(*inner, Expect::Expr, parent_depth, loops);
            }
            return;
        }
        let index = id.index() as usize;
        let my_depth = parent_depth + 1;
        if my_depth > self.limits.max_hir_depth {
            self.violation(
                ViolationKind::DepthExceeded,
                None,
                format!(
                    "nesting depth exceeds the limit ({})",
                    self.limits.max_hir_depth
                ),
            );
            return;
        }
        if self.active_nodes[index] {
            self.violation(
                ViolationKind::CyclicReference,
                None,
                format!("node {id} forms a cycle"),
            );
            return;
        }
        if self.visited_nodes[index] {
            return;
        }
        self.visited_nodes[index] = true;
        self.active_nodes[index] = true;
        match kind {
            HirKind::Expr(kind) => self.walk_expr(id, kind, my_depth, loops),
            HirKind::Stmt(kind) => match kind {
                StmtKind::Let { pattern, init } => {
                    self.walk_node(*pattern, Expect::Pat, my_depth, loops);
                    self.walk_node(*init, Expect::Expr, my_depth, loops);
                }
                StmtKind::Expr(expr) => {
                    self.walk_node(*expr, Expect::Expr, my_depth, loops);
                }
                StmtKind::Item(item) => {
                    // Nested items are separate declaration contexts: they
                    // do not extend the chain and do not inherit loop
                    // targets.
                    self.walk_item(*item, my_depth);
                }
            },
            HirKind::Pat(kind) => match kind {
                PatKind::Wildcard | PatKind::Error => {}
                PatKind::Binding { local, candidate } => {
                    if self.module.defs.get(*local).is_none() {
                        self.violation(
                            ViolationKind::IndexOutOfBounds,
                            None,
                            format!("pattern binding references unknown definition {local}"),
                        );
                    }
                    self.use_(*candidate, None);
                }
                PatKind::Lit(lit) => self.walk_literal(lit, None),
                PatKind::Variant { path, args } => {
                    for use_ in path {
                        self.use_(*use_, None);
                    }
                    for arg in args {
                        self.walk_node(*arg, Expect::Pat, my_depth, loops);
                    }
                }
                PatKind::Tuple(elements) => {
                    for element in elements {
                        self.walk_node(*element, Expect::Pat, my_depth, loops);
                    }
                }
            },
        }
        self.active_nodes[index] = false;
    }

    fn walk_expr(
        &mut self,
        node_id: HirId,
        kind: &ExprKind,
        my_depth: u32,
        loops: &mut Vec<HirId>,
    ) {
        match kind {
            ExprKind::Lit(lit) => self.walk_literal(lit, None),
            ExprKind::Path { segments } => {
                for use_ in segments {
                    self.use_(*use_, None);
                }
            }
            ExprKind::Unary(_, operand) => {
                self.walk_node(*operand, Expect::Expr, my_depth, loops);
            }
            ExprKind::Binary(_, left, right) => {
                self.walk_node(*left, Expect::Expr, my_depth, loops);
                self.walk_node(*right, Expect::Expr, my_depth, loops);
            }
            ExprKind::Assign(_, target, value) => {
                self.walk_node(*target, Expect::Expr, my_depth, loops);
                self.walk_node(*value, Expect::Expr, my_depth, loops);
            }
            ExprKind::Call(callee, args) => {
                self.walk_node(*callee, Expect::Expr, my_depth, loops);
                for arg in args {
                    self.walk_node(*arg, Expect::Expr, my_depth, loops);
                }
            }
            ExprKind::Field(base, selection) => {
                self.walk_node(*base, Expect::Expr, my_depth, loops);
                if let FieldSel::Unresolved { name } = selection {
                    self.use_(*name, None);
                }
            }
            ExprKind::Index(base, index) => {
                self.walk_node(*base, Expect::Expr, my_depth, loops);
                self.walk_node(*index, Expect::Expr, my_depth, loops);
            }
            ExprKind::Cast(inner, ty) => {
                self.walk_node(*inner, Expect::Expr, my_depth, loops);
                self.walk_type(*ty, my_depth);
            }
            ExprKind::If(cond, then_body, else_expr) => {
                self.walk_node(*cond, Expect::Expr, my_depth, loops);
                self.walk_body(*then_body, my_depth, loops);
                if let Some(else_expr) = else_expr {
                    self.walk_node(*else_expr, Expect::Expr, my_depth, loops);
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.walk_node(*scrutinee, Expect::Expr, my_depth, loops);
                for arm in arms {
                    self.walk_node(arm.pat, Expect::Pat, my_depth, loops);
                    self.walk_node(arm.body, Expect::Expr, my_depth, loops);
                }
            }
            ExprKind::Block(body) => self.walk_body(*body, my_depth, loops),
            ExprKind::While(cond, body) => {
                self.walk_node(*cond, Expect::Expr, my_depth, loops);
                loops.push(node_id);
                self.walk_body(*body, my_depth, loops);
                loops.pop();
            }
            ExprKind::Loop(body) => {
                loops.push(node_id);
                self.walk_body(*body, my_depth, loops);
                loops.pop();
            }
            ExprKind::Return(operand) => {
                if let Some(operand) = operand {
                    self.walk_node(*operand, Expect::Expr, my_depth, loops);
                }
            }
            ExprKind::Break { target } | ExprKind::Continue { target } => {
                if !loops.contains(target) {
                    self.violation(
                        ViolationKind::LoopTargetNotEnclosing,
                        None,
                        format!("{target} is not an enclosing loop in the same body"),
                    );
                }
            }
            ExprKind::Tuple(elements) | ExprKind::Array(elements) => {
                for element in elements {
                    self.walk_node(*element, Expect::Expr, my_depth, loops);
                }
            }
            ExprKind::Error => {}
            ExprKind::BuiltinCall { args, .. } => {
                for arg in args {
                    self.walk_node(*arg, Expect::Expr, my_depth, loops);
                }
            }
        }
    }

    fn walk_type(&mut self, id: TypeRefId, parent_depth: u32) {
        let index = id.index() as usize;
        if index >= self.module.types.types.len() {
            self.violation(
                ViolationKind::IndexOutOfBounds,
                None,
                format!("type reference {id} is out of bounds"),
            );
            return;
        }
        if matches!(self.module.types.types[index].kind, TypeRefKind::Error) {
            self.visited_types[index] = true;
            return;
        }
        let my_depth = parent_depth + 1;
        if my_depth > self.limits.max_hir_depth {
            self.violation(
                ViolationKind::DepthExceeded,
                None,
                format!(
                    "nesting depth exceeds the limit ({})",
                    self.limits.max_hir_depth
                ),
            );
            return;
        }
        if self.active_types[index] {
            self.violation(
                ViolationKind::CyclicReference,
                None,
                format!("type reference {id} forms a cycle"),
            );
            return;
        }
        if self.visited_types[index] {
            return;
        }
        self.visited_types[index] = true;
        self.active_types[index] = true;
        let kind = self.module.types.types[index].kind.clone();
        match kind {
            TypeRefKind::Path { segments, generics } => {
                for use_ in segments {
                    self.use_(use_, None);
                }
                for generic in generics {
                    self.walk_type(generic, my_depth);
                }
            }
            TypeRefKind::Tuple(elements) => {
                for element in elements {
                    self.walk_type(element, my_depth);
                }
            }
            TypeRefKind::Slice(element) => self.walk_type(element, my_depth),
            TypeRefKind::Array { elem, len } => {
                self.walk_type(elem, my_depth);
                let mut loops = Vec::new();
                self.walk_node(len, Expect::Expr, my_depth, &mut loops);
            }
            TypeRefKind::Fn { params, ret } => {
                for param in params {
                    self.walk_type(param, my_depth);
                }
                if let Some(ret) = ret {
                    self.walk_type(ret, my_depth);
                }
            }
            TypeRefKind::SelfTy | TypeRefKind::Error => {}
        }
        self.active_types[index] = false;
    }
}
