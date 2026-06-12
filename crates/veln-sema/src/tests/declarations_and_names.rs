use super::*;

#[test]
fn public_function_requires_explicit_type_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public parameter `value` has no type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public function has no return type annotation"
    }));
}

#[test]
fn public_function_accepts_omitted_empty_effect_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main() -> Int\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_function_may_omit_boundary_annotations_when_inference_is_complete() {
    let source = SourceFile::new("main.veln", "fn answer()\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn private_function_reports_incomplete_annotation_inference() {
    let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `value` has no inferred type"
            && diagnostic
                .details
                .to_json()
                .contains("\"missing_fact\":\"parameter_type\"")
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private function has no inferred return type"
            && diagnostic
                .details
                .to_json()
                .contains("\"missing_fact\":\"return_type\"")
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn test_declaration_requires_explicit_test_shape() {
    let source = SourceFile::new(
        "main_test.veln",
        "test bad(value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.parameters"
            && diagnostic.message == "test declaration has parameters"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.return_type"
            && diagnostic.message == "test declaration returns `Int`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn test_declaration_checks_omitted_effect_boundary() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test prints() -> ()\n",
            "  stdio::println(\"hello\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_test");
    assert_eq!(
        diagnostics[0].message,
        "test declaration uses undeclared effect `stdio`"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"node_id\":\"test-1\"")
    );
}

#[test]
fn function_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new("main.veln", "fn helper() -> Int effects []\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a function declaration"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"boundary\":\"private_function\""));
    assert!(details.contains("\"declared_effects\":[]"));
    assert_eq!(diagnostics[0].related.len(), 2);
}

#[test]
fn public_function_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new("main.veln", "pub fn helper() -> Int effects []\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a function declaration"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"boundary\":\"public_function\"")
    );
}

#[test]
fn test_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new(
        "main_test.veln",
        "test helper() -> () effects []\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a test declaration"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"boundary\":\"test_declaration\"")
    );
}

#[test]
fn test_declaration_accepts_result_unit_return() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test returns_result() -> Result<(), String>\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_declaration_accepts_unit_return() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!("test returns_unit() -> ()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

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
            "  length: UInt24be\n",
            "  kind: UInt8\n",
            "  stream_id: UInt31be\n",
            "  checksum: UInt32be\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 5);
    for primitive in ["UInt16be", "UInt24be", "UInt8", "UInt31be", "UInt32be"] {
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
fn exact_width_binary_schema_primitives_are_not_ordinary_types_or_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn ordinary_types(value: UInt16be, another: UInt8) -> {short: UInt24be, wide: UInt32be}\n",
            "  UInt31be\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 5);
    for (primitive, reason) in [
        ("UInt16be", "parameter_type"),
        ("UInt8", "parameter_type"),
        ("UInt24be", "return_type"),
        ("UInt32be", "return_type"),
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

#[test]
fn duplicate_codec_declaration_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec same for Header decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec same for Header encode\n",
            "  derive encode\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate codec declaration name `same`"
    }));
}

#[test]
fn codec_declarations_resolve_schema_targets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "codec MissingCodec for Missing decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved codec schema `Missing`"
    }));
}

#[test]
fn codec_declarations_resolve_imported_public_schema_targets() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app.main\n",
            "use app.wire\n",
            "codec ImportedDecode for wire::Packet decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod app.wire\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: app.codecs,
        functions: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn codec_declarations_require_written_use_for_imported_schema_targets() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "codec MissingUseDecode for other::Packet decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );
    let other_source = SourceFile::new(
        "other.veln",
        concat!(
            "mod other\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let other = lower_surface_ast(&parse(&other_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        schemas: other.schemas,
        codecs: app.codecs,
        functions: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(
        diagnostics[0].message,
        "unresolved codec schema `other::Packet`"
    );
}

#[test]
fn codec_declarations_reject_private_and_wrong_kind_imported_schema_targets() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "codec PrivateDecode for wire::PrivatePacket decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec FunctionDecode for wire::make_packet decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec TypeDecode for wire::PacketShape decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec CodecDecode for wire::PacketCodec decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "pub schema PublicPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "schema PrivatePacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn make_packet() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub type PacketShape\n",
            "  pub Packet(Int)\n",
            "end\n",
            "\n",
            "pub codec PacketCodec for PublicPacket decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        codecs: [app.codecs, wire.codecs].concat(),
        functions: wire.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.visibility"
            && diagnostic.message == "codec schema `wire::PrivatePacket` is private"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message
                == "codec schema target `wire::make_packet` is a function, not a schema"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message
                == "codec schema target `wire::PacketShape` is a type, not a schema"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message
                == "codec schema target `wire::PacketCodec` is a codec, not a schema"
    }));
}

