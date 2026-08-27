use super::*;
use veln_source::SourceFile;
use veln_syntax::parse;

fn lower_source(text: &str) -> SurfaceModule {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    lower_surface_ast(&parsed.tree)
}

fn lower_source_allowing_diagnostics(text: &str) -> SurfaceModule {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    lower_surface_ast(&parsed.tree)
}

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
    ];

    for source in sources {
        let module = lower_source(source);
        let encoded = encode_surface_module(&module);
        let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");

        assert_eq!(encode_surface_module(&decoded), encoded);
    }
}

#[test]
fn lowers_public_alias_target_leaf_spans_and_invalid_case_occurrences() {
    let module = lower_source(concat!(
        "fn parse() -> Int\n",
        "  1\n",
        "end\n",
        "pub fn exposed = api::Parse\n",
        "pub type Exposed = api::_item\n",
        "pub schema Packet = api::packet\n",
    ));

    assert_eq!(module.aliases.len(), 3);
    assert_eq!(module.aliases[0].target, ["api", "Parse"]);
    assert_eq!(module.aliases[0].target_spans[1].start.column, 23);
    assert_eq!(module.aliases[0].target_spans[1].end.column, 28);
    assert_eq!(module.aliases[1].target, ["api", "_item"]);
    assert_eq!(module.aliases[1].target_spans[1].start.column, 25);
    assert_eq!(module.aliases[1].target_spans[1].end.column, 30);
    assert_eq!(module.aliases[2].target, ["api", "packet"]);
    assert_eq!(module.aliases[2].target_spans[1].start.column, 26);
    assert_eq!(module.aliases[2].target_spans[1].end.column, 32);

    let invalid = module
        .invalid_names
        .iter()
        .filter(|name| name.occurrence == NameOccurrence::AliasTarget)
        .collect::<Vec<_>>();
    assert_eq!(invalid.len(), 2, "{:#?}", module.invalid_names);
    assert_eq!(invalid[0].name, "Parse");
    assert_eq!(invalid[0].class, NameClass::Function);
    assert_eq!(invalid[0].span.start.column, 23);
    assert_eq!(invalid[1].name, "_item");
    assert_eq!(invalid[1].class, NameClass::Type);
    assert_eq!(invalid[1].span.start.column, 25);

    let encoded = encode_surface_module(&module);
    let decoded = decode_surface_module(&encoded).expect("wire round trip should decode");
    assert_eq!(
        decoded.aliases[0].target_spans[1],
        module.aliases[0].target_spans[1]
    );
    assert_eq!(
        decoded.invalid_names[0].occurrence,
        module.invalid_names[0].occurrence
    );
}

fn expr_line(function: &Function, index: usize) -> &Expr {
    let BodyLineKind::Expr { expr } = &function.body[index].kind else {
        panic!("expected expression line");
    };
    expr
}

fn let_line(function: &Function, index: usize) -> (&Pattern, &Option<String>, &Expr) {
    let BodyLineKind::Let {
        pattern,
        annotation,
        expr,
    } = &function.body[index].kind
    else {
        panic!("expected let line");
    };
    (pattern, annotation, expr)
}

fn collect_module_node_ids(module: &SurfaceModule) -> Vec<u32> {
    let mut ids = Vec::new();
    for function in &module.functions {
        collect_function_node_ids(function, &mut ids);
    }
    ids
}

fn collect_function_node_ids(function: &Function, ids: &mut Vec<u32>) {
    ids.push(function.node_id.as_u32());
    ids.extend(function.params.iter().map(|param| param.node_id.as_u32()));
    ids.extend(
        function
            .return_binding
            .iter()
            .map(|binding| binding.node_id.as_u32()),
    );
    ids.extend(
        function
            .contracts
            .iter()
            .map(|contract| contract.node_id.as_u32()),
    );
    for line in &function.body {
        ids.push(line.node_id.as_u32());
        match &line.kind {
            BodyLineKind::Let { expr, .. } | BodyLineKind::Expr { expr } => {
                collect_expr_node_ids(expr, ids);
            }
        }
    }
}

