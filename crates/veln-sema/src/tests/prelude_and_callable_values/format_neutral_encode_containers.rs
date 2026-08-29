use super::*;

#[test]
fn generated_schema_encode_helpers_resolve_anonymous_record_fields() {
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
            "pub fn write(packet: {prefix: Int, header: {kind: Int, detail: {code: Int, tail: Int}, marker: Int}, suffix: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_anonymous_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let write = core
        .functions
        .iter()
        .find(|function| function.name == "write")
        .expect("write should be lowered");
    let CoreStmtKind::Return { expr } = &write.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "AnonymousPacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "AnonymousPacket")
        .expect("anonymous schema should be emitted");
    let header = schema.fields[1]
        .payload_schema
        .as_ref()
        .expect("anonymous record should carry nested metadata");
    let detail = header.fields[1]
        .payload_schema
        .as_ref()
        .expect("nested anonymous record should carry nested metadata");
    assert_eq!(
        detail
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![("code", 2, 0xffff), ("tail", 2, 0xffff)]
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_from_format_neutral_schema_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PlainPacket\n",
            "  code: Int\n",
            "  label: String\n",
            "  items: List<Int>\n",
            "  flags: List<Bool>\n",
            "  ratios: List<Float>\n",
            "  names: List<String>\n",
            "  scores: Dict<String, Int>\n",
            "  labels: Dict<String, String>\n",
            "  states: Dict<String, Bool>\n",
            "  weights: Dict<String, Float>\n",
            "  metadata: {ready: Bool, score: Float}\n",
            "  optional_code: Option<Int>\n",
            "  optional_metadata: Option<{ready: Bool, score: Float}>\n",
            "end\n",
            "\n",
            "pub fn main(packet: {code: Int, label: String, items: List<Int>, flags: List<Bool>, ratios: List<Float>, names: List<String>, scores: Dict<String, Int>, labels: Dict<String, String>, states: Dict<String, Bool>, weights: Dict<String, Float>, metadata: {ready: Bool, score: Float}, optional_code: Option<Int>, optional_metadata: Option<{ready: Bool, score: Float}>}) -> Result<{code: Int, label: String, items: List<Int>, flags: List<Bool>, ratios: List<Float>, names: List<String>, scores: Dict<String, Int>, labels: Dict<String, String>, states: Dict<String, Bool>, weights: Dict<String, Float>, metadata: {ready: Bool, score: Float}, optional_code: Option<Int>, optional_metadata: Option<{ready: Bool, score: Float}>}, String>\n",
            "  byte_decode_plain_packet(packet)\n",
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
            target: CoreCallTarget::SchemaNeutralDecode(name),
            ..
        } if name == "PlainPacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "PlainPacket");
    assert_eq!(schema.function_name, "byte_decode_plain_packet");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width))
            .collect::<Vec<_>>(),
        vec![
            ("code", 0),
            ("label", 0),
            ("items", 0),
            ("flags", 0),
            ("ratios", 0),
            ("names", 0),
            ("scores", 0),
            ("labels", 0),
            ("states", 0),
            ("weights", 0),
            ("metadata", 0),
            ("optional_code", 0),
            ("optional_metadata", 0),
        ]
    );
}

