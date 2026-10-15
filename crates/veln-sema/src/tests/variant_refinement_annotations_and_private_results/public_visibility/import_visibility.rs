use super::*;

#[test]
fn qualified_private_base_selects_only_the_final_base_segment() {
    let diagnostics = visibility_diagnostics(concat!(
        "mod sample\n",
        "type Secret\n",
        "  pub Hidden\n",
        "end\n",
        "pub fn expose(value: sample::Secret::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let diagnostic = &diagnostics[0];
    let span = diagnostic.span.as_ref().expect("visibility span");
    assert_eq!((span.start.line, span.start.column), (5, 30));
    assert_eq!((span.end.line, span.end.column), (5, 36));

    let observed = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
    assert!(observed.contains(
        "\"details\":{\"written_type\":\"sample::Secret::Hidden\",\"base_type\":\"sample::Secret\",\"variant\":\"Hidden\",\"boundary\":\"public_parameter\",\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"}]}"
    ));
    assert!(observed.contains("\"kind\":\"private_type_declaration\""));
    assert!(observed.contains("\"kind\":\"public_parameter_boundary\""));
}

#[test]
fn imported_refinements_preserve_qualified_and_unqualified_visibility_identity() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "imported.veln",
            concat!("mod imported\n", "pub type State\n", "  Hidden\n", "end\n",),
        ))
        .tree,
    );
    let base = TypeEnvironment::from_module(&dependency);
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            concat!(
                "mod consumer\n",
                "use imported\n",
                "pub fn unqualified(value: State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
                "pub fn qualified(value: imported::State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );

    let diagnostics = analyze_surface_module_with_base_for_test(&consumer, &base)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let written_types = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(written_types[0].contains("\"written_type\":\"State::Hidden\""));
    assert!(written_types[1].contains("\"written_type\":\"imported::State::Hidden\""));
}

#[test]
fn unrelated_modules_imports_do_not_change_unqualified_refinement_visibility() {
    let without_unrelated_import = unqualified_import_isolation_observation(false);
    let with_unrelated_import = unqualified_import_isolation_observation(true);

    assert_eq!(
        with_unrelated_import.visibility_json, without_unrelated_import.visibility_json,
        "an unrelated module's same-leaf import must not change consumer diagnostics"
    );
    assert_eq!(with_unrelated_import.visibility_json.len(), 10);
    assert_eq!(
        with_unrelated_import.recovery_signatures,
        without_unrelated_import.recovery_signatures
    );
    assert!(with_unrelated_import.recovery_signatures.iter().all(
        |(_, params, result, published)| {
            params == &[Type::Unknown] && result == &Type::Unknown && !*published
        }
    ));
    assert_eq!(
        with_unrelated_import.downstream_visibility,
        without_unrelated_import.downstream_visibility
    );
    assert!(
        with_unrelated_import
            .downstream_visibility
            .iter()
            .all(|(_, qualified, unqualified)| !qualified && !unqualified)
    );
    assert!(!with_unrelated_import.has_core);
    assert!(!with_unrelated_import.has_ir);
}

#[derive(Debug, PartialEq)]
struct UnqualifiedImportIsolationObservation {
    visibility_json: Vec<String>,
    recovery_signatures: Vec<(String, Vec<Type>, Type, bool)>,
    downstream_visibility: Vec<(String, bool, bool)>,
    has_core: bool,
    has_ir: bool,
}

fn unqualified_import_isolation_observation(
    unrelated_imports_public_namesake: bool,
) -> UnqualifiedImportIsolationObservation {
    let project = unqualified_import_isolation_project(unrelated_imports_public_namesake);
    let environment = TypeEnvironment::from_module(&project);
    let recovery_signatures = unqualified_import_recovery_signatures(&environment);
    let (diagnostics, lowered) = check_project_surface_module(&project);
    let visibility_json = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect();
    let downstream_visibility = unqualified_import_downstream_visibility(&environment);

    UnqualifiedImportIsolationObservation {
        visibility_json,
        recovery_signatures,
        downstream_visibility,
        has_core: lowered.core.is_some(),
        has_ir: lowered.ir.is_some(),
    }
}

