use super::*;

#[test]
fn union_visibility_reports_distinct_paths_in_written_order() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  First\n",
        "  pub Public\n",
        "  Second\n",
        "end\n",
        "pub fn expose(value: State::Second | State::First | State::Second) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let variants = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .map(|json| {
            if json.contains("\"variant\":\"Second\"") {
                "Second"
            } else if json.contains("\"variant\":\"First\"") {
                "First"
            } else {
                panic!("missing variant detail: {json}")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(variants, ["Second", "First"]);
}

#[test]
fn union_visibility_keeps_distinct_generic_arguments_and_removes_exact_repeats() {
    let source = concat!(
        "pub type Arg\n",
        "  pub Value\n",
        "end\n",
        "pub type Alias = Arg\n",
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "end\n",
        "pub fn expose(value: State<Alias>::Hidden | State<Arg>::Hidden | State<String>::Hidden | State<Alias>::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 2, "{diagnostics:#?}");
    let hidden_starts = source
        .match_indices("Hidden")
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>();
    assert_eq!(
        visibility
            .iter()
            .map(|diagnostic| diagnostic
                .span
                .as_ref()
                .expect("visibility span")
                .start
                .offset)
            .collect::<Vec<_>>(),
        [hidden_starts[1], hidden_starts[3]],
        "{diagnostics:#?}"
    );
    for (diagnostic, written_type) in visibility
        .iter()
        .zip(["State<Alias>::Hidden", "State<String>::Hidden"])
    {
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(
            json.contains(&format!("\"written_type\":\"{written_type}\"")),
            "{json}"
        );
        assert!(json.contains("\"base_type\":\"State\""), "{json}");
        assert!(json.contains("\"variant\":\"Hidden\""), "{json}");
    }

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
}

#[test]
fn union_visibility_preserves_distinct_unresolved_generic_arguments() {
    let source = concat!(
        "pub type Box<A>\n",
        "  pub Boxed(A)\n",
        "end\n",
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "end\n",
        "pub fn expose(value: State<missing::A>::Hidden | State<other::B>::Hidden | State<missing::A>::Hidden | State<Box<missing::C>>::Hidden | State<Box<other::D>>::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 4, "{diagnostics:#?}");
    let expected = [
        "State<missing::A>::Hidden",
        "State<other::B>::Hidden",
        "State<Box<missing::C>>::Hidden",
        "State<Box<other::D>>::Hidden",
    ];
    for (diagnostic, written_type) in visibility.iter().zip(expected) {
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(
            json.contains(&format!("\"written_type\":\"{written_type}\"")),
            "{json}"
        );
    }
    assert_eq!(
        visibility
            .iter()
            .map(|diagnostic| diagnostic
                .span
                .as_ref()
                .expect("visibility span")
                .start
                .offset)
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|alternative| source.find(alternative).expect("written alternative"))
            .map(|offset| offset + source[offset..].find("Hidden").expect("variant segment"))
            .collect::<Vec<_>>(),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_refinements_and_private_function_refinements_remain_valid() {
    let diagnostics = diagnostics_for(concat!(
        "type Secret\n",
        "  pub Hidden\n",
        "end\n",
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
        "pub fn accepted(value: State::Ready) -> State::Ready\n",
        "  value\n",
        "end\n",
        "fn private(value: State::Hidden) -> State::Hidden\n",
        "  value\n",
        "end\n",
        "fn private_base(value: Secret::Hidden) -> Secret::Hidden\n",
        "  value\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn invalid_generic_arity_retains_independent_visibility_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type Secret<A>\n",
        "  pub Hidden(A)\n",
        "end\n",
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "  pub Ready(A)\n",
        "end\n",
        "pub type Public<A>\n",
        "  pub Open(A)\n",
        "end\n",
        "pub fn private_base(value: Secret::Hidden | Public<Int>::Open) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant(value: State::Hidden | State<Int>::Ready) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let observed = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                || diagnostic.id == "type.variant_refinement_private"
                || diagnostic.id == "type.variant_mismatch"
        })
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        [
            "type.invalid_annotation",
            "type.variant_refinement_private",
            "type.invalid_annotation",
            "type.variant_refinement_private",
        ],
        "{diagnostics:#?}"
    );
}
