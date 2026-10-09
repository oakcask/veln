use super::*;

#[test]
fn equivalent_source_roots_produce_the_same_virtual_doctest_identity() {
    let first_root = test_root("virtual-doctest-first");
    let second_root = test_root("virtual-doctest-second");
    let relative = std::path::Path::new("docs/guide.veln");
    let text = "## ```veln\n## 1\n## ```\n";
    let observed = [
        virtual_doctest_identity(&first_root, relative, text),
        virtual_doctest_identity(&second_root, relative, text),
    ];

    assert_eq!(observed[0], observed[1]);
    assert_eq!(observed[0].0, "docs/guide.veln#doctest-1_test.veln");
    let rendered = format!("{observed:?}");
    assert!(!rendered.contains(&first_root.to_string_lossy().to_string()));
    assert!(!rendered.contains(&second_root.to_string_lossy().to_string()));

    fs::remove_dir_all(first_root).unwrap();
    fs::remove_dir_all(second_root).unwrap();
}

#[test]
fn rejected_virtual_doctest_origins_produce_diagnostics_without_partial_sources() {
    let sources = [
        SourceFile::new("guide:part.veln", "## ```veln\n## 1\n## ```\n"),
        SourceFile::new("guide#part.veln", "## ```veln\n## 2\n## ```\n"),
        SourceFile::new("valid.veln", "## ```veln\n## 3\n## ```\n"),
    ];

    let doctests = doctest_sources(&sources);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].path().as_str(),
        "valid.veln#doctest-1_test.veln"
    );
    assert_eq!(doctests.visible_source_locations.len(), 1);
    assert_eq!(doctests.expectations.len(), 1);
    assert!(doctests.expected_failures.is_empty());
    assert_eq!(doctests.diagnostics.len(), 2);
    assert_eq!(
        doctests
            .diagnostics
            .iter()
            .map(|diagnostic| (
                diagnostic.id.as_str(),
                diagnostic
                    .span
                    .as_ref()
                    .expect("source-path diagnostic span")
                    .file
                    .as_str(),
            ))
            .collect::<Vec<_>>(),
        [
            ("module.invalid_source_path", "guide:part.veln"),
            ("module.invalid_source_path", "guide#part.veln"),
        ]
    );
}

fn virtual_doctest_identity(
    root: &std::path::Path,
    relative: &std::path::Path,
    text: &str,
) -> (String, veln_source::SourcePath) {
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join(relative), text).unwrap();
    let source = SourceFile::read(root, &root.join(relative)).unwrap();
    let doctests = doctest_sources(&[source]);
    let generated = &doctests.sources[0];
    let fallback = generated.span(TextRange::at(0)).resolved_or_generated();
    (generated.path().as_str().to_string(), fallback.file)
}

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

    let generated_doctest = generated_doctest("unicode", &doctest);
    let generated_start = generated_doctest.copied_regions[0].0.start;
    let generated = SourceFile::generated_with_copied_regions(
        "main.veln#doctest-1_test.veln",
        generated_doctest.text,
        SourcePath::new("main.veln"),
        generated_doctest.copied_regions,
    );
    let mapped = "aé界z"
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once("aé界z".len()))
        .map(|offset| {
            generated
                .span(TextRange::at(generated_start + offset))
                .resolved_origin()
                .expect("copied scalar boundary should resolve")
                .start
        })
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
fn generated_doctest_regions_follow_reordered_declarations() {
    let code = [
        "let chosen = Choice::Some(1)",
        "pub type Choice",
        "\tSome(value: Int)",
        "end",
        "chosen",
    ];
    let doctest = ExtractedDoctest {
        code: code.iter().map(|line| (*line).to_string()).collect(),
        source_locations: code
            .iter()
            .enumerate()
            .map(|(index, line)| SourceSpan {
                file: "main.veln".into(),
                start: LineCol {
                    line: index + 10,
                    column: 3,
                    offset: index * 100,
                },
                end: LineCol {
                    line: index + 10,
                    column: 3 + line.chars().count(),
                    offset: index * 100 + line.len(),
                },
                generated_origin: None,
            })
            .collect(),
        ..ExtractedDoctest::default()
    };

    let generated_doctest = generated_doctest("reordered", &doctest);
    let copied_lines = generated_doctest
        .copied_regions
        .iter()
        .map(|(range, start, _)| {
            (
                &generated_doctest.text[range.start..range.end],
                start.line,
                start.offset,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        copied_lines,
        [
            ("pub type Choice", 11, 100),
            ("\tSome(value: Int)", 12, 200),
            ("end", 13, 300),
            ("let chosen = Choice::Some(1)", 10, 0),
            ("chosen", 14, 400),
        ]
    );
}

#[test]
fn large_doctest_construction_retains_one_region_per_copied_line() {
    for size in [256 * 1024, 512 * 1024] {
        let source = SourceFile::new(
            "main.veln",
            format!("## ```veln\n## # {}\n## ```\n", "a".repeat(size)),
        );

        let doctests = doctest_sources(&[source]);
        let generated = &doctests.sources[0];
        let origin = generated
            .generated_origin()
            .expect("doctest source should retain its origin");
        assert_eq!(origin.explicit_boundary_count(), 0);
        assert_eq!(origin.copied_region_count(), 1);

        let parsed = veln_syntax::parse(generated);
        assert!(parsed.diagnostics.is_empty());
        let lowered = veln_ast::lower_surface_ast(&parsed.tree);
        assert_eq!(lowered.functions.len(), 1);
    }
}

#[test]
#[ignore = "manual guarded resource-growth probe"]
fn generated_ascii_doctest_pipeline_probe() {
    let size = std::env::var("VELN_DOCTEST_ASCII_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1024 * 1024);
    let source = SourceFile::new(
        "main.veln",
        format!("## ```veln\n## # {}\n## ```\n", "a".repeat(size)),
    );

    let doctests = doctest_sources(&[source]);
    let generated = &doctests.sources[0];
    let origin = generated
        .generated_origin()
        .expect("doctest source should retain its origin");
    assert_eq!(origin.explicit_boundary_count(), 0);
    assert_eq!(origin.copied_region_count(), 1);
    let parsed = veln_syntax::parse(generated);
    assert!(parsed.diagnostics.is_empty());
    let lowered = veln_ast::lower_surface_ast(&parsed.tree);
    assert_eq!(lowered.functions.len(), 1);
    eprintln!(
        "generated ASCII doctest pipeline completed: bytes={size}, regions={}",
        origin.copied_region_count()
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
