use super::*;

#[test]
fn generated_format_neutral_schema_encode_helpers_retain_variant_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(value: Int)\n",
            "  pub Closed(value: Int)\n",
            "  pub Failed(value: Int)\n",
            "end\n",
            "\n",
            "pub type Envelope\n",
            "  pub Empty\n",
            "  pub Nested(value: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  singleton: State::Ready\n",
            "  union: State::Ready | State::Closed\n",
            "  nested: {payload: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>, envelope: Envelope}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let value_type = schema_encode_value_type(&module, &module.schemas[0])
        .expect("encode helper should retain eligible refinements");
    let rendered = value_type.render();

    assert!(rendered.contains("singleton: State::Ready"), "{rendered}");
    assert!(
        rendered.contains("union: State::Ready | State::Closed"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Result<State::Ready, State::Ready | State::Closed>"),
        "{rendered}"
    );
}

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_admitted_variants() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(value: Int)\n",
            "  pub Closed(value: Int)\n",
            "  pub Failed(value: Int)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  singleton: State::Ready\n",
            "  union: State::Ready | State::Closed\n",
            "end\n",
            "\n",
            "pub fn direct(ready: State::Ready, closed: State::Closed) -> Result<{singleton: State::Ready, union: State::Ready | State::Closed}, String>\n",
            "  byte_encode_packet({singleton: ready, union: closed})\n",
            "end\n",
            "\n",
            "pub fn explicit(ready: State::Ready) -> Result<{singleton: State::Ready, union: State::Ready | State::Closed}, String>\n",
            "  encode Packet from {singleton: ready, union: ready}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    for function_name in ["direct", "explicit"] {
        let function = ir
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be in IR");
        let IrStmtKind::Return { value } = &function.body[0].kind else {
            panic!("tail expression should lower as IR return");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::Call {
                target: IrCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "Packet" && args.len() == 1
        ));
    }
}

#[test]
fn generated_format_neutral_schema_encode_helpers_reject_base_and_excluded_variants() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "  pub Failed\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  state: State::Ready | State::Closed\n",
            "end\n",
            "\n",
            "pub fn base(state: State) -> Result<{state: State::Ready | State::Closed}, String>\n",
            "  byte_encode_packet({state: state})\n",
            "end\n",
            "\n",
            "pub fn excluded(state: State::Failed) -> Result<{state: State::Ready | State::Closed}, String>\n",
            "  encode Packet from {state: state}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 2, "{diagnostics:#?}");
    assert!(
        mismatches
            .iter()
            .all(|diagnostic| diagnostic.message.contains("State::Ready | State::Closed")),
        "{mismatches:#?}"
    );
}

#[test]
fn nested_schema_encode_refinements_report_variant_mismatches_only_at_schema_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  state: Option<List<State::Ready>>\n",
            "end\n",
            "\n",
            "pub fn encode_base(state: Option<List<State>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  encode Packet from {state: state}\n",
            "end\n",
            "\n",
            "pub fn encode_excluded(state: Option<List<State::Closed>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  encode Packet from {state: state}\n",
            "end\n",
            "\n",
            "pub fn encode_unrelated(state: Option<List<String>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  encode Packet from {state: state}\n",
            "end\n",
            "\n",
            "pub fn ordinary(state: Option<List<State>>) -> Option<List<State::Ready>>\n",
            "  state\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);
    let variant_mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    let type_mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(variant_mismatches.len(), 2, "{diagnostics:#?}");
    assert!(
        variant_mismatches
            .iter()
            .all(|diagnostic| diagnostic.message.contains("State::Ready")),
        "{variant_mismatches:#?}"
    );
    assert_eq!(type_mismatches.len(), 2, "{diagnostics:#?}");
    assert!(
        type_mismatches
            .iter()
            .all(|diagnostic| diagnostic.message.contains("Option<List<State::Ready>>")),
        "{type_mismatches:#?}"
    );
    assert!(
        type_mismatches
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Option<List<String>>")),
        "{type_mismatches:#?}"
    );
    assert!(
        type_mismatches
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Option<List<State>>")),
        "{type_mismatches:#?}"
    );
}

