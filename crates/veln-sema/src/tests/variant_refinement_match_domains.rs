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
        "fn refined_local(value: Boxed<Int>::Filled | Boxed<Int>::Empty) -> ()\n",
        "  let current: Boxed<Int>::Filled | Boxed<Int>::Empty = value\n",
        "  match current\n",
        "    Filled(_) => accept_filled(current)\n",
        "    remaining => begin\n",
        "      accept_empty(remaining)\n",
        "      accept_empty(current)\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn parenthesized_bindings_refine_base_and_finite_domains_and_restore_scope() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "  Excluded\n",
        "end\n",
        "fn accept_ready(value: State::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_closed(value: State::Closed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_failed(value: State::Failed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn base(value: State) -> ()\n",
        "  match (value)\n",
        "    Ready => accept_ready(value)\n",
        "    Closed => accept_closed(value)\n",
        "    Failed => accept_failed(value)\n",
        "    Excluded => ()\n",
        "  end\n",
        "end\n",
        "fn refined(value: State::Ready | State::Closed | State::Failed) -> ()\n",
        "  match (((value)))\n",
        "    Ready => accept_ready(value)\n",
        "    remaining => begin\n",
        "      match ((remaining))\n",
        "        Closed => accept_closed(remaining)\n",
        "        Failed => accept_failed(remaining)\n",
        "      end\n",
        "      match (value)\n",
        "        Closed => accept_closed(value)\n",
        "        Failed => accept_failed(value)\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "  match (value)\n",
        "    Ready => accept_ready(value)\n",
        "    Closed => accept_closed(value)\n",
        "    Failed => accept_failed(value)\n",
        "  end\n",
        "end\n",
        "fn wildcard(value: State::Ready | State::Closed) -> ()\n",
        "  match ((value))\n",
        "    Ready => accept_ready(value)\n",
        "    _ => accept_closed(value)\n",
        "  end\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn catch_alls_preserve_the_complete_multi_variant_residual() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "  Excluded\n",
        "end\n",
        "fn binding(value: State::Ready | State::Closed | State::Failed) -> ()\n",
        "  match value\n",
        "    Ready => ()\n",
        "    remaining => begin\n",
        "      match remaining\n",
        "        Closed => ()\n",
        "        Failed => ()\n",
        "      end\n",
        "      match value\n",
        "        Closed => ()\n",
        "        Failed => ()\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn wildcard(value: State::Ready | State::Closed | State::Failed) -> ()\n",
        "  match value\n",
        "    Ready => ()\n",
        "    _ => match value\n",
        "      Closed => ()\n",
        "      Failed => ()\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn transparent_alias_chains_and_complete_pattern_bindings_share_arm_refinements() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "  Excluded\n",
        "end\n",
        "fn accept_ready(value: State::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_closed(value: State::Closed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_failed(value: State::Failed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_remaining(value: State::Closed | State::Failed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn inferred_aliases(value: State) -> ()\n",
        "  let direct = value\n",
        "  let transitive = direct\n",
        "  match transitive\n",
        "    Ready => begin\n",
        "      accept_ready(value)\n",
        "      accept_ready(direct)\n",
        "      accept_ready(transitive)\n",
        "    end\n",
        "    Closed => ()\n",
        "    Failed => ()\n",
        "    Excluded => ()\n",
        "  end\n",
        "end\n",
        "fn residual_aliases(value: State::Ready | State::Closed | State::Failed) -> ()\n",
        "  let direct: State::Ready | State::Closed | State::Failed = value\n",
        "  let annotated: State::Ready | State::Closed | State::Failed = direct\n",
        "  let transitive: State::Ready | State::Closed | State::Failed = ((annotated))\n",
        "  match (((transitive)))\n",
        "    Ready => begin\n",
        "      accept_ready(value)\n",
        "      accept_ready(direct)\n",
        "      accept_ready(annotated)\n",
        "      accept_ready(transitive)\n",
        "    end\n",
        "    remaining => begin\n",
        "      accept_remaining(value)\n",
        "      accept_remaining(direct)\n",
        "      accept_remaining(annotated)\n",
        "      accept_remaining(transitive)\n",
        "      accept_remaining(remaining)\n",
        "      match remaining\n",
        "        Closed => begin\n",
        "          accept_closed(value)\n",
        "          accept_closed(direct)\n",
        "          accept_closed(annotated)\n",
        "          accept_closed(transitive)\n",
        "          accept_closed(remaining)\n",
        "        end\n",
        "        Failed => begin\n",
        "          accept_failed(value)\n",
        "          accept_failed(direct)\n",
        "          accept_failed(annotated)\n",
        "          accept_failed(transitive)\n",
        "          accept_failed(remaining)\n",
        "        end\n",
        "      end\n",
        "      accept_remaining(value)\n",
        "      accept_remaining(remaining)\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn residual_alias_wildcard(value: State::Ready | State::Closed | State::Failed) -> ()\n",
        "  let direct = value\n",
        "  let transitive = direct\n",
        "  match transitive\n",
        "    Ready => accept_ready(value)\n",
        "    _ => begin\n",
        "      accept_remaining(value)\n",
        "      accept_remaining(direct)\n",
        "      accept_remaining(transitive)\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn inferred_alias_group_type_is_visible_to_every_member_and_restores_after_match() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "fn inferred(value) -> ()\n",
        "  let direct = value\n",
        "  let transitive = direct\n",
        "  match transitive\n",
        "    Ready => ()\n",
        "    Closed => ()\n",
        "    Failed => ()\n",
        "  end\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn inferred_nested_alias_arguments_share_refinements_and_restore_presentations() {
    let diagnostics = diagnostics_for(concat!(
        "type Box<A>\n",
        "  Boxed(A)\n",
        "  Empty\n",
        "end\n",
        "fn accept_boxed(value: Box<Vec<Int>>::Boxed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_base(value: Box<Vec<Int>>) -> ()\n",
        "  ()\n",
        "end\n",
        "fn inspect(source) -> ()\n",
        "  let direct = source\n",
        "  let transitive = direct\n",
        "  let annotated: Box<Vec<Int>> = transitive\n",
        "  match annotated\n",
        "    complete => begin\n",
        "      match complete\n",
        "        Boxed(_) => begin\n",
        "          accept_boxed(source)\n",
        "          accept_boxed(direct)\n",
        "          accept_boxed(transitive)\n",
        "          accept_boxed(annotated)\n",
        "          accept_boxed(complete)\n",
        "        end\n",
        "      end\n",
        "      accept_boxed(source)\n",
        "      accept_boxed(direct)\n",
        "      accept_boxed(transitive)\n",
        "      accept_boxed(annotated)\n",
        "      accept_boxed(complete)\n",
        "      accept_base(annotated)\n",
        "      accept_base(complete)\n",
        "    end\n",
        "  end\n",
        "  accept_boxed(source)\n",
        "  accept_boxed(direct)\n",
        "  accept_boxed(transitive)\n",
        "  accept_base(annotated)\n",
        "  accept_boxed(annotated)\n",
        "end\n",
        "fn infer_nested_argument() -> ()\n",
        "  inspect(Boxed([]))\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_mismatch");
    assert_eq!(
        detail(&diagnostics[0], "actual_type").as_text(),
        Some("Box<Vec<Int>>")
    );
    assert_eq!(
        detail(&diagnostics[0], "expected_type").as_text(),
        Some("Box<Vec<Int>>::Boxed")
    );
}

