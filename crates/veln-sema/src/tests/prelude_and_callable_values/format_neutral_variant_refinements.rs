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
fn contextual_schema_encode_members_preserve_variant_mismatch_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  states: Vec<State::Ready>\n",
            "  states_by_name: Dict<String, State::Ready>\n",
            "  optional_state: Option<State::Ready>\n",
            "  success: Result<State::Ready, String>\n",
            "  failure: Result<Int, State::Ready>\n",
            "  unrelated: Vec<Int>\n",
            "end\n",
            "\n",
            "fn accept(ready: State::Ready) -> Result<{states: Vec<State::Ready>, states_by_name: Dict<String, State::Ready>, optional_state: Option<State::Ready>, success: Result<State::Ready, String>, failure: Result<Int, State::Ready>, unrelated: Vec<Int>}, String>\n",
            "  byte_encode_packet({states: [ready], states_by_name: {\"state\": ready}, optional_state: Some(ready), success: Ok(ready), failure: Err(ready), unrelated: [1]})\n",
            "end\n",
            "\n",
            "fn accept_explicit(ready: State::Ready) -> Result<{states: Vec<State::Ready>, states_by_name: Dict<String, State::Ready>, optional_state: Option<State::Ready>, success: Result<State::Ready, String>, failure: Result<Int, State::Ready>, unrelated: Vec<Int>}, String>\n",
            "  encode Packet from {states: [ready], states_by_name: {\"state\": ready}, optional_state: Some(ready), success: Ok(ready), failure: Err(ready), unrelated: [1]}\n",
            "end\n",
            "\n",
            "fn reject_base(state: State) -> Result<{states: Vec<State::Ready>, states_by_name: Dict<String, State::Ready>, optional_state: Option<State::Ready>, success: Result<State::Ready, String>, failure: Result<Int, State::Ready>, unrelated: Vec<Int>}, String>\n",
            "  let encoder = byte_encode_packet\n",
            "  encoder({states: [state], states_by_name: {\"state\": state}, optional_state: Some(state), success: Ok(state), failure: Err(state), unrelated: [1]})\n",
            "end\n",
            "\n",
            "fn reject_excluded(closed: State::Closed) -> Result<{states: Vec<State::Ready>, states_by_name: Dict<String, State::Ready>, optional_state: Option<State::Ready>, success: Result<State::Ready, String>, failure: Result<Int, State::Ready>, unrelated: Vec<Int>}, String>\n",
            "  encode Packet from {states: [closed], states_by_name: {\"state\": closed}, optional_state: Some(closed), success: Ok(closed), failure: Err(closed), unrelated: [1]}\n",
            "end\n",
            "\n",
            "fn reject_unrelated(ready: State::Ready) -> Result<{states: Vec<State::Ready>, states_by_name: Dict<String, State::Ready>, optional_state: Option<State::Ready>, success: Result<State::Ready, String>, failure: Result<Int, State::Ready>, unrelated: Vec<Int>}, String>\n",
            "  byte_encode_packet({states: [ready], states_by_name: {\"state\": ready}, optional_state: Some(ready), success: Ok(ready), failure: Err(ready), unrelated: [\"wrong\"]})\n",
            "end\n",
            "\n",
            "fn ordinary_context(closed: State::Closed) -> ()\n",
            "  let states: Vec<State::Ready> = [closed]\n",
            "  let states_by_name: Dict<String, State::Ready> = {\"state\": closed}\n",
            "  let optional_state: Option<State::Ready> = Some(closed)\n",
            "  let success: Result<State::Ready, String> = Ok(closed)\n",
            "  let failure: Result<Int, State::Ready> = Err(closed)\n",
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

    assert_eq!(variant_mismatches.len(), 10, "{diagnostics:#?}");
    assert_eq!(type_mismatches.len(), 6, "{diagnostics:#?}");
    assert_eq!(diagnostics.len(), 16, "{diagnostics:#?}");
    for mismatch in &variant_mismatches {
        let details = mismatch.details.to_json();
        assert!(
            details.contains("\"expected_type\":\"State::Ready\""),
            "{details}"
        );
        assert!(
            details.contains("\"actual_type\":\"State\"")
                || details.contains("\"actual_type\":\"State::Closed\""),
            "{details}"
        );
        assert!(
            mismatch.related.iter().any(|related| related
                .to_json()
                .contains("\"kind\":\"expected_type_origin\"")),
            "{mismatch:#?}"
        );
    }
    for (constraint, expected_count) in
        [("list_element", 2), ("dict_value", 2), ("call_argument", 6)]
    {
        assert_eq!(
            variant_mismatches
                .iter()
                .filter(|diagnostic| diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"constraint\":\"{constraint}\"")))
                .count(),
            expected_count,
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn bound_schema_encode_records_report_nested_variant_mismatches() {
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
            "pub fn direct(packet: {state: Option<List<State>>}) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {state: Option<List<State::Closed>>}) -> Result<{state: Option<List<State::Ready>>}, String>\n",
            "  encode Packet from packet\n",
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
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn refinement_fields_remain_decode_eligible_and_validate_predicate_references() {
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

    assert_eq!(
        schema_decode_value_type(&module, &module.schemas[0])
            .expect("a typed pass-through decoder should retain the refinement")
            .render(),
        "{state: State::Ready}"
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
fn generated_format_neutral_schema_decode_helpers_retain_variant_refinements() {
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
            "  nested: {payload: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>}\n",
            "end\n",
            "\n",
            "pub fn decode(packet: {singleton: State::Ready, union: State::Ready | State::Closed, nested: {payload: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>}}) -> Result<{singleton: State::Ready, union: State::Ready | State::Closed, nested: {payload: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>}}, String>\n",
            "  byte_decode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn decode_bound(packet: {singleton: State::Ready, union: State::Ready | State::Closed, nested: {payload: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>}}) -> Result<{singleton: State::Ready, union: State::Ready | State::Closed, nested: {payload: Option<List<Vec<Dict<String, Result<State::Ready, State::Ready | State::Closed>>>>>}}, String>\n",
            "  let decoder = byte_decode_packet\n",
            "  decoder(packet)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let value_type = schema_decode_value_type(&module, &module.schemas[0])
        .expect("decode helper should retain eligible refinements");
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

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_strengthening_inputs_and_results() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready\n",
            "  pub Closed\n",
            "  pub Failed\n",
            "end\n",
            "\n",
            "schema RefinedPacket\n",
            "  state: State::Ready | State::Closed\n",
            "end\n",
            "\n",
            "schema BasePacket\n",
            "  state: State\n",
            "end\n",
            "\n",
            "schema SingletonPacket\n",
            "  state: State::Ready\n",
            "end\n",
            "\n",
            "schema NestedPacket\n",
            "  states: Option<List<State::Ready>>\n",
            "end\n",
            "\n",
            "pub fn base_input(packet: {state: State}) -> Result<{state: State::Ready | State::Closed}, String>\n",
            "  byte_decode_refined_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn excluded_input(packet: {state: State::Failed}) -> Result<{state: State::Ready | State::Closed}, String>\n",
            "  byte_decode_refined_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn base_result(packet: {state: State}) -> Result<{state: State::Ready}, String>\n",
            "  byte_decode_base_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn singleton_base_input(packet: {state: State}) -> Result<{state: State::Ready}, String>\n",
            "  byte_decode_singleton_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn nested_base_input(packet: {states: Option<List<State>>}) -> Result<{states: Option<List<State::Ready>>}, String>\n",
            "  byte_decode_nested_packet(packet)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
}

#[test]
fn format_neutral_schema_decode_visibility_pairs_public_and_companion_boundaries() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test decode_private(packet: {state: math::State::Hidden}) -> Result<{state: math::State::Hidden}, String>\n",
            "  math::byte_decode_private_packet(packet)\n",
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
            "schema PrivatePacket\n",
            "  state: State::Hidden\n",
            "end\n",
            "pub schema PublicPacket\n",
            "  state: State::Hidden\n",
            "end\n",
            "pub schema VisiblePacket\n",
            "  state: State::Visible\n",
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
        schemas: target.schemas,
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    assert_eq!(
        schema_decode_value_type(&module, &module.schemas[0])
            .expect("an exact companion may use the private target helper")
            .render(),
        "{state: State::Hidden}"
    );
    assert!(
        schema_decode_value_type(&module, &module.schemas[1]).is_none(),
        "a public helper must not expose a private constructor"
    );
    assert_eq!(
        schema_decode_value_type(&module, &module.schemas[2])
            .expect("a public helper may expose a public constructor")
            .render(),
        "{state: State::Visible}"
    );
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "call.private_companion_target"),
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
