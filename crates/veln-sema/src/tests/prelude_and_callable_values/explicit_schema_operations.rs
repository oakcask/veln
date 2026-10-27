use super::*;

#[test]
fn explicit_schema_validate_expression_erases_direct_refinement_inputs() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Connected\n",
            "  Closing\n",
            "  Closed\n",
            "end\n",
            "schema UnrefinedPacket\n",
            "  state: State\n",
            "end\n",
            "schema ConnectedPacket\n",
            "  state: State::Connected | State::Closing\n",
            "  states: List<State>\n",
            "  nested: {state: State}\n",
            "  composed: UnrefinedPacket\n",
            "end\n",
            "fn check(packet: {state: State, states: List<State>, nested: {state: State}, composed: {state: State}}, base: ByteOffset) -> Result<{state: State::Connected | State::Closing, states: List<State>, nested: {state: State}, composed: {state: State}}, DecodeError>\n",
            "  validate ConnectedPacket from packet at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let function = ir
        .functions
        .iter()
        .find(|function| function.name == "check")
        .expect("validation function should lower");
    let IrStmtKind::Return { value } = &function.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaRefinementValidate(name),
            args,
        } if name == "ConnectedPacket" && args.len() == 2
    ));
    assert_eq!(
        ir.schema_decoders
            .iter()
            .find(|schema| schema.schema_name == "ConnectedPacket")
            .expect("validation schema metadata")
            .fields[0]
            .variant_refinement
            .as_ref()
            .map(|refinement| refinement.expected_variants.as_slice()),
        Some(["Connected".to_string(), "Closing".to_string()].as_slice())
    );
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ConnectedPacket")
        .expect("validation schema metadata");
    assert!(
        schema.fields[1..]
            .iter()
            .all(|field| field.variant_refinement.is_none())
    );
}

#[test]
fn explicit_schema_validate_expression_checks_erased_inputs_and_offset() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type OtherState\n  Connected\nend\n",
            "type Box<A>\n  Wrapped(value: A)\nend\n",
            "schema Packet\n",
            "  state: State::Connected\n",
            "  boxed: Box<Int>::Wrapped\n",
            "end\n",
            "fn wrong_identity(packet: {state: OtherState, boxed: Box<Int>}, base: ByteOffset) -> ()\n",
            "  validate Packet from packet at base\n  ()\nend\n",
            "fn wrong_generic(packet: {state: State, boxed: Box<String>}, base: ByteOffset) -> ()\n",
            "  validate Packet from packet at base\n  ()\nend\n",
            "fn wrong_shape(packet: {state: State}, base: ByteOffset) -> ()\n",
            "  validate Packet from packet at base\n  ()\nend\n",
            "fn wrong_offset(packet: {state: State, boxed: Box<Int>}, base: Int) -> ()\n",
            "  validate Packet from packet at base\n  ()\nend\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    let mismatches = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 4, "{:#?}", lowered.diagnostics);
    for constraint in ["schema_validation_value", "schema_validation_offset"] {
        assert!(
            mismatches.iter().any(|diagnostic| {
                matches!(
                    &diagnostic.details,
                    veln_diagnostics::JsonValue::Object(entries)
                        if entries.iter().any(|(key, value)| {
                            key == "constraint"
                                && value
                                    == &veln_diagnostics::JsonValue::string(constraint)
                        })
                )
            }),
            "missing {constraint}: {mismatches:#?}"
        );
    }
    for expected in ["OtherState", "Box<String>", "boxed", "ByteOffset"] {
        assert!(
            mismatches
                .iter()
                .any(|diagnostic| diagnostic.message.contains(expected)),
            "missing mismatch for {expected}: {mismatches:#?}"
        );
    }
}

