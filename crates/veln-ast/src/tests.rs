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
    if let PatternKind::Constructor { args, .. } = &pattern.kind {
        for arg in args {
            collect_pattern_node_ids(arg, ids);
        }
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
        "fn main() -> () effects []\n",
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
        "pub fn publish(user: User, count: Int) -> output: Result((), Error) effects [db, log]\n",
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
    assert_eq!(function.return_type.as_deref(), Some("Result((), Error)"));
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
fn allocates_unique_contiguous_node_ids_across_nested_nodes_and_functions() {
    let module = lower_source(concat!(
        "fn first() -> ()\n",
        "  {x: [1]}\n",
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
