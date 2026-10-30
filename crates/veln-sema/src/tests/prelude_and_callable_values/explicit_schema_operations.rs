use super::*;

#[test]
fn explicit_schema_validate_expression_erases_direct_and_nested_record_refinement_inputs() {
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
            "  nested: {state: State::Connected, detail: {phase: State::Closing}}\n",
            "  composed: UnrefinedPacket\n",
            "end\n",
            "fn check(packet: {state: State, states: List<State>, nested: {state: State, detail: {phase: State}}, composed: {state: State}}, base: ByteOffset) -> Result<{state: State::Connected | State::Closing, states: List<State>, nested: {state: State::Connected, detail: {phase: State::Closing}}, composed: {state: State}}, DecodeError>\n",
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
    let nested = schema.fields[2]
        .payload_schema
        .as_deref()
        .expect("nested record validation metadata");
    assert_eq!(
        nested.fields[0]
            .variant_refinement
            .as_ref()
            .map(|refinement| refinement.expected_variants.as_slice()),
        Some(["Connected".to_string()].as_slice())
    );
    assert_eq!(
        nested.fields[1]
            .payload_schema
            .as_deref()
            .expect("recursive nested record validation metadata")
            .fields[0]
            .variant_refinement
            .as_ref()
            .map(|refinement| refinement.expected_variants.as_slice()),
        Some(["Closing".to_string()].as_slice())
    );
}

#[test]
fn explicit_schema_validate_expression_checks_nested_record_identity_arguments_and_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type OtherState\n  Connected\nend\n",
            "type Box<A>\n  Open(value: A)\n  Shut\nend\n",
            "schema NestedPacket\n",
            "  nested: {state: State::Connected, detail: {box: Box<Int>::Open}}\n",
            "end\n",
            "fn wrong_identity(value: {nested: {state: OtherState, detail: {box: Box<Int>}}}, base: ByteOffset) -> ()\n",
            "  validate NestedPacket from value at base\n  ()\nend\n",
            "fn wrong_generic(value: {nested: {state: State, detail: {box: Box<String>}}}, base: ByteOffset) -> ()\n",
            "  validate NestedPacket from value at base\n  ()\nend\n",
            "fn wrong_shape(value: {nested: {state: State, detail: {value: Int}}}, base: ByteOffset) -> ()\n",
            "  validate NestedPacket from value at base\n  ()\nend\n",
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
    assert_eq!(mismatches.len(), 3, "{:#?}", lowered.diagnostics);
    for expected in ["OtherState", "Box<String>", "box"] {
        assert!(
            mismatches
                .iter()
                .any(|diagnostic| diagnostic.message.contains(expected)),
            "missing mismatch for {expected}: {mismatches:#?}"
        );
    }
    assert!(
        lowered
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.validate_expression"),
        "nested record validation should be eligible: {:#?}",
        lowered.diagnostics
    );
}

#[test]
fn explicit_schema_validate_expression_erases_composed_schema_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type OtherState\n  Connected\nend\n",
            "type Box<A>\n  Open(value: A)\n  Shut\nend\n",
            "schema Leaf\n",
            "  state: State::Connected\n",
            "  box: Box<Int>::Open\n",
            "end\n",
            "schema Child\n",
            "  leaf: Leaf\n",
            "end\n",
            "schema Parent\n",
            "  first: Child\n",
            "  count: Int\n",
            "  second: Child\n",
            "end\n",
            "fn check(value: {first: {leaf: {state: State, box: Box<Int>}}, count: Int, second: {leaf: {state: State, box: Box<Int>}}}, base: ByteOffset) -> Result<{first: {leaf: {state: State::Connected, box: Box<Int>::Open}}, count: Int, second: {leaf: {state: State::Connected, box: Box<Int>::Open}}}, DecodeError>\n",
            "  validate Parent from value at base\n",
            "end\n",
            "fn wrong_identity(value: {first: {leaf: {state: OtherState, box: Box<Int>}}, count: Int, second: {leaf: {state: State, box: Box<Int>}}}, base: ByteOffset) -> ()\n",
            "  validate Parent from value at base\n  ()\nend\n",
            "fn wrong_generic(value: {first: {leaf: {state: State, box: Box<String>}}, count: Int, second: {leaf: {state: State, box: Box<Int>}}}, base: ByteOffset) -> ()\n",
            "  validate Parent from value at base\n  ()\nend\n",
            "fn wrong_shape(value: {first: {leaf: {state: State}}, count: Int, second: {leaf: {state: State, box: Box<Int>}}}, base: ByteOffset) -> ()\n",
            "  validate Parent from value at base\n  ()\nend\n",
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
    assert_eq!(mismatches.len(), 3, "{:#?}", lowered.diagnostics);
    for expected in ["OtherState", "Box<String>", "box"] {
        assert!(
            mismatches
                .iter()
                .any(|diagnostic| diagnostic.message.contains(expected)),
            "missing mismatch for {expected}: {mismatches:#?}"
        );
    }
    assert!(
        lowered
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.validate_expression"),
        "composed schema validation should be eligible: {:#?}",
        lowered.diagnostics
    );
}

