use super::*;

#[test]
fn surface_wire_round_trip_preserves_variant_refinement_structure_and_spans() {
    let source = concat!(
        "effect Transition\n",
        "  move(state: State::Ready | State::Closed) -> Result<Int, Error>::Ok\n",
        "end\n",
        "type Boxed\n",
        "  Box(value: protocol::State::Ready | protocol::State::Closed)\n",
        "end\n",
        "schema Packet\n",
        "  state: State::Ready | State::Closed\n",
        "end\n",
        "fn advance(state: State::Ready | State::Closed) -> Result<List<domain::Item>, Wrapper<State::Ready | State::Closed>>::Ok\n",
        "  let exact: State::Ready | State::Ready = state\n",
        "  sink<State::Ready | State::Closed>(exact)\n",
        "  exact\n",
        "end\n",
    );
    let module = lower_source(source);
    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

    assert_parameter_refinement(source, &decoded);
    assert_result_refinement(&decoded);
    assert_refinement_positions(&decoded);
    assert_eq!(encode_surface_module(&decoded), encoded);
}

fn assert_parameter_refinement(source: &str, decoded: &SurfaceModule) {
    let parameter_union = &decoded.functions[0].params[0].ty_refinements[0];
    assert_eq!(parameter_union.alternatives.len(), 2);
    assert_eq!(parameter_union.alternatives[0].base.segments, ["State"]);
    assert_eq!(parameter_union.alternatives[0].variant, "Ready");
    assert_eq!(parameter_union.alternatives[1].variant, "Closed");
    assert_eq!(
        &source
            [parameter_union.pipe_spans[0].start.offset..parameter_union.pipe_spans[0].end.offset],
        "|"
    );
}

fn assert_result_refinement(decoded: &SurfaceModule) {
    let result = &decoded.functions[0].return_type_refinements[0].alternatives[0];
    assert_eq!(result.base.segments, ["Result"]);
    assert_eq!(result.type_arguments.len(), 2);
    assert_eq!(
        result.type_arguments[0].ty_fragments,
        ["List<domain::Item>"]
    );
    assert_eq!(
        result.type_arguments[0].ty_paths[0].segments,
        ["domain", "Item"]
    );
    assert_eq!(result.type_arguments[1].ty_fragments, ["Wrapper<", ">"]);
    assert_eq!(result.type_arguments[1].ty_refinements.len(), 1);
    assert_eq!(
        result.type_arguments[1].ty_refinements[0].alternatives[0].variant,
        "Ready"
    );
    assert_eq!(
        result.type_arguments[1].ty_refinements[0].alternatives[1].variant,
        "Closed"
    );
    assert_eq!(result.variant, "Ok");
}

