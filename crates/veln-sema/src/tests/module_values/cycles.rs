use super::*;

fn cross_module_cycle_sources(
    reverse_sources: bool,
    reverse_imports: bool,
) -> Vec<(&'static str, SourceFile)> {
    let alpha_imports = if reverse_imports {
        "use gamma\nuse beta\n"
    } else {
        "use beta\nuse gamma\n"
    };
    let mut sources = vec![
        (
            "alpha",
            SourceFile::new(
                "alpha.veln",
                format!(
                    "{alpha_imports}pub let left: Int = beta::right\npub let healthy: Bool = gamma::ready\n"
                ),
            ),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "use alpha\npub let right: Int = alpha::left\n"),
        ),
        (
            "gamma",
            SourceFile::new("gamma.veln", "pub let ready: Bool = true\n"),
        ),
        (
            "consumer",
            SourceFile::new(
                "consumer.veln",
                concat!(
                    "use alpha\n",
                    "pub let blocked: Int = alpha::left\n",
                    "fn reads_blocked() -> Int\n",
                    "  blocked\n",
                    "end\n",
                    "fn reads_healthy() -> String\n",
                    "  healthy\n",
                    "end\n",
                ),
            ),
        ),
        (
            "downstream",
            SourceFile::new(
                "downstream.veln",
                concat!(
                    "use consumer\n",
                    "pub let twice_blocked: Int = consumer::blocked\n",
                    "fn reads_twice_blocked() -> Bool\n",
                    "  twice_blocked\n",
                    "end\n",
                ),
            ),
        ),
    ];
    if reverse_sources {
        sources.reverse();
    }
    sources
}

fn assert_cross_module_cycle_quarantine(
    invalid: &std::collections::BTreeSet<(Option<String>, Option<String>, String)>,
    environment: &crate::types::TypeEnvironment,
    reverse_sources: bool,
    reverse_imports: bool,
) {
    for identity in [
        (None, Some("alpha".to_string()), "left".to_string()),
        (None, Some("beta".to_string()), "right".to_string()),
        (None, Some("consumer".to_string()), "blocked".to_string()),
        (
            None,
            Some("downstream".to_string()),
            "twice_blocked".to_string(),
        ),
    ] {
        assert!(
            invalid.contains(&identity),
            "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {invalid:#?}"
        );
    }
    assert!(matches!(
        environment.module_value_path_in_owner(&["healthy".to_string()], Some("consumer"), None,),
        crate::types::ModuleValueLookup::Available(_)
    ));
    assert!(matches!(
        environment.module_value_path_in_owner(&["blocked".to_string()], Some("consumer"), None,),
        crate::types::ModuleValueLookup::Unavailable
    ));
    assert!(matches!(
        environment.module_value_path_in_owner(
            &["twice_blocked".to_string()],
            Some("downstream"),
            None,
        ),
        crate::types::ModuleValueLookup::Unavailable
    ));
}

fn cross_module_cycle_observation(diagnostics: &[Diagnostic]) -> veln_diagnostics::JsonValue {
    let cycles = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .collect::<Vec<_>>();

    assert_eq!(cycles.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        cycles[0].message.as_str(),
        "module value reference `left` closes an initializer cycle"
    );
    assert_eq!(invariants::cycle_detail(cycles[0], "name"), "right");
    assert_eq!(invariants::cycle_detail(cycles[0], "target"), "left");
    assert_eq!(
        invariants::cycle_related_messages(cycles[0]),
        vec![
            "Module value `left` participates in this initializer cycle.",
            "Module value `right` participates in this initializer cycle.",
            "Module value `left` depends on `right` here.",
            "Module value `right` depends on `left` here.",
        ]
    );
    veln_diagnostics::JsonValue::object([
        (
            "span",
            cycles[0]
                .span
                .as_ref()
                .map(crate::diagnostics::span_json)
                .unwrap_or(veln_diagnostics::JsonValue::Null),
        ),
        ("details", cycles[0].details.clone()),
        (
            "related",
            veln_diagnostics::JsonValue::array(cycles[0].related.clone()),
        ),
    ])
}

fn assert_cross_module_cycle_semantic_diagnostics(diagnostics: &[Diagnostic]) {
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.message == "expected `String`, but found `Bool`"
    }));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "name.ambiguous"
            && !(diagnostic.id == "type.mismatch"
                && diagnostic.span.as_ref().is_some_and(|span| {
                    span.file.as_str() == "consumer.veln" && span.start.line == 4
                }))
            && !(diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `Bool`, but found `Int`")
            && !(diagnostic.id == "name.unresolved" && diagnostic.message.contains("healthy"))
    }));
}

#[test]
fn cross_module_initializer_cycles_quarantine_dependents_in_every_order() {
    let mut expected_cycle = None;
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let sources = cross_module_cycle_sources(reverse_sources, reverse_imports);
            let module = merged_modules_with_identities(sources);
            let mut environment = crate::types::TypeEnvironment::from_module(&module);
            let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
            let analysis = crate::analysis::check_module_values(
                &module,
                &mut environment,
                &mut variant_diagnostics,
            );
            assert_cross_module_cycle_quarantine(
                &analysis.invalid,
                &environment,
                reverse_sources,
                reverse_imports,
            );
            let diagnostics = analyze_surface_module(&module);
            let cycle = cross_module_cycle_observation(&diagnostics);
            if let Some(expected) = &expected_cycle {
                assert_eq!(
                    &cycle, expected,
                    "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}"
                );
            } else {
                expected_cycle = Some(cycle);
            }
            assert_cross_module_cycle_semantic_diagnostics(&diagnostics);
        }
    }
}

