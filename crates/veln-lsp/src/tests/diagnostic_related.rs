fn related_position(line: i64, column: i64, offset: i64) -> JsonValue {
    JsonValue::object([
        ("line", JsonValue::Number(line)),
        ("column", JsonValue::Number(column)),
        ("offset", JsonValue::Number(offset)),
    ])
}

fn related_note(message: &str, start: JsonValue, end: JsonValue) -> JsonValue {
    related_note_for("main.veln", message, start, end)
}

fn related_note_for(file: &str, message: &str, start: JsonValue, end: JsonValue) -> JsonValue {
    JsonValue::object([
        ("message", JsonValue::string(message)),
        (
            "span",
            JsonValue::object([
                ("file", JsonValue::string(file)),
                ("start", start),
                ("end", end),
            ]),
        ),
    ])
}

fn related_span_for(file: JsonValue, start: JsonValue, end: JsonValue) -> JsonValue {
    JsonValue::object([("file", file), ("start", start), ("end", end)])
}

fn diagnostic_with_related(related: Vec<JsonValue>) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "test.related",
        Severity::Error,
        DiagnosticKind::Type,
        "related position boundary",
        None,
        JsonValue::Null,
    );
    diagnostic.related = related;
    diagnostic
}

fn diagnostic_with_span_and_related(column: usize, related: JsonValue) -> Diagnostic {
    let offset = column - 1;
    let position = veln_source::LineCol {
        line: 1,
        column,
        offset,
    };
    let mut diagnostic = Diagnostic::new(
        "test.related",
        Severity::Error,
        DiagnosticKind::Type,
        "bounded projection work",
        Some(SourceSpan {
            file: veln_source::SourcePath::new("main.veln"),
            start: position,
            end: position,
            generated_origin: None,
        }),
        JsonValue::Null,
    );
    diagnostic.related = vec![related];
    diagnostic
}

fn leading_related_position_boundary_notes() -> Vec<JsonValue> {
    let without_offset = JsonValue::object([
        ("line", JsonValue::Number(1)),
        ("column", JsonValue::Number(1)),
    ]);
    let without_column = JsonValue::object([
        ("line", JsonValue::Number(1)),
        ("offset", JsonValue::Number(0)),
    ]);
    vec![
        related_note("missing offset", without_offset, related_position(1, 2, 1)),
        related_note("valid non-BMP", related_position(1, 2, 1), related_position(1, 3, 5)),
        related_note(
            "valid interior zero-width",
            related_position(1, 4, 6),
            related_position(1, 4, 6),
        ),
        related_note("missing column", without_column, related_position(1, 2, 1)),
        related_note("zero line", related_position(0, 1, 0), related_position(1, 2, 1)),
        related_note("beyond EOF", related_position(1, 1, 11), related_position(1, 1, 11)),
        related_note("inside scalar", related_position(1, 2, 2), related_position(1, 3, 5)),
        related_note("wrong line", related_position(2, 1, 1), related_position(2, 1, 1)),
        related_note("valid CRLF", related_position(2, 1, 8), related_position(2, 2, 9)),
    ]
}

fn trailing_related_position_boundary_notes() -> Vec<JsonValue> {
    let without_line = JsonValue::object([
        ("column", JsonValue::Number(1)),
        ("offset", JsonValue::Number(0)),
    ]);
    vec![
        related_note("wrong column", related_position(1, 3, 1), related_position(1, 3, 1)),
        related_note("out-of-source line", related_position(4, 1, 10), related_position(4, 1, 10)),
        related_note("out-of-source column", related_position(3, 2, 10), related_position(3, 2, 10)),
        related_note("reversed", related_position(1, 3, 5), related_position(1, 2, 1)),
        related_note("missing line", without_line, related_position(1, 2, 1)),
        related_note("zero column", related_position(1, 0, 0), related_position(1, 2, 1)),
        related_note("valid EOF", related_position(3, 1, 10), related_position(3, 1, 10)),
        related_note_for(
            "empty.veln",
            "valid empty",
            related_position(1, 1, 0),
            related_position(1, 1, 0),
        ),
    ]
}

