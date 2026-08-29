use super::*;

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_nested_scalar_vec_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  items: Vec<Vec<Int>>\n",
            "  flags: Vec<Vec<Bool>>\n",
            "  ratios: Vec<Vec<Float>>\n",
            "  labels: Vec<Vec<String>>\n",
            "  metadata: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>, metadata: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>}}) -> Result<{items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>, metadata: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>}}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>, metadata: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>}}) -> Result<{items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>, metadata: {items: Vec<Vec<Int>>, flags: Vec<Vec<Bool>>, ratios: Vec<Vec<Float>>, labels: Vec<Vec<String>>}}, String>\n",
            "  encode Packet from packet\n",
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
            } if name == "Packet" && args.len() == 1
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
            } if name == "Packet" && args.len() == 1
        ));
    }
}

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_three_deep_vec_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  items: Vec<Vec<Vec<Int>>>\n",
            "end\n",
            "\n",
            "pub fn main(packet: {items: Vec<Vec<Vec<Int>>>}) -> Result<{items: Vec<Vec<Vec<Int>>>}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let function = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
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

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_recursive_result_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Payload\n",
            "  Payload(value: Int)\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  outcome: Result<Option<Int>, String>\n",
            "  details: Result<Vec<Int>, Dict<String, String>>\n",
            "  nested: {payload: Result<Result<Int, String>, Payload>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {outcome: Result<Option<Int>, String>, details: Result<Vec<Int>, Dict<String, String>>, nested: {payload: Result<Result<Int, String>, Payload>}}) -> Result<{outcome: Result<Option<Int>, String>, details: Result<Vec<Int>, Dict<String, String>>, nested: {payload: Result<Result<Int, String>, Payload>}}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {outcome: Result<Option<Int>, String>, details: Result<Vec<Int>, Dict<String, String>>, nested: {payload: Result<Result<Int, String>, Payload>}}) -> Result<{outcome: Result<Option<Int>, String>, details: Result<Vec<Int>, Dict<String, String>>, nested: {payload: Result<Result<Int, String>, Payload>}}, String>\n",
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
fn generated_format_neutral_schema_encode_helpers_reject_unsupported_result_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  callback: Result<Int, fn(Int) -> String>\n",
            "  bad_dict: Result<Dict<Int, Int>, String>\n",
            "end\n",
            "\n",
            "pub fn main(packet: {callback: Result<Int, fn(Int) -> String>, bad_dict: Result<Dict<Int, Int>, String>}) -> Result<{callback: Result<Int, fn(Int) -> String>, bad_dict: Result<Dict<Int, Int>, String>}, String>\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.encode_expression")
        .expect("unsupported format-neutral encode helper should be rejected");
    assert_eq!(
        diagnostic.message,
        "schema encode expression cannot resolve `Packet` as an eligible schema encode helper"
    );
}

