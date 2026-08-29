use super::*;

#[test]
fn public_schema_aliases_reject_unresolved_private_and_wrong_kind_targets() {
    let facade_source = SourceFile::new(
        "facade.veln",
        concat!(
            "mod facade\n",
            "use wire\n",
            "pub schema MissingPacket = wire::MissingPacket\n",
            "pub schema PrivatePacket = wire::PrivatePacket\n",
            "pub schema FunctionPacket = wire::make_packet\n",
            "pub schema TypePacket = wire::PacketShape\n",
            "pub schema MissingCodecPacket = wire::PacketCodec\n",
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
        ),
    );
    let facade = lower_surface_ast(&parse(&facade_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: facade.module,
        uses: facade.uses,
        aliases: facade.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: wire.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    for (id, message) in [
        (
            "name.unresolved",
            "unresolved schema alias target `wire::MissingPacket`",
        ),
        (
            "name.visibility",
            "schema alias target `wire::PrivatePacket` is private",
        ),
        (
            "name.kind_mismatch",
            "public alias target `wire::make_packet` is a function, not a schema",
        ),
        (
            "name.kind_mismatch",
            "public alias target `wire::PacketShape` is a type, not a schema",
        ),
        (
            "name.unresolved",
            "unresolved schema alias target `wire::PacketCodec`",
        ),
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == id && diagnostic.message == message),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn cyclic_public_schema_aliases_remain_unresolved() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod api\n",
            "pub schema Request = Response\n",
            "pub schema Response = Request\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    for target in ["Response", "Request"] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == format!("unresolved schema alias target `{target}`")
            }),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn dispatch_payload_schema_references_report_resolution_diagnostics() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "type Shape\n",
            "  Shape(Int)\n",
            "end\n",
            "\n",
            "schema MissingPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => MissingPayload)\n",
            "end\n",
            "\n",
            "schema NonSchemaPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => Shape)\n",
            "end\n",
            "\n",
            "schema ImportedPrivatePacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::PrivatePayload)\n",
            "end\n",
            "\n",
            "schema ImportedMissingPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::MissingPayload)\n",
            "end\n",
            "\n",
            "schema ImportedWrongKindPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::WireShape)\n",
            "end\n",
            "\n",
            "schema ImportedPublicPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::PublicPayload)\n",
            "end\n",
            "\n",
            "schema ImportedTextPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::TextPayload)\n",
            "end\n",
            "\n",
            "schema SelfPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => SelfPacket)\n",
            "end\n",
            "\n",
            "schema ForwardPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => LaterPayload)\n",
            "end\n",
            "\n",
            "schema PriorPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "  value: UInt8\n",
            "end\n",
            "\n",
            "schema MixedPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => PriorPayload)\n",
            "end\n",
            "\n",
            "schema LaterPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "schema PrivatePayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub type WireShape\n",
            "  WireShape(Int)\n",
            "end\n",
            "\n",
            "pub schema PublicPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub schema TextPayload\n",
            "  code: Int\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let mut schemas = app.schemas;
    schemas.extend(wire.schemas);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, wire.types].concat(),
        schemas,
        codecs: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_payload_schema",
            "dispatch payload schema `MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "dispatch payload `Shape` resolves to a type, not a schema",
        ),
        (
            "private_imported_payload_schema",
            "imported dispatch payload schema `wire::PrivatePayload` is private",
        ),
        (
            "unknown_payload_schema",
            "dispatch payload schema `wire::MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "dispatch payload `wire::WireShape` resolves to a type, not a schema",
        ),
        (
            "non_binary_payload_schema",
            "dispatch payload schema `wire::TextPayload` must use `format binary`",
        ),
        (
            "recursive_payload_missing_length_bound",
            "dispatch payload schema `SelfPacket` requires parent dispatch field `payload` to include a length field",
        ),
        (
            "forward_payload_schema",
            "dispatch payload schema `LaterPayload` must be declared before schema `ForwardPacket`",
        ),
        (
            "incompatible_payload_type",
            "dispatch payload case `2` decodes as `{code: Int, value: Int}`, but earlier cases decode as `Int`",
        ),
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.dispatch_payload"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{diagnostics:#?}"
        );
    }
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "schema.dispatch_payload"
                || !diagnostic.message.contains("wire::PublicPayload")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn dispatch_payload_schema_incompatible_helper_reports_helper_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ForwardByteViewPayload\n",
            "  format binary\n",
            "  payload: ByteView(later_length)\n",
            "  later_length: UInt8\n",
            "end\n",
            "\n",
            "schema ClosedPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => ForwardByteViewPayload)\n",
            "end\n",
            "\n",
            "schema ExtensionPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => ForwardByteViewPayload)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let payload_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.dispatch_payload")
        .collect::<Vec<_>>();
    assert_eq!(payload_diagnostics.len(), 2, "{diagnostics:#?}");
    for diagnostic in payload_diagnostics {
        assert_eq!(
            diagnostic.message,
            "dispatch payload schema `ForwardByteViewPayload` is outside the generated binary schema helper slice"
        );
        let details = diagnostic.details.to_json();
        assert!(details.contains("\"reason\":\"incompatible_payload_schema\""));
        assert!(
            details.contains(
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"ClosedPacket\"},{\"kind\":\"field\",\"name\":\"payload\"}]"
            ) || details.contains(
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"ExtensionPacket\"},{\"kind\":\"field\",\"name\":\"payload\"}]"
            )
        );
        assert!(
            details.contains(
                "\"expected_decode_helper\":\"byte_decode_step_forward_byte_view_payload\""
            )
        );
        assert!(
            details.contains("\"decode_helper_boundary\":\"generated_binary_schema_decode_step\"")
        );
        assert!(
            details
                .contains("\"expected_encode_helper\":\"byte_encode_forward_byte_view_payload\"")
        );
        assert!(details.contains("\"encode_helper_boundary\":\"generated_binary_schema_encode\""));
        assert!(details.contains("\"unsupported_nested_schema\":\"ForwardByteViewPayload\""));
        assert!(details.contains("\"unsupported_nested_field\":\"payload\""));
        assert!(details.contains(
            "\"unsupported_nested_layout_reason\":\"ineligible_byte_view_length_reference\""
        ));
        assert!(details.contains("\"unavailable_helper_directions\":[\"decode\",\"encode\"]"));
        assert_eq!(diagnostic.related.len(), 3);
        assert!(diagnostic.related[0].to_json().contains(
            "does not expose the generated `byte_decode_step_forward_byte_view_payload` helper"
        ));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("Nested dispatch payload field `ForwardByteViewPayload.payload` prevents generated decode and encode helpers")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("length reference `later_length` to be declared before field `payload`")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("expected `byte_decode_step_forward_byte_view_payload` and `byte_encode_forward_byte_view_payload`")
        );
    }
}

