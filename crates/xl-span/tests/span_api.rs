use xl_span::{FileId, LineCol, SourceMap, Span, SpanError};

fn map_with(name: &str, source: &str) -> (SourceMap, FileId) {
    let mut map = SourceMap::new();
    let id = map.add_file(name, source).expect("registration succeeds");
    (map, id)
}

#[test]
fn add_file_registers_stable_ids_and_metadata() {
    let mut map = SourceMap::new();
    let first = map.add_file("first.x", "let a = 1;").unwrap();
    let second = map.add_file("second.x", "").unwrap();

    assert_eq!(first.index(), 0);
    assert_eq!(second.index(), 1);
    assert_ne!(first, second);

    let first_file = map.file(first).unwrap();
    assert_eq!(first_file.name(), "first.x");
    assert_eq!(first_file.source(), "let a = 1;");
    assert_eq!(first_file.len(), 10);
    assert!(!first_file.is_empty());
    assert_eq!(first_file.line_count(), 1);

    let second_file = map.file(second).unwrap();
    assert!(second_file.is_empty());
    assert_eq!(second_file.len(), 0);
    assert_eq!(second_file.line_count(), 1);

    assert_eq!(map.source(first).unwrap(), "let a = 1;");
}

#[test]
fn span_new_accepts_ordered_offsets() {
    let (_map, file) = map_with("t.x", "abcd");
    let span = Span::new(file, 1, 3).unwrap();
    assert_eq!(span.file, file);
    assert_eq!(span.lo, 1);
    assert_eq!(span.hi, 3);
    assert_eq!(span.len(), 2);
    assert!(!span.is_empty());
}

#[test]
fn span_new_rejects_inverted_offsets() {
    let (_map, file) = map_with("t.x", "abcd");
    assert_eq!(
        Span::new(file, 3, 2),
        Err(SpanError::Inverted { lo: 3, hi: 2 })
    );
}

#[test]
fn unchecked_inverted_span_is_rejected_by_lookups_without_panicking() {
    let (map, file) = map_with("t.x", "abcd");
    let bad = Span { file, lo: 3, hi: 1 };
    assert_eq!(bad.len(), 0);
    assert_eq!(
        map.span_slice(bad),
        Err(SpanError::Inverted { lo: 3, hi: 1 })
    );
    assert_eq!(
        map.line_col_range(bad),
        Err(SpanError::Inverted { lo: 3, hi: 1 })
    );
}

#[test]
fn span_slice_returns_exact_source_ranges() {
    let (map, file) = map_with("t.x", "hello world");
    let hello = Span::new(file, 0, 5).unwrap();
    let world = Span::new(file, 6, 11).unwrap();
    let empty = Span::new(file, 5, 5).unwrap();
    assert_eq!(map.span_slice(hello).unwrap(), "hello");
    assert_eq!(map.span_slice(world).unwrap(), "world");
    assert_eq!(map.span_slice(empty).unwrap(), "");
}

#[test]
fn span_slice_at_end_of_file() {
    let (map, file) = map_with("t.x", "abc");
    let eof = Span::new(file, 3, 3).unwrap();
    assert!(eof.is_empty());
    assert_eq!(map.span_slice(eof).unwrap(), "");
    assert_eq!(
        map.span_slice(Span::new(file, 0, 3).unwrap()).unwrap(),
        "abc"
    );
}

#[test]
fn span_slice_rejects_out_of_bounds_offsets() {
    let (map, file) = map_with("t.x", "abc");
    assert_eq!(
        map.span_slice(Span { file, lo: 2, hi: 4 }),
        Err(SpanError::OutOfBounds { offset: 4, len: 3 })
    );
    assert_eq!(
        map.span_slice(Span {
            file,
            lo: u32::MAX - 1,
            hi: u32::MAX,
        }),
        Err(SpanError::OutOfBounds {
            offset: u32::MAX,
            len: 3,
        })
    );
}

#[test]
fn span_slice_rejects_non_utf8_boundaries() {
    let (map, file) = map_with("t.x", "añb");
    assert_eq!(
        map.span_slice(Span { file, lo: 0, hi: 2 }),
        Err(SpanError::NotCharBoundary { offset: 2 })
    );
    assert_eq!(
        map.span_slice(Span { file, lo: 2, hi: 4 }),
        Err(SpanError::NotCharBoundary { offset: 2 })
    );
    assert_eq!(map.span_slice(Span { file, lo: 1, hi: 3 }).unwrap(), "ñ");
    assert_eq!(map.span_slice(Span { file, lo: 0, hi: 4 }).unwrap(), "añb");
    assert_eq!(
        map.line_col(file, 2),
        Err(SpanError::NotCharBoundary { offset: 2 })
    );
}

#[test]
fn lookups_reject_unknown_files() {
    let (map, _file) = map_with("t.x", "abc");
    for index in [1, 42, u32::MAX] {
        let unknown = FileId::from_index(index);
        assert_eq!(
            map.file(unknown).unwrap_err(),
            SpanError::UnknownFile(unknown)
        );
        assert_eq!(
            map.source(unknown).unwrap_err(),
            SpanError::UnknownFile(unknown)
        );
        assert_eq!(
            map.span_slice(Span {
                file: unknown,
                lo: 0,
                hi: 0,
            }),
            Err(SpanError::UnknownFile(unknown))
        );
        assert_eq!(
            map.line_col(unknown, 0),
            Err(SpanError::UnknownFile(unknown))
        );
        assert_eq!(
            map.line_col_range(Span {
                file: unknown,
                lo: 0,
                hi: 0,
            }),
            Err(SpanError::UnknownFile(unknown))
        );
        assert_eq!(
            map.line_text(unknown, 1),
            Err(SpanError::UnknownFile(unknown))
        );
    }
}

