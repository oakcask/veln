use super::*;

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