#[test]
fn explicit_schema_validate_expression_rejects_unsupported_refinement_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "schema RefinedChild\n  state: State::Connected\nend\n",
            "schema NestedPacket\n  direct: State::Connected\n  nested: {state: State::Connected}\nend\n",
            "schema ContainerPacket\n  direct: State::Connected\n  states: List<State::Connected>\nend\n",
            "schema ComposedPacket\n  direct: State::Connected\n  child: RefinedChild\nend\n",
            "schema PredicatePacket\n  direct: State::Connected where true\nend\n",
            "fn nested(value: {direct: State, nested: {state: State}}, base: ByteOffset) -> ()\n",
            "  validate NestedPacket from value at base\n  ()\nend\n",
            "fn container(value: {direct: State, states: List<State>}, base: ByteOffset) -> ()\n",
            "  validate ContainerPacket from value at base\n  ()\nend\n",
            "fn composed(value: {direct: State, child: {state: State}}, base: ByteOffset) -> ()\n",
            "  validate ComposedPacket from value at base\n  ()\nend\n",
            "fn predicate(value: {direct: State}, base: ByteOffset) -> ()\n",
            "  validate PredicatePacket from value at base\n  ()\nend\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    let validation_diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.validate_expression")
        .collect::<Vec<_>>();

    assert_eq!(
        validation_diagnostics.len(),
        4,
        "{:#?}",
        lowered.diagnostics
    );
    for schema in [
        "NestedPacket",
        "ContainerPacket",
        "ComposedPacket",
        "PredicatePacket",
    ] {
        assert!(
            validation_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(schema)),
            "missing unsupported target {schema}: {validation_diagnostics:#?}"
        );
    }
}

#[test]
fn explicit_schema_validate_expression_resolves_alias_and_rejects_invalid_targets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n  pub Connected\n  pub Closed\nend\n",
            "pub schema Packet\n  state: State::Connected\nend\n",
            "pub schema AliasPacket = Packet\n",
            "schema BinaryPacket\n  format binary\n  value: UInt8\nend\n",
            "fn good(value: {state: State}, base: ByteOffset) -> Result<{state: State::Connected}, DecodeError>\n",
            "  validate AliasPacket from value at base\nend\n",
            "fn bad(value: {state: State}, base: ByteOffset) -> ()\n",
            "  validate Missing from value at base\n",
            "  validate State from value at base\n",
            "  validate BinaryPacket from value at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    let validate_diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.validate_expression")
        .collect::<Vec<_>>();
    assert_eq!(validate_diagnostics.len(), 3, "{:#?}", lowered.diagnostics);
    for reason in ["unresolved_schema", "wrong_kind"] {
        assert!(
            validate_diagnostics.iter().any(|diagnostic| {
                matches!(
                    &diagnostic.details,
                    veln_diagnostics::JsonValue::Object(entries)
                        if entries.iter().any(|(key, value)| {
                            key == "reason"
                                && value == &veln_diagnostics::JsonValue::string(reason)
                        })
                )
            }),
            "missing reason {reason}: {validate_diagnostics:#?}"
        );
    }
    assert!(validate_diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("eligible format-neutral schema")
            && diagnostic.message.contains("BinaryPacket")
    }));
}

#[test]
fn explicit_schema_validate_expression_selects_schema_before_equal_spelling_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Packet\n  Packet\nend\n",
            "schema Packet\n  state: State::Connected\nend\n",
            "fn check(value: {state: State}, base: ByteOffset) -> Result<{state: State::Connected}, DecodeError>\n",
            "  validate Packet from value at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let function = ir
        .functions
        .iter()
        .find(|function| function.name == "check")
        .expect("validation function should lower");
    let IrStmtKind::Return { value } = &function.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaRefinementValidate(name),
            args,
        } if name == "Packet" && args.len() == 2
    ));
}