fn collect_expr_node_ids(expr: &Expr, ids: &mut Vec<u32>) {
    ids.push(expr.node_id.as_u32());
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            collect_expr_node_ids(callee, ids);
            for arg in args {
                collect_expr_node_ids(arg, ids);
            }
        }
        ExprKind::TypeApply { callee, .. } => collect_expr_node_ids(callee, ids),
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_expr_node_ids(arg, ids);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_expr_node_ids(body, ids);
            for arg in args {
                collect_expr_node_ids(arg, ids);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_expr_node_ids(input, ids);
            collect_expr_node_ids(base, ids);
        }
        ExprKind::SchemaEncode { value, .. } => collect_expr_node_ids(value, ids),
        ExprKind::FieldAccess { base, .. } => collect_expr_node_ids(base, ids),
        ExprKind::Try(expr) => collect_expr_node_ids(expr, ids),
        ExprKind::Record(fields) => {
            for field in fields {
                ids.push(field.node_id.as_u32());
                collect_expr_node_ids(&field.expr, ids);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                ids.push(entry.node_id.as_u32());
                collect_expr_node_ids(&entry.key, ids);
                collect_expr_node_ids(&entry.value, ids);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_expr_node_ids(item, ids);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_node_ids(scrutinee, ids);
            for arm in arms {
                ids.push(arm.node_id.as_u32());
                collect_pattern_node_ids(&arm.pattern, ids);
                collect_expr_node_ids(&arm.expr, ids);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr_node_ids(condition, ids);
            collect_expr_node_ids(then_branch, ids);
            for branch in else_if_branches {
                ids.push(branch.node_id.as_u32());
                collect_expr_node_ids(&branch.condition, ids);
                collect_expr_node_ids(&branch.expr, ids);
            }
            collect_expr_node_ids(else_branch, ids);
        }
        ExprKind::Prefix { expr, .. } => collect_expr_node_ids(expr, ids),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_node_ids(left, ids);
            collect_expr_node_ids(right, ids);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_pattern_node_ids(pattern: &Pattern, ids: &mut Vec<u32>) {
    ids.push(pattern.node_id.as_u32());
    match &pattern.kind {
        PatternKind::Record(fields) => {
            for field in fields {
                ids.push(field.node_id.as_u32());
                collect_pattern_node_ids(&field.pattern, ids);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_node_ids(arg, ids);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Binding(_)
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

#[test]
fn assigns_session_stable_node_ids() {
    let module = lower_source("fn id(value: Int) -> Int\n  value\nend\n");

    assert_eq!(module.functions[0].node_id.display("fn"), "fn-1");
    assert_eq!(
        module.functions[0].params[0].node_id.display("param"),
        "param-2"
    );
    assert_eq!(
        module.functions[0].body[0].node_id.display("expr"),
        "expr-3"
    );
    let BodyLineKind::Expr { expr } = &module.functions[0].body[0].kind else {
        panic!("expected expression line");
    };
    assert_eq!(expr.node_id.display("expr"), "expr-4");
}

#[test]
fn lowers_module_header_and_use_aliases() {
    let module = lower_source(concat!(
        "mod app.core\n",
        "use platform.io\n",
        "fn main() -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(module.module.as_ref().unwrap().name, "app.core");
    assert_eq!(
        module.module.as_ref().unwrap().node_id.display("mod"),
        "mod-1"
    );
    assert_eq!(module.uses[0].name, "platform.io");
    assert_eq!(module.uses[0].alias, "io");
    assert_eq!(module.uses[0].node_id.display("use"), "use-2");
    assert_eq!(module.functions[0].node_id.display("fn"), "fn-3");
}

#[test]
fn lowers_type_declarations_with_variant_fields() {
    let module = lower_source(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn main() -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(module.types.len(), 1);
    let list = &module.types[0];
    assert_eq!(list.name.as_deref(), Some("List"));
    assert_eq!(list.params, vec!["A"]);
    assert_eq!(list.variants.len(), 2);
    assert_eq!(list.variants[0].name.as_deref(), Some("Nil"));
    assert!(list.variants[0].fields.is_empty());
    assert_eq!(list.variants[1].name.as_deref(), Some("Cons"));
    assert_eq!(list.variants[1].fields[0].name, "head");
    assert_eq!(list.variants[1].fields[0].ty, "A");
    assert_eq!(list.variants[1].fields[1].name, "tail");
    assert_eq!(list.variants[1].fields[1].ty, "List<A>");
}

#[test]
fn lowers_schema_declarations_as_distinct_module_items() {
    let module = lower_source(concat!(
        "pub schema Http2FrameHeader\n",
        "  format binary\n",
        "\n",
        "  length: UInt24be\n",
        "  kind: UInt8\n",
        "  padding_length: UInt8 where padding_length <= length\n",
        "  stream_reserved: ReservedBits(1, 0)\n",
        "  stream_id: UInt31be\n",
        "  settings: Repeat(length - padding_length, UInt16be)\n",
        "  payload: ByteView(length - padding_length)\n",
        "  validate padding_length <= length\n",
        "end\n",
    ));

    assert!(module.functions.is_empty());
    assert!(module.types.is_empty());
    assert_eq!(module.schemas.len(), 1);
    let schema = &module.schemas[0];
    assert_eq!(schema.node_id.display("schema"), "schema-1");
    assert_eq!(schema.visibility, Visibility::Public);
    assert_eq!(schema.name.as_deref(), Some("Http2FrameHeader"));
    assert_eq!(
        schema.format.as_ref().map(|format| format.name.as_str()),
        Some("binary")
    );
    assert_eq!(schema.fields.len(), 7);
    assert_eq!(schema.fields[0].name, "length");
    assert_eq!(schema.fields[0].ty, "UInt24be");
    assert_eq!(schema.fields[1].name, "kind");
    assert_eq!(schema.fields[1].ty, "UInt8");
    assert_eq!(schema.fields[2].name, "padding_length");
    assert_eq!(schema.fields[2].ty, "UInt8");
    let where_clause = schema.fields[2]
        .where_clause
        .as_ref()
        .expect("field should lower where clause");
    assert_eq!(
        where_clause.node_id.display("schema_field_where"),
        "schema_field_where-6"
    );
    assert_eq!(where_clause.predicate, "padding_length <= length");
    assert_eq!(schema.fields[3].name, "stream_reserved");
    assert_eq!(schema.fields[3].ty, "ReservedBits(1, 0)");
    assert_eq!(schema.fields[4].name, "stream_id");
    assert_eq!(schema.fields[4].ty, "UInt31be");
    assert_eq!(schema.fields[5].name, "settings");
    assert_eq!(
        schema.fields[5].ty,
        "Repeat(length - padding_length, UInt16be)"
    );
    assert_eq!(schema.fields[6].name, "payload");
    assert_eq!(schema.fields[6].ty, "ByteView(length - padding_length)");
    assert_eq!(schema.validations.len(), 1);
    assert_eq!(
        schema.validations[0].node_id.display("schema_validation"),
        "schema_validation-11"
    );
    assert_eq!(schema.validations[0].predicate, "padding_length <= length");
}

#[test]
fn lowers_schema_operations_without_codec_items() {
    let module = lower_source(concat!(
        "schema Http2FrameHeader\n",
        "  format binary\n",
        "  length: UInt8\n",
        "end\n",
        "\n",
        "pub fn decode_header(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
        "  decode Http2FrameHeader from view at base\n",
        "end\n",
        "\n",
        "pub fn encode_header(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
        "  encode Http2FrameHeader from packet\n",
        "end\n",
    ));

    assert!(module.types.is_empty());
    assert_eq!(module.schemas.len(), 1);
    assert_eq!(module.functions.len(), 2);
    assert!(module.codecs.is_empty());
    assert!(matches!(
        &expr_line(&module.functions[0], 0).kind,
        ExprKind::SchemaDecode { schema, .. } if schema == &vec!["Http2FrameHeader".to_string()]
    ));
    assert!(matches!(
        &expr_line(&module.functions[1], 0).kind,
        ExprKind::SchemaEncode { schema, .. } if schema == &vec!["Http2FrameHeader".to_string()]
    ));
}

#[test]
fn lowers_effect_declarations_and_perform_expressions() {
    let module = lower_source(concat!(
        "mod app.audit\n",
        "pub effect Audit\n",
        "  record(user: String, count: Int) -> String\n",
        "  flush() -> ()\n",
        "end\n",
        "\n",
        "pub fn record_once() -> String effects [Audit]\n",
        "  perform Audit::record(\"user\", 1)\n",
        "end\n",
    ));

    assert_eq!(module.effects.len(), 1);
    let effect = &module.effects[0];
    assert_eq!(effect.node_id.display("effect"), "effect-2");
    assert_eq!(effect.module_name.as_deref(), Some("app.audit"));
    assert_eq!(effect.visibility, Visibility::Public);
    assert_eq!(effect.name.as_deref(), Some("Audit"));
    assert_eq!(effect.operations.len(), 2);
    assert_eq!(
        effect.operations[0].node_id.display("effect_operation"),
        "effect_operation-3"
    );
    assert_eq!(effect.operations[0].name.as_deref(), Some("record"));
    assert_eq!(effect.operations[0].params.len(), 2);
    assert_eq!(effect.operations[0].params[0].name, "user");
    assert_eq!(effect.operations[0].params[0].ty.as_deref(), Some("String"));
    assert_eq!(effect.operations[0].params[1].name, "count");
    assert_eq!(effect.operations[0].params[1].ty.as_deref(), Some("Int"));
    assert_eq!(effect.operations[0].return_type.as_deref(), Some("String"));
    assert_eq!(effect.operations[1].name.as_deref(), Some("flush"));
    assert_eq!(effect.operations[1].return_type.as_deref(), Some("()"));

    let function = &module.functions[0];
    let ExprKind::Perform {
        effect,
        operation,
        args,
        ..
    } = &expr_line(function, 0).kind
    else {
        panic!("expected perform expression");
    };
    assert_eq!(effect, &vec!["Audit".to_string()]);
    assert_eq!(operation, "record");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind, ExprKind::StringLiteral(value) if value == "\"user\""));
    assert!(matches!(&args[1].kind, ExprKind::IntLiteral(value) if value == "1"));
}

#[test]
fn lowers_holes_to_node_id_backed_expression_nodes() {
    let module = lower_source("fn todo() -> ()\n  _answer\nend\n");

    let BodyLineKind::Expr { expr } = &module.functions[0].body[0].kind else {
        panic!("expected expression line");
    };
    assert_eq!(expr.node_id.display("expr"), "expr-3");
    assert!(matches!(
        &expr.kind,
        ExprKind::Hole {
            name: Some(name), ..
        } if name == "answer"
    ));
}

#[test]
fn lowers_function_metadata_contracts_and_let_lines() {
    let module = lower_source(concat!(
        "pub fn publish(user: User, count: Int) -> output: Result<(), Error> effects [db, log]\n",
        "  require count >= 0\n",
        "  invariant count >= 0\n",
        "  ensure output == output\n",
        "  let message: String = \"ready\"\n",
        "  message\n",
        "end\n",
    ));

    let function = &module.functions[0];
    assert_eq!(function.visibility, Visibility::Public);
    assert_eq!(function.name.as_deref(), Some("publish"));
    assert_eq!(
        function
            .return_binding
            .as_ref()
            .map(|binding| binding.name.as_str()),
        Some("output")
    );
    assert_eq!(function.return_type.as_deref(), Some("Result<(), Error>"));
    assert_eq!(
        function.effects,
        Some(vec!["db".to_string(), "log".to_string()])
    );

    assert_eq!(function.params.len(), 2);
    assert_eq!(function.params[0].name, "user");
    assert_eq!(function.params[0].ty.as_deref(), Some("User"));
    assert_eq!(function.params[1].name, "count");
    assert_eq!(function.params[1].ty.as_deref(), Some("Int"));

    assert_eq!(function.contracts.len(), 3);
    assert_eq!(function.contracts[0].kind, ContractKind::Require);
    assert_eq!(function.contracts[0].text, "count >= 0");
    assert_eq!(function.contracts[1].kind, ContractKind::Invariant);
    assert_eq!(function.contracts[1].text, "count >= 0");
    assert_eq!(function.contracts[2].kind, ContractKind::Ensure);
    assert_eq!(function.contracts[2].text, "output == output");

    let (pattern, annotation, expr) = let_line(function, 0);
    assert!(matches!(&pattern.kind, PatternKind::Binding(name) if name == "message"));
    assert_eq!(annotation.as_deref(), Some("String"));
    assert!(matches!(&expr.kind, ExprKind::StringLiteral(value) if value == "\"ready\""));

    assert!(matches!(
        &expr_line(function, 1).kind,
        ExprKind::NamePath(segments) if segments == &vec!["message".to_string()]
    ));
}

#[test]
fn lowers_nested_expression_edge_cases() {
    let module = lower_source(concat!(
        "fn build(input: Int) -> ()\n",
        "  let data = {answer: [1, 2.5, -input?], check: _value satisfy candidate => candidate > 0}\n",
        "  data |> sink(\"ok\", ())\n",
        "end\n",
    ));
    let function = &module.functions[0];

    let (_, _, expr) = let_line(function, 0);
    let ExprKind::Record(fields) = &expr.kind else {
        panic!("expected record expression");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "answer");
    let ExprKind::List(items) = &fields[0].expr.kind else {
        panic!("expected list field expression");
    };
    assert!(matches!(&items[0].kind, ExprKind::IntLiteral(value) if value == "1"));
    assert!(matches!(&items[1].kind, ExprKind::FloatLiteral(value) if value == "2.5"));
    assert!(matches!(
        &items[2].kind,
        ExprKind::Prefix {
            op: PrefixOp::Negate,
            expr,
        } if matches!(
            &expr.kind,
            ExprKind::Try(inner)
                if matches!(&inner.kind, ExprKind::NamePath(segments) if segments == &vec!["input".to_string()])
        )
    ));

    assert_eq!(fields[1].name, "check");
    let ExprKind::Hole {
        name,
        satisfy: Some(satisfy),
    } = &fields[1].expr.kind
    else {
        panic!("expected hole with satisfy clause");
    };
    assert_eq!(name.as_deref(), Some("value"));
    assert_eq!(satisfy.candidate.as_deref(), Some("candidate"));
    assert_eq!(satisfy.predicate, "candidate > 0");

    let ExprKind::Binary {
        op: BinaryOp::PipeGreater,
        left,
        right,
    } = &expr_line(function, 1).kind
    else {
        panic!("expected pipe expression");
    };
    assert!(
        matches!(&left.kind, ExprKind::NamePath(segments) if segments == &vec!["data".to_string()])
    );
    let ExprKind::Call { callee, args } = &right.kind else {
        panic!("expected call on right side of pipe");
    };
    assert!(
        matches!(&callee.kind, ExprKind::NamePath(segments) if segments == &vec!["sink".to_string()])
    );
    assert!(matches!(&args[0].kind, ExprKind::StringLiteral(value) if value == "\"ok\""));
    assert!(matches!(&args[1].kind, ExprKind::Unit));
}

#[test]
fn lowers_boolean_literals_as_literals() {
    let module = lower_source("fn main() -> Bool\n  true\nend\n");
    let expr = expr_line(&module.functions[0], 0);

    assert!(matches!(expr.kind, ExprKind::BoolLiteral(true)));
}

#[test]
fn preserves_if_expression_as_surface_ast_node() {
    let module = lower_source(concat!(
        "fn choose(first: Bool, second: Bool) -> Int\n",
        "  if first\n",
        "    1\n",
        "  else if second\n",
        "    2\n",
        "  else\n",
        "    3\n",
        "  end\n",
        "end\n",
    ));
    let ExprKind::If {
        condition,
        then_branch,
        else_if_branches,
        else_branch,
    } = &expr_line(&module.functions[0], 0).kind
    else {
        panic!("expected if expression");
    };

    assert!(
        matches!(&condition.kind, ExprKind::NamePath(segments) if segments == &vec!["first".to_string()])
    );
    assert!(matches!(&then_branch.kind, ExprKind::IntLiteral(value) if value == "1"));
    assert_eq!(else_if_branches.len(), 1);
    assert!(
        matches!(&else_if_branches[0].condition.kind, ExprKind::NamePath(segments) if segments == &vec!["second".to_string()])
    );
    assert!(matches!(&else_if_branches[0].expr.kind, ExprKind::IntLiteral(value) if value == "2"));
    assert!(matches!(&else_branch.kind, ExprKind::IntLiteral(value) if value == "3"));
}

#[test]
fn lowers_record_patterns_with_field_node_ids() {
    let module = lower_source(concat!(
        "fn describe(value: {count: Int, label: String}) -> String\n",
        "  match value\n",
        "    {count: 0, label: name} => name\n",
        "  end\n",
        "end\n",
    ));
    let ExprKind::Match { arms, .. } = &expr_line(&module.functions[0], 0).kind else {
        panic!("expected match expression");
    };
    let PatternKind::Record(fields) = &arms[0].pattern.kind else {
        panic!("expected record pattern");
    };

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "count");
    assert_eq!(fields[0].node_id.display("field"), "field-8");
    assert!(matches!(
        &fields[0].pattern.kind,
        PatternKind::IntLiteral(value) if value == "0"
    ));
    assert_eq!(fields[1].name, "label");
    assert_eq!(fields[1].node_id.display("field"), "field-10");
    assert!(matches!(
        &fields[1].pattern.kind,
        PatternKind::Binding(name) if name == "name"
    ));
}

#[test]
fn allocates_unique_contiguous_node_ids_across_nested_nodes_and_functions() {
    let module = lower_source(concat!(
        "fn first(input: {x: Int}) -> Int\n",
        "  match input\n",
        "    {x: value} => value\n",
        "  end\n",
        "end\n",
        "fn second() -> ()\n",
        "  _\n",
        "end\n",
    ));

    let mut ids = collect_module_node_ids(&module);
    ids.sort_unstable();
    assert_eq!(ids, (1..=ids.len() as u32).collect::<Vec<_>>());
    assert_eq!(module.functions[0].node_id.as_u32(), 1);
    assert!(module.functions[1].node_id > module.functions[0].node_id);
}

#[test]
fn lowers_missing_let_initializers_to_missing_expressions() {
    let module = lower_source_allowing_diagnostics("fn broken() -> ()\n  let value =\nend\n");
    let (_, _, expr) = let_line(&module.functions[0], 0);

    assert!(matches!(&expr.kind, ExprKind::Missing));
}