fn assert_refinement_positions(decoded: &SurfaceModule) {
    assert_eq!(
        decoded.effects[0].operations[0].params[0]
            .ty_refinements
            .len(),
        1
    );
    assert_eq!(
        decoded.effects[0].operations[0]
            .return_type_refinements
            .len(),
        1
    );
    assert_eq!(
        decoded.types[0].variants[0].fields[0].ty_refinements.len(),
        1
    );
    assert_eq!(decoded.schemas[0].fields[0].ty_refinements.len(), 1);
    let BodyLineKind::Let {
        annotation_structure,
        ..
    } = &decoded.functions[0].body[0].kind
    else {
        panic!("expected annotated let");
    };
    assert_eq!(
        annotation_structure.variant_refinements[0]
            .alternatives
            .len(),
        2
    );
    let BodyLineKind::Expr { expr } = &decoded.functions[0].body[1].kind else {
        panic!("expected call expression line");
    };
    let ExprKind::Call { callee, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    let ExprKind::TypeApply {
        type_arg_refinements,
        ..
    } = &callee.kind
    else {
        panic!("expected type application");
    };
    assert_eq!(type_arg_refinements[0][0].alternatives.len(), 2);
}

#[test]
fn deep_refinement_structure_remains_linear_through_lowering_and_wire_round_trip() {
    let sizes = [64, 128, 256].map(refinement_wire_size);
    eprintln!("variant refinement wire sizes at depths 64, 128, and 256: {sizes:?}");
    let first_growth = sizes[1] - sizes[0];
    let second_growth = sizes[2] - sizes[1];
    assert!(
        second_growth < first_growth * 3,
        "doubling refinement depth should grow wire bytes proportionally: {sizes:?}"
    );
}

#[test]
fn excessive_source_refinement_nesting_remains_bounded_through_lowering_and_wire() {
    let mut annotation = "Leaf::Value".to_string();
    for _ in 0..4_096 {
        annotation = format!("Layer<{annotation}>::Wrapped");
    }
    let source = format!("fn nested(value: {annotation}) -> ()\n  ()\nend\n");

    let module = lower_source_allowing_diagnostics(&source);
    let mut refinements = module.functions[0].params[0].ty_refinements.as_slice();
    let mut nesting = 0;
    while let Some(refinement) = refinements.first() {
        nesting += 1;
        refinements = refinement.alternatives[0]
            .type_arguments
            .first()
            .map_or(&[], |argument| argument.ty_refinements.as_slice());
    }
    assert_eq!(nesting, veln_syntax::MAX_VARIANT_REFINEMENT_NESTING + 1);
    let encoded = encode_surface_module(&module);
    decode_surface_module(&encoded).expect("bounded recovery AST should cross the wire boundary");
}

#[test]
fn surface_wire_rejects_refinement_nesting_beyond_the_parser_limit() {
    let mut module = lower_source("fn nested(value: Leaf::Value) -> ()\n  ()\nend\n");
    let refinements = &mut module.functions[0].params[0].ty_refinements;
    let template = refinements[0].alternatives[0].clone();
    for _ in 0..=veln_syntax::MAX_VARIANT_REFINEMENT_NESTING {
        let child = std::mem::take(refinements);
        *refinements = vec![VariantRefinementType {
            alternatives: vec![VariantRefinementAlternative {
                base: template.base.clone(),
                type_arguments: vec![VariantRefinementTypeArgument {
                    ty_fragments: vec![String::new(), String::new()],
                    ty_paths: Vec::new(),
                    ty_refinements: child,
                    span: template.span.clone(),
                }],
                variant: template.variant.clone(),
                variant_span: template.variant_span.clone(),
                span: template.span.clone(),
            }],
            pipe_spans: Vec::new(),
            span: template.span.clone(),
        }];
    }

    let encoded = encode_surface_module(&module);
    let error = decode_surface_module(&encoded).expect_err("excessive nesting must be rejected");
    assert_eq!(
        error,
        "surface module variant refinements are nested too deeply"
    );
}

fn refinement_wire_size(depth: usize) -> usize {
    let mut annotation = "domain::Leaf::Value".to_string();
    for _ in 0..depth {
        annotation = format!("domain::Layer<{annotation}>::Wrapped");
    }
    let source = format!("fn nested(value: {annotation}) -> ()\n  ()\nend\n");

    let module = lower_source(&source);
    assert_refinement_chain(&module.functions[0].params[0].ty_refinements, depth + 1);

    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");
    assert_refinement_chain(&decoded.functions[0].params[0].ty_refinements, depth + 1);
    assert_eq!(encode_surface_module(&decoded), encoded);
    encoded.len()
}

fn assert_refinement_chain(refinements: &[VariantRefinementType], expected_nodes: usize) {
    let mut refinements = refinements;
    let mut nodes = 0;
    loop {
        assert_eq!(refinements.len(), 1);
        assert_eq!(refinements[0].alternatives.len(), 1);
        nodes += 1;
        let alternative = &refinements[0].alternatives[0];
        let Some(argument) = alternative.type_arguments.first() else {
            break;
        };
        assert_eq!(alternative.type_arguments.len(), 1);
        assert!(
            argument.ty_paths.is_empty(),
            "paths owned by a child refinement must not be repeated by its parent argument"
        );
        refinements = &argument.ty_refinements;
    }
    assert_eq!(nodes, expected_nodes);
}
