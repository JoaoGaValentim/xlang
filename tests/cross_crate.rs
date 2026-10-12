//! Workspace-level integration test exercising the public APIs of `xl-span`
//! and `xl-diag` together, as they will be used by later compiler phases.
//!
//! The source snippet below is illustrative test data; it is not a claim
//! about XLANG syntax.

use xl_diag::{Diagnostic, render};
use xl_span::{SourceMap, Span};

#[test]
fn sample_diagnostic_renders_all_required_elements() {
    let mut sources = SourceMap::new();
    let file = sources
        .add_file(
            "examples/greeting.x",
            "let greeting = name;\nlet name = \"Ada\";\n",
        )
        .expect("registration succeeds");

    let diagnostic = Diagnostic::error(
        "cannot find value `name` in this scope",
        Span::new(file, 15, 19).unwrap(),
    )
    .with_primary_label("not found in this scope")
    .with_secondary(
        Span::new(file, 25, 29).unwrap(),
        "a value with a similar name exists",
    );

    let output = render(&diagnostic, &sources);

    let expected = r#"error: cannot find value `name` in this scope
 --> examples/greeting.x:1:16
  |
1 | let greeting = name;
  |                ^^^^ not found in this scope
 --> examples/greeting.x:2:5
  |
2 | let name = "Ada";
  |     ---- a value with a similar name exists
"#;
    assert_eq!(output, expected);
}
