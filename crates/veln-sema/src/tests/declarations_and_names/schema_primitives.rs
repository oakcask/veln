use super::*;

#[test]
fn binary_schema_accepts_reserved_bits_literal_primitive() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Http2FrameHeader\n",
            "  format binary\n",
            "\n",
            "  priority: UInt16be\n",
            "  length: UInt24be\n",
            "  kind: UInt8\n",
            "  stream_reserved: ReservedBits(1, 0)\n",
            "  stream_id: UInt31be\n",
            "  checksum: UInt32be\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn exact_width_binary_schema_primitives_require_binary_schema_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadHeader\n",
            "  format text\n",
            "\n",
            "  priority: UInt16be\n",
            "  little_priority: UInt16le\n",
            "  length: UInt24be\n",
            "  little_length: UInt24le\n",
            "  tiny: UInt5\n",
            "  kind: UInt8\n",
            "  stream_id: UInt31be\n",
            "  little_stream_id: UInt31le\n",
            "  checksum: UInt32be\n",
            "  little_checksum: UInt32le\n",
            "  trace_id: UInt40be\n",
            "  little_trace_id: UInt40le\n",
            "  extended_checksum: UInt48be\n",
            "  little_extended_checksum: UInt48le\n",
            "  seven_byte_checksum: UInt56be\n",
            "  little_seven_byte_checksum: UInt56le\n",
            "  massive_checksum: UInt64be\n",
            "  little_massive_checksum: UInt64le\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 18);
    for primitive in [
        "UInt16be", "UInt16le", "UInt24be", "UInt24le", "UInt5", "UInt8", "UInt31be", "UInt31le",
        "UInt32be", "UInt32le", "UInt40be", "UInt40le", "UInt48be", "UInt48le", "UInt56be",
        "UInt56le", "UInt64be", "UInt64le",
    ] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.exact_width_primitive"
                && diagnostic.message
                    == format!(
                        "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
                    )
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"reason\":\"non_binary_format\"")
        }));
    }
}

#[test]
fn binary_schema_primitives_without_format_clause_report_schema_wrong_kind() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingFormatUInt\n",
            "  length: UInt16be\n",
            "end\n",
            "\n",
            "schema MissingFormatReserved\n",
            "  padding: ReservedBits(8, 0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.exact_width_primitive"
            && diagnostic.message
                == "binary schema primitive `UInt16be` can only be used in a `format binary` schema field"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_binary_format\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.reserved_bits_primitive"
            && diagnostic.message
                == "`ReservedBits` can only be used in a `format binary` schema field"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_binary_format\"")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_width_binary_schema_primitives_are_not_ordinary_types_or_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn ordinary_types(value: UInt16be, little: UInt16le, little_length: UInt24le, little_stream: UInt31le, trace: UInt40be, extended: UInt48be, seven_byte: UInt56be, massive: UInt64be, tiny: UInt5, another: UInt8) -> {short: UInt24be, wide: UInt32be, little_wide: UInt32le, little_trace: UInt40le, little_extended: UInt48le, little_seven_byte: UInt56le, little_massive: UInt64le}\n",
            "  UInt31be\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 18);
    for (primitive, reason) in [
        ("UInt16be", "parameter_type"),
        ("UInt16le", "parameter_type"),
        ("UInt24le", "parameter_type"),
        ("UInt31le", "parameter_type"),
        ("UInt40be", "parameter_type"),
        ("UInt48be", "parameter_type"),
        ("UInt56be", "parameter_type"),
        ("UInt64be", "parameter_type"),
        ("UInt5", "parameter_type"),
        ("UInt8", "parameter_type"),
        ("UInt24be", "return_type"),
        ("UInt32be", "return_type"),
        ("UInt32le", "return_type"),
        ("UInt40le", "return_type"),
        ("UInt48le", "return_type"),
        ("UInt56le", "return_type"),
        ("UInt64le", "return_type"),
        ("UInt31be", "value_position"),
    ] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.exact_width_primitive"
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"primitive\":\"{primitive}\""))
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"reason\":\"{reason}\""))
        }));
    }
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn binary_schema_rejects_malformed_reserved_bits_primitive() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadHeader\n",
            "  format binary\n",
            "\n",
            "  missing: ReservedBits()\n",
            "  bare: ReservedBits\n",
            "  named: ReservedBits(width, 0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "schema.reserved_bits_primitive"
                    && diagnostic.message
                        == "`ReservedBits` requires width and value integer arguments"
                    && diagnostic
                        .details
                        .to_json()
                        .contains("\"reason\":\"argument_count\"")
            })
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.reserved_bits_primitive"
            && diagnostic.message
                == "`ReservedBits` arguments must be literal non-negative integers"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_literal_argument\"")
    }));
}

#[test]
fn reserved_bits_prefix_does_not_capture_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "\n",
            "  field: ReservedBits::Visible\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.reserved_bits_primitive"),
        "{diagnostics:#?}"
    );
}

#[test]
fn reserved_bits_primitive_reports_non_binary_schema_format() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadHeader\n",
            "  format text\n",
            "\n",
            "  stream_reserved: ReservedBits(1, 0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.reserved_bits_primitive"
            && diagnostic.message
                == "`ReservedBits` can only be used in a `format binary` schema field"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_binary_format\"")
    }));
}

#[test]
fn test_declaration_requires_return_annotation() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!("test missing_return()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "test.return_type");
    assert_eq!(
        diagnostics[0].message,
        "test declaration has no return type annotation"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"() or Result<(), E>\",\"actual_type\":\"missing\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn public_function_rejects_unknown_declared_effect_label() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> () effects [telepathy]\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.unknown");
    assert_eq!(
        diagnostics[0].message,
        "declared effect `telepathy` is not known"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"boundary\":\"public_function\""));
    assert!(details.contains("\"effect\":\"telepathy\""));
    assert!(details.contains("\"known_effects\":[\"stdio\",\"fs\",\"net\",\"db\",\"time\",\"random\",\"process\",\"concurrency\"]"));
}

#[test]
fn accepts_coarse_declared_effect_labels() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio, fs, net, db, time, random, process, concurrency]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn test_declarations_are_not_callable_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "test helper() -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  helper()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(diagnostics[0].message, "unresolved call_target `helper`");
}

#[test]
fn duplicate_function_like_declaration_names_are_static_errors() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test same() -> ()\n",
            "  ()\n",
            "end\n",
            "fn same() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate function declaration name `same`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"function\"")
    );
}
