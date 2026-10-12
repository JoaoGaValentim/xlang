use xl_diag::{Diagnostic, Label, Severity, render};
use xl_span::{FileId, SourceMap, Span};

fn map_with(name: &str, source: &str) -> (SourceMap, FileId) {
    let mut map = SourceMap::new();
    let id = map.add_file(name, source).expect("registration succeeds");
    (map, id)
}

#[test]
fn renders_primary_and_secondary_labels() {
    let (map, file) = map_with("src/main.x", "let alpha = beta + 1;\nlet beta = \"text\";");
    let diagnostic = Diagnostic::error("mismatched types", Span::new(file, 12, 16).unwrap())
        .with_primary_label("expected `i64`, found `str`")
        .with_secondary(
            Span::new(file, 26, 30).unwrap(),
            "`beta` declared with type `str` here",
        );

    let expected = r#"error: mismatched types
 --> src/main.x:1:13
  |
1 | let alpha = beta + 1;
  |             ^^^^ expected `i64`, found `str`
 --> src/main.x:2:5
  |
2 | let beta = "text";
  |     ---- `beta` declared with type `str` here
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_warning_severity() {
    let (map, file) = map_with("t.x", "let x = 1;");
    let diagnostic = Diagnostic::warning("unused variable", Span::new(file, 4, 5).unwrap())
        .with_primary_label("`x` is never used");

    let expected = r#"warning: unused variable
 --> t.x:1:5
  |
1 | let x = 1;
  |     ^ `x` is never used
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_note_severity() {
    let (map, file) = map_with("t.x", "abc");
    let diagnostic = Diagnostic::note("extra context", Span::new(file, 0, 3).unwrap())
        .with_primary_label("see above");

    let expected = r#"note: extra context
 --> t.x:1:1
  |
1 | abc
  | ^^^ see above
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_empty_label_text_without_trailing_text() {
    let (map, file) = map_with("t.x", "abc");
    let diagnostic = Diagnostic::error("boom", Span::new(file, 0, 1).unwrap());

    let expected = r#"error: boom
 --> t.x:1:1
  |
1 | abc
  | ^
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_unicode_sources_with_scalar_columns() {
    let (map, file) = map_with("unicode.x", "let café = 1;");
    let diagnostic = Diagnostic::error("café is not numeric", Span::new(file, 4, 9).unwrap())
        .with_primary_label("`café` is not numeric");

    let expected = r#"error: café is not numeric
 --> unicode.x:1:5
  |
1 | let café = 1;
  |     ^^^^ `café` is not numeric
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_emoji_sources_deterministically() {
    let (map, file) = map_with("emoji.x", "let x = 🦀;");
    let diagnostic = Diagnostic::error("wrong type", Span::new(file, 8, 12).unwrap())
        .with_primary_label("expected integer");

    let expected = r#"error: wrong type
 --> emoji.x:1:9
  |
1 | let x = 🦀;
  |         ^ expected integer
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_multiline_spans_to_end_of_first_line() {
    let (map, file) = map_with("t.x", "alpha\nbeta");
    let diagnostic = Diagnostic::error("spans lines", Span::new(file, 2, 8).unwrap())
        .with_primary_label("multi-line");

    let expected = r#"error: spans lines
 --> t.x:1:3
  |
1 | alpha
  |   ^^^ multi-line
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_empty_span_as_single_marker() {
    let (map, file) = map_with("t.x", "alpha\nbeta");
    let diagnostic =
        Diagnostic::error("empty span", Span::new(file, 6, 6).unwrap()).with_primary_label("here");

    let expected = r#"error: empty span
 --> t.x:2:1
  |
2 | beta
  | ^ here
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_at_end_of_file_with_empty_line() {
    let (map, file) = map_with("t.x", "x\n");
    let diagnostic = Diagnostic::error("at end of file", Span::new(file, 2, 2).unwrap())
        .with_primary_label("end of file");

    let expected = [
        "error: at end of file",
        " --> t.x:2:1",
        "  |",
        "2 | ",
        "  | ^ end of file",
    ]
    .join("\n")
        + "\n";
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn renders_two_digit_line_numbers_with_aligned_gutter() {
    let (map, file) = map_with("t.x", "a\na\na\na\na\na\na\na\na\nb");
    let diagnostic =
        Diagnostic::error("late", Span::new(file, 18, 19).unwrap()).with_primary_label("ten");

    let expected = r#"error: late
 --> t.x:10:1
   |
10 | b
   | ^ ten
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn falls_back_for_unknown_files() {
    let (map, _file) = map_with("t.x", "abc");
    let unknown = FileId::from_index(9);
    let diagnostic =
        Diagnostic::error("boom", Span::new(unknown, 0, 1).unwrap()).with_primary_label("gone");

    let expected = r#"error: boom
 --> <unknown file: 9>
  = gone
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn falls_back_for_out_of_range_spans() {
    let (map, file) = map_with("t.x", "abc");
    let diagnostic = Diagnostic::error(
        "boom",
        Span {
            file,
            lo: 50,
            hi: 60,
        },
    )
    .with_primary_label("escaped");

    let expected = r#"error: boom
 --> t.x: <invalid span 50..60>
  = escaped
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn falls_back_for_misaligned_offsets() {
    let (map, file) = map_with("t.x", "añb");
    let diagnostic =
        Diagnostic::error("boom", Span { file, lo: 2, hi: 4 }).with_primary_label("split");

    let expected = r#"error: boom
 --> t.x: <invalid span 2..4>
  = split
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn falls_back_for_invalid_secondary_but_renders_primary() {
    let (map, file) = map_with("t.x", "abc");
    let diagnostic = Diagnostic::error("boom", Span::new(file, 0, 1).unwrap())
        .with_primary_label("primary")
        .with_secondary(
            Span {
                file,
                lo: 50,
                hi: 60,
            },
            "secondary",
        );

    let expected = r#"error: boom
 --> t.x:1:1
  |
1 | abc
  | ^ primary
 --> t.x: <invalid span 50..60>
  = secondary
"#;
    assert_eq!(render(&diagnostic, &map), expected);
}

#[test]
fn rendering_is_deterministic() {
    let (map, file) = map_with("t.x", "let x = 1;");
    let diagnostic = Diagnostic::error("boom", Span::new(file, 4, 5).unwrap())
        .with_primary_label("here")
        .with_secondary(Span::new(file, 0, 3).unwrap(), "there");
    assert_eq!(render(&diagnostic, &map), render(&diagnostic, &map));

    let (map2, file2) = map_with("t.x", "let x = 1;");
    let diagnostic2 = Diagnostic::error("boom", Span::new(file2, 4, 5).unwrap())
        .with_primary_label("here")
        .with_secondary(Span::new(file2, 0, 3).unwrap(), "there");
    assert_eq!(render(&diagnostic, &map), render(&diagnostic2, &map2));
}

#[test]
fn diagnostic_exposes_its_parts() {
    let (_map, file) = map_with("t.x", "abc");
    let span = Span::new(file, 0, 1).unwrap();
    let diagnostic = Diagnostic::new(Severity::Warning, "msg", span)
        .with_primary_label("p")
        .with_secondary(Span::new(file, 1, 2).unwrap(), "s1")
        .with_secondary(Span::new(file, 2, 3).unwrap(), "s2");

    assert_eq!(diagnostic.severity(), Severity::Warning);
    assert_eq!(diagnostic.message(), "msg");
    assert_eq!(
        diagnostic.primary(),
        &Label {
            span,
            message: "p".to_string(),
        }
    );
    assert_eq!(diagnostic.secondary().len(), 2);
    assert_eq!(diagnostic.secondary()[0].message, "s1");
    assert_eq!(diagnostic.secondary()[1].message, "s2");
}

#[test]
fn secondary_labels_render_in_insertion_order() {
    let (map, file) = map_with("t.x", "abc");
    let diagnostic = Diagnostic::error("boom", Span::new(file, 0, 1).unwrap())
        .with_secondary(Span::new(file, 1, 2).unwrap(), "first secondary")
        .with_secondary(Span::new(file, 2, 3).unwrap(), "second secondary");

    let output = render(&diagnostic, &map);
    let first = output.find("first secondary").unwrap();
    let second = output.find("second secondary").unwrap();
    assert!(first < second);
}