#[test]
fn encode_only_refinement_fields_still_validate_predicate_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  state: State::Ready where state == missing\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    assert!(
        schema_decode_value_type(&module, &module.schemas[0]).is_none(),
        "a refinement field must remain absent from the decode record"
    );
    assert!(
        schema_encode_value_type(&module, &module.schemas[0]).is_some(),
        "the refinement field should still publish its encode helper"
    );
    let diagnostics = analyze_surface_module(&module);
    let predicate_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.field_predicate_reference")
        .collect::<Vec<_>>();

    assert_eq!(predicate_diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(
        predicate_diagnostics[0].message.contains("missing"),
        "{predicate_diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.format_neutral_encode_helper"),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_format_neutral_schema_encode_helpers_reject_private_variants() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "  pub Visible\n",
            "end\n",
            "\n",
            "schema PrivatePacket\n",
            "  state: State::Hidden\n",
            "end\n",
            "\n",
            "pub schema HiddenPacket\n",
            "  state: State::Hidden\n",
            "end\n",
            "\n",
            "pub schema VisiblePacket\n",
            "  state: State::Visible\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    assert!(
        schema_encode_value_type(&module, &module.schemas[0]).is_some(),
        "a private same-module helper may retain a private constructor"
    );
    assert!(
        schema_encode_value_type(&module, &module.schemas[1]).is_none(),
        "a public helper must not expose a private constructor"
    );
    assert_eq!(
        schema_encode_value_type(&module, &module.schemas[2])
            .expect("public constructor should remain eligible")
            .render(),
        "{state: State::Visible}"
    );
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.format_neutral_encode_helper"),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_format_neutral_schema_encode_helpers_check_written_collapsed_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type Singleton\n",
            "  Hidden\n",
            "end\n",
            "\n",
            "pub type State\n",
            "  Hidden\n",
            "  pub Visible\n",
            "end\n",
            "\n",
            "pub schema PublicSingleton\n",
            "  value: Option<List<Singleton::Hidden>>\n",
            "end\n",
            "\n",
            "pub schema PublicFullSet\n",
            "  value: Option<List<State::Hidden | State::Visible>>\n",
            "end\n",
            "\n",
            "schema PrivateSingleton\n",
            "  value: Option<List<Singleton::Hidden>>\n",
            "end\n",
            "\n",
            "schema PrivateFullSet\n",
            "  value: Option<List<State::Hidden | State::Visible>>\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    assert!(
        schema_encode_value_type(&module, &module.schemas[0]).is_none(),
        "a public helper must reject a nested private singleton before canonicalization"
    );
    assert!(
        schema_encode_value_type(&module, &module.schemas[1]).is_none(),
        "a public helper must reject a nested private full-set selection before canonicalization"
    );
    for schema in &module.schemas[2..] {
        assert!(
            schema_encode_value_type(&module, schema).is_some(),
            "a private same-module helper may use collapsed private selections"
        );
    }
}

#[test]
fn composed_format_neutral_schema_encode_helpers_resolve_child_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Inner\n",
            "  state: State::Ready\n",
            "end\n",
            "\n",
            "schema Outer\n",
            "  inner: Inner\n",
            "end\n",
            "\n",
            "pub fn encode_outer(ready: State::Ready) -> Result<{inner: {state: State::Ready}}, String>\n",
            "  byte_encode_outer({inner: {state: ready}})\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    assert_eq!(
        schema_encode_value_type(&module, &module.schemas[1])
            .expect("composed encode helper should resolve the refined child field")
            .render(),
        "{inner: {state: State::Ready}}"
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn composed_format_neutral_schema_encode_helpers_reject_child_variant_mismatches() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Inner\n",
            "  state: State::Ready\n",
            "end\n",
            "\n",
            "schema Outer\n",
            "  inner: Inner\n",
            "end\n",
            "\n",
            "pub fn encode_base(state: State) -> Result<{inner: {state: State::Ready}}, String>\n",
            "  byte_encode_outer({inner: {state: state}})\n",
            "end\n",
            "\n",
            "pub fn encode_excluded(state: State::Closed) -> Result<{inner: {state: State::Ready}}, String>\n",
            "  byte_encode_outer({inner: {state: state}})\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.composition_reference"
                && diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn format_neutral_schema_encode_helpers_reject_opaque_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!("schema Packet\n", "  stream: NetStream::Open\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    assert!(schema_encode_value_type(&module, &module.schemas[0]).is_none());
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.format_neutral_encode_helper"),
        "{diagnostics:#?}"
    );
}

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_imported_public_refinements() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "schema Packet\n",
            "  state: wire::State::Ready | wire::State::Closed\n",
            "end\n",
            "pub fn encode_state(state: wire::State::Ready) -> Result<{state: State::Ready | State::Closed}, String>\n",
            "  byte_encode_packet({state: state})\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
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
        schemas: app.schemas,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}
