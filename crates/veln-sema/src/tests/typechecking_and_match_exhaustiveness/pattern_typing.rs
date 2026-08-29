use super::*;

#[test]
fn record_patterns_bind_field_types_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {count: Int, label: String}) -> String\n",
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
fn record_let_patterns_bind_nested_field_types_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {outer: {count: Int, label: String}, ignored: Bool}) -> String\n",
            "  let {outer: {count: count, label: label}, ignored: _} = value\n",
            "  let _: Int = count + 1\n",
            "  label\n",
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
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            CoreStmtKind::Let { name, ty, .. }
                if name == "count" && ty == &CoreType::int()
        )
    }));
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            CoreStmtKind::Let { name, ty, .. }
                if name == "label" && ty == &CoreType::string()
        )
    }));

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            IrStmtKind::Let { name, ty, .. }
                if name == "count" && ty == &CoreType::int()
        )
    }));
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            IrStmtKind::Let { name, ty, .. }
                if name == "label" && ty == &CoreType::string()
        )
    }));
}

#[test]
fn record_let_pattern_missing_field_reports_field_missing() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {count: Int}) -> Int\n",
            "  let {missing: amount} = value\n",
            "  amount\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.field_missing");
    assert_eq!(
        diagnostics[0].message,
        "type `{count: Int}` has no field `missing`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type_source\":\"record_pattern\""));
    assert!(details.contains("\"constraint\":\"record_pattern\""));
}

#[test]
fn omitted_record_let_pattern_binding_requires_concrete_field_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  let {items: items} = {items: []}\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.local_inference_incomplete");
    assert_eq!(
        diagnostics[0].message,
        "omitted local binding `items` has no concrete inferred type"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"inferred_type\":\"Vec<unknown>\"")
    );
}

#[test]
fn constructor_let_patterns_bind_payload_types_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> Int\n",
            "  let Some(count) = value\n",
            "  count + 1\n",
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
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            CoreStmtKind::Let { name, ty, expr }
                if name == "count"
                    && ty == &CoreType::int()
                    && matches!(expr.kind, CoreExprKind::Match { .. })
        )
    }));

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            IrStmtKind::Let { name, ty, value }
                if name == "count"
                    && ty == &CoreType::int()
                    && matches!(value.kind, IrExprKind::Match { .. })
        )
    }));
}

#[test]
fn nested_constructor_let_patterns_bind_payload_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Option<Int>, String>) -> Int\n",
            "  let Ok(Some(count)) = value\n",
            "  count + 1\n",
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
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            CoreStmtKind::Let { name, ty, .. }
                if name == "count" && ty == &CoreType::int()
        )
    }));
}

#[test]
fn constructor_record_pattern_binds_nested_payload_field_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<{count: Int}>) -> Int\n",
            "  let Some({count: count}) = value\n",
            "  count + 1\n",
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
    assert!(main.body.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            CoreStmtKind::Let { name, ty, .. }
                if name == "count" && ty == &CoreType::int()
        )
    }));
}

#[test]
fn constructor_let_pattern_wrong_descriptor_reports_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> Int\n",
            "  let Result::Ok(count) = value\n",
            "  count\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("wrong constructor descriptor should be diagnosed");
    assert_eq!(
        diagnostic.message,
        "expected `Option<Int>`, but found `Result<unknown, unknown>`"
    );
    assert_diagnostic_span(diagnostic, 2, 7, 2, 24);
    assert!(
        diagnostic
            .details
            .to_json()
            .contains("\"constraint\":\"constructor_pattern\"")
    );
}

#[test]
fn match_expression_type_checks_inside_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wrap(value: String) -> String\n",
            "  value\n",
            "end\n",
            "fn describe(value: Option<Int>) -> String\n",
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
fn descriptor_routed_constructor_patterns_bind_payload_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> Int\n",
            "  match value\n",
            "    Option::Some(count) => count + 1\n",
            "    Option::None => 0\n",
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
    assert_eq!(arms[0].expr.ty, CoreType::int());
    assert_eq!(arms[1].expr.ty, CoreType::int());
}

#[test]
fn descriptor_routed_result_pattern_reports_payload_type_mismatch_at_branch_expr() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> Int\n",
            "  match value\n",
            "    Result::Ok(count) => count\n",
            "    Result::Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("result error branch payload mismatch should be diagnosed");
    assert_eq!(diagnostic.message, "expected `Int`, but found `String`");
    assert_diagnostic_span(diagnostic, 4, 27, 4, 32);
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"actual_type\":\"String\""));
    assert!(details.contains("\"constraint\":\"match_arm\""));
}

#[test]
fn match_arm_result_inference_reports_later_branch_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(flag: Bool)\n",
            "  match flag\n",
            "    true => 1\n",
            "    false => \"no\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("later match arm should be checked against the first arm's inferred type");
    assert_eq!(diagnostic.message, "expected `Int`, but found `String`");
    assert_diagnostic_span(diagnostic, 4, 14, 4, 18);
    assert!(
        diagnostic
            .details
            .to_json()
            .contains("\"constraint\":\"match_arm\"")
    );
}

#[test]
fn match_arm_bindings_do_not_leak_into_siblings_or_after_match() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn label(input: Option<Int>) -> String\n",
            "  let selected = match input\n",
            "    Some(payload) => \"some\"\n",
            "    None => payload\n",
            "  end\n",
            "  payload\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let unresolved_payloads = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `payload`"
        })
        .count();
    assert_eq!(unresolved_payloads, 2, "{diagnostics:#?}");
}
