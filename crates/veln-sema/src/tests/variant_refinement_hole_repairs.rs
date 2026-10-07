use super::*;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

fn hole_details(diagnostics: &[Diagnostic], label: &str) -> String {
    diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "hole.unfilled"
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"label\":\"_{label}\""))
        })
        .map(|diagnostic| diagnostic.details.to_json())
        .unwrap_or_else(|| panic!("missing hole `{label}`: {diagnostics:#?}"))
}

#[test]
fn match_alias_refinements_drive_hole_candidates_and_restore_after_the_arm() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "  Outside\n",
        "end\n",
        "fn accept_ready(value: State::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_remaining(value: State::Closed | State::Failed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_state(value: State) -> ()\n",
        "  ()\n",
        "end\n",
        "fn inspect(value: State::Ready | State::Closed | State::Failed) -> ()\n",
        "  let alias: State = value\n",
        "  match value\n",
        "    Ready => begin\n",
        "      accept_ready(_constructor_exact)\n",
        "      accept_state(_constructor_assignable)\n",
        "      accept_ready(_constructor_satisfy satisfy candidate => candidate == candidate)\n",
        "    end\n",
        "    remaining => begin\n",
        "      accept_remaining(_residual_exact)\n",
        "      accept_state(_residual_assignable)\n",
        "      accept_remaining(_residual_satisfy satisfy candidate => candidate == candidate)\n",
        "    end\n",
        "  end\n",
        "  accept_state(_restored)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 7, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "hole.unfilled"),
        "{diagnostics:#?}"
    );

    let constructor = hole_details(&diagnostics, "constructor_exact");
    assert!(constructor.contains(concat!(
        "\"local_bindings\":[{\"name\":\"value\",\"type\":\"State::Ready\"},",
        "{\"name\":\"alias\",\"type\":\"State::Ready\"}]"
    )));
    assert!(constructor.contains("\"query\":\"fn(State::Ready, State::Ready) -> State::Ready\""));
    assert!(constructor.contains(concat!(
        "\"name\":\"alias\",\"type\":\"State::Ready\",\"rank\":1,",
        "\"reason\":\"exact_type_match\""
    )));
    assert!(constructor.contains(concat!(
        "\"name\":\"value\",\"type\":\"State::Ready\",\"rank\":2,",
        "\"reason\":\"exact_type_match\""
    )));
    assert!(constructor.contains(concat!(
        "\"expected_type\":\"State::Ready\",",
        "\"candidate_type\":\"State::Ready\""
    )));

    let constructor_assignable = hole_details(&diagnostics, "constructor_assignable");
    assert!(constructor_assignable.contains(concat!(
        "\"name\":\"alias\",\"type\":\"State::Ready\",\"rank\":1,",
        "\"reason\":\"assignable_type_match\""
    )));

    let constructor_satisfy = hole_details(&diagnostics, "constructor_satisfy");
    assert!(constructor_satisfy.contains("\"repair_status\":\"statically_satisfied\""));
    assert!(constructor_satisfy.contains(concat!(
        "\"name\":\"alias\",\"type\":\"State::Ready\",\"rank\":1,",
        "\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\""
    )));
    assert!(constructor_satisfy.contains("\"satisfy_status\":\"statically_satisfied\""));

    let residual = hole_details(&diagnostics, "residual_exact");
    let residual_type = "State::Closed | State::Failed";
    assert!(residual.contains(&format!(
        "\"local_bindings\":[{{\"name\":\"value\",\"type\":\"{residual_type}\"}},{{\"name\":\"alias\",\"type\":\"{residual_type}\"}},{{\"name\":\"remaining\",\"type\":\"{residual_type}\"}}]"
    )), "{residual}");
    assert!(residual.contains(&format!(
        "\"query\":\"fn({residual_type}, {residual_type}, {residual_type}) -> {residual_type}\""
    )));
    assert!(residual.contains(&format!(
        "\"name\":\"remaining\",\"type\":\"{residual_type}\",\"rank\":1,\"reason\":\"exact_type_match\""
    )));
    assert!(residual.contains(&format!(
        "\"name\":\"alias\",\"type\":\"{residual_type}\",\"rank\":2,\"reason\":\"exact_type_match\""
    )));
    assert!(residual.contains(&format!(
        "\"name\":\"value\",\"type\":\"{residual_type}\",\"rank\":3,\"reason\":\"exact_type_match\""
    )));
    assert!(residual.contains(&format!(
        "\"expected_type\":\"{residual_type}\",\"candidate_type\":\"{residual_type}\""
    )));

    let residual_assignable = hole_details(&diagnostics, "residual_assignable");
    assert!(residual_assignable.contains(&format!(
        "\"name\":\"remaining\",\"type\":\"{residual_type}\",\"rank\":1,\"reason\":\"assignable_type_match\""
    )));

    let residual_satisfy = hole_details(&diagnostics, "residual_satisfy");
    assert!(residual_satisfy.contains("\"repair_status\":\"statically_satisfied\""));
    assert!(residual_satisfy.contains(&format!(
        "\"name\":\"remaining\",\"type\":\"{residual_type}\",\"rank\":1,\"reason\":\"satisfy_tautology\""
    )));

    let restored = hole_details(&diagnostics, "restored");
    assert!(restored.contains(concat!(
        "\"local_bindings\":[{\"name\":\"value\",",
        "\"type\":\"State::Ready | State::Closed | State::Failed\"},",
        "{\"name\":\"alias\",\"type\":\"State\"}]"
    )));
    assert!(restored.contains(
        "\"query\":\"fn(State::Ready | State::Closed | State::Failed, State) -> State\""
    ));
    assert!(restored.contains(concat!(
        "\"name\":\"alias\",\"type\":\"State\",\"rank\":1,",
        "\"reason\":\"exact_type_match\""
    )));
    assert!(restored.contains(concat!(
        "\"name\":\"value\",",
        "\"type\":\"State::Ready | State::Closed | State::Failed\",",
        "\"rank\":2,\"reason\":\"assignable_type_match\""
    )));
}

#[test]
fn hole_candidates_include_only_the_visible_shadowed_binding() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "fn consume(value: Int) -> ()\n",
        "  ()\n",
        "end\n",
        "fn accept_state(value: State) -> ()\n",
        "  ()\n",
        "end\n",
        "fn inspect() -> ()\n",
        "  begin\n",
        "    let value: Int = 1\n",
        "    defer\n",
        "      consume(value)\n",
        "    end\n",
        "    let value: State = Ready\n",
        "    accept_state(_shadowed)\n",
        "  end\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let details = hole_details(&diagnostics, "shadowed");
    assert!(details.contains("\"local_bindings\":[{\"name\":\"value\",\"type\":\"State\"}]"));
    assert!(details.contains("\"query\":\"fn(State) -> State\""));
    assert_eq!(details.matches("\"name\":\"value\"").count(), 2);
    assert!(!details.contains("\"type\":\"Int\""));
}