#[test]
fn related_information_requires_complete_consistent_source_positions() {
    let source = DiagnosticSource::new("file://main.veln".to_string(), "a😀b\r\nz\n".to_string());
    let empty = DiagnosticSource::new("file://empty.veln".to_string(), String::new());
    let sources = DiagnosticSources::from([
        ("empty.veln".to_string(), empty),
        ("main.veln".to_string(), source),
    ]);
    let related = leading_related_position_boundary_notes()
        .into_iter()
        .chain(trailing_related_position_boundary_notes())
        .collect();

    let published = lsp_diagnostic_json(&diagnostic_with_related(related), &sources);

    assert_eq!(published.matches(r#""location""#).count(), 5, "{published}");
    assert!(
        published.contains(
            r#""range":{"start":{"line":0,"character":1},"end":{"line":0,"character":3}}},"message":"valid non-BMP""#
        ),
        "{published}"
    );
    assert!(
        published.contains(
            r#""uri":"file://empty.veln","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}}},"message":"valid empty""#
        ),
        "{published}"
    );
    let positions = [
        "valid non-BMP",
        "valid interior zero-width",
        "valid CRLF",
        "valid EOF",
        "valid empty",
    ]
    .map(|message| published.find(message).unwrap());
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{published}");
    assert_eq!(published.matches("valid ").count(), 5, "{published}");
}

#[test]
fn diagnostic_projection_handles_many_same_line_endpoints() {
    const ENDPOINTS: usize = 512;
    let text = "a😀".repeat(ENDPOINTS);
    let positions = text
        .char_indices()
        .map(|(offset, _)| offset)
        .take(ENDPOINTS)
        .collect::<Vec<_>>();
    let sources = DiagnosticSources::from([(
        "main.veln".to_string(),
        DiagnosticSource::new("file://main.veln".to_string(), text),
    )]);
    let diagnostics = positions
        .into_iter()
        .enumerate()
        .map(|(index, offset)| {
            let column = index + 1;
            let position = related_position(1, column as i64, offset as i64);
            diagnostic_with_span_and_related(
                column,
                related_note("same-line endpoint", position.clone(), position),
            )
        })
        .collect::<Vec<_>>();

    let published = publish_diagnostics_for_uri("file://main.veln", &diagnostics, &sources);

    assert_eq!(published.matches("same-line endpoint").count(), ENDPOINTS);
    assert_eq!(
        sources["main.veln"].projection_work(),
        (ENDPOINTS * 2, ENDPOINTS * 4)
    );
}

#[test]
fn related_information_omits_malformed_envelopes_and_unavailable_sources_in_order() {
    let position = related_position(1, 1, 0);
    let span = related_span_for(
        JsonValue::string("main.veln"),
        position.clone(),
        position.clone(),
    );
    let notes = vec![
        related_note("valid before", position.clone(), position.clone()),
        JsonValue::object([("span", span.clone())]),
        JsonValue::object([
            ("message", JsonValue::Number(1)),
            ("span", span.clone()),
        ]),
        JsonValue::object([("message", JsonValue::string("missing span"))]),
        JsonValue::object([
            ("message", JsonValue::string("non-object span")),
            ("span", JsonValue::string("main.veln")),
        ]),
        JsonValue::object([
            ("message", JsonValue::string("missing file")),
            (
                "span",
                JsonValue::object([
                    ("start", position.clone()),
                    ("end", position.clone()),
                ]),
            ),
        ]),
        JsonValue::object([
            ("message", JsonValue::string("non-text file")),
            (
                "span",
                related_span_for(JsonValue::Number(1), position.clone(), position.clone()),
            ),
        ]),
        related_note_for(
            "absent.veln",
            "unavailable source",
            position.clone(),
            position.clone(),
        ),
        related_note("valid after", position.clone(), position),
    ];
    let sources = DiagnosticSources::from([(
        "main.veln".to_string(),
        DiagnosticSource::new("file://main.veln".to_string(), String::new()),
    )]);

    let published = lsp_diagnostic_json(&diagnostic_with_related(notes), &sources);

    assert_eq!(published.matches(r#""location""#).count(), 2, "{published}");
    let before = published.find("valid before").unwrap();
    let after = published.find("valid after").unwrap();
    assert!(before < after, "{published}");
    for omitted in [
        "missing span",
        "non-object span",
        "missing file",
        "non-text file",
        "unavailable source",
    ] {
        assert!(!published.contains(omitted), "{published}");
    }
}

#[test]
fn document_publication_omits_cross_file_related_information_in_order() {
    let position = related_position(1, 1, 0);
    let related = vec![
        related_note("valid before", position.clone(), position.clone()),
        related_note_for(
            "other.veln",
            "cross-file unavailable",
            position.clone(),
            position.clone(),
        ),
        related_note("valid after", position.clone(), position),
    ];
    let sources = DiagnosticSources::from([(
        "main.veln".to_string(),
        DiagnosticSource::new("file://main.veln".to_string(), String::new()),
    )]);

    let published = publish_diagnostics_for_uri(
        "file://main.veln",
        &[diagnostic_with_related(related)],
        &sources,
    );

    assert_eq!(published.matches(r#""location""#).count(), 2, "{published}");
    let before = published.find("valid before").unwrap();
    let after = published.find("valid after").unwrap();
    assert!(before < after, "{published}");
    assert!(!published.contains("cross-file unavailable"), "{published}");
}