#[test]
fn explicit_schema_validate_expression_checks_all_unrefined_composed_branches() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "schema Refined\n  state: State::Connected\nend\n",
            "schema Plain\n  value: Int\nend\n",
            "schema Parent\n",
            "  before: Plain\n",
            "  first: Refined\n",
            "  second: Refined\n",
            "  after: Plain\n",
            "end\n",
            "schema PlainOnly\n  child: Plain\nend\n",
            "fn accepted(value: {before: {value: Int}, first: {state: State}, second: {state: State}, after: {value: Int}}, base: ByteOffset) -> Result<{before: {value: Int}, first: {state: State::Connected}, second: {state: State::Connected}, after: {value: Int}}, DecodeError>\n",
            "  validate Parent from value at base\n",
            "end\n",
            "fn no_refinement(value: {child: {value: Int}}, base: ByteOffset) -> ()\n",
            "  validate PlainOnly from value at base\n",
            "  ()\n",
            "end\n",
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
        1,
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        validation_diagnostics[0].message.contains("PlainOnly"),
        "{validation_diagnostics:#?}"
    );
}

#[test]
fn explicit_schema_validate_expression_rejects_predicates_in_any_composed_branch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "schema Refined\n  state: State::Connected\nend\n",
            "schema FieldPredicate\n  value: Int where true\nend\n",
            "schema SchemaPredicate\n  format binary\n  value: Int\n  validate value == value\nend\n",
            "schema FieldIntermediate\n  nested: FieldPredicate\nend\n",
            "schema SchemaIntermediate\n  nested: SchemaPredicate\nend\n",
            "schema FieldDirectBefore\n  blocked: FieldPredicate\n  refined: Refined\nend\n",
            "schema FieldDirectAfter\n  refined: Refined\n  blocked: FieldPredicate\nend\n",
            "schema FieldNestedBefore\n  blocked: FieldIntermediate\n  refined: Refined\nend\n",
            "schema FieldNestedAfter\n  refined: Refined\n  blocked: FieldIntermediate\nend\n",
            "schema SchemaDirectBefore\n  blocked: SchemaPredicate\n  refined: Refined\nend\n",
            "schema SchemaDirectAfter\n  refined: Refined\n  blocked: SchemaPredicate\nend\n",
            "schema SchemaNestedBefore\n  blocked: SchemaIntermediate\n  refined: Refined\nend\n",
            "schema SchemaNestedAfter\n  refined: Refined\n  blocked: SchemaIntermediate\nend\n",
            "fn field_direct_before(value: {blocked: {value: Int}, refined: {state: State}}, base: ByteOffset) -> ()\n  validate FieldDirectBefore from value at base\n  ()\nend\n",
            "fn field_direct_after(value: {refined: {state: State}, blocked: {value: Int}}, base: ByteOffset) -> ()\n  validate FieldDirectAfter from value at base\n  ()\nend\n",
            "fn field_nested_before(value: {blocked: {nested: {value: Int}}, refined: {state: State}}, base: ByteOffset) -> ()\n  validate FieldNestedBefore from value at base\n  ()\nend\n",
            "fn field_nested_after(value: {refined: {state: State}, blocked: {nested: {value: Int}}}, base: ByteOffset) -> ()\n  validate FieldNestedAfter from value at base\n  ()\nend\n",
            "fn schema_direct_before(value: {blocked: {value: Int}, refined: {state: State}}, base: ByteOffset) -> ()\n  validate SchemaDirectBefore from value at base\n  ()\nend\n",
            "fn schema_direct_after(value: {refined: {state: State}, blocked: {value: Int}}, base: ByteOffset) -> ()\n  validate SchemaDirectAfter from value at base\n  ()\nend\n",
            "fn schema_nested_before(value: {blocked: {nested: {value: Int}}, refined: {state: State}}, base: ByteOffset) -> ()\n  validate SchemaNestedBefore from value at base\n  ()\nend\n",
            "fn schema_nested_after(value: {refined: {state: State}, blocked: {nested: {value: Int}}}, base: ByteOffset) -> ()\n  validate SchemaNestedAfter from value at base\n  ()\nend\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    module
        .schemas
        .iter_mut()
        .find(|schema| schema.name.as_deref() == Some("SchemaPredicate"))
        .expect("schema predicate fixture")
        .format = None;
    let lowered = lower_checked_surface_module(&module);
    let validation_diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.validate_expression")
        .collect::<Vec<_>>();

    assert_eq!(
        validation_diagnostics.len(),
        8,
        "{:#?}",
        lowered.diagnostics
    );
    for schema in [
        "FieldDirectBefore",
        "FieldDirectAfter",
        "FieldNestedBefore",
        "FieldNestedAfter",
        "SchemaDirectBefore",
        "SchemaDirectAfter",
        "SchemaNestedBefore",
        "SchemaNestedAfter",
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
fn explicit_schema_validate_expression_carries_recursive_container_plan_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Failure\n  Denied\n  Expired\nend\n",
            "schema ContainerPacket\n  direct: State::Connected\n  states: List<State::Connected>\nend\n",
            "schema RecursiveContainers\n",
            "  optional: Option<State::Connected>\n",
            "  outcome: Result<State::Connected, Failure::Denied>\n",
            "  linked: List<State::Connected>\n",
            "  vector: Vec<State::Connected>\n",
            "  table: Dict<String, State::Connected>\n",
            "  mixed: Option<Vec<Dict<String, Result<State::Connected, Failure::Denied>>>>\n",
            "end\n",
            "fn container(value: {optional: Option<State>, outcome: Result<State, Failure>, linked: List<State>, vector: Vec<State>, table: Dict<String, State>, mixed: Option<Vec<Dict<String, Result<State, Failure>>>>}, base: ByteOffset) -> Result<{optional: Option<State::Connected>, outcome: Result<State::Connected, Failure::Denied>, linked: List<State::Connected>, vector: Vec<State::Connected>, table: Dict<String, State::Connected>, mixed: Option<Vec<Dict<String, Result<State::Connected, Failure::Denied>>>>}, DecodeError>\n",
            "  validate RecursiveContainers from value at base\nend\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core");
    let core_spec = core
        .schema_refinement_validators
        .iter()
        .find(|spec| spec.schema_name == "RecursiveContainers")
        .expect("core validation plan");
    let CoreSchemaRefinementPlanNode::Record(core_fields) = core_spec.plan.root_node() else {
        panic!("root validation plan should be a record");
    };
    assert_eq!(
        core_fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["optional", "outcome", "linked", "vector", "table", "mixed"]
    );
    assert!(matches!(
        core_spec.plan.node(core_fields[0].plan),
        CoreSchemaRefinementPlanNode::Option(_)
    ));
    let CoreSchemaRefinementPlanNode::Result {
        ok: Some(core_ok),
        err: Some(core_err),
    } = core_spec.plan.node(core_fields[1].plan)
    else {
        panic!("result validation plan should retain both branches");
    };
    assert!(matches!(
        core_spec.plan.node(*core_ok),
        CoreSchemaRefinementPlanNode::Variant(variants) if variants == &["Connected"]
    ));
    assert!(matches!(
        core_spec.plan.node(*core_err),
        CoreSchemaRefinementPlanNode::Variant(variants) if variants == &["Denied"]
    ));
    assert!(matches!(
        core_spec.plan.node(core_fields[2].plan),
        CoreSchemaRefinementPlanNode::List(_)
    ));
    assert!(matches!(
        core_spec.plan.node(core_fields[3].plan),
        CoreSchemaRefinementPlanNode::Vec(_)
    ));
    assert!(matches!(
        core_spec.plan.node(core_fields[4].plan),
        CoreSchemaRefinementPlanNode::Dict(_)
    ));

    let ir = lowered.ir.as_ref().expect("typed IR");
    let ir_spec = ir
        .schema_refinement_validators
        .iter()
        .find(|spec| spec.schema_name == "RecursiveContainers")
        .expect("IR validation plan");
    let IrSchemaRefinementPlanNode::Record(ir_fields) = ir_spec.plan.root_node() else {
        panic!("root IR validation plan should be a record");
    };
    let IrSchemaRefinementPlanNode::Result {
        ok: Some(ir_ok),
        err: Some(ir_err),
    } = ir_spec.plan.node(ir_fields[1].plan)
    else {
        panic!("IR result validation plan should retain both branches");
    };
    assert!(matches!(
        ir_spec.plan.node(*ir_ok),
        IrSchemaRefinementPlanNode::Variant(refinement)
            if refinement.expected_variants == ["Connected"]
    ));
    assert!(matches!(
        ir_spec.plan.node(*ir_err),
        IrSchemaRefinementPlanNode::Variant(refinement)
            if refinement.expected_variants == ["Denied"]
    ));
}

