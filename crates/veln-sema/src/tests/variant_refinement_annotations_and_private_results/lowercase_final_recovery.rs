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
        "pub fn rejected(value: State::ready) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.variant_refinement_unknown"
            && diagnostic.id != "type.variant_refinement_private"
            && diagnostic.id != "type.mismatch"
    }));
}
