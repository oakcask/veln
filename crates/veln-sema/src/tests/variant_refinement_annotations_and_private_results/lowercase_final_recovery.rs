use super::*;
use crate::type_syntax::parse_type_annotation;

fn recoverable_diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(
        !parsed.diagnostics.is_empty()
            && parsed
                .diagnostics
                .iter()
                .all(veln_syntax::ParseDiagnostic::allows_semantic_analysis),
        "{:#?}",
        parsed.diagnostics
    );
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn unique_lowercase_final_retains_identity_for_independent_checks() {
    let source = concat!(
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "  pub Closed\n",
        "end\n",
        "fn needs_closed(value: State::Closed) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn rejected(value: State::hidden) -> State::ready\n",
        "  needs_closed(value)\n",
        "  value\n",
        "end\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let recovered = environment.canonicalize_type_annotation(
        parse_type_annotation("State::hidden").expect("lowercase refinement type syntax"),
        None,
    );
    assert_eq!(recovered.render(), "State::Hidden");
    let signature = environment
        .function("rejected")
        .expect("invalid declaration keeps its symbol identity");
    assert_eq!(signature.params, [Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);

    let diagnostics = analyze_surface_module(&module);

    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "name.invalid_case",
            "name.invalid_case",
            "type.variant_mismatch",
            "type.variant_mismatch",
            "type.variant_refinement_private",
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.variant_refinement_unknown"
            && diagnostic.id != "type.invalid_annotation"
    }));
}

#[test]
fn missing_lowercase_final_retains_only_base_for_union_comparison() {
    let diagnostics = recoverable_diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Other\n",
        "  OtherOnly\n",
        "end\n",
        "fn rejected(value: State::missing | Other::OtherOnly) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "type.variant_refinement_unknown",
            "type.variant_union_base",
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[1].details.to_json(),
        "{\"base_type\":\"State\",\"variant\":\"missing\"}"
    );
}