#[test]
fn generated_schema_encode_helpers_resolve_from_supported_format_neutral_encode_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ScalarPacket\n",
            "  code: Int\n",
            "  ready: Bool\n",
            "  ratio: Float\n",
            "  label: String\n",
            "  optional_code: Option<Int>\n",
            "  optional_ready: Option<Bool>\n",
            "  optional_ratio: Option<Float>\n",
            "  optional_label: Option<String>\n",
            "  items: List<Int>\n",
            "  flags: List<Bool>\n",
            "  ratios: List<Float>\n",
            "  labels: List<String>\n",
            "  vec_items: Vec<Int>\n",
            "  vec_flags: Vec<Bool>\n",
            "  vec_ratios: Vec<Float>\n",
            "  vec_labels: Vec<String>\n",
            "  scores: Dict<String, Int>\n",
            "  states: Dict<String, Bool>\n",
            "  weights: Dict<String, Float>\n",
            "  names: Dict<String, String>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {code: Int, ready: Bool, ratio: Float, label: String, optional_code: Option<Int>, optional_ready: Option<Bool>, optional_ratio: Option<Float>, optional_label: Option<String>, items: List<Int>, flags: List<Bool>, ratios: List<Float>, labels: List<String>, vec_items: Vec<Int>, vec_flags: Vec<Bool>, vec_ratios: Vec<Float>, vec_labels: Vec<String>, scores: Dict<String, Int>, states: Dict<String, Bool>, weights: Dict<String, Float>, names: Dict<String, String>}) -> Result<{code: Int, ready: Bool, ratio: Float, label: String, optional_code: Option<Int>, optional_ready: Option<Bool>, optional_ratio: Option<Float>, optional_label: Option<String>, items: List<Int>, flags: List<Bool>, ratios: List<Float>, labels: List<String>, vec_items: Vec<Int>, vec_flags: Vec<Bool>, vec_ratios: Vec<Float>, vec_labels: Vec<String>, scores: Dict<String, Int>, states: Dict<String, Bool>, weights: Dict<String, Float>, names: Dict<String, String>}, String>\n",
            "  byte_encode_scalar_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {code: Int, ready: Bool, ratio: Float, label: String, optional_code: Option<Int>, optional_ready: Option<Bool>, optional_ratio: Option<Float>, optional_label: Option<String>, items: List<Int>, flags: List<Bool>, ratios: List<Float>, labels: List<String>, vec_items: Vec<Int>, vec_flags: Vec<Bool>, vec_ratios: Vec<Float>, vec_labels: Vec<String>, scores: Dict<String, Int>, states: Dict<String, Bool>, weights: Dict<String, Float>, names: Dict<String, String>}) -> Result<{code: Int, ready: Bool, ratio: Float, label: String, optional_code: Option<Int>, optional_ready: Option<Bool>, optional_ratio: Option<Float>, optional_label: Option<String>, items: List<Int>, flags: List<Bool>, ratios: List<Float>, labels: List<String>, vec_items: Vec<Int>, vec_flags: Vec<Bool>, vec_ratios: Vec<Float>, vec_labels: Vec<String>, scores: Dict<String, Int>, states: Dict<String, Bool>, weights: Dict<String, Float>, names: Dict<String, String>}, String>\n",
            "  encode ScalarPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "ScalarPacket" && args.len() == 1
        ));
    }

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
            } if name == "ScalarPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_schema_encode_helpers_resolve_from_format_neutral_container_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ContainerPacket\n",
            "  items: Option<List<Int>>\n",
            "  metadata: {names: List<String>, flags: Option<List<Bool>>}\n",
            "  outcome: Result<Int, String>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {items: Option<List<Int>>, metadata: {names: List<String>, flags: Option<List<Bool>>}, outcome: Result<Int, String>}) -> Result<{items: Option<List<Int>>, metadata: {names: List<String>, flags: Option<List<Bool>>}, outcome: Result<Int, String>}, String>\n",
            "  byte_encode_container_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {items: Option<List<Int>>, metadata: {names: List<String>, flags: Option<List<Bool>>}, outcome: Result<Int, String>}) -> Result<{items: Option<List<Int>>, metadata: {names: List<String>, flags: Option<List<Bool>>}, outcome: Result<Int, String>}, String>\n",
            "  encode ContainerPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "ContainerPacket" && args.len() == 1
        ));
    }

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
            } if name == "ContainerPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_schema_encode_helpers_resolve_from_result_option_encode_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ResultOptionPacket\n",
            "  code: Result<Int, Option<String>>\n",
            "  ready: Result<Bool, Option<Int>>\n",
            "  ratio: Result<Float, Option<Bool>>\n",
            "  metadata: {label: Result<String, Option<Float>>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {code: Result<Int, Option<String>>, ready: Result<Bool, Option<Int>>, ratio: Result<Float, Option<Bool>>, metadata: {label: Result<String, Option<Float>>}}) -> Result<{code: Result<Int, Option<String>>, ready: Result<Bool, Option<Int>>, ratio: Result<Float, Option<Bool>>, metadata: {label: Result<String, Option<Float>>}}, String>\n",
            "  byte_encode_result_option_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {code: Result<Int, Option<String>>, ready: Result<Bool, Option<Int>>, ratio: Result<Float, Option<Bool>>, metadata: {label: Result<String, Option<Float>>}}) -> Result<{code: Result<Int, Option<String>>, ready: Result<Bool, Option<Int>>, ratio: Result<Float, Option<Bool>>, metadata: {label: Result<String, Option<Float>>}}, String>\n",
            "  encode ResultOptionPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "ResultOptionPacket" && args.len() == 1
        ));
    }

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
            } if name == "ResultOptionPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_schema_encode_helpers_resolve_from_option_dict_encode_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema OptionDictPacket\n",
            "  scores: Option<Dict<String, Int>>\n",
            "  states: Option<Dict<String, Bool>>\n",
            "  weights: Option<Dict<String, Float>>\n",
            "  names: Option<Dict<String, String>>\n",
            "  metadata: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>, metadata: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>}}) -> Result<{scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>, metadata: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>}}, String>\n",
            "  byte_encode_option_dict_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>, metadata: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>}}) -> Result<{scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>, metadata: {scores: Option<Dict<String, Int>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, names: Option<Dict<String, String>>}}, String>\n",
            "  encode OptionDictPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "OptionDictPacket" && args.len() == 1
        ));
    }

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
            } if name == "OptionDictPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_schema_encode_helpers_resolve_from_option_vec_encode_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema OptionVecPacket\n",
            "  items: Vec<Option<Int>>\n",
            "  flags: Vec<Option<Bool>>\n",
            "  ratios: Vec<Option<Float>>\n",
            "  labels: Vec<Option<String>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {items: Vec<Option<Int>>, flags: Vec<Option<Bool>>, ratios: Vec<Option<Float>>, labels: Vec<Option<String>>}) -> Result<{items: Vec<Option<Int>>, flags: Vec<Option<Bool>>, ratios: Vec<Option<Float>>, labels: Vec<Option<String>>}, String>\n",
            "  byte_encode_option_vec_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {items: Vec<Option<Int>>, flags: Vec<Option<Bool>>, ratios: Vec<Option<Float>>, labels: Vec<Option<String>>}) -> Result<{items: Vec<Option<Int>>, flags: Vec<Option<Bool>>, ratios: Vec<Option<Float>>, labels: Vec<Option<String>>}, String>\n",
            "  encode OptionVecPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "OptionVecPacket" && args.len() == 1
        ));
    }

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
            } if name == "OptionVecPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_schema_encode_helpers_resolve_from_list_option_encode_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ListOptionPacket\n",
            "  items: List<Option<Int>>\n",
            "  flags: List<Option<Bool>>\n",
            "  ratios: List<Option<Float>>\n",
            "  labels: List<Option<String>>\n",
            "  nested_items: List<Option<List<Int>>>\n",
            "  nested_flags: List<Option<List<Bool>>>\n",
            "  nested_ratios: List<Option<List<Float>>>\n",
            "  nested_labels: List<Option<List<String>>>\n",
            "  metadata: {items: List<Option<Int>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_labels: List<Option<List<String>>>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {items: List<Option<Int>>, flags: List<Option<Bool>>, ratios: List<Option<Float>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_flags: List<Option<List<Bool>>>, nested_ratios: List<Option<List<Float>>>, nested_labels: List<Option<List<String>>>, metadata: {items: List<Option<Int>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_labels: List<Option<List<String>>>}}) -> Result<{items: List<Option<Int>>, flags: List<Option<Bool>>, ratios: List<Option<Float>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_flags: List<Option<List<Bool>>>, nested_ratios: List<Option<List<Float>>>, nested_labels: List<Option<List<String>>>, metadata: {items: List<Option<Int>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_labels: List<Option<List<String>>>}}, String>\n",
            "  byte_encode_list_option_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {items: List<Option<Int>>, flags: List<Option<Bool>>, ratios: List<Option<Float>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_flags: List<Option<List<Bool>>>, nested_ratios: List<Option<List<Float>>>, nested_labels: List<Option<List<String>>>, metadata: {items: List<Option<Int>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_labels: List<Option<List<String>>>}}) -> Result<{items: List<Option<Int>>, flags: List<Option<Bool>>, ratios: List<Option<Float>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_flags: List<Option<List<Bool>>>, nested_ratios: List<Option<List<Float>>>, nested_labels: List<Option<List<String>>>, metadata: {items: List<Option<Int>>, labels: List<Option<String>>, nested_items: List<Option<List<Int>>>, nested_labels: List<Option<List<String>>>}}, String>\n",
            "  encode ListOptionPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "ListOptionPacket" && args.len() == 1
        ));
    }

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
            } if name == "ListOptionPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_schema_encode_helpers_resolve_from_nested_vec_scalar_encode_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema NestedVecPacket\n",
            "  metadata: {items: Vec<Int>, flags: Vec<Bool>, ratios: Vec<Float>, labels: Vec<String>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {metadata: {items: Vec<Int>, flags: Vec<Bool>, ratios: Vec<Float>, labels: Vec<String>}}) -> Result<{metadata: {items: Vec<Int>, flags: Vec<Bool>, ratios: Vec<Float>, labels: Vec<String>}}, String>\n",
            "  byte_encode_nested_vec_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {metadata: {items: Vec<Int>, flags: Vec<Bool>, ratios: Vec<Float>, labels: Vec<String>}}) -> Result<{metadata: {items: Vec<Int>, flags: Vec<Bool>, ratios: Vec<Float>, labels: Vec<String>}}, String>\n",
            "  encode NestedVecPacket from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for function_name in ["direct", "explicit"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaNeutralEncode(name),
                args,
            } if name == "NestedVecPacket" && args.len() == 1
        ));
    }

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
            } if name == "NestedVecPacket" && args.len() == 1
        ));
    }
}

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_deep_recursive_container_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  items: Option<List<List<Int>>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {items: Option<List<List<Int>>>}) -> Result<{items: Option<List<List<Int>>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {items: Option<List<List<Int>>>}) -> Result<{items: Option<List<List<Int>>>}, String>\n",
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