#[test]
fn codec_decode_with_accepts_boundary_signature() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderDecode for Header decode\n",
            "  decode with decode_header\n",
            "end\n",
            "\n",
            "fn decode_header(input: ByteView, base: ByteOffset) -> DecodeStep<Int>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn codec_decode_with_reports_signature_shape_at_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderDecode for Header decode\n",
            "  decode with decode_header\n",
            "end\n",
            "\n",
            "fn decode_header(input: ByteChunk, base: ByteCount) -> Result<Int, String>\n",
            "  Ok(0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    for (reason, message, actual) in [
        (
            "input_view_type",
            "decode function first parameter must be `ByteView`",
            "ByteChunk",
        ),
        (
            "base_offset_type",
            "decode function second parameter must be `ByteOffset`",
            "ByteCount",
        ),
        (
            "return_type",
            "decode function must return `DecodeStep<T>`",
            "Result<Int, String>",
        ),
    ] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "codec.decode_signature"
                && diagnostic.message == message
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 7)
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"reason\":\"{reason}\""))
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"actual_signature\":\"{actual}\""))
                && diagnostic.related.len() == 1
        }));
    }
}

#[test]
fn codec_decode_with_reports_unresolved_function_at_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderDecode for Header decode\n",
            "  decode with missing_decode\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(
        diagnostics[0].message,
        "unresolved decode function `missing_decode`"
    );
    assert!(diagnostics[0].related.is_empty());
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"phase\":\"codec\"")
    );
}

#[test]
fn codec_encode_with_accepts_boundary_signature() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderEncode for Header encode\n",
            "  encode with encode_header\n",
            "end\n",
            "\n",
            "fn encode_header(chunks: List<ByteChunk>) -> EncodeStep<String>\n",
            "  Encoded(chunks)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn codec_encode_with_reports_return_shape_at_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderEncode for Header encode\n",
            "  encode with encode_header\n",
            "end\n",
            "\n",
            "fn encode_header() -> Result<List<ByteChunk>, String>\n",
            "  Ok(list_nil())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.id, "codec.encode_signature");
    assert_eq!(
        diagnostic.message,
        "encode function must return `EncodeStep<TState>`"
    );
    assert!(
        diagnostic
            .span
            .as_ref()
            .is_some_and(|span| span.start.line == 7)
    );
    assert!(
        diagnostic
            .details
            .to_json()
            .contains("\"reason\":\"return_type\"")
    );
    assert!(
        diagnostic
            .details
            .to_json()
            .contains("\"actual_signature\":\"Result<List<ByteChunk>, String>\"")
    );
    assert_eq!(diagnostic.related.len(), 1);
}

#[test]
fn codec_encode_with_reports_unresolved_function_at_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderEncode for Header encode\n",
            "  encode with missing_encode\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(
        diagnostics[0].message,
        "unresolved encode function `missing_encode`"
    );
    assert!(diagnostics[0].related.is_empty());
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"phase\":\"codec\"")
    );
}

#[test]
fn codec_encode_with_ignores_functions_from_other_modules() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "codec HeaderEncode for Header encode\n",
            "  encode with encode_header\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "pub fn encode_header(chunks: List<ByteChunk>) -> EncodeStep<String>\n",
            "  Encoded(chunks)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        schemas: app.schemas,
        codecs: app.codecs,
        functions: wire.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(
        diagnostics[0].message,
        "unresolved encode function `encode_header`"
    );
}

