use std::collections::BTreeSet;

use super::*;

fn recovery_module(
    reverse_sources: bool,
    reverse_imports: bool,
    unrelated: usize,
) -> SurfaceModule {
    let imports = if reverse_imports {
        "use beta\nuse alpha\n"
    } else {
        "use alpha\nuse beta\n"
    };
    let unrelated = (0..unrelated)
        .map(|index| format!("pub let unrelated_{index}: Int = {index}\n"))
        .collect::<String>();
    let mut sources = vec![
        (
            "bad",
            SourceFile::new("bad.veln", "pub let broken: Missing = 0\n"),
        ),
        (
            "alpha",
            SourceFile::new("alpha.veln", "use bad\npub let shared: Int = bad::broken\n"),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
        ),
        (
            "owner",
            SourceFile::new(
                "owner.veln",
                format!(
                    "{imports}pub let selected: Int = shared\npub let local: Int = selected\n{unrelated}"
                ),
            ),
        ),
        (
            "consumer",
            SourceFile::new(
                "consumer.veln",
                "use owner\npub let observed: Int = selected\npub let independent: Int = 4\n",
            ),
        ),
        (
            "consumer",
            SourceFile::new(
                "consumer.veln",
                "use owner\npub let observed: Int = selected\nfn read_selected() -> String\n  selected\nend\nfn read_observed() -> String\n  observed\nend\nfn read_local() -> String\n  local\nend\n",
            ),
        ),
    ];
    if reverse_sources {
        sources.reverse();
    }
    merged_modules_with_identities(sources)
}

#[test]
fn ambiguous_initializers_recover_after_delayed_invalidation_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let module = recovery_module(reverse_sources, reverse_imports, 0);
            let mut environment = crate::types::TypeEnvironment::from_module(&module);
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["shared".to_string()],
                    Some("owner"),
                    None
                ),
                crate::types::ModuleValueLookup::Ambiguous
            ));
            let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
            let analysis = crate::analysis::check_module_values(
                &module,
                &mut environment,
                &mut variant_diagnostics,
            );
            let identity =
                |module: &str, name: &str| (None, Some(module.to_string()), name.to_string());
            assert_eq!(
                analysis.invalid,
                [identity("bad", "broken"), identity("alpha", "shared")]
                    .into_iter()
                    .collect(),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}"
            );
            let crate::types::ModuleValueLookup::Available(selected) = environment
                .module_value_path_in_owner(&["shared".to_string()], Some("owner"), None)
            else {
                panic!("valid namesake must be selected")
            };
            assert_eq!(
                (
                    selected.declaration.package_name.as_deref(),
                    selected.declaration.module_name.as_deref(),
                    selected.declaration.name.as_deref(),
                ),
                (None, Some("beta"), Some("shared"))
            );
            for name in ["selected", "local", "observed"] {
                assert!(
                    matches!(
                        environment.module_value_path_in_owner(
                            &[name.to_string()],
                            Some("consumer"),
                            None
                        ),
                        crate::types::ModuleValueLookup::Available(_)
                    ),
                    "{name}"
                );
            }
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id != "name.ambiguous"),
                "{:#?}",
                analysis.diagnostics
            );
            let diagnostics = analyze_surface_module(&module);
            let mismatches = diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.id == "type.mismatch"
                        && diagnostic.message == "expected `String`, but found `Int`"
                })
                .collect::<Vec<_>>();
            assert_eq!(mismatches.len(), 3, "{diagnostics:#?}");
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && ["selected", "observed", "local"]
                                .iter()
                                .any(|name| diagnostic.message.contains(name)))),
                "{diagnostics:#?}"
            );
        }
    }
}

#[test]
fn availability_recovery_rechecks_only_candidate_dependents() {
    let mut measurements = Vec::new();
    for unrelated in [32, 64, 128] {
        let module = recovery_module(false, false, unrelated);
        let mut environment = crate::types::TypeEnvironment::from_module(&module);
        let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
        reset_module_value_availability_work();
        let analysis = crate::analysis::check_module_values(
            &module,
            &mut environment,
            &mut variant_diagnostics,
        );
        let work = take_module_value_availability_work();
        assert_eq!(analysis.invalid.len(), 2, "{:#?}", analysis.invalid);
        assert!(
            work.semantic_visits <= module.module_values.len() * 2 + 12,
            "{work:#?}"
        );
        assert!(work.dependency_index_entries <= 6, "{work:#?}");
        assert!(work.declaration_recheck_lookups <= 12, "{work:#?}");
        measurements.push(work);
    }
    assert!(
        measurements
            .windows(2)
            .all(|pair| pair[0].declaration_recheck_lookups == pair[1].declaration_recheck_lookups),
        "{measurements:#?}"
    );
}

fn non_convergent_lookup_module(reverse_sources: bool, reverse_imports: bool) -> SurfaceModule {
    let owner_imports = if reverse_imports {
        "use c\nuse b\n"
    } else {
        "use b\nuse c\n"
    };
    let mut sources = vec![
        (
            "b",
            SourceFile::new(
                "b.veln",
                "use owner\npub let shared: Int = owner::selected\n",
            ),
        ),
        ("c", SourceFile::new("c.veln", "pub let shared: Int = 1\n")),
        (
            "owner",
            SourceFile::new(
                "owner.veln",
                format!(
                    "{owner_imports}pub let selected: Int = shared\npub let local: Int = selected\npub let independent: Int = 4\n"
                ),
            ),
        ),
    ];
    if reverse_sources {
        sources.reverse();
    }
    merged_modules_with_identities(sources)
}

#[test]
fn non_convergent_imported_lookup_is_rejected_in_every_order() {
    let identity = |module: &str, name: &str| (None, Some(module.to_string()), name.to_string());
    let expected_invalid = [
        identity("b", "shared"),
        identity("owner", "local"),
        identity("owner", "selected"),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let mut baseline_diagnostics = None;

    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let module = non_convergent_lookup_module(reverse_sources, reverse_imports);
            let mut environment = crate::types::TypeEnvironment::from_module(&module);
            let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
            reset_module_value_availability_work();
            let analysis = crate::analysis::check_module_values(
                &module,
                &mut environment,
                &mut variant_diagnostics,
            );
            let work = take_module_value_availability_work();

            assert_eq!(analysis.invalid, expected_invalid);
            assert!(work.declaration_recheck_lookups <= 12, "{work:#?}");
            assert!(work.availability_updates <= 10, "{work:#?}");
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["selected".to_string()],
                    Some("owner"),
                    None,
                ),
                crate::types::ModuleValueLookup::Unavailable
            ));
            let crate::types::ModuleValueLookup::Available(selected) = environment
                .module_value_path_in_owner(&["shared".to_string()], Some("owner"), None)
            else {
                panic!("independent valid candidate must remain selected")
            };
            assert_eq!(selected.declaration.module_name.as_deref(), Some("c"));
            assert!(matches!(
                environment
                    .module_value_path_in_owner(&["local".to_string()], Some("owner"), None,),
                crate::types::ModuleValueLookup::Unavailable
            ));
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["independent".to_string()],
                    Some("owner"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));

            let mut diagnostics = analysis
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    (
                        diagnostic.id.clone(),
                        diagnostic.message.as_str().to_string(),
                        diagnostic
                            .span
                            .as_ref()
                            .map(|span| (span.file.as_str().to_string(), span.start.offset)),
                    )
                })
                .collect::<Vec<_>>();
            diagnostics.sort();
            if let Some(baseline) = &baseline_diagnostics {
                assert_eq!(&diagnostics, baseline);
            } else {
                baseline_diagnostics = Some(diagnostics);
            }
        }
    }
}
