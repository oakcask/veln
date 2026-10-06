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
fn refined_domains_drive_constructor_and_residual_catch_all_refinement() {
    let diagnostics = diagnostics_for(concat!(
        "type Boxed<A>\n",
        "  Filled(A)\n",
        "  Empty\n",
        "  Failed\n",
        "end\n",
        "fn accept_filled(value: Boxed<Int>::Filled) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_empty(value: Boxed<Int>::Empty) -> ()\n",
        "  ()\n",
        "end\n",
        "fn constructor_then_binding(value: Boxed<Int>::Filled | Boxed<Int>::Empty) -> ()\n",
        "  match value\n",
        "    Filled(_) => accept_filled(value)\n",
        "    remaining => begin\n",
        "      accept_empty(remaining)\n",
        "      accept_empty(value)\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn constructor_then_wildcard(value: Boxed<Int>::Filled | Boxed<Int>::Empty) -> ()\n",
        "  match value\n",
        "    Filled(_) => accept_filled(value)\n",
        "    _ => accept_empty(value)\n",
        "  end\n",
        "end\n",
        "fn singleton(value: Boxed<Int>::Empty) -> ()\n",
        "  match value\n",
        "    Empty => accept_empty(value)\n",
        "  end\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn refined_domain_classification_is_deterministic_and_preserves_related_arms() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn classify(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    Failed => ()\n",
            "    Ready => ()\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "    _ => ()\n",
            "    remaining => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    let classified = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "type.match_impossible_variant" | "type.match_redundant_arm"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(classified.len(), 5, "{diagnostics:#?}");
    assert_eq!(classified[0].id, "type.match_impossible_variant");
    assert_eq!(
        detail(classified[0], "scrutinee_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert_eq!(
        detail(classified[0], "arm_variant").as_text(),
        Some("Failed")
    );

    assert_eq!(classified[1].id, "type.match_redundant_arm");
    assert_eq!(
        detail(classified[1], "reason").as_text(),
        Some("duplicate_variant")
    );
    assert_eq!(
        detail(classified[1], "arm_variant").as_text(),
        Some("Ready")
    );
    assert_eq!(classified[1].related.len(), 1);

    assert_eq!(
        detail(classified[2], "reason").as_text(),
        Some("complete_prior_coverage")
    );
    assert_eq!(detail(classified[2], "arm_variant"), &JsonValue::Null);
    assert_eq!(classified[2].related.len(), 2);

    assert_eq!(
        detail(classified[3], "reason").as_text(),
        Some("complete_prior_coverage")
    );
    assert_eq!(classified[3].related.len(), 2);

    assert_eq!(classified[4].id, "type.match_impossible_variant");
}

#[test]
fn preceding_catch_all_reason_wins_for_later_constructor_and_catch_all() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn classify(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    _ => ()\n",
            "    Ready => ()\n",
            "    remaining => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    let redundant = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.match_redundant_arm")
        .collect::<Vec<_>>();
    assert_eq!(redundant.len(), 2, "{diagnostics:#?}");
    assert!(redundant.iter().all(|diagnostic| {
        detail(diagnostic, "reason").as_text() == Some("preceding_catch_all")
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn invalid_heads_keep_intrinsic_diagnostics_and_do_not_consume_coverage() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Other\n",
            "  Foreign\n",
            "end\n",
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    State::ready => ()\n",
            "    Ready(_) => ()\n",
            "    Other::Foreign => ()\n",
            "    missing::Missing => ()\n",
            "    missing::Ready => ()\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    let invalid_case = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.invalid_case")
        .expect("invalid-cased arm should retain its intrinsic diagnostic");
    assert_eq!(
        invalid_case.message,
        "constructor name `ready` must start with an ASCII uppercase letter"
    );

    let wrong_arity = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.constructor_pattern_arity")
        .expect("wrong-arity arm should retain its intrinsic diagnostic");
    assert_eq!(
        wrong_arity.message,
        "constructor pattern expects 0 payload(s), but got 1"
    );
    assert_eq!(
        wrong_arity.details,
        JsonValue::object([
            ("constructor", JsonValue::string("State::Ready")),
            ("expected_payload_count", JsonValue::Number(0)),
            ("actual_payload_count", JsonValue::Number(1)),
        ])
    );
    assert!(wrong_arity.related.is_empty());
    let wrong_arity_span = wrong_arity.span.as_ref().expect("wrong-arity span");
    assert_eq!(
        (
            wrong_arity_span.start.line,
            wrong_arity_span.start.column,
            wrong_arity_span.end.line,
            wrong_arity_span.end.column,
        ),
        (12, 5, 12, 13)
    );

    let wrong_adt = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 13)
        })
        .expect("wrong-ADT arm should retain its intrinsic diagnostic");
    assert_eq!(
        wrong_adt.message,
        "expected `State::Ready | State::Closed`, but found `Other`"
    );

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved,
        [
            "unresolved constructor `missing::Missing`",
            "unresolved constructor `missing::Ready`",
        ]
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved constructor `missing::Ready`"
    }));
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.match_impossible_variant"
                && diagnostic.id != "type.match_redundant_arm"
                && diagnostic.id != "type.match_non_exhaustive"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn wrong_arity_does_not_consume_refined_coverage_and_keeps_payload_and_body_checks() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    Ready(BadBinding) => missing_body\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "name.invalid_case",
            "type.constructor_pattern_arity",
            "name.unresolved",
            "type.match_non_exhaustive",
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[0].message,
        "binding name `BadBinding` must start with an ASCII lowercase letter"
    );
    assert_eq!(diagnostics[2].message, "unresolved value `missing_body`");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.match_impossible_variant"
            && diagnostic.id != "type.match_redundant_arm"
    }));
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
            "end\n",
        )
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn non_bare_refined_scrutinees_keep_the_existing_match_behavior() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn make() -> State::Ready\n",
            "  Ready\n",
            "end\n",
            "fn check(value: State::Ready) -> ()\n",
            "  match (value)\n",
            "    Failed => ()\n",
            "    _ => ()\n",
            "  end\n",
            "  match make()\n",
            "    Ready => ()\n",
            "    _ => ()\n",
            "  end\n",
            "  match Ready\n",
            "    Ready => ()\n",
            "    _ => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.match_impossible_variant"
                && diagnostic.id != "type.match_redundant_arm"
        }),
        "{diagnostics:#?}"
    );
}