#[test]
fn ambiguous_lowercase_repair_suppresses_constructor_dependent_failures() {
    let diagnostics = recoverable_diagnostics_for(concat!(
        "pub type State\n",
        "  Ready\n",
        "  Ready\n",
        "  pub Closed\n",
        "end\n",
        "pub type Other\n",
        "  pub OtherOnly\n",
        "end\n",
        "fn needs_closed(value: State::Closed) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn rejected(value: State::ready | Other::OtherOnly) -> ()\n",
        "  needs_closed(value)\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    let union_failure = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.variant_union_base")
        .expect("ambiguous final recovery retains the independently resolved base");
    assert_eq!(
        union_failure.details.to_json(),
        "{\"expected_base_type\":\"State\",\"actual_base_type\":\"Other\"}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.variant_refinement_unknown"
                && diagnostic.id != "type.variant_refinement_private"
                && diagnostic.id != "type.variant_mismatch"
                && diagnostic.id != "type.invalid_annotation"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn single_ambiguous_parameter_quarantines_constructor_dependent_checks() {
    let source = concat!(
        "pub type State\n",
        "  Ready\n",
        "  Ready\n",
        "  pub Closed\n",
        "end\n",
        "fn needs_closed(value: State::Closed) -> ()\n  ()\nend\n",
        "pub fn rejected(value: State::ready) -> ()\n",
        "  needs_closed(value)\n",
        "  ()\n",
        "end\n",
    );
    assert_single_ambiguous_annotation_is_quarantined(source);
}

#[test]
fn single_ambiguous_result_quarantines_constructor_dependent_checks() {
    let source = concat!(
        "pub type State\n",
        "  Ready\n",
        "  Ready\n",
        "  pub Closed\n",
        "end\n",
        "pub fn rejected() -> State::ready\n",
        "  Closed\n",
        "end\n",
    );
    assert_single_ambiguous_annotation_is_quarantined(source);
}

fn assert_single_ambiguous_annotation_is_quarantined(source: &str) {
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert_eq!(parsed.diagnostics.len(), 1, "{:#?}", parsed.diagnostics);
    assert_eq!(parsed.diagnostics[0].id, "parse.variant_refinement_type");
    assert!(parsed.diagnostics[0].allows_semantic_analysis());
    let parse_span = parsed.diagnostics[0].span.as_ref().expect("parse span");
    assert_eq!(
        &source[parse_span.start.offset..parse_span.end.offset],
        "ready"
    );

    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "name.duplicate"],
        "{diagnostics:#?}"
    );
}

#[test]
fn combined_recovery_in_explicit_type_arguments_retains_parser_details() {
    let source = concat!(
        "type State\n  Ready\nend\n",
        "fn sink(value: A) -> ()\n  ()\nend\n",
        "fn invoke(value: State) -> ()\n",
        "  sink<state::ready>(value)\n",
        "end\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    let parse = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
        .expect("selected explicit type argument emits the delayed parse diagnostic");
    assert_eq!(
        parse.details.to_json(),
        concat!(
            "{\"phase\":\"parse\",\"node_id\":null,",
            "\"parser_context\":\"expression_line\",",
            "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"ready\"},",
            "\"expected\":[\"NamedAdtType::Variant\"],",
            "\"recovery\":{\"strategy\":\"insert_token\",",
            "\"anchor\":\"type argument\",\"dropped_token_count\":0}}"
        )
    );
}

#[test]
fn combined_return_recovery_retains_declaration_specific_parser_details() {
    let source = concat!(
        "type State\n  Ready\nend\n",
        "test recovered() -> Result<(), state::ready>\n",
        "  Ok(())\n",
        "end\n",
        "fn ordinary() -> state::ready\n",
        "  Ready\n",
        "end\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    let parse_details = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>();
    assert_eq!(
        parse_details,
        [
            concat!(
                "{\"phase\":\"parse\",\"node_id\":null,",
                "\"parser_context\":\"test_return\",",
                "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"ready\"},",
                "\"expected\":[\"NamedAdtType::Variant\"],",
                "\"recovery\":{\"strategy\":\"insert_token\",",
                "\"anchor\":\"type annotation\",\"dropped_token_count\":0}}"
            ),
            concat!(
                "{\"phase\":\"parse\",\"node_id\":null,",
                "\"parser_context\":\"function_return\",",
                "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"ready\"},",
                "\"expected\":[\"NamedAdtType::Variant\"],",
                "\"recovery\":{\"strategy\":\"insert_token\",",
                "\"anchor\":\"type annotation\",\"dropped_token_count\":0}}"
            ),
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn combined_base_and_final_recovery_retains_constructor_identity() {
    let source = concat!(
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "  pub Closed\n",
        "end\n",
        "fn needs_closed(value: State::Closed) -> ()\n  ()\nend\n",
        "pub fn rejected(value: state::hidden) -> state::ready\n",
        "  needs_closed(value)\n",
        "  value\n",
        "end\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .canonicalize_type_annotation(
                parse_type_annotation("state::hidden").expect("named candidate syntax"),
                None,
            )
            .render(),
        "State::Hidden"
    );

    let diagnostics = analyze_surface_module(&module);
    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids[0], "parse.variant_refinement_type", "{diagnostics:#?}");
    assert_eq!(
        ids.iter().filter(|id| **id == "name.invalid_case").count(),
        4,
        "{diagnostics:#?}"
    );
    assert!(
        ids.contains(&"type.variant_refinement_private"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        ids.iter()
            .filter(|id| **id == "type.variant_mismatch")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.variant_refinement_unknown"
            && diagnostic.id != "type.invalid_annotation"
            && diagnostic.id != "name.unresolved"
    }));
    let signature = environment.function("rejected").expect("retained symbol");
    assert_eq!(signature.params, [Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
}

#[test]
fn every_written_union_alternative_uses_combined_case_recovery() {
    let source = concat!(
        "type State\n  Ready\n  Closed\n  Failed\nend\n",
        "fn forward(value: state::ready | State::closed) -> ()\n  ()\nend\n",
        "fn reverse(value: State::closed | state::ready) -> ()\n  ()\nend\n",
        "fn later(value: State::Ready | state::closed) -> ()\n  ()\nend\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(veln_syntax::ParseDiagnostic::allows_semantic_analysis),
        "{:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    for annotation in [
        "state::ready | State::closed",
        "State::closed | state::ready",
        "State::Ready | state::closed",
    ] {
        let recovered = environment.canonicalize_type_annotation(
            parse_type_annotation(annotation).expect("variant union syntax"),
            None,
        );
        assert_eq!(
            recovered.render(),
            "State::Ready | State::Closed",
            "{annotation}"
        );
    }
    for function in ["forward", "reverse", "later"] {
        assert_eq!(
            environment
                .function(function)
                .expect("invalid declaration retains its symbol")
                .params,
            [Type::Unknown],
            "{function} must remain quarantined"
        );
    }

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                && diagnostic.id != "type.variant_refinement_unknown"
                && diagnostic.id != "type.invalid_annotation"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn generic_combined_casing_preserves_independent_arity_failure() {
    let source = concat!(
        "type GenericState<A>\n  Ready(value: A)\nend\n",
        "fn rejected(value: genericState<Int, String>::ready) -> ()\n  ()\nend\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.id == "parse.variant_refinement_type" })
    );
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.invalid_annotation"
            && diagnostic
                .message
                .contains("expects 1 type argument(s), found 2")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.id != "type.variant_refinement_unknown" })
    );
}

#[test]
fn generic_combined_casing_preserves_independent_base_failure() {
    let source = concat!(
        "mod main\n",
        "type GenericEmpty<A>\nend\n",
        "fn rejected(value: genericEmpty<Int>::missing) -> ()\n  ()\nend\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert_eq!(parsed.diagnostics.len(), 1, "{:#?}", parsed.diagnostics);
    assert!(parsed.diagnostics[0].allows_semantic_analysis());
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("rejected")
            .expect("retained symbol")
            .params,
        [Type::Unknown]
    );

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "name.invalid_case",
            "type.variant_refinement_base",
        ],
        "{diagnostics:#?}"
    );
    let base = &diagnostics[2];
    assert_eq!(
        base.details.to_json(),
        "{\"written_type\":\"genericEmpty<Int>\",\"reason\":\"not_adt\"}"
    );
    assert_eq!(base.related.len(), 1, "{base:#?}");
    let related = base.related[0].to_json();
    assert!(related.contains("\"kind\":\"base_type_declaration\""));
    assert!(related.contains("Type `GenericEmpty` is declared here without finite ADT variants."));
    assert!(related.contains("\"resolved_identity\":\"main::GenericEmpty\""));
    assert!(related.contains("\"line\":2,\"column\":1"));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.variant_refinement_unknown"
            && diagnostic.id != "type.variant_refinement_private"
            && diagnostic.id != "type.variant_mismatch"
            && diagnostic.id != "type.invalid_annotation"
    }));
}