fn unqualified_import_isolation_project(unrelated_imports_public_namesake: bool) -> SurfaceModule {
    let private_dependency = SourceFile::new(
        "dep_private.veln",
        concat!(
            "pub type State<A>\n",
            "  Hidden(A)\n",
            "  Other(A)\n",
            "end\n",
        ),
    );
    let public_dependency = SourceFile::new(
        "dep_public.veln",
        concat!(
            "pub type State<A>\n",
            "  pub Hidden(A)\n",
            "  pub Other(A)\n",
            "end\n",
        ),
    );
    let consumer = SourceFile::new(
        "consumer.veln",
        concat!(
            "use dep_private\n",
            "pub fn exact(value: State<Int>::Hidden) -> State<Int>::Hidden\n",
            "  value\n",
            "end\n",
            "pub fn wrong_arity(value: State::Hidden) -> State::Hidden\n",
            "  value\n",
            "end\n",
            "pub fn recovered(value: state<Int>::Hidden) -> state<Int>::Hidden\n",
            "  value\n",
            "end\n",
            "pub fn union(value: State<Int>::Other | State<Int>::Hidden) -> State<Int>::Other | State<Int>::Hidden\n",
            "  value\n",
            "end\n",
        ),
    );
    let unrelated = SourceFile::new(
        "unrelated.veln",
        if unrelated_imports_public_namesake {
            "use dep_public\n"
        } else {
            ""
        },
    );
    let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&consumer).tree,
        "consumer".to_string(),
        consumer.span(veln_source::TextRange::new(0, 0)),
    );
    let mut project = merged_modules_with_identities(vec![
        ("dep_private", private_dependency),
        ("dep_public", public_dependency),
        ("consumer", consumer),
        ("unrelated", unrelated),
    ]);
    project.module = consumer_identity.module;
    project
}

fn unqualified_import_recovery_signatures(
    environment: &TypeEnvironment,
) -> Vec<(String, Vec<Type>, Type, bool)> {
    unqualified_import_function_names()
        .iter()
        .map(|name| {
            let signature = environment
                .function(name)
                .unwrap_or_else(|| panic!("{name} recovery signature"));
            (
                (*name).to_string(),
                signature.params.clone(),
                signature.return_type.clone(),
                environment.function_is_published_for_test(name),
            )
        })
        .collect()
}

fn unqualified_import_downstream_visibility(
    environment: &TypeEnvironment,
) -> Vec<(String, bool, bool)> {
    let downstream = lower_surface_ast(
        &parse(&SourceFile::new(
            "downstream.veln",
            "mod downstream\nuse consumer\n",
        ))
        .tree,
    );
    let downstream = TypeEnvironment::from_module_with_base_for_test(&downstream, environment);
    unqualified_import_function_names()
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                downstream
                    .function_path(
                        &["consumer".to_string(), (*name).to_string()],
                        Some("downstream"),
                    )
                    .is_some(),
                !downstream
                    .unqualified_function_import_candidates(name, Some("downstream"))
                    .is_empty(),
            )
        })
        .collect()
}

fn unqualified_import_function_names() -> [&'static str; 4] {
    ["exact", "wrong_arity", "recovered", "union"]
}

#[test]
fn local_refinement_descriptor_shadows_imported_namesake() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "imported.veln",
            concat!(
                "mod imported\n",
                "pub type State\n",
                "  pub Hidden\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let base = TypeEnvironment::from_module(&dependency);
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            concat!(
                "mod consumer\n",
                "use imported\n",
                "type State\n",
                "  pub Hidden\n",
                "end\n",
                "pub fn local(value: State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );

    let diagnostics = analyze_surface_module_with_base_for_test(&consumer, &base)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let observed = veln_diagnostics::diagnostic_to_json(&diagnostics[0]).to_json();
    assert!(observed.contains("\"written_type\":\"State::Hidden\""));
    assert!(observed.contains("\"kind\":\"private_type_declaration\""));
    assert!(observed.contains("\"file\":\"consumer.veln\""));
}

#[test]
fn qualified_imported_refinement_uses_written_identity_across_local_collisions() {
    for imported_first in [true, false] {
        let project = colliding_qualified_refinement_project(imported_first);
        let environment = TypeEnvironment::from_module(&project);
        assert_colliding_recovery_signatures(&environment);
        assert!(environment.function_is_published_for_test("local_control"));

        let (diagnostics, lowered) = check_project_surface_module(&project);
        assert_colliding_visibility_diagnostics(&diagnostics);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "type.variant_refinement_private"
                    || !veln_diagnostics::diagnostic_to_json(diagnostic)
                        .to_json()
                        .contains("local_control")
            }),
            "the unqualified spelling must keep selecting the local ADT: {diagnostics:#?}"
        );
        assert!(
            lowered.core.is_none(),
            "invalid project must not produce core"
        );
        assert!(lowered.ir.is_none(), "invalid project must not produce IR");

        assert_colliding_downstream_publication(&environment);
    }
}
