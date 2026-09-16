use super::*;

#[test]
fn schema_fields_compose_format_neutral_schemas_as_nested_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Metadata\n",
            "  version: Int where version > 0\n",
            "  label: String\n",
            "  validate version < 10\n",
            "end\n",
            "schema Envelope\n",
            "  metadata: Metadata\n",
            "  payload: String\n",
            "end\n",
            "pub fn main(value: {metadata: {version: Int, label: String}, payload: String}) -> Result<{metadata: {version: Int, label: String}, payload: String}, String>\n",
            "  byte_decode_envelope(value)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    let ir = lowered.ir.expect("typed IR should be built");
    let envelope = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "Envelope")
        .expect("envelope metadata should be emitted");
    let metadata = envelope.fields[0]
        .payload_schema
        .as_ref()
        .expect("composed target metadata should be retained");
    assert_eq!(metadata.schema_name, "Metadata");
    assert_eq!(metadata.fields[0].predicate.as_deref(), Some("version > 0"));
    assert_eq!(metadata.validation.as_deref(), Some("version < 10"));
}

#[test]
fn schema_composition_preserves_alias_resolution_failures_and_type_alias_ambiguity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type PayloadShape\n",
            "  PayloadShape(Int)\n",
            "end\n",
            "pub type Shared = PayloadShape\n",
            "schema Shared\n",
            "  value: Int\n",
            "end\n",
            "pub schema WrongAlias = PayloadShape\n",
            "pub schema CycleA = CycleB\n",
            "pub schema CycleB = CycleA\n",
            "schema AmbiguousHost\n",
            "  child: Shared\n",
            "end\n",
            "schema WrongKindHost\n",
            "  child: WrongAlias\n",
            "end\n",
            "schema CyclicAliasHost\n",
            "  child: CycleA\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    for (schema, reason) in [
        ("AmbiguousHost", "ambiguous_type_and_schema"),
        ("WrongKindHost", "wrong_kind"),
        ("CyclicAliasHost", "cyclic_composition"),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.composition_reference"
                    && matches!(
                        &diagnostic.details,
                        veln_diagnostics::JsonValue::Object(entries)
                            if entries.iter().any(|(key, value)| {
                                key == "schema"
                                    && value == &veln_diagnostics::JsonValue::string(schema)
                            }) && entries.iter().any(|(key, value)| {
                                key == "reason"
                                    && value == &veln_diagnostics::JsonValue::string(reason)
                            })
                    )
            }),
            "missing {reason} for {schema}: {:#?}",
            lowered.diagnostics
        );
    }
}

#[test]
fn schema_field_grammar_precedes_colliding_schema_names_and_aliases() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub schema Int\n",
            "  nested: String\n",
            "end\n",
            "pub schema String = Int\n",
            "schema NeutralHost\n",
            "  count: Int\n",
            "  label: String\n",
            "end\n",
            "pub schema UInt8\n",
            "  value: Int\n",
            "end\n",
            "pub schema UInt16be = UInt8\n",
            "schema BinaryHost\n",
            "  format binary\n",
            "  byte: UInt8\n",
            "  word: UInt16be\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(
        schema_decode_value_type(&module, &module.schemas[1]),
        Some(Type::Record(vec![
            ("count".to_string(), Type::int()),
            ("label".to_string(), Type::string()),
        ])),
    );
    assert_eq!(
        schema_decode_value_type(&module, &module.schemas[3]),
        Some(Type::Record(vec![
            ("byte".to_string(), Type::int()),
            ("word".to_string(), Type::int()),
        ])),
    );
    let decode_specs = schema_decode_specs(&module);
    for schema_name in ["NeutralHost", "BinaryHost"] {
        let spec = decode_specs
            .iter()
            .find(|spec| spec.schema_name == schema_name)
            .expect("host schema should lower to decode metadata");
        assert!(
            spec.fields
                .iter()
                .all(|field| field.payload_schema.is_none()),
            "{schema_name} primitives must not carry nested schema metadata"
        );
    }
}

