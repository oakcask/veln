use super::*;

#[test]
fn unknown_wrong_kind_and_wrong_owner_finals_use_the_focused_diagnostic() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "type Marker\n",
        "  Marked\n",
        "end\n",
        "type Other\n",
        "  OtherOnly\n",
        "end\n",
        "fn unknown(value: State::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn wrong_kind(value: State::Marker) -> ()\n",
        "  ()\n",
        "end\n",
        "fn wrong_owner(value: State::OtherOnly) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.variant_refinement_unknown"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.details.to_json())
            .collect::<Vec<_>>(),
        [
            "{\"base_type\":\"State\",\"variant\":\"Missing\"}",
            "{\"base_type\":\"State\",\"variant\":\"Marker\"}",
            "{\"base_type\":\"State\",\"variant\":\"OtherOnly\"}",
        ]
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().unwrap().start.column)
            .collect::<Vec<_>>(),
        [26, 29, 30]
    );
    for diagnostic in &diagnostics {
        let related = diagnostic
            .related
            .iter()
            .map(|value| value.to_json())
            .collect::<Vec<_>>();
        assert_eq!(related.len(), 3, "{related:#?}");
        assert!(related[0].contains("\"kind\":\"base_type_declaration\""));
        assert!(related[1].contains("\"variant\":\"Ready\""));
        assert!(related[2].contains("\"variant\":\"Closed\""));
    }
}

#[test]
fn unknown_final_retains_only_the_base_for_independent_union_comparison() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Other\n",
        "  OtherOnly\n",
        "end\n",
        "fn rejected(value: State::Missing | Other::OtherOnly) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["type.variant_refinement_unknown", "type.variant_union_base"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[1].details.to_json(),
        "{\"expected_base_type\":\"State\",\"actual_base_type\":\"Other\"}"
    );
}

#[test]
fn unknown_final_precedes_union_base_failure_for_the_same_alternative() {
    let diagnostics = diagnostics_for(concat!(
        "type Left\n",
        "  A\n",
        "end\n",
        "type Right\n",
        "  B\n",
        "end\n",
        "fn rejected(value: Left::A | Right::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["type.variant_refinement_unknown", "type.variant_union_base"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().expect("diagnostic has a span");
                (span.start.column, span.end.column)
            })
            .collect::<Vec<_>>(),
        [(37, 44), (30, 44)],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.details.to_json())
            .collect::<Vec<_>>(),
        [
            "{\"base_type\":\"Right\",\"variant\":\"Missing\"}",
            "{\"expected_base_type\":\"Left\",\"actual_base_type\":\"Right\"}",
        ]
    );
}

#[test]
fn casing_recovered_unknown_final_keeps_visible_variant_context() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "fn rejected(value: state::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let unknown = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.variant_refinement_unknown")
        .unwrap_or_else(|| panic!("{diagnostics:#?}"));
    assert!(unknown.related.iter().any(|entry| {
        let json = entry.to_json();
        json.contains("\"kind\":\"visible_variant_declaration\"")
            && json.contains("\"variant\":\"Ready\"")
    }));
}

#[test]
fn type_alias_unknown_final_keeps_visible_variant_context() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "pub type Alias = State\n",
        "fn rejected(value: Alias::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_unknown");
    assert!(diagnostics[0].related.iter().any(|entry| {
        let json = entry.to_json();
        json.contains("\"kind\":\"visible_variant_declaration\"")
            && json.contains("\"variant\":\"Ready\"")
    }));
}

#[test]
fn source_less_unknown_final_keeps_ordered_spanless_variant_context() {
    let diagnostics = diagnostics_for(concat!(
        "fn rejected(value: Option<Int>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let related = diagnostics[0]
        .related
        .iter()
        .map(|entry| entry.to_json())
        .collect::<Vec<_>>();
    assert_eq!(related.len(), 3, "{related:#?}");
    assert!(related[0].contains("\"kind\":\"base_type_declaration\""));
    assert!(related[1].contains("\"variant\":\"Some\""));
    assert!(related[2].contains("\"variant\":\"None\""));
    assert!(related.iter().all(|entry| !entry.contains("\"span\"")));
}

#[test]
fn unknown_final_quarantines_dependent_failures_and_invalid_children_suppress_outer_finals() {
    let diagnostics = diagnostics_for(concat!(
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
        "type Box<A>\n",
        "  Boxed(A)\n",
        "  Empty\n",
        "end\n",
        "fn needs_string(value: String) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn rejected(value: State::Missing) -> State::Missing\n",
        "  needs_string(value)\n",
        "  value\n",
        "end\n",
        "fn nested(value: Box<State::Missing>::Absent) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_unknown",
            "type.variant_refinement_unknown",
            "type.variant_refinement_unknown",
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.variant_refinement_private"
            && diagnostic.id != "type.invalid_annotation"
            && diagnostic.id != "type.mismatch"
    }));
    assert_eq!(
        diagnostics[2].details.to_json(),
        "{\"base_type\":\"State\",\"variant\":\"Missing\"}"
    );
}

#[test]
fn unknown_final_related_variants_follow_constructor_visibility() {
    let target = lower_surface_ast(
        &parse(&SourceFile::new(
            "math.veln",
            concat!(
                "mod math\n",
                "type State\n",
                "  Hidden\n",
                "  pub Ready\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let companion = lower_surface_ast(
        &parse(&SourceFile::new(
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "use math\n",
                "fn inspect(value: math::State::Missing) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let module = veln_ast::SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: target.types,
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_unknown");
    let related = diagnostics[0]
        .related
        .iter()
        .map(|value| value.to_json())
        .collect::<Vec<_>>();
    assert_eq!(related.len(), 3, "{related:#?}");
    assert!(related[1].contains("\"variant\":\"Hidden\""));
    assert!(related[2].contains("\"variant\":\"Ready\""));

    let foreign = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            concat!(
                "mod consumer\n",
                "use math\n",
                "fn inspect(value: math::State::Missing) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let target = lower_surface_ast(
        &parse(&SourceFile::new(
            "math.veln",
            concat!(
                "mod math\n",
                "pub type State\n",
                "  Hidden\n",
                "  pub Ready\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let module = veln_ast::SurfaceModule {
        module: foreign.module,
        uses: foreign.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: target.types,
        functions: foreign.functions,
        invalid_names: Vec::new(),
    };
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let related = diagnostics[0]
        .related
        .iter()
        .map(|value| value.to_json())
        .collect::<Vec<_>>();
    assert_eq!(related.len(), 2, "{related:#?}");
    assert!(related[1].contains("\"variant\":\"Ready\""));
    assert!(!related.iter().any(|entry| entry.contains("Hidden")));
}
