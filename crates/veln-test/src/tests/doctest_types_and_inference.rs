use super::*;

#[test]
fn extracts_hidden_doctest_setup_lines() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  let greeting = \"ready\"\n",
            "  stdio::println(greeting)\n",
            "  ()\n",
            "end\n",
        )
    );
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn extracts_hash_doc_comment_doctests_with_visible_hash_comments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## # visible example comment\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  let greeting = \"ready\"\n",
            "  # visible example comment\n",
            "  stdio::println(greeting)\n",
            "  ()\n",
            "end\n",
        )
    );
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("expected output should be recorded");
    assert_eq!(
        expected
            .expected_output
            .as_ref()
            .and_then(|output| output.stdout.as_deref()),
        Some("ready")
    );
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn unknown_doctest_fence_attribute_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln skip=true\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.unknown_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "unknown doctest attribute `skip`"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"skip\",\"fence\":\"veln\"}"
    );
}

#[test]
fn empty_doctest_error_type_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln error=\n",
            "## let value = parse(\"1\")?\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  ()\n",
            "end\n",
        )
    );
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(doctests.diagnostics[0].message, "empty doctest error type");
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"error\"}"
    );
}

#[test]
fn extracts_doctest_error_type_fence_attribute() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln error=AppError\n",
            "## let value = parse(\"1\")?\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  stdio::println(\"ready\")\n",
            "  Ok(())\n",
            "end\n",
        )
    );
}

#[test]
fn infers_doctest_error_type_from_documented_public_result() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## let value: Int = Ok(1)?\n",
            "## ```\n",
            "pub fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
            "  let value: Int = Ok(1)?\n",
            "  Ok(())\n",
            "end\n",
        )
    );
}

#[test]
fn infers_doctest_error_type_from_single_result_operation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "## ```veln\n",
            "## let value = parse(\"1\")?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  Ok(())\n",
            "end\n",
        )
    );
}

#[test]
fn infers_doctest_error_type_from_result_binding_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> result: Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "## ```veln\n",
            "## let value = parse(\"1\")?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  Ok(())\n",
            "end\n",
        )
    );
}

#[test]
fn infers_doctest_error_type_after_nested_result_success_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Vec<Result<Int, ParseError>>, AppError>\n",
            "  Ok([])\n",
            "end\n",
            "## ```veln\n",
            "## let value = parse(\"1\")?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  Ok(())\n",
            "end\n",
        )
    );
}

#[test]
fn does_not_infer_doctest_error_type_from_mixed_result_operations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "fn read(raw: String) -> Result<String, IoError>\n",
            "  Ok(raw)\n",
            "end\n",
            "## ```veln\n",
            "## let value = parse(\"1\")?\n",
            "## let text = read(\"x\")?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  let text = read(\"x\")?\n",
            "  ()\n",
            "end\n",
        )
    );
}

#[test]
fn explicit_doctest_error_type_handles_mixed_result_operations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "fn read(raw: String) -> Result<String, IoError>\n",
            "  Ok(raw)\n",
            "end\n",
            "## ```veln error=ExampleError\n",
            "## let value = parse(\"1\")?\n",
            "## let text = read(\"x\")?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> Result<(), ExampleError> effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  let text = read(\"x\")?\n",
            "  Ok(())\n",
            "end\n",
        )
    );
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn does_not_infer_doctest_error_type_from_ambiguous_function_signatures() {
    let primary = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "## ```veln\n",
            "## let value = parse(\"1\")?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let imported = SourceFile::new(
        "other.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, ParseError>\n",
            "  Ok(1)\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[primary, imported]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  let value = parse(\"1\")?\n",
            "  ()\n",
            "end\n",
        )
    );
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}