#[test]
fn explicit_schema_validate_expression_erases_source_adt_payloads_and_carries_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Box<A>\n",
            "  Boxed(A)\n",
            "  Named(value: A)\n",
            "  Empty\n",
            "end\n",
            "schema Packet\n",
            "  item: Box<State::Connected>\n",
            "end\n",
            "fn check(value: {item: Box<State>}, base: ByteOffset) -> Result<{item: Box<State::Connected>}, DecodeError>\n",
            "  validate Packet from value at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);

    let core = lowered.core.as_ref().expect("checked core");
    let spec = core
        .schema_refinement_validators
        .iter()
        .find(|spec| spec.schema_name == "Packet")
        .expect("source ADT validation plan");
    let CoreSchemaRefinementPlanNode::Record(fields) = spec.plan.root_node() else {
        panic!("root plan should be a record");
    };
    let CoreSchemaRefinementPlanNode::Adt {
        expected_variants,
        variants,
    } = spec.plan.node(fields[0].plan)
    else {
        panic!("field plan should dispatch a source ADT");
    };
    assert!(expected_variants.is_empty());
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.name.as_str())
            .collect::<Vec<_>>(),
        ["Boxed", "Named", "Empty"]
    );
    assert_eq!(variants[0].payloads[0].index, 0);
    assert_eq!(variants[0].payloads[0].path_segment, "0");
    assert_eq!(variants[1].payloads[0].index, 0);
    assert_eq!(variants[1].payloads[0].path_segment, "value");
    assert!(variants[2].payloads.is_empty());
    assert!(matches!(
        spec.plan.node(variants[0].payloads[0].plan),
        CoreSchemaRefinementPlanNode::Variant(expected) if expected == &["Connected"]
    ));

    let ir = lowered.ir.as_ref().expect("typed IR");
    let spec = ir
        .schema_refinement_validators
        .iter()
        .find(|spec| spec.schema_name == "Packet")
        .expect("IR source ADT validation plan");
    let IrSchemaRefinementPlanNode::Record(fields) = spec.plan.root_node() else {
        panic!("IR root plan should be a record");
    };
    assert!(matches!(
        spec.plan.node(fields[0].plan),
        IrSchemaRefinementPlanNode::Adt { .. }
    ));
}