#[test]
fn explicit_schema_validate_expression_reports_ambiguous_schema_qualifier() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use first as wire\n",
            "use second as wire\n",
            "fn check(value: {state: Int}, base: ByteOffset) -> ()\n",
            "  validate wire::Packet from value at base\n",
            "  ()\n",
            "end\n",
        ),
    );
    let first_source = SourceFile::new(
        "first.veln",
        "mod first\npub schema Packet\n  state: Int\nend\n",
    );
    let second_source = SourceFile::new(
        "second.veln",
        "mod second\npub schema Packet\n  state: Int\nend\n",
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let first = lower_surface_ast(&parse(&first_source).tree);
    let second = lower_surface_ast(&parse(&second_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: app.types,
        schemas: [first.schemas, second.schemas].concat(),
        module_values: Vec::new(),
        functions: [app.functions, first.functions, second.functions].concat(),
        invalid_names: app.invalid_names,
    };

    let lowered = lower_checked_surface_module(&module);
    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.validate_expression")
        .expect("ambiguous validation target should be diagnosed");
    assert!(
        diagnostic
            .message
            .contains("ambiguous in the schema namespace"),
        "{diagnostic:#?}"
    );
    assert!(matches!(
        &diagnostic.details,
        veln_diagnostics::JsonValue::Object(entries)
            if entries.iter().any(|(key, value)| {
                key == "reason"
                    && value == &veln_diagnostics::JsonValue::string("ambiguous_target")
            })
    ));
}

#[test]
fn explicit_schema_validate_expression_rejects_inaccessible_private_target() {
    let caller_source = SourceFile::new(
        "other.test.veln",
        concat!(
            "mod other__test_companion\n",
            "use math\n",
            "fn check(value: {state: math::State}, base: ByteOffset) -> ()\n",
            "  let checked: Result<{state: math::State::Connected}, DecodeError> = validate math::Packet from value at base\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "pub type State\n  pub Connected\n  pub Closed\nend\n",
            "schema Packet\n  state: State::Connected\nend\n",
        ),
    );
    let caller = lower_surface_ast(&parse(&caller_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: caller.module,
        uses: caller.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: target.types,
        schemas: target.schemas,
        module_values: Vec::new(),
        functions: caller.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.validate_expression"
                && diagnostic.message
                    == "schema validate expression schema `math::Packet` is private"
        }),
        "{:#?}",
        lowered.diagnostics
    );
}

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_functions_inside_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type LocalPayload\n",
            "  LocalPayload(value: Int)\n",
            "end\n",
            "\n",
            "schema BadPacket\n",
            "  metadata: {payload: LocalPayload, callback: fn(Int) -> String}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper")
        .expect("unsupported function field should be reported");
    assert_eq!(
        diagnostic.message,
        format!(
            "format-neutral schema field `metadata` cannot expose a generated decode helper because `{{ payload : LocalPayload, callback : fn(Int) -> String }}` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
        )
    );
}

#[test]
fn explicit_schema_decode_expression_resolves_as_schema_decode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode PacketWire from view at base\n",
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
            target: CoreCallTarget::SchemaDecodeStep(name),
            args,
        } if name == "PacketWire" && args.len() == 2
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            args,
        } if name == "PacketWire" && args.len() == 2
    ));
}

#[test]
fn explicit_schema_encode_expression_resolves_as_schema_encode_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire from packet\n",
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
            target: CoreCallTarget::SchemaEncode(name),
            args,
        } if name == "PacketWire" && args.len() == 1
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            args,
        } if name == "PacketWire" && args.len() == 1
    ));
}

#[test]
fn explicit_schema_decode_expression_resolves_qualified_public_schema_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode wire::PacketWire from view at base\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        module_values: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn explicit_import_alias_schema_operations_keep_selected_target_identity() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use transport::wire as conduit\n",
            "pub fn decode_packet(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode conduit::PacketWire from view at base\n",
            "end\n",
            "pub fn encode_packet(packet: {wire_length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode conduit::PacketWire from packet\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod transport::wire\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "  wire_length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        module_values: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    assert_eq!(
        crate::name_recovery::normal_imported_use_for_path(
            &module,
            &["conduit".to_string()],
            Some("app")
        )
        .map(|use_decl| use_decl.name.as_str()),
        Some("transport::wire")
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert_named_schema_boundary(
        &lowered,
        "decode_packet",
        "PacketWire",
        ExpectedSchemaBoundary::DecodeStep,
    );
    assert_named_schema_boundary(
        &lowered,
        "encode_packet",
        "PacketWire",
        ExpectedSchemaBoundary::Encode,
    );
}

#[test]
fn schema_import_selection_uses_the_containing_schema_owner_package() {
    fn lower(source: SourceFile, package: Option<&str>, module: &str) -> SurfaceModule {
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module_span = source.span(veln_source::TextRange::new(0, 0));
        match package {
            Some(package) => veln_ast::lower_surface_ast_with_package_module_identity(
                &parsed.tree,
                package.to_string(),
                module.to_string(),
                module_span,
            ),
            None => veln_ast::lower_surface_ast_with_module_identity(
                &parsed.tree,
                module.to_string(),
                module_span,
            ),
        }
    }

    let app_owner = lower(
        SourceFile::new(
            "app/shared.veln",
            "use std::payload as item\nschema Holder\n  body: item::Payload\nend\n",
        ),
        None,
        "std::shared",
    );
    let std_owner = lower(
        SourceFile::new(
            "std/shared.veln",
            "use payload as item\nschema Holder\n  body: item::Payload\nend\n",
        ),
        Some("std"),
        "shared",
    );
    let app_target = lower(
        SourceFile::new(
            "app/payload.veln",
            "pub schema Payload\n  app_value: Int\nend\n",
        ),
        None,
        "std::payload",
    );
    let std_target = lower(
        SourceFile::new(
            "std/payload.veln",
            "pub schema Payload\n  std_value: Int\nend\n",
        ),
        Some("std"),
        "payload",
    );

    let mut module = app_owner;
    for source in [std_owner, app_target, std_target] {
        module.uses.extend(source.uses);
        module.schemas.extend(source.schemas);
    }
    let standard_holder = module
        .schemas
        .iter()
        .find(|schema| {
            schema.name.as_deref() == Some("Holder")
                && schema.package_name.as_deref() == Some("std")
        })
        .expect("standard holder schema");
    let selected = schema_field_target(&module, standard_holder, "item::Payload")
        .expect("standard owner import should select a schema");

    assert_eq!(selected.package_name.as_deref(), Some("std"));
    assert_eq!(selected.fields[0].name, "std_value");
}

