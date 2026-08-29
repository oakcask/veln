use super::*;

#[test]
fn lowers_prefixed_integer_expressions_and_patterns_to_canonical_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn classify(value: Int) -> Int\n",
            "  match value\n",
            "    0x0A => 0b001010\n",
            "    _ => 0xCafe\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let function = &core.functions[0];
    let CoreStmtKind::Return { expr } = &function.body[0].kind else {
        panic!("match should lower as the return expression");
    };
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("return expression should remain a match");
    };
    assert!(matches!(
        &arms[0].pattern.kind,
        CorePatternKind::IntLiteral(value) if value == "10"
    ));
    assert!(matches!(
        &arms[0].expr.kind,
        CoreExprKind::IntLiteral(value) if value == "10"
    ));
    assert!(matches!(
        &arms[1].expr.kind,
        CoreExprKind::IntLiteral(value) if value == "51966"
    ));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let IrStmtKind::Return { value: expr } = &ir.functions[0].body[0].kind else {
        panic!("typed IR should retain the return expression");
    };
    let IrExprKind::Match { arms, .. } = &expr.kind else {
        panic!("typed IR return expression should remain a match");
    };
    assert!(matches!(
        &arms[0].pattern.kind,
        IrPatternKind::IntLiteral(value) if value == "10"
    ));
}

#[test]
fn satisfy_candidate_reports_shadowing_and_unused_predicates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn default_port(max: Int) -> Int\n",
            "  _port satisfy max => true\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_shadow"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy candidate `max` shadows a visible binding"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_unused"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate does not reference candidate `max`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn satisfy_predicate_is_checked_with_candidate_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(limit: Int) -> Int\n",
            "  _value satisfy candidate => candidate > 0 and candidate <= limit\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
}

#[test]
fn satisfy_predicate_reports_non_boolean_candidate_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_type_mismatch"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"actual_type\":\"Int\"")
    }));
}

#[test]
fn satisfy_predicate_reports_unresolved_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate == missing\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.kind == DiagnosticKind::Name
            && diagnostic.message == "unresolved satisfy_predicate `missing`"
    }));
}
