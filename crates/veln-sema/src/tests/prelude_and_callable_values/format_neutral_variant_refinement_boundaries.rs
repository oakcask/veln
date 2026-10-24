use super::*;

#[test]
fn binary_schema_decoders_reject_refinements_they_cannot_establish() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "pub type Wrapper<A>\n",
            "  pub Wrapped(value: A)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  format binary\n",
            "  singleton: State::Ready\n",
            "  union: State::Ready | State::Closed\n",
            "  nested: Wrapper<State::Ready>::Wrapped\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let nested_inner = module.schemas[0].fields[2].ty_refinements[0].alternatives[0].type_arguments
        [0]
    .ty_refinements[0]
        .span
        .clone();

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.variant_refinement_decode_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.composition_reference"),
        "{diagnostics:#?}"
    );
    assert_eq!(unsupported[2].span.as_ref(), Some(&nested_inner));
    assert_eq!(
        unsupported[1].details.to_json(),
        concat!(
            "{\"target_type\":\"State::Ready | State::Closed\",",
            "\"base_type\":\"State\",\"expected_variants\":[\"Ready\",\"Closed\"],",
            "\"decoder\":\"byte_decode_packet\"}"
        )
    );
    assert!(unsupported.iter().all(|diagnostic| {
        diagnostic.related.len() == 1
            && diagnostic.related[0]
                .to_json()
                .contains("\"provenance\":\"generated_binary_schema\"")
    }));
}

#[test]
fn format_neutral_decoder_can_establish_refinements_from_its_typed_input() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  singleton: State::Ready\n",
            "  union: State::Ready | State::Closed\n",
            "  nested: Option<List<State::Ready>>\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(
        schema_decode_value_type(&module, &module.schemas[0])
            .expect("typed pass-through decoder should establish its refined input")
            .render(),
        "{singleton: State::Ready, union: State, nested: Option<List<State::Ready>>}"
    );
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
fn format_neutral_schema_encode_helpers_reject_source_less_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Direct\n",
            "  value: Option<Int>::Some\n",
            "end\n",
            "\n",
            "schema Nested\n",
            "  value: List<Option<Int>::Some>\n",
            "end\n",
            "\n",
            "schema Ordinary\n",
            "  value: Option<Result<Int, String>>\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    for schema in &module.schemas[..2] {
        assert!(
            schema_encode_value_type(&module, schema).is_none(),
            "source-less refinements must not publish encode helpers"
        );
    }
    assert!(
        schema_encode_value_type(&module, &module.schemas[2]).is_some(),
        "ordinary source-less containers must remain eligible"
    );
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "schema.format_neutral_encode_helper")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.composition_reference"),
        "{diagnostics:#?}"
    );
}

#[test]
fn bound_schema_encode_helpers_preserve_variant_mismatch_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  state: State::Ready\n",
            "end\n",
            "\n",
            "schema NestedPacket\n",
            "  state: Option<List<State::Ready>>\n",
            "end\n",
            "\n",
            "fn direct_base(state: State) -> Result<{state: State::Ready}, String>\n",
            "  byte_encode_packet({state: state})\n",
            "end\n",
            "\n",
            "fn bound_base(state: State) -> Result<{state: State::Ready}, String>\n",
            "  let encoder = byte_encode_packet\n",
            "  encoder({state: state})\n",
            "end\n",
            "\n",
            "fn direct_excluded(state: State::Closed) -> Result<{state: State::Ready}, String>\n",
            "  byte_encode_packet({state: state})\n",
            "end\n",
            "\n",
            "fn bound_excluded(state: State::Closed) -> Result<{state: State::Ready}, String>\n",
            "  let encoder = byte_encode_packet\n",
            "  encoder({state: state})\n",
            "end\n",
            "\n",
            "fn direct_nested_base(state: Option<List<State>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  byte_encode_nested_packet({state: state})\n",
            "end\n",
            "\n",
            "fn bound_nested_base(state: Option<List<State>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  let encoder = byte_encode_nested_packet\n",
            "  encoder({state: state})\n",
            "end\n",
            "\n",
            "fn direct_nested_excluded(state: Option<List<State::Closed>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  byte_encode_nested_packet({state: state})\n",
            "end\n",
            "\n",
            "fn bound_nested_excluded(state: Option<List<State::Closed>>) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  let encoder = byte_encode_nested_packet\n",
            "  encoder({state: state})\n",
            "end\n",
            "\n",
            "fn ordinary_encoder(value: {state: State::Ready}) -> Result<{state: State::Ready}, String>\n",
            "  Ok(value)\n",
            "end\n",
            "\n",
            "fn ordinary_bound(state: State) -> Result<{state: State::Ready}, String>\n",
            "  let encoder = ordinary_encoder\n",
            "  encoder({state: state})\n",
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
        8,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        1,
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
