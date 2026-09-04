use super::*;

#[test]
fn minimal_list_adt_declaration_type_checks_constructor_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    Nil => 0\n",
            "    Cons(head, _) => head\n",
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
fn minimal_list_adt_qualified_constructors_type_check_and_bind_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    List::Nil => 0\n",
            "    List::Cons(head, tail) => head + length(tail)\n",
            "  end\n",
            "end\n",
            "fn length(value: List<Int>) -> Int\n",
            "  match value\n",
            "    List::Nil => 0\n",
            "    List::Cons(_, tail) => 1 + length(tail)\n",
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
fn minimal_list_adt_constructor_calls_lower_with_declared_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main() -> List<Int>\n",
            "  List::Cons(1, List::Nil)\n",
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
    assert_eq!(expr.ty, CoreType::named("List", vec![CoreType::int()]));
    let CoreExprKind::ListCons { head, tail } = &expr.kind else {
        panic!("List::Cons call should lower to a list constructor");
    };
    assert_eq!(head.ty, CoreType::int());
    assert_eq!(tail.ty, CoreType::named("List", vec![CoreType::int()]));
    assert!(matches!(tail.kind, CoreExprKind::ListNil));
}

#[test]
fn arbitrary_source_adt_constructors_type_check_and_lower() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Maybe<A>\n",
            "  Missing\n",
            "  Just(value: A)\n",
            "end\n",
            "fn empty() -> Maybe<Int>\n",
            "  Maybe::Missing\n",
            "end\n",
            "fn filled() -> Maybe<Int>\n",
            "  Maybe::Just(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let empty = core
        .functions
        .iter()
        .find(|function| function.name == "empty")
        .expect("empty should be lowered");
    let CoreStmtKind::Return { expr } = &empty.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::named("Maybe", vec![CoreType::int()]));
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
        if name == &vec!["Maybe".to_string(), "Missing".to_string()] && payloads.is_empty())
    );

    let filled = core
        .functions
        .iter()
        .find(|function| function.name == "filled")
        .expect("filled should be lowered");
    let CoreStmtKind::Return { expr } = &filled.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::named("Maybe", vec![CoreType::int()]));
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
        if name == &vec!["Maybe".to_string(), "Just".to_string()] && payloads.len() == 1)
    );
}

#[test]
fn record_shaped_source_adt_constructor_calls_lower_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Shape\n",
            "  Point {x: Int, y: Int}\n",
            "  Origin\n",
            "end\n",
            "fn main() -> Shape\n",
            "  Shape::Point(2, 5)\n",
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
    assert_eq!(expr.ty, CoreType::named("Shape", Vec::new()));
    let CoreExprKind::AdtVariant { name, payloads } = &expr.kind else {
        panic!("Shape::Point call should lower to a source ADT variant");
    };
    assert_eq!(name, &vec!["Shape".to_string(), "Point".to_string()]);
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0].ty, CoreType::int());
    assert_eq!(payloads[1].ty, CoreType::int());
}

#[test]
fn nullary_generic_source_adt_constructor_requires_type_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Maybe<A>\n",
            "  Missing\n",
            "  Just(A)\n",
            "end\n",
            "fn main()\n",
            "  let value = Missing\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.inference_ambiguous"
            && diagnostic.message == "constructor `Missing` needs type context"
    }));
}

#[test]
fn duplicate_source_adt_constructor_names_are_rejected_per_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!("type Left\n", "  Same\n", "  Same\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `Same`"
    }));
}

#[test]
fn same_module_constructor_leaf_conflicts_resolve_through_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Same\n",
            "end\n",
            "type Right\n",
            "  Same\n",
            "end\n",
            "fn left() -> Left\n",
            "  Left::Same\n",
            "end\n",
            "fn right() -> Right\n",
            "  Right::Same\n",
            "end\n",
            "fn ambiguous() -> Left\n",
            "  Same\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.ambiguous" && diagnostic.message == "ambiguous value `Same`"
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `Same`"
    }));
}

#[test]
fn same_module_payload_constructor_leaf_conflicts_resolve_through_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Build(value: Int)\n",
            "end\n",
            "type Right\n",
            "  Build(value: String)\n",
            "end\n",
            "fn left() -> Left\n",
            "  Left::Build(1)\n",
            "end\n",
            "fn right() -> Right\n",
            "  Right::Build(\"ok\")\n",
            "end\n",
            "fn ambiguous() -> Left\n",
            "  Build(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.ambiguous" && diagnostic.message == "ambiguous call_target `Build`"
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `Build`"
    }));
}

#[test]
fn ambiguous_unqualified_imported_source_adt_constructor_is_rejected() {
    let first = SourceFile::new(
        "first.veln",
        concat!("mod first\n", "pub type Left\n", "  pub Same\n", "end\n",),
    );
    let second = SourceFile::new(
        "second.veln",
        concat!("mod second\n", "pub type Right\n", "  pub Same\n", "end\n",),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use first\n",
            "use second\n",
            "fn main() -> Left\n",
            "  Same\n",
            "end\n",
        ),
    );
    let first = lower_surface_ast(&parse(&first).tree);
    let second = lower_surface_ast(&parse(&second).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: first.types.into_iter().chain(second.types).collect(),
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.ambiguous" && diagnostic.message == "ambiguous value `Same`"
    }));
}

#[test]
fn ambiguous_constructor_patterns_do_not_infer_known_scrutinee_domain() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Same\n",
            "end\n",
            "type Right\n",
            "  Same\n",
            "end\n",
            "fn label(value: Left) -> Int\n",
            "  match value\n",
            "    Same => 1\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.inference_ambiguous"
            && diagnostic.message == "match scrutinee type is ambiguous"
    }));
}

#[test]
fn ambiguous_constructor_patterns_report_unknown_scrutinee_domain() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Same(Int)\n",
            "end\n",
            "type Right\n",
            "  Same(String)\n",
            "end\n",
            "fn label(value) -> Int\n",
            "  match value\n",
            "    Same(_) => 1\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "type.inference_ambiguous"
                && diagnostic.message == "match scrutinee type is ambiguous"
        })
        .expect("unknown scrutinee should retain the ambiguous constructor domains");
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"constraint\":\"match_constructor_pattern_domain\""));
    assert!(details.contains("\"left\""));
    assert!(details.contains("\"right\""));
}