#[test]
fn explicit_schema_validate_expression_plans_nested_source_adt_container_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Box<A>\n  Boxed(A)\nend\n",
            "type Envelope<A>\n",
            "  Packed(meta: {optional: Option<A>, outcome: Result<A, A>}, items: List<Vec<A>>, table: Dict<String, A>)\n",
            "  Nested(child: Box<A>)\n",
            "end\n",
            "schema Packet\n",
            "  item: Envelope<State::Connected>\n",
            "end\n",
            "fn check(value: {item: Envelope<State>}, base: ByteOffset) -> Result<{item: Envelope<State::Connected>}, DecodeError>\n",
            "  validate Packet from value at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);

    let spec = lowered
        .core
        .as_ref()
        .expect("checked core")
        .schema_refinement_validators
        .iter()
        .find(|spec| spec.schema_name == "Packet")
        .expect("nested source ADT validation plan");
    let CoreSchemaRefinementPlanNode::Record(fields) = spec.plan.root_node() else {
        panic!("root plan should be a record");
    };
    let CoreSchemaRefinementPlanNode::Adt { variants, .. } = spec.plan.node(fields[0].plan) else {
        panic!("field plan should dispatch Envelope");
    };
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.name.as_str())
            .collect::<Vec<_>>(),
        ["Packed", "Nested"]
    );
    assert_eq!(
        variants[0]
            .payloads
            .iter()
            .map(|payload| (payload.index, payload.path_segment.as_str()))
            .collect::<Vec<_>>(),
        [(0, "meta"), (1, "items"), (2, "table")]
    );
    let CoreSchemaRefinementPlanNode::Record(meta_fields) =
        spec.plan.node(variants[0].payloads[0].plan)
    else {
        panic!("metadata payload should retain its record plan");
    };
    assert!(matches!(
        spec.plan.node(meta_fields[0].plan),
        CoreSchemaRefinementPlanNode::Option(inner)
            if matches!(spec.plan.node(*inner), CoreSchemaRefinementPlanNode::Variant(_))
    ));
    assert!(matches!(
        spec.plan.node(meta_fields[1].plan),
        CoreSchemaRefinementPlanNode::Result { ok: Some(ok), err: Some(err) }
            if matches!(spec.plan.node(*ok), CoreSchemaRefinementPlanNode::Variant(_))
                && matches!(spec.plan.node(*err), CoreSchemaRefinementPlanNode::Variant(_))
    ));
    assert!(matches!(
        spec.plan.node(variants[0].payloads[1].plan),
        CoreSchemaRefinementPlanNode::List(list)
            if matches!(spec.plan.node(*list), CoreSchemaRefinementPlanNode::Vec(vec)
                if matches!(spec.plan.node(*vec), CoreSchemaRefinementPlanNode::Variant(_)))
    ));
    assert!(matches!(
        spec.plan.node(variants[0].payloads[2].plan),
        CoreSchemaRefinementPlanNode::Dict(inner)
            if matches!(spec.plan.node(*inner), CoreSchemaRefinementPlanNode::Variant(_))
    ));
    assert!(matches!(
        spec.plan.node(variants[1].payloads[0].plan),
        CoreSchemaRefinementPlanNode::Adt { variants, .. }
            if matches!(spec.plan.node(variants[0].payloads[0].plan), CoreSchemaRefinementPlanNode::Variant(_))
    ));
}

