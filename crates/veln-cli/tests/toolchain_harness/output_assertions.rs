use super::*;

pub(super) fn stream_text(bytes: Vec<u8>, context: &CaseRunContext<'_>, stream: &str) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("{}: {stream} should be UTF-8: {error}", context.label()))
}

pub(super) fn assert_stream(
    context: &CaseRunContext<'_>,
    name: &str,
    expectation: &StreamExpectation,
    actual: &str,
) {
    match expectation.format {
        Some(StreamFormat::Empty) => assert_eq!(
            actual,
            "",
            "{}: expected {name} to be empty, got:\n{actual}",
            context.label()
        ),
        Some(StreamFormat::Text) | Some(StreamFormat::Json) | None => {}
    }

    if let Some(expected) = &expectation.equals {
        assert_eq!(
            actual,
            expected,
            "{}: expected {name} to equal configured text",
            context.label()
        );
    }
    for fragment in &expectation.contains {
        assert_contains_fragment(context, name, actual, fragment);
    }
    for fragment in &expectation.not_contains {
        assert!(
            !actual.contains(fragment),
            "{}: expected {name} not to contain `{fragment}`, got:\n{actual}",
            context.label()
        );
    }
}

pub(super) fn assert_help_section(
    context: &CaseRunContext<'_>,
    surface: &str,
    stream: &str,
    section: &str,
    fragments: &[String],
) {
    if fragments.is_empty() {
        return;
    }
    assert_contains_fragment(context, surface, stream, &format!("{section}:\n"));
    for fragment in fragments {
        assert_contains_fragment(context, surface, stream, fragment);
    }
}

pub(super) fn assert_contains_fragment(
    context: &CaseRunContext<'_>,
    surface: &str,
    actual: &str,
    fragment: &str,
) {
    assert!(
        actual.contains(fragment),
        "{}: expected {surface} to contain `{fragment}`, got:\n{actual}",
        context.label()
    );
}

pub(super) fn assert_binary_fixture(
    context: &CaseRunContext<'_>,
    stdout: &str,
    fixture: &BinaryFixtureExpectation,
) {
    let expected = expected_binary_fixture_line(fixture);
    assert!(
        stdout.lines().any(|line| line == expected),
        "{}: expected binary fixture line `{expected}`, got:\n{stdout}",
        context.label()
    );
}

pub(super) fn assert_output_chunk_list(
    context: &CaseRunContext<'_>,
    stdout: &str,
    chunks: &OutputChunkListExpectation,
) {
    let expected = expected_output_chunk_list_lines(chunks);
    let actual = stdout.lines().collect::<Vec<_>>();
    let matches = actual.windows(expected.len()).any(|window| {
        window
            .iter()
            .zip(&expected)
            .all(|(actual, expected)| *actual == expected.as_str())
    });
    assert!(
        matches,
        "{}: expected output chunk list:\n{}\ngot:\n{stdout}",
        context.label(),
        expected.join("\n")
    );
}

pub(super) fn expected_binary_fixture_line(fixture: &BinaryFixtureExpectation) -> String {
    if let Some(bytes) = &fixture.bytes {
        let consumed = fixture
            .consumed
            .map_or_else(|| "none".to_string(), |value| value.to_string());
        let mut line = format!(
            "fixture {} hex {} count {} consumed {}",
            fixture.name,
            bytes.hex,
            bytes.bytes.len(),
            consumed
        );
        if let Some(byte_diagnostic) = &fixture.byte_diagnostic {
            if let Some(diagnostic_id) = &byte_diagnostic.diagnostic_id {
                line.push_str(&format!(" diagnostic {diagnostic_id}"));
            }
            if let Some(byte_offset) = byte_diagnostic.byte_offset {
                line.push_str(&format!(" offset {byte_offset}"));
            }
            if let Some(expected_count) = byte_diagnostic.expected_count {
                line.push_str(&format!(" expected {expected_count}"));
            }
            if let Some(available_count) = byte_diagnostic.available_count {
                line.push_str(&format!(" available {available_count}"));
            }
            if let Some(readiness) = &byte_diagnostic.readiness {
                line.push_str(&format!(" readiness {readiness}"));
            }
            if let Some(field_path) = &byte_diagnostic.field_path {
                line.push_str(&format!(" field_path {}", field_path.to_compact_string()));
            }
        }
        return line;
    }

    format!(
        "fixture {} error {}",
        fixture.name,
        fixture
            .error
            .as_deref()
            .expect("binary fixture error should be present")
    )
}

pub(super) fn expected_output_chunk_list_lines(chunks: &OutputChunkListExpectation) -> Vec<String> {
    let chunk_values = chunks
        .chunks
        .as_deref()
        .expect("output chunk list chunks should be present");
    let mut lines = vec![format!(
        "output_chunk_list {} count {}",
        chunks.name,
        chunk_values.len()
    )];
    for (index, chunk) in chunk_values.iter().enumerate() {
        lines.push(format!(
            "output_chunk {} index {} hex \"{}\" count {}",
            chunks.name,
            index,
            chunk.hex,
            chunk.bytes.len()
        ));
    }
    lines
}

pub(super) fn jdk_is_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("java")
            .arg("--list-modules")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("jdk.compiler"))
}