fn bare_imported_cycle_module(reverse_sources: bool, reverse_imports: bool) -> SurfaceModule {
    let alpha_imports = if reverse_imports {
        concat!(
            "use beta as namesake from \"namesake/pkg\"\n",
            "use beta from \"cycle/pkg\"\n",
        )
    } else {
        concat!(
            "use beta from \"cycle/pkg\"\n",
            "use beta as namesake from \"namesake/pkg\"\n",
        )
    };
    let mut sources = vec![
        (
            "cycle/pkg",
            "alpha",
            SourceFile::new(
                "cycle/alpha.veln",
                format!("{alpha_imports}pub let left: Int = right\n"),
            ),
        ),
        (
            "cycle/pkg",
            "beta",
            SourceFile::new(
                "cycle/beta.veln",
                concat!(
                    "use alpha from \"cycle/pkg\"\n",
                    "pub let right: Int = left\n",
                ),
            ),
        ),
        (
            "cycle/pkg",
            "consumer",
            SourceFile::new(
                "cycle/consumer.veln",
                concat!(
                    "use alpha from \"cycle/pkg\"\n",
                    "pub let blocked: Int = left\n",
                ),
            ),
        ),
        (
            "namesake/pkg",
            "beta",
            SourceFile::new("namesake/beta.veln", "pub let right: Int = 7\n"),
        ),
    ];
    if reverse_sources {
        sources.reverse();
    }
    let mut modules = sources
        .into_iter()
        .map(|(package, module_name, source)| {
            let parsed = parse(&source);
            assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
            let mut module = veln_ast::lower_surface_ast_with_package_module_identity(
                &parsed.tree,
                package.to_string(),
                module_name.to_string(),
                source.span(veln_source::TextRange::new(0, 0)),
            );
            for use_decl in &mut module.uses {
                let target_package = use_decl.package.as_deref().unwrap_or(package);
                use_decl.name = format!("{target_package}::{}", use_decl.name);
            }
            module
        })
        .collect::<Vec<_>>();
    let mut module = modules.remove(0);
    for source_module in modules {
        module.uses.extend(source_module.uses);
        module.module_values.extend(source_module.module_values);
    }
    module
}

fn assert_bare_imported_cycle_identities(
    invalid: &std::collections::BTreeSet<(Option<String>, Option<String>, String)>,
    environment: &crate::types::TypeEnvironment,
    reverse_sources: bool,
    reverse_imports: bool,
) {
    for identity in [
        (
            Some("cycle/pkg".to_string()),
            Some("cycle/pkg::alpha".to_string()),
            "left".to_string(),
        ),
        (
            Some("cycle/pkg".to_string()),
            Some("cycle/pkg::beta".to_string()),
            "right".to_string(),
        ),
        (
            Some("cycle/pkg".to_string()),
            Some("cycle/pkg::consumer".to_string()),
            "blocked".to_string(),
        ),
    ] {
        assert!(
            invalid.contains(&identity),
            "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {invalid:#?}"
        );
    }
    assert!(!invalid.contains(&(
        Some("namesake/pkg".to_string()),
        Some("namesake/pkg::beta".to_string()),
        "right".to_string(),
    )));
    assert!(matches!(
        environment.module_value_path_in_owner(
            &["right".to_string()],
            Some("namesake/pkg::beta"),
            Some("namesake/pkg"),
        ),
        crate::types::ModuleValueLookup::Available(_)
    ));
}

fn assert_bare_imported_cycle_span(
    diagnostics: &[Diagnostic],
    reverse_sources: bool,
    reverse_imports: bool,
) {
    let cycle_diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .unwrap_or_else(|| {
            panic!(
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            )
        });
    let cycle_span = cycle_diagnostic.span.as_ref().expect("cycle primary span");
    assert_eq!(cycle_span.file.as_str(), "cycle/beta.veln");
    assert_eq!((cycle_span.start.line, cycle_span.start.column), (2, 22));
    assert_eq!((cycle_span.end.line, cycle_span.end.column), (2, 26));
}

fn observe_bare_imported_cycle_variant(
    reverse_sources: bool,
    reverse_imports: bool,
) -> veln_diagnostics::JsonValue {
    let module = bare_imported_cycle_module(reverse_sources, reverse_imports);

    let mut environment = crate::types::TypeEnvironment::from_module(&module);
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    let analysis =
        crate::analysis::check_module_values(&module, &mut environment, &mut variant_diagnostics);

    assert_bare_imported_cycle_identities(
        &analysis.invalid,
        &environment,
        reverse_sources,
        reverse_imports,
    );
    let diagnostics = &analysis.diagnostics;
    assert_bare_imported_cycle_span(diagnostics, reverse_sources, reverse_imports);

    cross_module_cycle_observation(diagnostics)
}

#[test]
fn bare_imported_cycle_edges_preserve_package_identity() {
    let mut expected_cycle = None;
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let cycle = observe_bare_imported_cycle_variant(reverse_sources, reverse_imports);
            if let Some(expected) = &expected_cycle {
                assert_eq!(
                    &cycle, expected,
                    "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}"
                );
            } else {
                expected_cycle = Some(cycle);
            }
        }
    }
}