#[test]
fn explicit_schema_validate_expression_rejects_hidden_and_unsupported_adt_payloads() {
    let unsupported = SourceFile::new(
        "unsupported.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Box<A>\n  Boxed(A)\n  Callback(fn(Int) -> String)\nend\n",
            "schema Packet\n  item: Box<State::Connected>\nend\n",
            "fn check(value: {item: Box<State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
        ),
    );
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parse(&unsupported).tree));
    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.validate_expression"),
        "unsupported payload should reject validation: {:#?}",
        lowered.diagnostics
    );

    let assert_import_rejected = |declaration: &str, boundary: &str| {
        let app_source = SourceFile::new(
            "app.veln",
            concat!(
                "mod app\n",
                "use api\n",
                "schema Packet\n  item: api::Box<api::State::Connected>\nend\n",
                "fn check(value: {item: api::Box<api::State>}, base: ByteOffset) -> ()\n",
                "  validate Packet from value at base\n  ()\nend\n",
            ),
        );
        let api_source = SourceFile::new(
            "api.veln",
            format!("mod api\npub type State\n  pub Connected\n  pub Closed\nend\n{declaration}"),
        );
        let app = lower_surface_ast(&parse(&app_source).tree);
        let api = lower_surface_ast(&parse(&api_source).tree);
        let module = SurfaceModule {
            module: app.module,
            uses: app.uses,
            aliases: Vec::new(),
            effects: Vec::new(),
            handlers: Vec::new(),
            types: [app.types, api.types].concat(),
            schemas: app.schemas,
            module_values: Vec::new(),
            functions: app.functions,
            invalid_names: Vec::new(),
        };
        let lowered = lower_checked_surface_module(&module);
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "schema.validate_expression"),
            "{boundary} should reject validation: {:#?}",
            lowered.diagnostics
        );
    };
    assert_import_rejected(
        "pub type Box<A>\n  pub Boxed(value: A)\n  Hidden(value: A)\nend\n",
        "private imported constructor",
    );
    assert_import_rejected(
        concat!(
            "type Secret\n  Hidden\nend\n",
            "pub type Box<A>\n  pub Boxed(value: A)\n  pub SecretBox(value: Secret)\nend\n",
        ),
        "private imported payload type",
    );
}

#[test]
fn explicit_schema_validate_expression_traverses_visible_imported_source_adts() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use api\n",
            "schema Packet\n  item: api::Box<api::State::Connected>\nend\n",
            "fn check(value: {item: api::Box<api::State>}, base: ByteOffset) -> Result<{item: api::Box<api::State::Connected>}, DecodeError>\n",
            "  validate Packet from value at base\n",
            "end\n",
        ),
    );
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "mod api\n",
            "pub type State\n  pub Connected\n  pub Closed\nend\n",
            "pub type Box<A>\n  pub Next(value: Box<A>)\n  pub Boxed(value: A)\n  pub Empty\nend\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let api = lower_surface_ast(&parse(&api_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, api.types].concat(),
        schemas: app.schemas,
        module_values: Vec::new(),
        functions: app.functions,
        invalid_names: Vec::new(),
    };
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let spec = lowered
        .core
        .as_ref()
        .expect("checked core")
        .schema_refinement_validators
        .iter()
        .find(|spec| spec.declaration_name == "Packet")
        .expect("imported source ADT plan");
    let CoreSchemaRefinementPlanNode::Record(fields) = spec.plan.root_node() else {
        panic!("root plan should be a record");
    };
    let field_plan = fields[0].plan;
    let CoreSchemaRefinementPlanNode::Adt { variants, .. } = spec.plan.node(field_plan) else {
        panic!("imported recursive source ADT plan");
    };
    assert_eq!(variants[0].payloads[0].path_segment, "value");
    assert_eq!(variants[0].payloads[0].plan, field_plan);
    assert_eq!(variants[1].payloads[0].path_segment, "value");
}