#[test]
fn schema_field_predicates_reject_invalid_nested_references_at_declaration_time() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  payload: ByteView(length)\n",
            "end\n",
            "schema Host\n",
            "  format binary\n",
            "  checked: UInt8 where checked == later.length\n",
            "  later: Header\n",
            "  missing: UInt8 where missing == later.unknown\n",
            "  incompatible: UInt8 where incompatible == later.payload\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);
    let reasons = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.field_predicate_reference")
        .filter_map(|diagnostic| match &diagnostic.details {
            veln_diagnostics::JsonValue::Object(entries) => {
                entries.iter().find_map(|(key, value)| {
                    (key == "reason").then(|| value.to_json().trim_matches('"').to_string())
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        reasons,
        vec![
            "forward_field_reference",
            "unknown_field_reference",
            "incompatible_field_reference",
        ]
    );
}

#[test]
fn binary_schema_composition_resolves_later_targets_and_nested_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Frame\n",
            "  format binary\n",
            "  header: Header\n",
            "  payload: ByteView(header.length)\n",
            "  validate header.kind >= 0\n",
            "end\n",
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "end\n",
            "pub fn main(view: ByteView) -> Result<{header: {length: Int, kind: Int}, payload: ByteView}, String>\n",
            "  byte_decode_frame(view)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let frame = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "Frame")
        .expect("frame metadata should be emitted");
    assert_eq!(frame.fields[0].name, "header");
    assert_eq!(
        frame.fields[1].length_field.as_deref(),
        Some("header.length")
    );
}

#[test]
fn binary_schema_composition_accepts_nested_references_in_every_expression_position() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  count: UInt8\n",
            "  kind: UInt8\n",
            "end\n",
            "schema RepeatHost\n",
            "  format binary\n",
            "  header: Header\n",
            "  items: Repeat(header.count, UInt8)\n",
            "end\n",
            "schema DispatchHost\n",
            "  format binary\n",
            "  header: Header\n",
            "  payload: Dispatch(header.kind, 1 => UInt8)\n",
            "end\n",
            "schema ExtensionHost\n",
            "  format binary\n",
            "  header: Header\n",
            "  payload: ExtensionDispatch(header.kind, header.length, 1 => UInt8)\n",
            "end\n",
            "schema PredicateHost\n",
            "  format binary\n",
            "  header: Header\n",
            "  checked: UInt8 where checked == header.kind\n",
            "  validate header.kind == 1\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let eligibility = module
        .schemas
        .iter()
        .skip(1)
        .map(|schema| {
            (
                schema.name.as_deref().unwrap_or("<missing>"),
                schema_decode_value_type(&module, schema).is_some(),
                schema_encode_value_type(&module, schema).is_some(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        eligibility,
        vec![
            ("RepeatHost", true, true),
            ("DispatchHost", true, true),
            ("ExtensionHost", true, true),
            ("PredicateHost", true, true),
        ]
    );
}

#[test]
fn schema_composition_checks_decode_and_encode_eligibility_independently() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Expanding<A>\n",
            "  Next(value: Expanding<fn(Int) -> String>)\n",
            "end\n",
            "schema DecodeOnlyTarget\n",
            "  payload: Expanding<Int>\n",
            "end\n",
            "schema Host\n",
            "  child: DecodeOnlyTarget\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let target = &module.schemas[0];

    assert!(schema_decode_value_type(&module, target).is_some());
    assert!(schema_encode_value_type(&module, target).is_none());

    let lowered = lower_checked_surface_module(&module);
    let host_reasons = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.composition_reference")
        .filter_map(|diagnostic| match &diagnostic.details {
            veln_diagnostics::JsonValue::Object(entries)
                if entries.iter().any(|(key, value)| {
                    key == "schema" && value == &veln_diagnostics::JsonValue::string("Host")
                }) =>
            {
                entries.iter().find_map(|(key, value)| {
                    (key == "reason").then(|| value.to_json().trim_matches('"').to_string())
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(host_reasons, vec!["encode_ineligible_target"]);
}

#[test]
fn generated_schema_decode_helpers_resolve_from_binary_schema_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ArithmeticPacket\n",
            "  format binary\n",
            "\n",
            "  width: UInt8\n",
            "  item_count: UInt8\n",
            "  payload_length: UInt16be where not (payload_length != width * item_count) and true\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{width: Int, item_count: Int, payload_length: Int}, String>\n",
            "  byte_decode_arithmetic_packet(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{width: Int, item_count: Int, payload_length: Int}>\n",
            "  byte_decode_step_arithmetic_packet(view, base)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecode(name),
            ..
        } if name == "ArithmeticPacket"
    ));
    let step = core
        .functions
        .iter()
        .find(|function| function.name == "step")
        .expect("step should be lowered");
    let CoreStmtKind::Return { expr } = &step.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "ArithmeticPacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "ArithmeticPacket");
    assert_eq!(schema.function_name, "byte_decode_arithmetic_packet");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.predicate.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("width", 1, None),
            ("item_count", 1, None),
            (
                "payload_length",
                2,
                Some("not(payload_length != width * item_count) and true"),
            ),
        ]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_anonymous_record_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema AnonymousPacket\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  header: {kind: UInt8, code: UInt16be, tail: UInt16le}\n",
            "  suffix: UInt8\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{prefix: Int, header: {kind: Int, code: Int, tail: Int}, suffix: Int}, String>\n",
            "  byte_decode_anonymous_packet(view)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "AnonymousPacket");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width))
            .collect::<Vec<_>>(),
        vec![("prefix", 1), ("header", 0), ("suffix", 1)]
    );
    let header = schema.fields[1]
        .payload_schema
        .as_ref()
        .expect("anonymous record should carry nested decode metadata");
    assert_eq!(header.schema_name, "");
    assert_eq!(
        header
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field.little_endian,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("kind", 1, 0xff, false),
            ("code", 2, 0xffff, false),
            ("tail", 2, 0xffff, true),
        ]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_nested_anonymous_record_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema AnonymousPacket\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  header: {kind: UInt8, detail: {code: UInt16be, tail: UInt16le}, marker: UInt8}\n",
            "  suffix: UInt8\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{prefix: Int, header: {kind: Int, detail: {code: Int, tail: Int}, marker: Int}, suffix: Int}, String>\n",
            "  byte_decode_anonymous_packet(view)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "AnonymousPacket");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width))
            .collect::<Vec<_>>(),
        vec![("prefix", 1), ("header", 0), ("suffix", 1)]
    );
    let header = schema.fields[1]
        .payload_schema
        .as_ref()
        .expect("anonymous record should carry nested decode metadata");
    assert_eq!(
        header
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width))
            .collect::<Vec<_>>(),
        vec![("kind", 1), ("detail", 0), ("marker", 1)]
    );
    let detail = header.fields[1]
        .payload_schema
        .as_ref()
        .expect("nested anonymous record should carry decode metadata");
    assert_eq!(
        detail
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field.little_endian,
                )
            })
            .collect::<Vec<_>>(),
        vec![("code", 2, 0xffff, false), ("tail", 2, 0xffff, true),]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_recursive_anonymous_record_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema RecursiveAnonymousPacket\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  header: {kind: UInt8, detail: {code: UInt16be, trailer: {tail: UInt16le}}, marker: UInt8}\n",
            "  suffix: UInt8\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{prefix: Int, header: {kind: Int, detail: {code: Int, trailer: {tail: Int}}, marker: Int}, suffix: Int}, String>\n",
            "  byte_decode_recursive_anonymous_packet(view)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "RecursiveAnonymousPacket");
    let header = schema.fields[1]
        .payload_schema
        .as_ref()
        .expect("anonymous record should carry nested decode metadata");
    let detail = header.fields[1]
        .payload_schema
        .as_ref()
        .expect("recursive anonymous record should carry nested decode metadata");
    let trailer = detail.fields[1]
        .payload_schema
        .as_ref()
        .expect("deeper anonymous record should carry decode metadata");
    assert_eq!(
        trailer
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field.little_endian,
                )
            })
            .collect::<Vec<_>>(),
        vec![("tail", 2, 0xffff, true)]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_sibling_nested_anonymous_record_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SiblingNestedAnonymousPacket\n",
            "  format binary\n",
            "\n",
            "  header: {left: {kind: UInt8}, right: {code: UInt8}}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    let header = schema.fields[0]
        .payload_schema
        .as_ref()
        .expect("anonymous record should carry nested decode metadata");
    assert_eq!(
        header
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width))
            .collect::<Vec<_>>(),
        vec![("left", 0), ("right", 0)]
    );
    let left = header.fields[0]
        .payload_schema
        .as_ref()
        .expect("left anonymous record should carry decode metadata");
    assert_eq!(
        left.fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![("kind", 1, 0xff)]
    );
    let right = header.fields[1]
        .payload_schema
        .as_ref()
        .expect("right anonymous record should carry decode metadata");
    assert_eq!(
        right
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![("code", 1, 0xff)]
    );
}
