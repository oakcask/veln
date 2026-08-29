use super::*;

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_same_module_source_adts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Payload\n",
            "  Empty\n",
            "  Scalar(value: Int)\n",
            "  Metadata(value: {label: String, scores: Dict<String, Int>})\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  payload: Payload\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {payload: Payload}) -> Result<{payload: Payload}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {payload: Payload}) -> Result<{payload: Payload}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
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
fn generated_format_neutral_schema_encode_helpers_accept_recursive_source_adts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Tree\n",
            "  Leaf(value: Int)\n",
            "  Branch(children: List<Tree>)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  forest: Option<Dict<String, Vec<List<Tree>>>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {forest: Option<Dict<String, Vec<List<Tree>>>>}) -> Result<{forest: Option<Dict<String, Vec<List<Tree>>>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {forest: Option<Dict<String, Vec<List<Tree>>>>}) -> Result<{forest: Option<Dict<String, Vec<List<Tree>>>>}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
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
fn generated_format_neutral_schema_encode_helpers_accept_recursive_source_adts_with_growing_type_arguments()
 {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Growing<A>\n",
            "  Stop(value: A)\n",
            "  Next(value: Growing<Option<A>>)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  payload: Growing<Int>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {payload: Growing<Int>}) -> Result<{payload: Growing<Int>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {payload: Growing<Int>}) -> Result<{payload: Growing<Int>}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
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
fn generated_format_neutral_schema_encode_helpers_accept_mutually_recursive_source_adts_with_growing_type_arguments()
 {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left<A>\n",
            "  LeftDone(value: A)\n",
            "  LeftNext(value: Right<Option<A>>)\n",
            "end\n",
            "\n",
            "type Right<B>\n",
            "  RightDone(value: B)\n",
            "  RightNext(value: Left<Vec<B>>)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  payload: Left<Int>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {payload: Left<Int>}) -> Result<{payload: Left<Int>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {payload: Left<Int>}) -> Result<{payload: Left<Int>}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
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
fn generated_format_neutral_schema_encode_helpers_reject_recursive_source_adts_with_unsupported_changed_type_arguments()
 {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Expanding<A>\n",
            "  Next(value: Expanding<fn(Int) -> String>)\n",
            "end\n",
            "\n",
            "schema BadPacket\n",
            "  payload: Expanding<Int>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {payload: Expanding<Int>}) -> Result<{payload: Expanding<Int>}, String>\n",
            "  byte_encode_bad_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {payload: Expanding<Int>}) -> Result<{payload: Expanding<Int>}, String>\n",
            "  encode BadPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);
    let field_span = module.schemas[0].fields[0].span.clone();

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.format_neutral_decode_helper"),
        "encode traversal must not change recursive decode eligibility: {:#?}",
        lowered.diagnostics
    );
    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.format_neutral_encode_helper")
        .expect("unsupported recursive payload should be rejected");
    assert_eq!(diagnostic.span, Some(field_span));
    assert_eq!(
        diagnostic.message,
        "format-neutral schema field `payload` cannot expose a generated encode helper because `Expanding<Int>` is not a recursive format-neutral visible shape"
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported schema must not lower to typed IR"
    );
}

#[test]
fn generated_format_neutral_schema_encode_helpers_memoize_repeated_child_adt_dags() {
    fn source_with_repeated_children(depth: usize, leaf_type: &str) -> String {
        let mut source = format!("type Dup0\n  Leaf(value: {leaf_type})\nend\n\n");
        for level in 1..=depth {
            source.push_str(&format!(
                "type Dup{level}\n  Pair(left: Dup{}, right: Dup{})\nend\n\n",
                level - 1,
                level - 1,
            ));
        }
        source.push_str(&format!("schema Packet\n  payload: Dup{depth}\nend\n"));
        source
    }

    for (leaf_type, supported) in [("Int", true), ("fn(Int) -> String", false)] {
        let source = SourceFile::new("main.veln", source_with_repeated_children(16, leaf_type));
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);
        let encode_diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "schema.format_neutral_encode_helper");
        if supported {
            assert!(
                encode_diagnostic.is_none(),
                "eligible repeated-child ADT DAG should be accepted: {diagnostics:#?}"
            );
        } else {
            assert!(
                encode_diagnostic.is_some(),
                "unsupported repeated-child ADT DAG should be rejected: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_public_imported_source_adts() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "schema Packet\n",
            "  payload: wire::Payload\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {payload: Payload}) -> Result<{payload: Payload}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {payload: Payload}) -> Result<{payload: Payload}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub type Payload\n",
            "  pub Empty\n",
            "  pub Scalar(value: Int)\n",
            "  pub Metadata(value: {label: String, scores: Dict<String, Int>})\n",
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
        codecs: Vec::new(),
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
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
fn generated_format_neutral_schema_encode_helpers_reject_source_adts_with_unsupported_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type CallbackPayload\n",
            "  CallbackPayload(callback: fn(Int) -> String)\n",
            "end\n",
            "\n",
            "schema BadPacket\n",
            "  payload: CallbackPayload\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.format_neutral_encode_helper")
        .expect("unsupported format-neutral encode helper should be rejected");
    assert_eq!(
        diagnostic.message,
        "format-neutral schema field `payload` cannot expose a generated encode helper because `CallbackPayload` is not a recursive format-neutral visible shape"
    );
    assert!(diagnostic.related.iter().any(|related| {
        let related = related.to_json();
        related.contains("Option<T>")
            && related.contains("Dict<String, T>")
            && related.contains("same-module or public imported source ADTs")
    }));
}

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_unsupported_field_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadPacket\n",
            "  code: Int\n",
            "  callbacks: Vec<fn(Int) -> String>\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper")
        .expect("unsupported field should be reported");
    assert_eq!(
        diagnostic.message,
        format!(
            "format-neutral schema field `callbacks` cannot expose a generated decode helper because `Vec<fn(Int) -> String>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
        )
    );
    assert!(diagnostic.related.iter().any(|related| {
        related
            .to_json()
            .contains("Generated format-neutral decode helpers for schema `BadPacket`")
    }));
}

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_unsupported_dict_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadPacket\n",
            "  numeric_scores: Dict<Int, Int>\n",
            "  optional_scores: Option<Dict<Int, Int>>\n",
            "  nested_record_scores: {scores: Dict<Int, Option<Int>>}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper")
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        messages,
        vec![
            format!(
                "format-neutral schema field `numeric_scores` cannot expose a generated decode helper because `Dict<Int, Int>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `optional_scores` cannot expose a generated decode helper because `Option<Dict<Int, Int>>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `nested_record_scores` cannot expose a generated decode helper because `{{ scores : Dict<Int, Option<Int>> }}` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
        ]
    );
}

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_non_visible_result_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type LocalPayload\n",
            "  LocalPayload(value: Int)\n",
            "end\n",
            "\n",
            "schema BadPacket\n",
            "  result_bad_dict_key: Result<Dict<Int, Int>, String>\n",
            "  result_callback: Result<Int, fn(Int) -> String>\n",
            "  result_vec: Result<Vec<fn(Int) -> String>, String>\n",
            "  metadata: {payload: Result<LocalPayload, String>, callback: Result<Int, fn(Int) -> String>}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper")
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        messages,
        vec![
            format!(
                "format-neutral schema field `result_bad_dict_key` cannot expose a generated decode helper because `Result<Dict<Int, Int>, String>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `result_callback` cannot expose a generated decode helper because `Result<Int, fn(Int) -> String>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `result_vec` cannot expose a generated decode helper because `Result<Vec<fn(Int) -> String>, String>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `metadata` cannot expose a generated decode helper because `{{ payload : Result<LocalPayload, String>, callback : Result<Int, fn(Int) -> String> }}` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
        ]
    );
}

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_source_adts_with_unsupported_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type CallbackPayload\n",
            "  CallbackPayload(callback: fn(Int) -> String)\n",
            "end\n",
            "\n",
            "type VecPayload\n",
            "  VecPayload(items: Vec<fn(Int) -> String>)\n",
            "end\n",
            "\n",
            "schema BadPacket\n",
            "  callback_payload: CallbackPayload\n",
            "  optional_vec_payload: Option<VecPayload>\n",
            "  metadata: {payload: CallbackPayload}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper")
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        messages,
        vec![
            format!(
                "format-neutral schema field `callback_payload` cannot expose a generated decode helper because `CallbackPayload` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `optional_vec_payload` cannot expose a generated decode helper because `Option<VecPayload>` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
            format!(
                "format-neutral schema field `metadata` cannot expose a generated decode helper because `{{ payload : CallbackPayload }}` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
            ),
        ]
    );
}