#[test]
fn explicit_schema_validate_expression_preserves_source_adt_identity_and_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type OtherState\n  Connected\nend\n",
            "type Box<A>\n  Boxed(A)\nend\n",
            "type OtherBox<A>\n  Boxed(A)\nend\n",
            "schema Packet\n  item: Box<State::Connected>\nend\n",
            "fn wrong_identity(value: {item: OtherBox<State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
            "fn wrong_argument(value: {item: Box<OtherState>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
        ),
    );
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parse(&source).tree));
    let mismatches = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 2, "{:#?}", lowered.diagnostics);
    assert!(
        mismatches
            .iter()
            .any(|diagnostic| diagnostic.message.contains("OtherBox"))
    );
    assert!(
        mismatches
            .iter()
            .any(|diagnostic| diagnostic.message.contains("OtherState"))
    );
}

#[test]
fn explicit_schema_validate_expression_shares_acyclic_diamond_plan_nodes() {
    const DEPTH: usize = 14;

    let mut source =
        String::from("type State\n  Connected\n  Closed\nend\ntype N0<A>\n  Leaf(value: A)\nend\n");
    for depth in 1..=DEPTH {
        source.push_str(&format!(
            "type N{depth}<A>\n  Left(value: N{}<A>)\n  Right(value: N{}<A>)\nend\n",
            depth - 1,
            depth - 1,
        ));
    }
    source.push_str(&format!(
        "schema Packet\n  item: N{DEPTH}<State::Connected>\nend\nfn check(value: {{item: N{DEPTH}<State>}}, base: ByteOffset) -> Result<{{item: N{DEPTH}<State::Connected>}}, DecodeError>\n  validate Packet from value at base\nend\n"
    ));

    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);

    let core_plan = &lowered
        .core
        .as_ref()
        .expect("checked core")
        .schema_refinement_validators[0]
        .plan;
    assert_eq!(core_plan.nodes.len(), DEPTH + 3);
    let CoreSchemaRefinementPlanNode::Record(fields) = core_plan.root_node() else {
        panic!("root plan should be a record");
    };
    let mut node = fields[0].plan;
    for _ in 0..DEPTH {
        let CoreSchemaRefinementPlanNode::Adt { variants, .. } = core_plan.node(node) else {
            panic!("diamond level should be an ADT node");
        };
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].payloads[0].plan, variants[1].payloads[0].plan);
        node = variants[0].payloads[0].plan;
    }

    let ir_plan = &lowered
        .ir
        .as_ref()
        .expect("typed IR")
        .schema_refinement_validators[0]
        .plan;
    assert_eq!(ir_plan.root, core_plan.root);
    assert_eq!(ir_plan.nodes.len(), core_plan.nodes.len());
    let IrSchemaRefinementPlanNode::Record(fields) = ir_plan.root_node() else {
        panic!("IR root plan should be a record");
    };
    let mut node = fields[0].plan;
    for _ in 0..DEPTH {
        let IrSchemaRefinementPlanNode::Adt { variants, .. } = ir_plan.node(node) else {
            panic!("IR diamond level should be an ADT node");
        };
        assert_eq!(variants[0].payloads[0].plan, variants[1].payloads[0].plan);
        node = variants[0].payloads[0].plan;
    }
}

#[test]
fn explicit_schema_validate_expression_builds_finite_regular_recursive_source_adt_graphs() {
    for (declaration, field_type, expected_nodes) in [
        (
            "type Node<A>\n  Next(value: Node<A>)\n  Value(value: A)\nend\n",
            "Node<State::Connected>",
            3,
        ),
        (
            "type Node<A>\n  Next(value: Other<A>)\n  Value(value: A)\nend\ntype Other<A>\n  Back(value: Node<A>)\nend\n",
            "Node<State::Connected>",
            4,
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "type State\n  Connected\n  Closed\nend\n{declaration}schema Packet\n  item: {field_type}\nend\nfn check(value: {{item: Node<State>}}, base: ByteOffset) -> Result<{{item: {field_type}}}, DecodeError>\n  validate Packet from value at base\nend\n"
            ),
        );
        let lowered = lower_checked_surface_module(&lower_surface_ast(&parse(&source).tree));
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        let plan = &lowered
            .core
            .as_ref()
            .expect("checked core")
            .schema_refinement_validators[0]
            .plan;
        let CoreSchemaRefinementPlanNode::Record(fields) = plan.root_node() else {
            panic!("root plan should be a record");
        };
        let first = fields[0].plan;
        assert_eq!(plan.nodes.len(), expected_nodes);
        let CoreSchemaRefinementPlanNode::Adt { variants, .. } = plan.node(first) else {
            panic!("field plan should be an ADT");
        };
        let recursive = variants
            .iter()
            .find(|variant| variant.name == "Next")
            .expect("Next plan")
            .payloads[0]
            .plan;
        if expected_nodes == 3 {
            assert_eq!(recursive, first);
        } else {
            let CoreSchemaRefinementPlanNode::Adt { variants, .. } = plan.node(recursive) else {
                panic!("mutual edge should target an ADT");
            };
            assert_eq!(variants[0].payloads[0].plan, first);
        }
    }
}

