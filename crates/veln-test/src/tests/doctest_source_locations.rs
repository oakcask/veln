use super::*;

#[test]
fn generated_doctest_boundaries_use_scalar_columns_and_byte_offsets() {
    let doctest = ExtractedDoctest {
        code: vec!["aé界z".to_string()],
        source_locations: vec![SourceSpan {
            file: "main.veln".into(),
            start: LineCol {
                line: 7,
                column: 10,
                offset: 100,
            },
            end: LineCol {
                line: 9,
                column: 2,
                offset: 999,
            },
            generated_origin: None,
        }],
        ..ExtractedDoctest::default()
    };

    let mapped = generated_doctest_boundary_mappings("unicode", &doctest)
        .into_iter()
        .map(|(_, original)| original)
        .collect::<Vec<_>>();

    assert_eq!(
        mapped,
        [
            LineCol {
                line: 7,
                column: 10,
                offset: 100,
            },
            LineCol {
                line: 7,
                column: 11,
                offset: 101,
            },
            LineCol {
                line: 7,
                column: 12,
                offset: 103,
            },
            LineCol {
                line: 7,
                column: 13,
                offset: 106,
            },
            LineCol {
                line: 9,
                column: 2,
                offset: 999,
            },
        ]
    );
}

#[test]
fn visible_doctest_spans_preserve_original_coordinates_and_hidden_marker_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "\t## ```veln\r\n",
            "\t## > let café = 1\r\n",
            "\t## café\r\n",
            "\t##  > literal\r\n",
            "\t## # visible\r\n",
            "\t## ```\r\n",
        ),
    );

    let doctests = doctest_sources(&[source]);
    let locations = &doctests.visible_source_locations["main.veln#doctest-1_test.veln"];
    let coordinates = locations
        .iter()
        .map(|span| {
            (
                span.file.as_str(),
                (span.start.line, span.start.column, span.start.offset),
                (span.end.line, span.end.column, span.end.offset),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        coordinates,
        [
            ("main.veln", (3, 5, 38), (3, 9, 43)),
            ("main.veln", (4, 5, 49), (4, 15, 59)),
            ("main.veln", (5, 5, 65), (5, 14, 74)),
        ]
    );

    let generated = &doctests.sources[0];
    let start = generated
        .text()
        .rfind("café")
        .expect("generated expression");
    let resolved = generated
        .span(TextRange::new(start, start + "café".len()))
        .resolved_origin()
        .expect("doctest boundaries should map to the documented source");
    assert_eq!(resolved.file.as_str(), "main.veln");
    assert_eq!(
        (
            resolved.start.line,
            resolved.start.column,
            resolved.start.offset
        ),
        (3, 5, 38)
    );
    assert_eq!(
        (resolved.end.line, resolved.end.column, resolved.end.offset),
        (3, 9, 43)
    );
}

#[test]
fn visible_doctest_maps_follow_generated_paths_across_sources_and_fence_kinds() {
    let sources = [
        SourceFile::new(
            "zeta.veln",
            concat!(
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "## ```veln-output stream=stdout\n",
                "## one\n",
                "## ```\n",
                "## ```veln ignore\n",
                "## ignored\n",
                "## ```\n",
                "## ```veln fail\n",
                "## (\n",
                "## ```\n",
                "## ```veln\n",
                "## unterminated\n",
            ),
        ),
        SourceFile::new("alpha.veln", "## ```veln\n## 2\n## ```\n"),
    ];

    let doctests = doctest_sources(&sources);
    let mapped_sources = doctests
        .sources
        .iter()
        .map(|source| {
            let locations = &doctests.visible_source_locations[source.path().as_str()];
            assert_eq!(locations.len(), 1);
            (source.path().as_str(), locations[0].start.line)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        doctests.visible_source_locations.len(),
        mapped_sources.len()
    );
    assert_eq!(
        mapped_sources,
        [
            ("zeta.veln#doctest-1_test.veln", 2),
            ("zeta.veln#doctest-2_test.veln", 11),
            ("alpha.veln#doctest-3_test.veln", 2),
        ]
    );
}