#[test]
fn generated_format_neutral_schema_decode_helpers_accept_lists_inside_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  metadata: {items: List<Int>, flags: List<Bool>, ratios: List<Float>, names: List<String>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {metadata: {items: List<Int>, flags: List<Bool>, ratios: List<Float>, names: List<String>}}) -> Result<{metadata: {items: List<Int>, flags: List<Bool>, ratios: List<Float>, names: List<String>}}, String>\n",
            "  byte_decode_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    assert_eq!(ir.schema_decoders[0].schema_name, "Packet");
}

#[test]
fn generated_format_neutral_schema_decode_helpers_accept_vecs_recursively() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  items: Vec<Int>\n",
            "  metadata: {items: Vec<Option<String>>, results: Result<Vec<Int>, Vec<String>>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {items: Vec<Int>, metadata: {items: Vec<Option<String>>, results: Result<Vec<Int>, Vec<String>>}}) -> Result<{items: Vec<Int>, metadata: {items: Vec<Option<String>>, results: Result<Vec<Int>, Vec<String>>}}, String>\n",
            "  byte_decode_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    assert_eq!(ir.schema_decoders[0].schema_name, "Packet");
}

#[test]
fn generated_format_neutral_schema_decode_helpers_accept_option_inside_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  metadata: {label: Option<String>, retries: Option<Int>, active: Option<Bool>, ratio: Option<Float>, items: Option<List<Int>>, flags: Option<List<Bool>>, ratios: Option<List<Float>>, names: Option<List<String>>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {metadata: {label: Option<String>, retries: Option<Int>, active: Option<Bool>, ratio: Option<Float>, items: Option<List<Int>>, flags: Option<List<Bool>>, ratios: Option<List<Float>>, names: Option<List<String>>}}) -> Result<{metadata: {label: Option<String>, retries: Option<Int>, active: Option<Bool>, ratio: Option<Float>, items: Option<List<Int>>, flags: Option<List<Bool>>, ratios: Option<List<Float>>, names: Option<List<String>>}}, String>\n",
            "  byte_decode_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    assert_eq!(ir.schema_decoders[0].schema_name, "Packet");
}

#[test]
fn generated_format_neutral_schema_decode_helpers_accept_dicts_inside_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  metadata: {scores: Dict<String, Int>, labels: Dict<String, String>, states: Dict<String, Bool>, weights: Dict<String, Float>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {metadata: {scores: Dict<String, Int>, labels: Dict<String, String>, states: Dict<String, Bool>, weights: Dict<String, Float>}}) -> Result<{metadata: {scores: Dict<String, Int>, labels: Dict<String, String>, states: Dict<String, Bool>, weights: Dict<String, Float>}}, String>\n",
            "  byte_decode_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    assert_eq!(ir.schema_decoders[0].schema_name, "Packet");
}
