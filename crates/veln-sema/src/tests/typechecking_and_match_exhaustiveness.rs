use super::*;

#[test]
fn marks_static_sept_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate =
        exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f", "g"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn marks_static_oct_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate =
        exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f", "g", "h"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn marks_distinct_literal_equality_contradiction_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: String, fallback: String) -> String effects []\n",
            "  _value satisfy candidate => not (candidate == \"ready\" and candidate == \"done\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",\
         \"type\":\"String\",\"rank\":1,\"reason\":\"satisfy_tautology\",\
         \"application_policy\":\"safe_repair_candidate\","
    ));
    assert!(details.contains(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",\
         \"type\":\"String\",\"rank\":2,\"reason\":\"satisfy_tautology\",\
         \"application_policy\":\"safe_repair_candidate\","
    ));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn does_not_mark_invalid_static_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => candidate.ready or (not candidate.ready and true)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_field_missing"
            && diagnostic.details.to_json().contains("\"field\":\"ready\"")
    }));
    let hole = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "hole.unfilled")
        .expect("unfilled hole diagnostic");
    let details = hole.details.to_json();
    assert!(!details.contains("\"reason\":\"satisfy_tautology\""));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(!details.contains("\"application_policy\":\"safe_repair_candidate\""));
}

#[test]
fn reports_return_type_mismatch() {
    let source = SourceFile::new("main.veln", "fn bad() -> Int\n  \"no\"\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-3\"]}"
        )
    );
}

#[test]
fn omitted_tail_expression_returns_unit() {
    let source = SourceFile::new("main.veln", "fn main() -> () effects []\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_tail_expression_checks_declared_return_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Int effects []\n  let value = 1\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `()`");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"fn-1\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"()\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"implicit_unit\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"fn-1\"]}"
        )
    );
}

#[test]
fn omitted_tail_expression_lowers_to_unit_return() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> () effects []\n", "  let value = 1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("omitted tail should lower as unit return");
    };
    assert!(matches!(expr.kind, CoreExprKind::Unit));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("omitted tail should lower as IR unit return");
    };
    assert!(matches!(value.kind, IrExprKind::Unit));
}

#[test]
fn ok_constructor_accepts_declared_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result((), AppError)\n  Ok(())\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn result_constructor_checks_expected_value_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result((), AppError)\n  Ok(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-5\",\"expected_type\":\"()\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"call_argument\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-5\"]}"
        )
    );
}

#[test]
fn accepts_supported_type_forms_and_record_expected_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> {score: Float, names: Vec(String), table: Dict(String, Int), ",
            "callback: fn(Int) -> String}\n",
            "  {score: _, names: [], table: _, callback: _}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("\"expected_type\":\"Float\""));
    assert!(rendered.contains("\"expected_type\":\"Dict(String, Int)\""));
    assert!(rendered.contains("\"expected_type\":\"fn(Int) -> String\""));
    assert!(rendered.contains("\"candidate_queries\":[{\"kind\":\"symbol\""));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.related.is_empty())
    );
}

#[test]
fn accepts_dictionary_literals_with_expected_key_and_value_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Dict(String, Int)\n",
            "  {\"one\": 1, \"two\": 2}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

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
    assert_eq!(expr.ty, CoreType::dict(CoreType::string(), CoreType::int()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key.ty, CoreType::string());
    assert_eq!(entries[0].value.ty, CoreType::int());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 2);
}

#[test]
fn accepts_dictionary_literals_with_identifier_led_expression_keys() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(seed: Int) -> Dict(Int, String)\n",
            "  {seed + 1: \"next\"}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

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
    assert_eq!(expr.ty, CoreType::dict(CoreType::int(), CoreType::string()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key.ty, CoreType::int());
    assert_eq!(entries[0].value.ty, CoreType::string());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 1);
}

#[test]
fn record_patterns_bind_field_types_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {count: Int, label: String}) -> String effects []\n",
            "  match value\n",
            "    {count: 0, label: name} => name\n",
            "    {count: count, label: _} => \"many\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

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
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("tail expression should lower as match");
    };
    let CorePatternKind::Record(fields) = &arms[0].pattern.kind else {
        panic!("first arm should lower as record pattern");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "count");
    assert_eq!(fields[1].name, "label");

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Match { arms, .. } = &value.kind else {
        panic!("tail expression should lower as IR match");
    };
    assert!(matches!(
        &arms[1].pattern.kind,
        IrPatternKind::Record(fields)
            if fields.iter().any(|field| field.name == "count")
    ));
}

#[test]
fn match_expression_type_checks_inside_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wrap(value: String) -> String effects []\n",
            "  value\n",
            "end\n",
            "fn describe(value: Option(Int)) -> String effects []\n",
            "  wrap(match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let describe = core
        .functions
        .iter()
        .find(|function| function.name == "describe")
        .expect("describe should be lowered");
    let CoreStmtKind::Return { expr } = &describe.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(args[0].kind, CoreExprKind::Match { .. }));
}

#[test]
fn match_exhaustiveness_accepts_finite_builtin_domains() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bool_label(value: Bool) -> String effects []\n",
            "  match value\n",
            "    true => \"true\"\n",
            "    false => \"false\"\n",
            "  end\n",
            "end\n",
            "fn option_label(value: Option(Int)) -> String effects []\n",
            "  match value\n",
            "    Some(_) => \"some\"\n",
            "    None => \"none\"\n",
            "  end\n",
            "end\n",
            "fn result_label(value: Result(Int, String)) -> String effects []\n",
            "  match value\n",
            "    Ok(_) => \"ok\"\n",
            "    Err(_) => \"err\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn match_exhaustiveness_accepts_catch_all_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wildcard(value: Option(Int)) -> String effects []\n",
            "  match value\n",
            "    Some(_) => \"some\"\n",
            "    _ => \"fallback\"\n",
            "  end\n",
            "end\n",
            "fn binding(value: Result(Int, String)) -> String effects []\n",
            "  match value\n",
            "    Ok(_) => \"ok\"\n",
            "    other => \"fallback\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn match_exhaustiveness_reports_missing_bool_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Bool) -> String effects []\n",
            "  match value\n",
            "    true => \"yes\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing bool case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case false");
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn match_exhaustiveness_reports_empty_finite_builtin_match() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Bool) -> String effects []\n",
            "  match value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("empty finite-domain match should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case false");
    assert_eq!(diagnostic.related.len(), 1);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn match_exhaustiveness_reports_missing_option_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option(Int)) -> String effects []\n",
            "  match value\n",
            "    Some(count) => \"some\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case None");
    assert_eq!(diagnostic.related.len(), 2);
}