#[test]
fn explicit_schema_validate_expression_erases_outer_refinements_from_recursive_payloads() {
    for field_type in [
        "Node<State>::Next",
        "Node<State>::Next | Node<State>::Value",
    ] {
        assert_outer_recursive_payloads_are_unrefined(field_type);
    }

    assert_nested_refinement_uses_distinct_recursive_plan();
}

fn assert_outer_recursive_payloads_are_unrefined(field_type: &str) {
    let source = SourceFile::new(
        "main.veln",
        format!(
            "type State\n  Connected\n  Closed\nend\ntype Node<A>\n  Next(value: Node<A>)\n  Value(value: A)\n  Empty\nend\nschema Packet\n  item: {field_type}\nend\nfn check(value: {{item: Node<State>}}, base: ByteOffset) -> Result<{{item: {field_type}}}, DecodeError>\n  validate Packet from value at base\nend\n"
        ),
    );
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parse(&source).tree));
    assert!(
        lowered.diagnostics.is_empty(),
        "{field_type}: {:#?}",
        lowered.diagnostics
    );
    let plan = &lowered
        .core
        .as_ref()
        .expect("checked core")
        .schema_refinement_validators[0]
        .plan;
    let CoreSchemaRefinementPlanNode::Record(fields) = plan.root_node() else {
        panic!("root plan should be a record");
    };
    let CoreSchemaRefinementPlanNode::Adt {
        expected_variants,
        variants,
    } = plan.node(fields[0].plan)
    else {
        panic!("outer refinement should retain ADT dispatch");
    };
    assert!(!expected_variants.is_empty());
    assert!(
        variants.iter().all(|variant| variant.payloads.is_empty()),
        "an outer constraint must not flow onto an unrefined recursive payload"
    );
}

fn assert_nested_refinement_uses_distinct_recursive_plan() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Node<A>\n  Next(value: Node<A>)\n  Value(value: A)\nend\n",
            "schema Packet\n  item: Node<State::Connected>::Next\nend\n",
            "fn check(value: {item: Node<State>}, base: ByteOffset) -> Result<{item: Node<State::Connected>::Next}, DecodeError>\n",
            "  validate Packet from value at base\n",
            "end\n",
        ),
    );
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parse(&source).tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let plan = &lowered
        .core
        .as_ref()
        .expect("checked core")
        .schema_refinement_validators[0]
        .plan;
    let CoreSchemaRefinementPlanNode::Record(fields) = plan.root_node() else {
        panic!("root plan should be a record");
    };
    let outer = fields[0].plan;
    let CoreSchemaRefinementPlanNode::Adt {
        expected_variants,
        variants,
    } = plan.node(outer)
    else {
        panic!("outer refinement should dispatch the source ADT");
    };
    assert_eq!(expected_variants, &["Next"]);
    let recursive = variants
        .iter()
        .find(|variant| variant.name == "Next")
        .expect("Next plan")
        .payloads[0]
        .plan;
    assert_ne!(recursive, outer);
    let CoreSchemaRefinementPlanNode::Adt {
        expected_variants,
        variants,
    } = plan.node(recursive)
    else {
        panic!("recursive payload should use an unrefined ADT cycle");
    };
    assert!(expected_variants.is_empty());
    assert_eq!(
        variants
            .iter()
            .find(|variant| variant.name == "Next")
            .expect("recursive Next plan")
            .payloads[0]
            .plan,
        recursive
    );
    let value_plan = variants
        .iter()
        .find(|variant| variant.name == "Value")
        .expect("Value plan")
        .payloads[0]
        .plan;
    assert!(matches!(
        plan.node(value_plan),
        CoreSchemaRefinementPlanNode::Variant(expected) if expected == &["Connected"]
    ));
}

#[test]
fn explicit_schema_validate_expression_rejects_non_regular_recursion() {
    for declaration in [
        "type Node<A>\n  Next(value: Node<Option<A>>)\n  Value(value: A)\nend\n",
        "type Node<A>\n  Next(value: Node<State>)\n  Value(value: A)\nend\n",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "type State\n  Connected\n  Closed\nend\n{declaration}schema Packet\n  item: Node<State::Connected>\nend\nfn check(value: {{item: Node<State>}}, base: ByteOffset) -> ()\n  validate Packet from value at base\n  ()\nend\n"
            ),
        );
        let lowered = lower_checked_surface_module(&lower_surface_ast(&parse(&source).tree));
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.id == "schema.validate_expression" }),
            "{:#?}",
            lowered.diagnostics
        );
    }
}