#[test]
fn generated_format_neutral_schema_encode_helpers_accept_option_scalar_dict_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  scores: Dict<String, Option<Int>>\n",
            "  states: Dict<String, Option<Bool>>\n",
            "  weights: Dict<String, Option<Float>>\n",
            "  names: Dict<String, Option<String>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {scores: Dict<String, Option<Int>>, states: Dict<String, Option<Bool>>, weights: Dict<String, Option<Float>>, names: Dict<String, Option<String>>}) -> Result<{scores: Dict<String, Option<Int>>, states: Dict<String, Option<Bool>>, weights: Dict<String, Option<Float>>, names: Dict<String, Option<String>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {scores: Dict<String, Option<Int>>, states: Dict<String, Option<Bool>>, weights: Dict<String, Option<Float>>, names: Dict<String, Option<String>>}) -> Result<{scores: Dict<String, Option<Int>>, states: Dict<String, Option<Bool>>, weights: Dict<String, Option<Float>>, names: Dict<String, Option<String>>}, String>\n",
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
fn generated_format_neutral_schema_encode_helpers_accept_list_scalar_dict_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  scores: Dict<String, List<Int>>\n",
            "  states: Dict<String, List<Bool>>\n",
            "  weights: Dict<String, List<Float>>\n",
            "  names: Dict<String, List<String>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {scores: Dict<String, List<Int>>, states: Dict<String, List<Bool>>, weights: Dict<String, List<Float>>, names: Dict<String, List<String>>}) -> Result<{scores: Dict<String, List<Int>>, states: Dict<String, List<Bool>>, weights: Dict<String, List<Float>>, names: Dict<String, List<String>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {scores: Dict<String, List<Int>>, states: Dict<String, List<Bool>>, weights: Dict<String, List<Float>>, names: Dict<String, List<String>>}) -> Result<{scores: Dict<String, List<Int>>, states: Dict<String, List<Bool>>, weights: Dict<String, List<Float>>, names: Dict<String, List<String>>}, String>\n",
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
fn generated_format_neutral_schema_encode_helpers_accept_vec_scalar_dict_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  scores: Dict<String, Vec<Int>>\n",
            "  states: Dict<String, Vec<Bool>>\n",
            "  weights: Dict<String, Vec<Float>>\n",
            "  names: Dict<String, Vec<String>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {scores: Dict<String, Vec<Int>>, states: Dict<String, Vec<Bool>>, weights: Dict<String, Vec<Float>>, names: Dict<String, Vec<String>>}) -> Result<{scores: Dict<String, Vec<Int>>, states: Dict<String, Vec<Bool>>, weights: Dict<String, Vec<Float>>, names: Dict<String, Vec<String>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {scores: Dict<String, Vec<Int>>, states: Dict<String, Vec<Bool>>, weights: Dict<String, Vec<Float>>, names: Dict<String, Vec<String>>}) -> Result<{scores: Dict<String, Vec<Int>>, states: Dict<String, Vec<Bool>>, weights: Dict<String, Vec<Float>>, names: Dict<String, Vec<String>>}, String>\n",
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
fn generated_format_neutral_schema_encode_helpers_accept_option_scalar_vec_dict_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  scores: Dict<String, Vec<Option<Int>>>\n",
            "  states: Dict<String, Vec<Option<Bool>>>\n",
            "  weights: Dict<String, Vec<Option<Float>>>\n",
            "  names: Dict<String, Vec<Option<String>>>\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {scores: Dict<String, Vec<Option<Int>>>, states: Dict<String, Vec<Option<Bool>>>, weights: Dict<String, Vec<Option<Float>>>, names: Dict<String, Vec<Option<String>>>}) -> Result<{scores: Dict<String, Vec<Option<Int>>>, states: Dict<String, Vec<Option<Bool>>>, weights: Dict<String, Vec<Option<Float>>>, names: Dict<String, Vec<Option<String>>>}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {scores: Dict<String, Vec<Option<Int>>>, states: Dict<String, Vec<Option<Bool>>>, weights: Dict<String, Vec<Option<Float>>>, names: Dict<String, Vec<Option<String>>>}) -> Result<{scores: Dict<String, Vec<Option<Int>>>, states: Dict<String, Vec<Option<Bool>>>, weights: Dict<String, Vec<Option<Float>>>, names: Dict<String, Vec<Option<String>>>}, String>\n",
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
fn generated_format_neutral_schema_encode_helpers_accept_result_container_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  items: List<Result<Int, String>>\n",
            "  vector: Vec<Result<Option<Int>, String>>\n",
            "  labels: Dict<String, Result<List<Int>, String>>\n",
            "  metadata: {flags: Vec<Result<Bool, String>>, aliases: Dict<String, Result<String, Option<Int>>>}\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {items: List<Result<Int, String>>, vector: Vec<Result<Option<Int>, String>>, labels: Dict<String, Result<List<Int>, String>>, metadata: {flags: Vec<Result<Bool, String>>, aliases: Dict<String, Result<String, Option<Int>>>}}) -> Result<{items: List<Result<Int, String>>, vector: Vec<Result<Option<Int>, String>>, labels: Dict<String, Result<List<Int>, String>>, metadata: {flags: Vec<Result<Bool, String>>, aliases: Dict<String, Result<String, Option<Int>>>}}, String>\n",
            "  byte_encode_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn explicit(packet: {items: List<Result<Int, String>>, vector: Vec<Result<Option<Int>, String>>, labels: Dict<String, Result<List<Int>, String>>, metadata: {flags: Vec<Result<Bool, String>>, aliases: Dict<String, Result<String, Option<Int>>>}}) -> Result<{items: List<Result<Int, String>>, vector: Vec<Result<Option<Int>, String>>, labels: Dict<String, Result<List<Int>, String>>, metadata: {flags: Vec<Result<Bool, String>>, aliases: Dict<String, Result<String, Option<Int>>>}}, String>\n",
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
fn generated_format_neutral_schema_encode_helpers_reject_dict_boundaries() {
    for (field_type, record_type) in [
        ("Dict<Int, String>", "{items: Dict<Int, String>}"),
        (
            "Option<Dict<Int, String>>",
            "{items: Option<Dict<Int, String>>}",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "schema Packet\n  items: {field_type}\nend\n\npub fn main(packet: {record_type}) -> Result<{record_type}, String>\n  encode Packet from packet\nend\n"
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        let diagnostic = lowered
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "schema.encode_expression")
            .unwrap_or_else(|| {
                panic!(
                    "unsupported format-neutral encode helper should be rejected for {field_type}"
                )
            });
        assert_eq!(
            diagnostic.message,
            "schema encode expression cannot resolve `Packet` as an eligible schema encode helper"
        );
    }
}