#[test]
fn widened_aliases_share_the_source_feasible_domain() {
    let source = [
        STATE_DECL,
        concat!(
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_closed(value: State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn singleton_source(value: State::Ready) -> ()\n",
            "  let widened: State = value\n",
            "  match widened\n",
            "    Ready => begin\n",
            "      accept_ready(value)\n",
            "      accept_ready(widened)\n",
            "    end\n",
            "  end\n",
            "end\n",
            "fn finite_source(value: State::Ready | State::Closed) -> ()\n",
            "  let wider_set: State::Ready | State::Closed | State::Failed = value\n",
            "  let base: State = wider_set\n",
            "  match base\n",
            "    Ready => begin\n",
            "      accept_ready(value)\n",
            "      accept_ready(wider_set)\n",
            "      accept_ready(base)\n",
            "    end\n",
            "    remaining => begin\n",
            "      accept_closed(value)\n",
            "      accept_closed(wider_set)\n",
            "      accept_closed(base)\n",
            "      accept_closed(remaining)\n",
            "      let nested: State = remaining\n",
            "      match nested\n",
            "        Closed => begin\n",
            "          accept_closed(value)\n",
            "          accept_closed(base)\n",
            "          accept_closed(remaining)\n",
            "          accept_closed(nested)\n",
            "        end\n",
            "      end\n",
            "      accept_closed(remaining)\n",
            "    end\n",
            "  end\n",
            "  match value\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "  match wider_set\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "  match base\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        ),
    ]
    .concat();
    let diagnostics = diagnostics_for(&source);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn widened_alias_impossible_arm_uses_the_shared_domain_and_recovers() {
    let source = [
        STATE_DECL,
        concat!(
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn classify(value: State::Ready) -> ()\n",
            "  let widened: State = value\n",
            "  match widened\n",
            "    Closed => begin\n",
            "      accept_ready(value)\n",
            "      accept_ready(widened)\n",
            "    end\n",
            "    Ready => ()\n",
            "  end\n",
            "end\n",
        ),
    ]
    .concat();
    let diagnostics = diagnostics_for(&source);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.match_impossible_variant");
    assert_eq!(
        detail(&diagnostics[0], "scrutinee_type").as_text(),
        Some("State::Ready")
    );
    assert_eq!(
        detail(&diagnostics[0], "arm_variant").as_text(),
        Some("Closed")
    );
}

#[test]
fn transparent_alias_refinements_restore_and_computed_values_stay_independent() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "fn accept_ready(value: State::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn identity(value: State) -> State\n",
        "  value\n",
        "end\n",
        "fn is_ready(value: State) -> Bool\n",
        "  value == Ready\n",
        "end\n",
        "fn restoration(value: State::Ready | State::Closed) -> ()\n",
        "  let alias = value\n",
        "  match alias\n",
        "    Ready => accept_ready(value)\n",
        "    Closed => ()\n",
        "  end\n",
        "  accept_ready(value)\n",
        "  accept_ready(alias)\n",
        "end\n",
        "fn widened_restoration(value: State::Ready | State::Closed) -> ()\n",
        "  let alias: State = value\n",
        "  match alias\n",
        "    Ready => accept_ready(value)\n",
        "    Closed => ()\n",
        "  end\n",
        "  accept_ready(value)\n",
        "  accept_ready(alias)\n",
        "end\n",
        "fn constructed(source: State) -> ()\n",
        "  let candidate = Ready\n",
        "  match candidate\n",
        "    Ready => begin\n",
        "      accept_ready(candidate)\n",
        "      accept_ready(source)\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn called(source: State) -> ()\n",
        "  let candidate = identity(source)\n",
        "  match candidate\n",
        "    Ready => begin\n",
        "      accept_ready(candidate)\n",
        "      accept_ready(source)\n",
        "    end\n",
        "    Closed => ()\n",
        "    Failed => ()\n",
        "  end\n",
        "end\n",
        "fn field_access(source: State) -> ()\n",
        "  let holder = { value: source }\n",
        "  let candidate = holder.value\n",
        "  match candidate\n",
        "    Ready => begin\n",
        "      accept_ready(candidate)\n",
        "      accept_ready(source)\n",
        "    end\n",
        "    Closed => ()\n",
        "    Failed => ()\n",
        "  end\n",
        "end\n",
        "fn pipeline_operator(source: State) -> ()\n",
        "  let candidate = source |> identity()\n",
        "  match candidate\n",
        "    Ready => begin\n",
        "      accept_ready(candidate)\n",
        "      accept_ready(source)\n",
        "    end\n",
        "    Closed => ()\n",
        "    Failed => ()\n",
        "  end\n",
        "end\n",
        "fn equality(source: State) -> ()\n",
        "  if source == Ready\n",
        "    accept_ready(source)\n",
        "  else\n",
        "    ()\n",
        "  end\n",
        "end\n",
        "fn boolean_helper(source: State) -> ()\n",
        "  if is_ready(source)\n",
        "    accept_ready(source)\n",
        "  else\n",
        "    ()\n",
        "  end\n",
        "end\n",
        "fn conditional(source: State) -> ()\n",
        "  let candidate = if true\n",
        "    source\n",
        "  else\n",
        "    source\n",
        "  end\n",
        "  match candidate\n",
        "    Ready => begin\n",
        "      accept_ready(candidate)\n",
        "      accept_ready(source)\n",
        "    end\n",
        "    Closed => ()\n",
        "    Failed => ()\n",
        "  end\n",
        "end\n",
        "fn contracted(source: State) -> ()\n",
        "require source == Ready\n",
        "  accept_ready(source)\n",
        "end\n",
    ));

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 12, "{diagnostics:#?}");
    assert_eq!(
        detail(mismatches[0], "actual_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert_eq!(
        detail(mismatches[1], "actual_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert_eq!(
        detail(mismatches[2], "actual_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert_eq!(
        detail(mismatches[3], "actual_type").as_text(),
        Some("State")
    );
    assert!(
        mismatches[4..]
            .iter()
            .all(|diagnostic| detail(diagnostic, "actual_type").as_text() == Some("State")),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_alias_annotations_do_not_share_arm_refinements() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "fn accept_ready(value: State::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn rejected_narrowing(source: State) -> ()\n",
        "  let alias: State::Ready = source\n",
        "  match alias\n",
        "    Ready => accept_ready(source)\n",
        "  end\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.variant_mismatch"),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        detail(diagnostic, "actual_type").as_text() == Some("State")
            && detail(diagnostic, "expected_type").as_text() == Some("State::Ready")
    }));
}