#[test]
fn generated_schema_mappings_report_source_target_and_type_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {length: Int, kind: String, flags: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt16be\n",
            "  kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    length = missing_length\n",
            "    missing_target = kind\n",
            "    kind = kind\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.mapping_source_field"
                && diagnostic.message
                    == "schema mapping source field `missing_length` is not declared"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.mapping_target_field"
                && diagnostic.message
                    == "schema mapping target field `missing_target` is not declared"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.mapping_type"
                && diagnostic.message
                    == "schema mapping target field `kind` expects `String`, but source field `kind` decodes as `Int`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.mapping_missing_target_field"
                && diagnostic.message == "schema mapping does not assign target field `flags`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_use_aliases_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use platform.io\n",
            "use local.io\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate import alias name `io`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"module\"")
    );
}

#[test]
fn public_function_alias_rejects_type_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub fn parse = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `Document` is a type, not a function"
    }));
}

#[test]
fn public_type_alias_rejects_function_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub type Document = parse\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `parse` is a function, not a type"
    }));
}

#[test]
fn public_alias_rejects_unresolved_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub fn parse = impl::parse\n",
            "pub type Document = impl::Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `impl::parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved type alias target `impl::Document`"
    }));
}

#[test]
fn public_alias_names_share_member_namespaces() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn parse = parse\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub type Document = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate function alias name `parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate type alias name `Document`"
    }));
}

#[test]
fn use_declarations_require_module_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!("use platform.io\n", "fn main() -> ()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "module.missing_identity");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Module);
    assert_eq!(
        diagnostics[0].message,
        "module import requires a module identity"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"field\":\"module_identity\"")
    );
}

#[test]
fn duplicate_parameter_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int, value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate parameter name `value`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn let_names_cannot_duplicate_the_function_value_scope() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int) -> Int\n  let value = 1\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate local binding name `value`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn wildcard_let_pattern_does_not_bind_or_shadow_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Int) -> Int\n",
            "  let _: Int = value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Expr { expr } = &main.body[0].kind else {
        panic!("wildcard let should lower as expression statement");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "value"));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "value"));
    assert!(lowered.ir.is_some());
}

#[test]
fn duplicate_record_field_names_are_static_errors() {
    let source = SourceFile::new("main.veln", "fn bad() -> {a: Int}\n  {a: 1, a: 2}\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate record field name `a`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"record_field\"")
    );
}

#[test]
fn duplicate_pattern_bindings_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: {left: Int, right: Int}) -> Int\n",
            "  match input\n",
            "    {left: value, right: value} => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate pattern binding name `value`"
                && diagnostic.related.len() == 1
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_record_pattern_field_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: {value: Int}) -> Int\n",
            "  match input\n",
            "    {value: first, value: second} => first\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate record pattern field name `value`"
                && diagnostic.related.len() == 1
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn reports_hole_with_declared_return_expected_type() {
    let source = SourceFile::new("main.veln", "fn todo() -> Result<(), AppError>\n  _\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Hole);
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result<(), AppError>\",",
            "\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"candidate_status\":\"query_only\",",
            "\"application_policy\":\"manual_review_required\",",
            "\"query\":\"fn() -> Result<(), AppError>\"}]}"
        )
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn ranks_visible_symbol_candidates_for_hole_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"candidates\":["));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains(concat!(
        "\"span\":{\"file\":\"main.veln\",",
        "\"start\":{\"line\":3,\"column\":3,\"offset\":48},",
        "\"end\":{\"line\":3,\"column\":4,\"offset\":49}}"
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains("\"replacement\":\"limit\""));
    assert!(details.contains("\"target\":{\"node_id\":\"hole-"));
    assert!(details.contains("\"edit_summary\":\"Replace hole with `fallback`\""));
    assert!(details.contains(concat!(
        "\"evidence\":[{\"kind\":\"type\",\"status\":\"passed\",",
        "\"expected_type\":\"Int\",\"candidate_type\":\"Int\"},",
        "{\"kind\":\"ranking\",\"status\":\"ranked\",\"rank\":1,"
    )));
    assert!(details.contains(concat!(
        "\"known_limits\":[\"edit is advisory and unapplied\",",
        "\"tests and examples have not been run\"]"
    )));
    assert!(details.contains(concat!(
        "\"blocking_obligations\":[\"manual_review_required\",",
        "\"verification.not_run\"]"
    )));
    assert!(details.contains(concat!(
        "\"verification_hint\":{\"command\":\"veln check --json main.veln\",",
        "\"scope\":\"after_applying_candidate_edit\"}"
    )));
    assert!(details.contains("\"application_status\":\"unapplied\""));
}