#[test]
fn file_ids_from_distinct_maps_alias_by_raw_index() {
    let (map_a, file_a) = map_with("a.x", "alpha");
    let (map_b, file_b) = map_with("b.x", "bravo");

    assert_eq!(file_a, file_b);
    assert_eq!(file_a.index(), 0);

    let span = Span::new(file_a, 0, 5).unwrap();
    assert_eq!(map_a.span_slice(span).unwrap(), "alpha");
    assert_eq!(map_b.span_slice(span).unwrap(), "bravo");

    let out_of_range = FileId::from_index(1);
    assert_eq!(
        map_a.file(out_of_range).unwrap_err(),
        SpanError::UnknownFile(out_of_range)
    );
}

#[test]
fn line_col_maps_multiline_positions() {
    let (map, file) = map_with("t.x", "ab\ncd\n");
    assert_eq!(map.line_col(file, 0).unwrap(), LineCol { line: 1, col: 1 });
    assert_eq!(map.line_col(file, 1).unwrap(), LineCol { line: 1, col: 2 });
    assert_eq!(map.line_col(file, 2).unwrap(), LineCol { line: 1, col: 3 });
    assert_eq!(map.line_col(file, 3).unwrap(), LineCol { line: 2, col: 1 });
    assert_eq!(map.line_col(file, 4).unwrap(), LineCol { line: 2, col: 2 });
    assert_eq!(map.line_col(file, 5).unwrap(), LineCol { line: 2, col: 3 });
    assert_eq!(map.line_col(file, 6).unwrap(), LineCol { line: 3, col: 1 });
}

#[test]
fn line_col_at_end_of_file_without_newline() {
    let (map, file) = map_with("t.x", "abc");
    assert_eq!(map.line_col(file, 3).unwrap(), LineCol { line: 1, col: 4 });
}

#[test]
fn line_col_counts_unicode_scalar_values() {
    let (map, file) = map_with("t.x", "aé\nb");
    assert_eq!(map.line_col(file, 1).unwrap(), LineCol { line: 1, col: 2 });
    assert_eq!(map.line_col(file, 3).unwrap(), LineCol { line: 1, col: 3 });
    assert_eq!(map.line_col(file, 4).unwrap(), LineCol { line: 2, col: 1 });
}

#[test]
fn line_col_rejects_invalid_positions() {
    let (map, file) = map_with("t.x", "abc");
    assert_eq!(
        map.line_col(file, 4),
        Err(SpanError::OutOfBounds { offset: 4, len: 3 })
    );
    assert_eq!(
        map.line_col(file, u32::MAX),
        Err(SpanError::OutOfBounds {
            offset: u32::MAX,
            len: 3,
        })
    );
}

#[test]
fn line_col_of_empty_file() {
    let (map, file) = map_with("empty.x", "");
    assert_eq!(map.line_col(file, 0).unwrap(), LineCol { line: 1, col: 1 });
}

#[test]
fn line_col_range_covers_multiline_spans() {
    let (map, file) = map_with("t.x", "alpha\nbeta\ngamma");
    let span = Span::new(file, 2, 8).unwrap();
    assert_eq!(
        map.line_col_range(span).unwrap(),
        (LineCol { line: 1, col: 3 }, LineCol { line: 2, col: 3 })
    );
    assert_eq!(map.span_slice(span).unwrap(), "pha\nbe");
}

#[test]
fn line_text_returns_lines_without_endings() {
    let (map, file) = map_with("t.x", "one\r\ntwo\r\nthree");
    assert_eq!(map.line_text(file, 1).unwrap(), "one");
    assert_eq!(map.line_text(file, 2).unwrap(), "two");
    assert_eq!(map.line_text(file, 3).unwrap(), "three");
    assert_eq!(map.file(file).unwrap().line_count(), 3);
}

#[test]
fn line_text_handles_trailing_newline_and_bounds() {
    let (map, file) = map_with("t.x", "x\n");
    assert_eq!(map.line_text(file, 1).unwrap(), "x");
    assert_eq!(map.line_text(file, 2).unwrap(), "");
    assert_eq!(
        map.line_text(file, 0),
        Err(SpanError::LineOutOfBounds { line: 0, lines: 2 })
    );
    assert_eq!(
        map.line_text(file, 3),
        Err(SpanError::LineOutOfBounds { line: 3, lines: 2 })
    );
}

#[test]
fn line_text_preserves_lone_carriage_returns_inside_lines() {
    let (map, file) = map_with("t.x", "a\rb");
    assert_eq!(map.line_text(file, 1).unwrap(), "a\rb");
}

#[test]
fn span_len_saturates_on_unchecked_inverted_spans() {
    let file = FileId::from_index(0);
    let bad = Span {
        file,
        lo: 10,
        hi: 4,
    };
    assert_eq!(bad.len(), 0);
    assert!(!bad.is_empty());
}
