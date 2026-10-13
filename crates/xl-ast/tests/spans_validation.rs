//! Suite B/D — span propagation contracts and structural validation.

mod common;

use common::*;
use xl_ast::*;
use xl_span::{FileId, Span};

fn has_kind(violations: &[Violation], kind: ViolationKind) -> bool {
    violations.iter().any(|v| v.kind == kind)
}

#[test]
fn example_program_fixture_is_violation_free() {
    for item in example_program() {
        let violations = validate_item(&item);
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:#?}"
        );
    }
}

#[test]
fn spans_carry_the_registered_file_id() {
    for item in example_program() {
        assert_eq!(item.span.file, FILE);
    }
}

#[test]
fn parent_child_containment_holds_for_representative_nodes() {
    let items = example_program();
    // fn area: the body block must contain the match tail and its arms.
    let ItemKind::Fn(area) = &items[4].kind else {
        panic!("items[4] is fn area");
    };
    let tail = area.body.tail.as_ref().expect("tail match");
    assert!(area.body.span.lo <= tail.span.lo && tail.span.hi <= area.body.span.hi);
    let ExprKind::Match(_, arms) = &tail.kind else {
        panic!("tail is a match");
    };
    for arm in arms {
        assert!(tail.span.lo <= arm.span.lo && arm.span.hi <= tail.span.hi);
    }
    // fn main: statements are ordered and inside the body.
    let ItemKind::Fn(main) = &items[6].kind else {
        panic!("items[6] is fn main");
    };
    let mut previous_hi = main.body.span.lo;
    for stmt in &main.body.stmts {
        assert!(stmt.span.lo >= previous_hi);
        assert!(stmt.span.hi <= main.body.span.hi);
        previous_hi = stmt.span.hi;
    }
}

#[test]
fn validation_reports_inverted_spans() {
    let expr = Expr {
        id: ExprId::from_index(0),
        kind: ExprKind::Break,
        span: Span {
            file: FILE,
            lo: 5,
            hi: 3,
        },
    };
    let violations = validate_expr(&expr);
    assert!(has_kind(&violations, ViolationKind::InvertedSpan));
}

#[test]
fn validation_reports_child_outside_parent() {
    let child = lit_int(1); // span (0, 1)
    let well_formed = ex(
        child.span,
        ExprKind::Unary(UnOp::Neg, Box::new(child.clone())),
    );
    assert!(validate_expr(&well_formed).is_empty());

    let mut misplaced = child;
    misplaced.span = fspan(0, 5);
    let bad_parent = ex(fspan(0, 1), ExprKind::Unary(UnOp::Neg, Box::new(misplaced)));
    let violations = validate_expr(&bad_parent);
    assert!(has_kind(&violations, ViolationKind::ChildOutsideParent));
}

#[test]
fn validation_reports_file_mismatch() {
    let other_file = FileId::from_index(1);
    let left = lit_int(1);
    let right = Expr {
        id: ExprId::from_index(0),
        kind: ExprKind::Lit(Lit::Bool(true)),
        span: Span {
            file: other_file,
            lo: 0,
            hi: 1,
        },
    };
    let root = bin(BinOp::Add, left, right);
    let violations = validate_expr(&root);
    assert!(has_kind(&violations, ViolationKind::FileMismatch));
}

#[test]
fn validation_reports_empty_identifier_and_path() {
    let empty_ident = Expr {
        id: ExprId::from_index(0),
        kind: ExprKind::Path(Path {
            segments: vec![Ident {
                name: String::new(),
                span: fspan(0, 1),
            }],
            span: fspan(0, 1),
        }),
        span: fspan(0, 1),
    };
    assert!(has_kind(
        &validate_expr(&empty_ident),
        ViolationKind::EmptyIdentifier
    ));

    let empty_path = Expr {
        id: ExprId::from_index(0),
        kind: ExprKind::Path(Path {
            segments: Vec::new(),
            span: fspan(0, 0),
        }),
        span: fspan(0, 0),
    };
    assert!(has_kind(
        &validate_expr(&empty_path),
        ViolationKind::EmptyPath
    ));
}

#[test]
fn validation_reports_empty_generics() {
    let ty = Type {
        kind: TypeKind::Path(PathType {
            segments: vec![ident("Vec", 0)],
            generics: Some(Vec::new()),
            span: fspan(0, 4),
        }),
        span: fspan(0, 4),
    };
    assert!(has_kind(&validate_type(&ty), ViolationKind::EmptyGenerics));
}

#[test]
fn validation_reports_tail_ordering() {
    // The tail starts before the statement ends.
    let stmt = Stmt {
        kind: StmtKind::Expr(lit_int(1)),
        span: fspan(10, 12),
    };
    let tail = lit_int(2); // span (0, 1)
    let block = Block {
        stmts: vec![stmt],
        tail: Some(Box::new(tail)),
        span: fspan(0, 12),
    };
    let violations = validate_expr(&ex(fspan(0, 12), ExprKind::Block(block)));
    assert!(has_kind(&violations, ViolationKind::TailOrdering));
}

#[test]
fn validation_reports_interpolation_order() {
    let lit = StringLit {
        parts: vec![
            StringPart::Text(TextSegment {
                text: "late".to_string(),
                span: fspan(5, 9),
            }),
            StringPart::Text(TextSegment {
                text: "early".to_string(),
                span: fspan(1, 3),
            }),
        ],
        span: fspan(0, 10),
    };
    let expr = ex(fspan(0, 10), ExprKind::Lit(Lit::Str(lit)));
    let violations = validate_expr(&expr);
    assert!(has_kind(&violations, ViolationKind::InterpolationOrder));
}

#[test]
fn recovered_zero_width_placeholders_are_tolerated() {
    let placeholder = Expr {
        id: ExprId::from_index(0),
        kind: ExprKind::Lit(Lit::Int(IntLit {
            magnitude: 0,
            radix: Radix::Dec,
        })),
        span: fspan(4, 4), // zero-width insertion-point span
    };
    let root = ex(
        fspan(4, 4),
        ExprKind::Unary(UnOp::Neg, Box::new(placeholder)),
    );
    assert!(validate_expr(&root).is_empty());
}

#[test]
fn empty_optional_lists_are_valid_where_the_grammar_permits() {
    // Empty parameter list, body, fields, variants, args, arms: all fine.
    let item = fn_item("f", Vec::new(), None, empty_block());
    assert!(validate_item(&item).is_empty());

    let empty_match = ex(fspan(0, 2), ExprKind::Match(Box::new(var("s")), Vec::new()));
    assert!(validate_expr(&empty_match).is_empty());

    let call_no_args = call(var("f"), Vec::new());
    assert!(validate_expr(&call_no_args).is_empty());
}