#[test]
fn refined_domain_classification_is_deterministic_and_preserves_related_arms() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn classify(value: State::Ready | State::Closed) -> ()\n",
            "  match ((value))\n",
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
        Some("preceding_catch_all")
    );
    assert_eq!(classified[3].related.len(), 1);

    assert_eq!(classified[4].id, "type.match_impossible_variant");
}

#[test]
fn first_valid_catch_all_controls_later_redundancy_after_constructor_coverage() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn classify(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "    _ => ()\n",
            "    remaining => ()\n",
            "    Ready => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    let redundant = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.match_redundant_arm")
        .collect::<Vec<_>>();
    assert_eq!(redundant.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        detail(redundant[0], "reason").as_text(),
        Some("complete_prior_coverage")
    );
    assert_eq!(redundant[0].related.len(), 2);
    for diagnostic in &redundant[1..] {
        assert_eq!(
            detail(diagnostic, "reason").as_text(),
            Some("preceding_catch_all")
        );
        assert_eq!(diagnostic.related.len(), 1);
        let span = diagnostic.related[0]
            .as_object()
            .expect("related context object")
            .iter()
            .find_map(|(field, value)| (field == "span").then_some(value))
            .and_then(JsonValue::as_object)
            .expect("related span object");
        let start = span
            .iter()
            .find_map(|(field, value)| (field == "start").then_some(value))
            .and_then(JsonValue::as_object)
            .expect("related start object");
        assert_eq!(
            start
                .iter()
                .find_map(|(field, value)| (field == "line").then_some(value)),
            Some(&JsonValue::Number(10))
        );
    }
}

