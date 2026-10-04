use super::*;

#[test]
fn surface_wire_round_trip_preserves_expression_families() {
    let sources = [
        concat!(
            "fn build(input: Int) -> ()\n",
            "  let data = {answer: [1, 2.5, -input?], check: _value satisfy candidate => candidate > 0}\n",
            "  let lookup = {\"one\": 1, \"two\": 2}\n",
            "  data.answer |> sink<String>(\"ok\", ())\n",
            "end\n",
        ),
        concat!(
            "schema Header\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "fn decode_header(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode Header from view at base\n",
            "end\n",
            "fn encode_header(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode Header from packet\n",
            "end\n",
        ),
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "fn handled() -> Int\n",
            "  handle perform Ask::value() with ask(41)\n",
            "end\n",
        ),
        concat!(
            "fn choose(first: Bool, second: Bool) -> Int\n",
            "  if first\n",
            "    match second\n",
            "      true => 1\n",
            "      false => 2\n",
            "    end\n",
            "  else if second\n",
            "    3\n",
            "  else\n",
            "    4\n",
            "  end\n",
            "end\n",
        ),
        concat!(
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn Exposed = api::Parse\n",
            "pub type Alias = api::_item\n",
            "pub schema Packet = api::packet\n",
        ),
        concat!(
            "fn cleanup() -> Int\n",
            "  let resource = 1\n",
            "  defer\n",
            "    ()\n",
            "  end\n",
            "  begin\n",
            "    let value = resource + 1\n",
            "    value\n",
            "  end\n",
            "end\n",
        ),
    ];

    for source in sources {
        let module = lower_source(source);
        let encoded = encode_surface_module(&module);
        let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");
        assert_eq!(encode_surface_module(&decoded), encoded);
    }
}

#[test]
fn surface_wire_round_trip_preserves_callsite_modifier_span() {
    let module = lower_source("fn located() -> SourceLocation callsite\n  callsite\nend\n");
    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

    let callsite = decoded.functions[0]
        .callsite
        .as_ref()
        .expect("callsite modifier span");
    assert_eq!((callsite.start.line, callsite.start.column), (1, 32));
    assert_eq!(encode_surface_module(&decoded), encoded);
}

#[test]
fn surface_wire_round_trip_preserves_dependency_package_identity() {
    let source = SourceFile::new(
        "shared.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "handler ask() handles Ask\n",
            "  value() => 1\n",
            "end\n",
            "fn located() -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty());
    let module = lower_surface_ast_with_package_module_identity(
        &parsed.tree,
        "example/dependency".to_string(),
        "shared".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );

    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

    assert_eq!(
        decoded.functions[0].package_name.as_deref(),
        Some("example/dependency")
    );
    assert_eq!(
        decoded.handlers[0].package_name.as_deref(),
        Some("example/dependency")
    );
    assert_eq!(encode_surface_module(&decoded), encoded);
}

#[test]
fn surface_wire_round_trip_preserves_generated_origin_boundaries() {
    let text = concat!(
        "fn located() -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  located()\n",
        "end\n",
    );
    let call_start = text.rfind("located()").expect("call expression start");
    let call_end = call_start + "located()".len();
    let original_start = veln_source::LineCol {
        line: 41,
        column: 7,
        offset: 1_001,
    };
    let original_end = veln_source::LineCol {
        line: 43,
        column: 19,
        offset: 1_099,
    };
    let source = SourceFile::generated_with_mappings(
        "virtual/generated.veln",
        text,
        veln_source::SourcePath::new("templates/original.veln"),
        [(call_start, original_start), (call_end, original_end)],
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty());
    let module = lower_surface_ast(&parsed.tree);

    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");
    let BodyLineKind::Expr { expr } = &decoded.functions[1].body[0].kind else {
        panic!("expected call expression body");
    };
    assert!(expr.span.generated_origin.is_some());
    assert_eq!(
        expr.span
            .original_path()
            .map(veln_source::SourcePath::as_str),
        Some("templates/original.veln")
    );
    assert_eq!(expr.span.original_start(), Some(original_start));
    assert_eq!(expr.span.original_end(), Some(original_end));
    let resolved = expr.span.resolved_origin().expect("complete mapped span");
    assert_eq!(resolved.file.as_str(), "templates/original.veln");
    assert_eq!(resolved.start, original_start);
    assert_eq!(resolved.end, original_end);
    assert_eq!(encode_surface_module(&decoded), encoded);
}

#[test]
fn surface_wire_round_trip_preserves_incomplete_generated_origin_fallback() {
    let text = concat!(
        "fn located() -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  located()\n",
        "end\n",
    );
    let call_start = text.rfind("located()").expect("call expression start");
    let call_end = call_start + "located()".len();
    let original = veln_source::LineCol {
        line: 41,
        column: 7,
        offset: 1_001,
    };
    let cases = [
        SourceFile::generated(
            "virtual/path-only.veln",
            text,
            Some(veln_source::SourcePath::new("templates/original.veln")),
        ),
        SourceFile::generated_with_mappings(
            "virtual/missing-end.veln",
            text,
            veln_source::SourcePath::new("templates/original.veln"),
            [(call_start, original)],
        ),
        SourceFile::generated_with_mappings(
            "virtual/missing-start.veln",
            text,
            veln_source::SourcePath::new("templates/original.veln"),
            [(call_end, original)],
        ),
    ];

    for source in cases {
        let generated = source.span(veln_source::TextRange::new(call_start, call_end));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty());
        let module = lower_surface_ast(&parsed.tree);
        let encoded = encode_surface_module(&module);
        let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");
        let BodyLineKind::Expr { expr } = &decoded.functions[1].body[0].kind else {
            panic!("expected call expression body");
        };

        assert_eq!(expr.span, generated);
        assert!(expr.span.generated_origin.is_some());
        assert!(expr.span.resolved_origin().is_none());
        assert_eq!(expr.span.resolved_or_generated(), generated);
        assert_eq!(encode_surface_module(&decoded), encoded);
    }
}