#[test]
fn repeat_payload_schema_references_report_resolution_diagnostics() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "type Shape\n",
            "  Shape(Int)\n",
            "end\n",
            "\n",
            "schema MissingCountPacket\n",
            "  format binary\n",
            "  items: Repeat(count, UInt8)\n",
            "end\n",
            "\n",
            "schema ForwardCountPacket\n",
            "  format binary\n",
            "  items: Repeat(count, UInt8)\n",
            "  count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindCountPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  items: Repeat(flags, UInt8)\n",
            "end\n",
            "\n",
            "schema MissingPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, MissingPayload)\n",
            "end\n",
            "\n",
            "schema NonSchemaPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, Shape)\n",
            "end\n",
            "\n",
            "schema ImportedPrivatePacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::PrivatePayload)\n",
            "end\n",
            "\n",
            "schema ImportedMissingPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::MissingPayload)\n",
            "end\n",
            "\n",
            "schema ImportedWrongKindPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::WireShape)\n",
            "end\n",
            "\n",
            "schema ImportedPublicPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::PublicPayload)\n",
            "end\n",
            "\n",
            "schema ImportedTextPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::TextPayload)\n",
            "end\n",
            "\n",
            "schema SelfPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, SelfPacket)\n",
            "end\n",
            "\n",
            "schema ForwardPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, LaterPayload)\n",
            "end\n",
            "\n",
            "schema LaterPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "schema PrivatePayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub type WireShape\n",
            "  WireShape(Int)\n",
            "end\n",
            "\n",
            "pub schema PublicPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub schema TextPayload\n",
            "  code: Int\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let mut schemas = app.schemas;
    schemas.extend(wire.schemas);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, wire.types].concat(),
        schemas,
        codecs: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count field `count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count field `count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count field `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{diagnostics:#?}"
        );
    }

    for (reason, message) in [
        (
            "unknown_payload_schema",
            "repeat payload schema `MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "repeat payload `Shape` resolves to a type, not a schema",
        ),
        (
            "private_imported_payload_schema",
            "imported repeat payload schema `wire::PrivatePayload` is private",
        ),
        (
            "unknown_payload_schema",
            "repeat payload schema `wire::MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "repeat payload `wire::WireShape` resolves to a type, not a schema",
        ),
        (
            "non_binary_payload_schema",
            "repeat payload schema `wire::TextPayload` must use `format binary`",
        ),
        (
            "self_payload_schema",
            "repeat payload schema `SelfPacket` cannot reference itself",
        ),
        (
            "forward_payload_schema",
            "repeat payload schema `LaterPayload` must be declared before schema `ForwardPacket`",
        ),
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_payload"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{diagnostics:#?}"
        );
    }
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "schema.repeat_payload"
                || !diagnostic.message.contains("wire::PublicPayload")
        }),
        "{diagnostics:#?}"
    );
}