#[test]
fn generated_format_neutral_schema_decode_helpers_accept_top_level_option_lists() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  items: Option<List<Int>>\n",
            "  flags: Option<List<Bool>>\n",
            "  ratios: Option<List<Float>>\n",
            "  names: Option<List<String>>\n",
            "end\n",
            "\n",
            "pub fn main(packet: {items: Option<List<Int>>, flags: Option<List<Bool>>, ratios: Option<List<Float>>, names: Option<List<String>>}) -> Result<{items: Option<List<Int>>, flags: Option<List<Bool>>, ratios: Option<List<Float>>, names: Option<List<String>>}, String>\n",
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
fn generated_format_neutral_schema_decode_helpers_accept_option_dicts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  scores: Option<Dict<String, Int>>\n",
            "  labels: Option<Dict<String, String>>\n",
            "  states: Option<Dict<String, Bool>>\n",
            "  weights: Option<Dict<String, Float>>\n",
            "  metadata: {scores: Option<Dict<String, Int>>, labels: Option<Dict<String, String>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {scores: Option<Dict<String, Int>>, labels: Option<Dict<String, String>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, metadata: {scores: Option<Dict<String, Int>>, labels: Option<Dict<String, String>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>}}) -> Result<{scores: Option<Dict<String, Int>>, labels: Option<Dict<String, String>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>, metadata: {scores: Option<Dict<String, Int>>, labels: Option<Dict<String, String>>, states: Option<Dict<String, Bool>>, weights: Option<Dict<String, Float>>}}, String>\n",
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
fn generated_format_neutral_schema_decode_helpers_accept_recursive_containers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  maybe_items: List<Option<Int>>\n",
            "  maybe_names: Option<List<Option<String>>>\n",
            "  maybe_scores: Dict<String, Option<Int>>\n",
            "  metadata: {maybe_items: List<Option<Int>>, maybe_names: Option<List<Option<String>>>, maybe_scores: Dict<String, Option<Int>>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {maybe_items: List<Option<Int>>, maybe_names: Option<List<Option<String>>>, maybe_scores: Dict<String, Option<Int>>, metadata: {maybe_items: List<Option<Int>>, maybe_names: Option<List<Option<String>>>, maybe_scores: Dict<String, Option<Int>>}}) -> Result<{maybe_items: List<Option<Int>>, maybe_names: Option<List<Option<String>>>, maybe_scores: Dict<String, Option<Int>>, metadata: {maybe_items: List<Option<Int>>, maybe_names: Option<List<Option<String>>>, maybe_scores: Dict<String, Option<Int>>}}, String>\n",
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
fn generated_format_neutral_schema_decode_helpers_accept_recursive_result_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  status: Result<Int, String>\n",
            "  enabled: Result<Bool, String>\n",
            "  ratio: Result<Float, String>\n",
            "  label: Result<String, Int>\n",
            "  result_items: Result<List<Int>, String>\n",
            "  result_labels: Result<Int, Dict<String, String>>\n",
            "  optional_items: Option<Result<List<Int>, String>>\n",
            "  metadata: {status: Result<Int, String>, enabled: Result<Bool, String>, ratio: Result<Float, String>, label: Result<String, Int>, result_items: Result<List<Int>, String>, result_labels: Result<Int, Dict<String, String>>}\n",
            "end\n",
            "\n",
            "pub fn main(packet: {status: Result<Int, String>, enabled: Result<Bool, String>, ratio: Result<Float, String>, label: Result<String, Int>, result_items: Result<List<Int>, String>, result_labels: Result<Int, Dict<String, String>>, optional_items: Option<Result<List<Int>, String>>, metadata: {status: Result<Int, String>, enabled: Result<Bool, String>, ratio: Result<Float, String>, label: Result<String, Int>, result_items: Result<List<Int>, String>, result_labels: Result<Int, Dict<String, String>>}}) -> Result<{status: Result<Int, String>, enabled: Result<Bool, String>, ratio: Result<Float, String>, label: Result<String, Int>, result_items: Result<List<Int>, String>, result_labels: Result<Int, Dict<String, String>>, optional_items: Option<Result<List<Int>, String>>, metadata: {status: Result<Int, String>, enabled: Result<Bool, String>, ratio: Result<Float, String>, label: Result<String, Int>, result_items: Result<List<Int>, String>, result_labels: Result<Int, Dict<String, String>>}}, String>\n",
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
fn generated_format_neutral_schema_decode_helpers_accept_same_module_source_adts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Payload\n",
            "  Empty\n",
            "  Scalar(value: Int)\n",
            "  Metadata(value: {label: String, scores: Dict<String, List<Option<Int>>>})\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  payload: Payload\n",
            "  nested: {payload: Payload}\n",
            "  optional_payload: Option<Payload>\n",
            "  payloads: List<Payload>\n",
            "  payload_by_name: Dict<String, Payload>\n",
            "  result_payload: Result<Payload, Payload>\n",
            "end\n",
            "\n",
            "pub fn main(packet: {payload: Payload, nested: {payload: Payload}, optional_payload: Option<Payload>, payloads: List<Payload>, payload_by_name: Dict<String, Payload>, result_payload: Result<Payload, Payload>}) -> Result<{payload: Payload, nested: {payload: Payload}, optional_payload: Option<Payload>, payloads: List<Payload>, payload_by_name: Dict<String, Payload>, result_payload: Result<Payload, Payload>}, String>\n",
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
fn generated_format_neutral_schema_decode_helpers_accept_public_imported_source_adts() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "schema Packet\n",
            "  payload: wire::Payload\n",
            "  nested: {payload: wire::Payload}\n",
            "  optional_payload: Option<wire::Payload>\n",
            "  payloads: List<wire::Payload>\n",
            "  payload_by_name: Dict<String, wire::Payload>\n",
            "  result_payload: Result<wire::Payload, wire::Payload>\n",
            "end\n",
            "\n",
            "pub fn main(packet: {payload: Payload, nested: {payload: Payload}, optional_payload: Option<Payload>, payloads: List<Payload>, payload_by_name: Dict<String, Payload>, result_payload: Result<Payload, Payload>}) -> Result<{payload: Payload, nested: {payload: Payload}, optional_payload: Option<Payload>, payloads: List<Payload>, payload_by_name: Dict<String, Payload>, result_payload: Result<Payload, Payload>}, String>\n",
            "  byte_decode_packet(packet)\n",
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
            "  pub Metadata(value: {label: String, scores: Dict<String, List<Option<Int>>>})\n",
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
    assert_eq!(ir.schema_decoders.len(), 1);
    assert_eq!(ir.schema_decoders[0].schema_name, "Packet");
}