#[test]
fn invalid_catch_all_head_does_not_change_redundancy_precedence() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn classify(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "    BadBinding => ()\n",
            "    Ready => ()\n",
            "    _ => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message
                == "binding name `BadBinding` must start with an ASCII lowercase letter"
    }));
    let redundant = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.match_redundant_arm")
        .collect::<Vec<_>>();
    assert_eq!(redundant.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        detail(redundant[0], "reason").as_text(),
        Some("duplicate_variant")
    );
    assert_eq!(
        detail(redundant[1], "reason").as_text(),
        Some("complete_prior_coverage")
    );
    assert_eq!(
        detail(redundant[2], "reason").as_text(),
        Some("preceding_catch_all")
    );
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
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved constructor `missing::Ready`"
        }),
        "{diagnostics:#?}"
    );
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
fn invalid_casing_recovery_distinguishes_ordinary_and_refined_coverage() {
    let ordinary = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn identity(value: State::Ready | State::Closed) -> State::Ready | State::Closed\n",
            "  value\n",
            "end\n",
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match identity(value)\n",
            "    State::ready => ()\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        ordinary
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case"],
        "{ordinary:#?}"
    );

    let refined = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn check(value: State::Ready | State::Closed) -> ()\n",
            "  match value\n",
            "    State::ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "end\n",
        )
    ));
    assert_eq!(
        refined
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "type.match_non_exhaustive"],
        "{refined:#?}"
    );
    assert_eq!(
        refined[1].message, "match is missing case Ready",
        "{refined:#?}"
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
fn unresolved_head_does_not_consume_coverage_and_keeps_recovery_checks() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready(Int)\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "fn check(value: State::Ready | State::Closed) -> State::Ready\n",
        "  match value\n",
        "    missing::Ready(BadBinding) => begin\n",
        "      missing_body\n",
        "      Closed\n",
        "    end\n",
        "    Closed => Ready(1)\n",
        "  end\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved constructor `missing::Ready`"
        }),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message
                == "binding name `BadBinding` must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved value `missing_body`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.variant_mismatch"
            && diagnostic.message
                == "value of type `State::Closed` is not assignable to variant type `State::Ready`"
    }));
    let non_exhaustive = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "type.match_non_exhaustive"
                && diagnostic.message == "match is missing case Ready(_)"
        })
        .unwrap_or_else(|| panic!("missing refined exhaustiveness diagnostic: {diagnostics:#?}"));
    assert_eq!(
        detail(non_exhaustive, "scrutinee_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.match_impossible_variant"
            && diagnostic.id != "type.match_redundant_arm"
    }));
}