#[test]
fn explicit_schema_operations_resolve_public_schema_alias_to_target_schema() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use facade\n",
            "\n",
            "pub fn decode_alias(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode facade::AliasPacket from view at base\n",
            "end\n",
            "\n",
            "pub fn encode_alias(packet: {wire_length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode facade::AliasPacket from packet\n",
            "end\n",
        ),
    );
    let facade_source = SourceFile::new(
        "facade.veln",
        concat!(
            "mod facade\n",
            "use wire\n",
            "\n",
            "pub schema AliasPacket = wire::PublicPacket\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PublicPacket\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let facade = lower_surface_ast(&parse(&facade_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: [app.uses, facade.uses].concat(),
        aliases: facade.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        module_values: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert_named_schema_boundary(
        &lowered,
        "decode_alias",
        "PublicPacket",
        ExpectedSchemaBoundary::DecodeStep,
    );
    assert_named_schema_boundary(
        &lowered,
        "encode_alias",
        "PublicPacket",
        ExpectedSchemaBoundary::Encode,
    );
}

#[test]
fn explicit_schema_validate_resolves_contextual_import_qualifier_and_public_alias() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use facade as handler\n",
            "use wire\n",
            "fn check(value: {state: wire::State}, base: ByteOffset) -> Result<{state: wire::State::Connected}, DecodeError>\n",
            "  validate handler::AliasPacket from value at base\n",
            "end\n",
        ),
    );
    let facade_source = SourceFile::new(
        "facade.veln",
        concat!(
            "mod facade\n",
            "use wire\n",
            "pub schema AliasPacket = wire::PublicPacket\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "pub type State\n  pub Connected\n  pub Closed\nend\n",
            "pub schema PublicPacket\n  state: State::Connected\nend\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let facade = lower_surface_ast(&parse(&facade_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: [app.uses, facade.uses].concat(),
        aliases: facade.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        module_values: Vec::new(),
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert_named_schema_boundary(
        &lowered,
        "check",
        "PublicPacket",
        ExpectedSchemaBoundary::Validate,
    );
}

#[test]
fn explicit_schema_decode_expression_reports_unresolved_private_and_wrong_kind_schema_paths() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "fn missing(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode MissingPacket from view at base\n",
            "end\n",
            "\n",
            "fn private_schema(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PrivatePacket from view at base\n",
            "end\n",
            "\n",
            "fn wrong_type(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PacketShape from view at base\n",
            "end\n",
            "\n",
            "fn wrong_function(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::make_packet from view at base\n",
            "end\n",
            "\n",
            "fn wrong_codec(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PacketCodec from view at base\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PublicPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "schema PrivatePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn make_packet() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub type PacketShape\n",
            "  Box\n",
            "end\n",
            "\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, wire.types].concat(),
        schemas: wire.schemas,
        module_values: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    let messages = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.decode_expression")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        messages.contains(
            &"schema decode expression cannot resolve `MissingPacket` as an eligible binary schema",
        ),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(&"schema decode expression schema `wire::PrivatePacket` is private"),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(
            &"schema decode expression target `wire::PacketShape` is a type, not a schema"
        ),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(
            &"schema decode expression target `wire::make_packet` is a function, not a schema",
        ),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(
            &"schema decode expression cannot resolve `wire::PacketCodec` as an eligible binary schema",
        ),
        "{:#?}",
        lowered.diagnostics
    );
    for (schema_path, reason) in [
        ("MissingPacket", "unresolved_schema"),
        ("wire::PrivatePacket", "private_schema"),
        ("wire::PacketShape", "wrong_kind"),
        ("wire::make_packet", "wrong_kind"),
        ("wire::PacketCodec", "unresolved_schema"),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.decode_expression"
                    && matches!(
                        &diagnostic.details,
                        veln_diagnostics::JsonValue::Object(entries)
                            if entries.iter().any(|(key, value)| {
                                key == "schema_path"
                                    && value
                                        == &veln_diagnostics::JsonValue::string(schema_path)
                            }) && entries.iter().any(|(key, value)| {
                                key == "reason"
                                    && value == &veln_diagnostics::JsonValue::string(reason)
                            })
                    )
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
}