#[test]
fn surface_wire_round_trip_preserves_contract_callsite_reference_span() {
    let module = lower_source("fn guarded() -> ()\nrequire callsite\n  ()\nend\n");
    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

    let reference = &decoded.functions[0].contracts[0].callsite_reference_spans[0];
    assert_eq!((reference.start.line, reference.start.column), (2, 9));
    assert_eq!((reference.end.line, reference.end.column), (2, 17));
    assert_eq!(encode_surface_module(&decoded), encoded);
}

#[test]
fn surface_wire_round_trip_preserves_contract_call_callee_spans() {
    let module = lower_source(concat!(
        "fn guarded() -> Bool\n",
        "require outer(inner::ready())\n",
        "  true\n",
        "end\n",
    ));
    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

    let calls = &decoded.functions[0].contracts[0].call_callee_spans;
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "outer");
    assert_eq!((calls[0].1.start.line, calls[0].1.start.column), (2, 9));
    assert_eq!(calls[1].0, "inner::ready");
    assert_eq!((calls[1].1.start.line, calls[1].1.start.column), (2, 15));
    assert_eq!(encode_surface_module(&decoded), encoded);
}

#[test]
fn surface_wire_round_trip_preserves_cleanup_introducer_spans() {
    let source = concat!(
        "fn parse() -> Result<(), String>\n",
        "  Ok(())\n",
        "end\n",
        "fn cleanup() -> ()\n",
        "  defer\n",
        "    parse()?\n",
        "  end\n",
        "  ()\n",
        "end\n",
    );
    let module = lower_source(source);
    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

    let BodyLineKind::Defer {
        body,
        keyword_span,
        block_span,
    } = &decoded.functions[1].body[0].kind
    else {
        panic!("expected defer statement");
    };
    assert_eq!(
        &source[keyword_span.start.offset..keyword_span.end.offset],
        "defer"
    );
    assert_eq!(
        &source[block_span.start.offset..block_span.end.offset],
        "    parse()?\n  "
    );

    let BodyLineKind::Expr { expr } = &body[0].kind else {
        panic!("expected deferred expression");
    };
    let ExprKind::Try { question_span, .. } = &expr.kind else {
        panic!("expected try expression");
    };
    assert_eq!(
        &source[question_span.start.offset..question_span.end.offset],
        "?"
    );
}