#[test]
fn explicit_schema_validate_expression_rejects_wrong_container_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type Failure\n  Denied\nend\n",
            "type OtherState\n  Connected\nend\n",
            "type Box<A>\n  Boxed(value: A)\nend\n",
            "schema Packet\n",
            "  optional: Option<State::Connected>\n",
            "  outcome: Result<State::Connected, Failure::Denied>\n",
            "  table: Dict<String, State::Connected>\n",
            "end\n",
            "fn wrong_identity(value: {optional: Box<State>, outcome: Result<State, Failure>, table: Dict<String, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
            "fn wrong_option_arg(value: {optional: Option<OtherState>, outcome: Result<State, Failure>, table: Dict<String, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
            "fn swapped_result_args(value: {optional: Option<State>, outcome: Result<Failure, State>, table: Dict<String, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
            "fn wrong_result_first_arg(value: {optional: Option<State>, outcome: Result<OtherState, Failure>, table: Dict<String, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
            "fn wrong_result_second_arg(value: {optional: Option<State>, outcome: Result<State, String>, table: Dict<String, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
            "fn wrong_dict_key(value: {optional: Option<State>, outcome: Result<State, Failure>, table: Dict<Int, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let lowered = lower_checked_surface_module(&module);
    let mismatches = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 6, "{:#?}", lowered.diagnostics);
}

#[test]
fn explicit_schema_validate_expression_rejects_same_spelled_source_container_types() {
    let cases = [
        (
            "Option<A>",
            "Wrapped(value: A)",
            "Option<State::Connected>",
            "Option<State>",
        ),
        (
            "Result<A, B>",
            "Wrapped(value: A, error: B)",
            "Result<State::Connected, State>",
            "Result<State, State>",
        ),
        (
            "List<A>",
            "Wrapped(value: A)",
            "List<State::Connected>",
            "List<State>",
        ),
        (
            "Vec<A>",
            "Wrapped(value: A)",
            "Vec<State::Connected>",
            "Vec<State>",
        ),
        (
            "Dict<K, V>",
            "Wrapped(key: K, value: V)",
            "Dict<String, State::Connected>",
            "Dict<String, State>",
        ),
    ];
    for (declaration, variant, refined, erased) in cases {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "type State\n  Connected\n  Closed\nend\ntype {declaration}\n  {variant}\nend\nschema Packet\n  value: {refined}\nend\nfn check(value: {{value: {erased}}}, base: ByteOffset) -> ()\n  validate Packet from value at base\n  ()\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{declaration}: {:#?}",
            parsed.diagnostics
        );
        let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "schema.validate_expression"),
            "same-spelled {declaration} should not be a standard container: {:#?}",
            lowered.diagnostics
        );
        assert!(
            lowered.core.as_ref().is_none_or(|core| {
                core.schema_refinement_validators
                    .iter()
                    .all(|spec| spec.schema_name != "Packet")
            }),
            "same-spelled {declaration} should not produce a validation plan"
        );
    }
}

#[test]
fn explicit_schema_validate_expression_rejects_application_owned_std_prelude_container() {
    let source = SourceFile::new(
        "std/prelude.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type List<A>\n  Nil\n  Cons(head: A, tail: List<A>)\nend\n",
            "schema Packet\n  value: List<State::Connected>\nend\n",
            "fn check(value: {value: List<State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast_with_module_identity(
        &parsed.tree,
        "std::prelude".to_string(),
        source.span(veln_source::TextRange::at(0)),
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.validate_expression"),
        "application-owned std::prelude List must not be a standard container: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.core.as_ref().is_none_or(|core| {
            core.schema_refinement_validators
                .iter()
                .all(|spec| spec.schema_name != "Packet")
        }),
        "application-owned std::prelude List must not produce a validation plan"
    );
}

#[test]
fn explicit_schema_validate_expression_rejects_same_spelled_source_string_dict_key() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Connected\n  Closed\nend\n",
            "type String\n  Text\nend\n",
            "schema Packet\n  value: Dict<String, State::Connected>\nend\n",
            "fn check(value: {value: Dict<String, State>}, base: ByteOffset) -> ()\n",
            "  validate Packet from value at base\n  ()\nend\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "schema.validate_expression"),
        "same-spelled String should not be a standard dictionary key: {:#?}",
        lowered.diagnostics
    );
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
