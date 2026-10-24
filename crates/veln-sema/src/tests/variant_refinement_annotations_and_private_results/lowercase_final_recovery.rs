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
        "end\n",
        "type Other\n",
        "  OtherOnly\n",
        "end\n",
        "fn rejected(value: State::ready | Other::OtherOnly) -> ()\n",
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
                && diagnostic.id != "type.mismatch"
                && diagnostic.id != "type.invalid_annotation"
        }),
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