#[test]
fn unresolved_generic_bases_use_name_resolution_for_lowercase_and_uppercase_finals() {
    for source in [
        "fn rejected(value: Missing<Int>::lower) -> ()\n  ()\nend\n",
        "fn rejected(value: Missing<Int>::Lower) -> ()\n  ()\nend\n",
    ] {
        let parsed = parse(&SourceFile::new("main.veln", source));
        if source.contains("::lower") {
            assert_eq!(parsed.diagnostics.len(), 1, "{:#?}", parsed.diagnostics);
            assert!(parsed.diagnostics[0].allows_semantic_analysis());
        } else {
            assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        }
        let module = lower_surface_ast(&parsed.tree);
        let environment = TypeEnvironment::from_module(&module);
        assert_eq!(
            environment
                .function("rejected")
                .expect("unresolved declaration keeps its symbol")
                .params,
            [Type::Unknown]
        );

        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            ["name.unresolved"],
            "{diagnostics:#?}"
        );
        let unresolved = &diagnostics[0];
        assert_eq!(unresolved.message, "unresolved type `Missing`");
        let span = unresolved.span.as_ref().expect("unresolved base span");
        assert_eq!(&source[span.start.offset..span.end.offset], "Missing");
        assert_eq!(
            unresolved.details.to_json(),
            concat!(
                "{\"phase\":\"name\",\"node_id\":null,",
                "\"symbol\":\"Missing\",\"namespace\":\"type\",",
                "\"resolution_status\":\"unresolved\",\"candidates\":[]}"
            )
        );
    }
}

#[test]
fn unresolved_generic_union_base_does_not_participate_in_comparison() {
    let source = concat!(
        "type Other\n  OtherOnly\nend\n",
        "fn forward(value: Missing<Int>::lower | Other::OtherOnly) -> ()\n  ()\nend\n",
        "fn reverse(value: Other::OtherOnly | Missing<Int>::Lower) -> ()\n  ()\nend\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert_eq!(parsed.diagnostics.len(), 1, "{:#?}", parsed.diagnostics);
    assert!(parsed.diagnostics[0].allows_semantic_analysis());

    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.unresolved", "name.unresolved"],
        "{diagnostics:#?}"
    );
}

#[test]
fn combined_missing_and_ambiguous_finals_retain_only_the_base() {
    let source = concat!(
        "type State\n  Ready\n  Ready\nend\n",
        "type Other\n  OtherOnly\nend\n",
        "fn missing(value: state::missing | Other::OtherOnly) -> ()\n  ()\nend\n",
        "fn ambiguous(value: state::ready | Other::OtherOnly) -> ()\n  ()\nend\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(veln_syntax::ParseDiagnostic::allows_semantic_analysis),
        "{:#?}",
        parsed.diagnostics
    );
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_union_base")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.variant_refinement_unknown"
            && diagnostic.details.to_json() == "{\"base_type\":\"State\",\"variant\":\"missing\"}"
    }));
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.invalid_annotation"
                && diagnostic.id != "type.variant_refinement_private"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn lowercase_qualified_named_type_is_not_selected_without_a_refinement_base() {
    let source = "fn ordinary(value: packet::item) -> ()\n  ()\nend\n";
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    assert_eq!(
        parsed.tree.items.first().and_then(|item| {
            let veln_syntax::SyntaxItem::Function(function) = item else {
                return None;
            };
            Some(function.params[0].ty_refinements.len())
        }),
        Some(1)
    );

    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "parse.variant_refinement_type"
                && !diagnostic.id.starts_with("type.variant_refinement")
        }),
        "{diagnostics:#?}"
    );
}
