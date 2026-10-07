use super::*;
use veln_diagnostics::JsonValue;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

fn detail<'a>(diagnostic: &'a Diagnostic, name: &str) -> &'a JsonValue {
    let JsonValue::Object(entries) = &diagnostic.details else {
        panic!("diagnostic details must be an object")
    };
    entries
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .unwrap_or_else(|| panic!("missing diagnostic detail `{name}`"))
}

const STATE_DECL: &str = concat!(
    "type State\n",
    "  Ready\n",
    "  Closed\n",
    "  Failed\n",
    "end\n",
);

#[test]
fn direct_refined_validation_does_not_change_other_match_boundaries() {
    let declaration = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
    );
    let ordinary_wrong_arity = diagnostics_for(&format!(
        "{declaration}{}",
        concat!(
            "fn check(value: State) -> ()\n",
            "  match value\n",
            "    Ready(extra) => ()\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert!(ordinary_wrong_arity.is_empty(), "{ordinary_wrong_arity:#?}");

    let direct_wrong_arity = diagnostics_for(&format!(
        "{declaration}{}",
        concat!(
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    Ready(extra) => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        direct_wrong_arity
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.constructor_pattern_arity",
            "type.match_non_exhaustive"
        ],
        "{direct_wrong_arity:#?}"
    );

    let parenthesized_wrong_arity = diagnostics_for(&format!(
        "{declaration}{}",
        concat!(
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match (((value)))\n",
            "    Ready(extra) => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        parenthesized_wrong_arity
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.constructor_pattern_arity",
            "type.match_non_exhaustive"
        ],
        "{parenthesized_wrong_arity:#?}"
    );

    let ordinary_unresolved = diagnostics_for(&format!(
        "{declaration}{}",
        concat!(
            "fn check(value: State) -> ()\n",
            "  match value\n",
            "    missing::Ready => ()\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        ordinary_unresolved
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["type.match_non_exhaustive"],
        "{ordinary_unresolved:#?}"
    );

    let direct_unresolved = diagnostics_for(&format!(
        "{declaration}{}",
        concat!(
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    missing::Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        direct_unresolved
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.unresolved", "type.match_non_exhaustive"],
        "{direct_unresolved:#?}"
    );

    let parenthesized_unresolved = diagnostics_for(&format!(
        "{declaration}{}",
        concat!(
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match (((value)))\n",
            "    missing::Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        parenthesized_unresolved
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.unresolved", "type.match_non_exhaustive"],
        "{parenthesized_unresolved:#?}"
    );
}

#[test]
fn wrong_generic_payload_does_not_consume_refined_coverage_and_keeps_body_checks() {
    let diagnostics = diagnostics_for(concat!(
        "type Boxed<A>\n",
        "  Filled(A)\n",
        "  Empty\n",
        "end\n",
        "fn check(value: Boxed<Int>::Filled | Boxed<Int>::Empty) -> ()\n",
        "  match value\n",
        "    Filled(\"wrong\") => missing_generic_body\n",
        "    Filled(_) => ()\n",
        "    Empty => ()\n",
        "  end\n",
        "end\n",
    ));

    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        ["type.mismatch", "name.unresolved"],
        "{diagnostics:#?}"
    );
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_eq!(
        diagnostics[1].message,
        "unresolved value `missing_generic_body`"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.match_impossible_variant"
            && diagnostic.id != "type.match_redundant_arm"
            && diagnostic.id != "type.match_non_exhaustive"
    }));
}

#[test]
fn inaccessible_heads_report_lookup_failure_without_consuming_refined_coverage() {
    let states = SourceFile::new(
        "states.veln",
        concat!(
            "mod states\n",
            "type Secret\n",
            "  pub HiddenByType\n",
            "end\n",
            "pub type State\n",
            "  Hidden\n",
            "  pub Ready\n",
            "end\n",
        ),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use states\n",
            "fn check(value: State::Ready) -> ()\n",
            "  match value\n",
            "    states::Hidden => ()\n",
            "    states::Secret::HiddenByType => ()\n",
            "    states::Ready => ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let states = lower_surface_ast(&parse(&states).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: states.types,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved constructor `states::Hidden`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved constructor `states::Secret::HiddenByType`"
    }));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.match_impossible_variant"
            && diagnostic.id != "type.match_redundant_arm"
            && diagnostic.id != "type.match_non_exhaustive"
    }));
}

#[test]
fn unreachable_arms_still_check_payloads_bodies_and_expected_results() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready(Int)\n",
        "  Closed(String)\n",
        "end\n",
        "fn accept_ready(value: State::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_closed(value: State::Closed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_string(value: String) -> ()\n",
        "  ()\n",
        "end\n",
        "fn impossible(value: State::Ready) -> State::Ready\n",
        "  match value\n",
        "    Ready(_) => Ready(1)\n",
        "    Closed(reason) => begin\n",
        "      accept_string(reason)\n",
        "      accept_closed(value)\n",
        "      Closed(reason)\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn redundant(value: State::Ready) -> ()\n",
        "  match value\n",
        "    Ready(_) => ()\n",
        "    Ready(_) => accept_ready(value)\n",
        "  end\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.match_impossible_variant")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.match_redundant_arm")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn unreachable_arm_recovery_types_are_observable() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready(Int)\n",
        "  Closed(String)\n",
        "  Failed\n",
        "end\n",
        "fn accept_union(value: State::Ready | State::Closed) -> ()\n  ()\nend\n",
        "fn accept_ready(value: State::Ready) -> ()\n  ()\nend\n",
        "fn accept_closed(value: State::Closed) -> ()\n  ()\nend\n",
        "fn accept_int(value: Int) -> ()\n  ()\nend\n",
        "fn accept_string(value: String) -> ()\n  ()\nend\n",
        "fn duplicate(value: State::Ready | State::Closed) -> ()\n",
        "  match value\n",
        "    Ready(_) => ()\n",
        "    Ready(payload) => begin\n",
        "      accept_ready(value)\n",
        "      accept_int(payload)\n",
        "      accept_closed(value)\n",
        "      accept_string(payload)\n",
        "    end\n",
        "    Closed(_) => ()\n",
        "  end\n",
        "end\n",
        "fn empty_residual(value: State::Ready | State::Closed) -> ()\n",
        "  match value\n",
        "    Ready(_) => ()\n",
        "    Closed(_) => ()\n",
        "    remaining => begin\n",
        "      accept_union(remaining)\n",
        "      accept_union(value)\n",
        "      accept_ready(remaining)\n",
        "      accept_closed(value)\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));

    let variant_mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(variant_mismatches.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        detail(variant_mismatches[0], "actual_type").as_text(),
        Some("State::Ready")
    );
    assert!(
        variant_mismatches[1..].iter().all(|diagnostic| {
            detail(diagnostic, "actual_type").as_text() == Some("State::Ready | State::Closed")
        }),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.message == "expected `String`, but found `Int`"
    }));
}