#[test]
fn matching_companion_resolves_private_target_schema_operations_and_composition() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "schema Wrapped\n",
            "  format binary\n",
            "\n",
            "  packet: math::Packet\n",
            "end\n",
            "fn companion_uses_private_schema(view: ByteView, offset: ByteOffset, value: {length: Int}) -> ()\n",
            "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
            "  let encoded: Result<ByteChunk, EncodeError> = encode math::Packet from value\n",
            "  let state: math::State::Hidden = math::State::Hidden\n",
            "  let refined: Result<{state: math::State::Hidden}, String> = encode math::RefinedPacket from {state: state}\n",
            "  let validated: Result<{state: math::State::Hidden}, DecodeError> = validate math::RefinedPacket from {state: state} at offset\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "pub type State\n",
            "  Hidden\n",
            "  pub Visible\n",
            "end\n",
            "schema RefinedPacket\n",
            "  state: State::Hidden\n",
            "end\n",
            "schema Packet\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
        ),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: target.types,
        schemas: companion
            .schemas
            .into_iter()
            .chain(target.schemas)
            .collect(),
        module_values: Vec::new(),
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn companion_private_target_schema_access_preserves_boundaries() {
    for (companion_path, companion_text, expected_message) in [
        (
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "fn missing_import(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression cannot resolve `math::Packet` as an eligible binary schema",
        ),
        (
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "use math\n",
                "fn bare_name(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression cannot resolve `Packet` as an eligible binary schema",
        ),
        (
            "other.test.veln",
            concat!(
                "mod other__test_companion\n",
                "use math\n",
                "fn wrong_target(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression schema `math::Packet` is private",
        ),
        (
            "math_test.veln",
            concat!(
                "mod math_test\n",
                "use math\n",
                "fn integration(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression schema `math::Packet` is private",
        ),
        (
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "use support\n",
                "fn non_transitive(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode support::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression schema `support::Packet` is private",
        ),
    ] {
        let companion_source = SourceFile::new(companion_path, companion_text);
        let target_source = SourceFile::new(
            "math.veln",
            concat!(
                "mod math\n",
                "use support\n",
                "schema Packet\n",
                "  format binary\n",
                "\n",
                "  length: UInt8\n",
                "end\n",
            ),
        );
        let support_source = SourceFile::new(
            "support.veln",
            concat!(
                "mod support\n",
                "schema Packet\n",
                "  format binary\n",
                "\n",
                "  length: UInt8\n",
                "end\n",
            ),
        );
        let companion = lower_surface_ast(&parse(&companion_source).tree);
        let target = lower_surface_ast(&parse(&target_source).tree);
        let support = lower_surface_ast(&parse(&support_source).tree);
        let module = SurfaceModule {
            module: companion.module,
            uses: companion.uses.into_iter().chain(target.uses).collect(),
            aliases: Vec::new(),
            effects: Vec::new(),
            handlers: Vec::new(),
            types: Vec::new(),
            schemas: target.schemas.into_iter().chain(support.schemas).collect(),
            module_values: Vec::new(),
            functions: companion.functions,
            invalid_names: Vec::new(),
        };

        let lowered = lower_checked_surface_module(&module);

        let diagnostic = lowered
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.id == "schema.decode_expression"
                    && diagnostic.message == expected_message
            })
            .unwrap_or_else(|| panic!("{:#?}", lowered.diagnostics));
        if matches!(companion_path, "other.test.veln" | "math.test.veln")
            && expected_message.contains(" is private")
        {
            let details = diagnostic.details.to_json();
            let expected_target = if companion_path == "other.test.veln" {
                "other"
            } else {
                "math"
            };
            assert!(
                details.contains(&format!(
                    "\"companion_target_module\":\"{expected_target}\""
                )),
                "{details}"
            );
            assert!(diagnostic.related.iter().any(|related| {
                let related = related.to_json();
                related.contains("\"kind\":\"companion_target\"")
                    && related.contains(&format!("`{expected_target}`"))
            }));
        }
    }
}