#[test]
fn nested_refined_matches_restore_each_enclosing_scope() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_closed(value: State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_union(value: State::Ready | State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    Ready => accept_ready(value)\n",
            "    _ => begin\n",
            "      match value\n",
            "        Closed => accept_closed(value)\n",
            "      end\n",
            "      accept_closed(value)\n",
            "    end\n",
            "  end\n",
            "  accept_union(value)\n",
            "  accept_ready(value)\n",
            "end\n",
        )
    ));

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        detail(mismatches[0], "actual_type").as_text(),
        Some("State::Ready | State::Closed")
    );
}

#[test]
fn computed_and_collection_lookup_scrutinees_keep_the_existing_match_behavior() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn make() -> State::Ready\n",
            "  Ready\n",
            "end\n",
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match make()\n",
            "    Ready => ()\n",
            "    _ => ()\n",
            "  end\n",
            "  match ((dict_get({\"state\": value}, \"state\")))\n",
            "    Some(Ready) => accept_ready(value)\n",
            "    Some(Closed) => ()\n",
            "    None => ()\n",
            "  end\n",
            "  match Ready\n",
            "    Ready => ()\n",
            "    _ => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["type.variant_mismatch"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        detail(&diagnostics[0], "actual_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.match_impossible_variant"
                && diagnostic.id != "type.match_redundant_arm"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_constructor_scrutinees_keep_ordinary_match_classification() {
    let states = lower_surface_ast(
        &parse(&SourceFile::new(
            "states.veln",
            concat!(
                "mod states\n",
                "pub type State\n",
                "  pub Ready\n",
                "  pub Closed\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let app = lower_surface_ast(
        &parse(&SourceFile::new(
            "app.veln",
            concat!(
                "mod app\n",
                "use states\n",
                "fn check() -> ()\n",
                "  match ((states::Ready))\n",
                "    states::Ready => ()\n",
                "    states::Closed => ()\n",
                "    _ => ()\n",
                "  end\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: states.types,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.match_impossible_variant"
                && diagnostic.id != "type.match_redundant_arm"
        }),
        "{diagnostics:#?}"
    );
}
