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
                "use owner\npub let observed: Int = selected\npub let independent: Int = 4\nfn read_selected() -> String\n  selected\nend\nfn read_observed() -> String\n  observed\nend\nfn read_local() -> String\n  local\nend\n",
            ),
        ),
    ];
    if reverse_sources {
        sources.reverse();
    }
    merged_modules_with_identities(sources)
}

fn same_name_candidate_fanout_module(
    candidate_count: usize,
    dependent_count: usize,
    reverse_sources: bool,
    reverse_imports: bool,
) -> SurfaceModule {
    let candidate_indices = if reverse_imports {
        (0..candidate_count).rev().collect::<Vec<_>>()
    } else {
        (0..candidate_count).collect::<Vec<_>>()
    };
    let imports = candidate_indices
        .iter()
        .map(|index| format!("use candidate_{index}\n"))
        .collect::<String>();
    let dependents = (0..dependent_count)
        .map(|index| format!("pub let dependent_{index}: Int = shared\n"))
        .collect::<String>();
    let mut sources = (0..candidate_count)
        .map(|index| {
            let annotation = if index == 0 {
                "Int".to_string()
            } else {
                format!("Missing{index}")
            };
            (
                format!("candidate_{index}"),
                SourceFile::new(
                    format!("candidate_{index}.veln"),
                    format!("pub let shared: {annotation} = {index}\n"),
                ),
            )
        })
        .collect::<Vec<_>>();
    sources.push((
        "owner".to_string(),
        SourceFile::new("owner.veln", format!("{imports}{dependents}")),
    ));
    if reverse_sources {
        sources.reverse();
    }
    let sources = sources
        .iter()
        .map(|(module, source)| (module.as_str(), source.clone()))
        .collect();
    merged_modules_with_identities(sources)
}

#[test]
fn same_name_imported_candidate_fanout_is_adjacent_linear_in_every_order() {
    let mut measurements = Vec::new();
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let mut order_measurements = Vec::new();
            for count in [32, 64] {
                let module = same_name_candidate_fanout_module(
                    count,
                    count,
                    reverse_sources,
                    reverse_imports,
                );
                let mut environment = crate::types::TypeEnvironment::from_module(&module);
                let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
                let analysis = crate::analysis::check_module_values(
                    &module,
                    &mut environment,
                    &mut variant_diagnostics,
                );
                assert_eq!(analysis.invalid.len(), count - 1, "{:#?}", analysis.invalid);
                assert!(analysis.invalid.iter().all(|identity| {
                    identity.1.as_deref().is_some_and(|module| {
                        module.starts_with("candidate_") && module != "candidate_0"
                    }) && identity.2 == "shared"
                }));
                let crate::types::ModuleValueLookup::Available(selected) = environment
                    .module_value_path_in_owner(&["shared".to_string()], Some("owner"), None)
                else {
                    panic!("valid namesake must be selected")
                };
                assert_eq!(
                    selected.declaration.module_name.as_deref(),
                    Some("candidate_0")
                );
                for index in 0..count {
                    assert!(matches!(
                        environment.module_value_path_in_owner(
                            &[format!("dependent_{index}")],
                            Some("owner"),
                            None,
                        ),
                        crate::types::ModuleValueLookup::Available(_)
                    ));
                }
                assert!(
                    analysis
                        .diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.id != "name.ambiguous"),
                    "{:#?}",
                    analysis.diagnostics
                );

                reset_module_value_availability_work();
                let diagnostics = analyze_surface_module(&module);
                let work = take_module_value_availability_work();
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.id != "name.ambiguous"),
                    "{diagnostics:#?}"
                );
                assert!(work.dependency_index_entries <= count * 2 + 2, "{work:#?}");
                assert!(
                    work.peak_retained_dependency_state <= count * 8 + 8,
                    "{work:#?}"
                );
                assert!(work.semantic_visits <= count * 6 + 16, "{work:#?}");
                assert!(
                    work.declaration_recheck_lookups <= count * 2 + 8,
                    "{work:#?}"
                );
                order_measurements.push(work);
            }
            let [smaller, larger] = order_measurements.as_slice() else {
                unreachable!()
            };
            assert!(
                larger.peak_retained_dependency_state
                    <= smaller.peak_retained_dependency_state * 2 + 16,
                "{smaller:#?} -> {larger:#?}"
            );
            assert!(
                larger.semantic_visits <= smaller.semantic_visits * 2 + 16,
                "{smaller:#?} -> {larger:#?}"
            );
            assert!(
                larger.declaration_recheck_lookups <= smaller.declaration_recheck_lookups * 2 + 16,
                "{smaller:#?} -> {larger:#?}"
            );
            measurements.push(order_measurements);
        }
    }
    assert!(
        measurements.windows(2).all(|pair| pair[0] == pair[1]),
        "{measurements:#?}"
    );
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic fanout work is asserted separately"]
fn same_name_imported_candidate_fanout_reports_adjacent_size_timings() {
    for count in [128, 256] {
        let module = same_name_candidate_fanout_module(count, count, false, false);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "{diagnostics:#?}"
        );
        eprintln!(
            "same-name imported module value fanout: {count} candidates and {count} dependents in {elapsed:?}"
        );
    }
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
        assert!(work.dependency_index_entries <= 8, "{work:#?}");
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

fn competing_fixed_points_module(
    source_order: [usize; 4],
    reverse_a_imports: bool,
    reverse_b_imports: bool,
) -> SurfaceModule {
    let a_imports = if reverse_a_imports {
        "use cb\nuse b\n"
    } else {
        "use b\nuse cb\n"
    };
    let b_imports = if reverse_b_imports {
        "use ca\nuse a\n"
    } else {
        "use a\nuse ca\n"
    };
    let sources = [
        (
            "a",
            SourceFile::new(
                "a.veln",
                format!("{a_imports}pub let a_value: Int = b_value\n"),
            ),
        ),
        (
            "b",
            SourceFile::new(
                "b.veln",
                format!("{b_imports}pub let b_value: Int = a_value\n"),
            ),
        ),
        (
            "ca",
            SourceFile::new("ca.veln", "pub let a_value: Int = 1\n"),
        ),
        (
            "cb",
            SourceFile::new("cb.veln", "pub let b_value: Int = 1\n"),
        ),
    ];
    merged_modules_with_identities(
        source_order
            .into_iter()
            .map(|index| (sources[index].0, sources[index].1.clone()))
            .collect(),
    )
}

#[test]
fn competing_imported_fixed_points_reject_the_same_identities_in_every_permutation() {
    let identity = |module: &str, name: &str| (None, Some(module.to_string()), name.to_string());
    let expected_invalid = [identity("a", "a_value"), identity("b", "b_value")]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut baseline_diagnostics = None;

    for first in 0..4 {
        for second in 0..4 {
            for third in 0..4 {
                for fourth in 0..4 {
                    let source_order = [first, second, third, fourth];
                    if source_order.iter().copied().collect::<BTreeSet<_>>().len() != 4 {
                        continue;
                    }
                    for reverse_a_imports in [false, true] {
                        for reverse_b_imports in [false, true] {
                            let module = competing_fixed_points_module(
                                source_order,
                                reverse_a_imports,
                                reverse_b_imports,
                            );
                            let mut environment =
                                crate::types::TypeEnvironment::from_module(&module);
                            let mut variant_diagnostics =
                                crate::analysis::VariantDiagnosticInterner::default();
                            let analysis = crate::analysis::check_module_values(
                                &module,
                                &mut environment,
                                &mut variant_diagnostics,
                            );

                            assert_eq!(analysis.invalid, expected_invalid, "{source_order:?}");
                            for (module_name, value_name, available) in [
                                ("a", "a_value", false),
                                ("b", "b_value", false),
                                ("ca", "a_value", true),
                                ("cb", "b_value", true),
                            ] {
                                let lookup = environment.module_value_path_in_owner(
                                    &[value_name.to_string()],
                                    Some(module_name),
                                    None,
                                );
                                assert_eq!(
                                    matches!(lookup, crate::types::ModuleValueLookup::Available(_)),
                                    available,
                                    "{source_order:?}: unexpected availability for {module_name}::{value_name}"
                                );
                            }

                            let mut diagnostics = analysis
                                .diagnostics
                                .iter()
                                .map(|diagnostic| {
                                    (
                                        diagnostic.id.clone(),
                                        diagnostic.message.as_str().to_string(),
                                        diagnostic.span.as_ref().map(|span| {
                                            (span.file.as_str().to_string(), span.start.offset)
                                        }),
                                    )
                                })
                                .collect::<Vec<_>>();
                            diagnostics.sort();
                            assert_eq!(
                                diagnostics
                                    .iter()
                                    .filter(|(id, _, _)| id == "name.ambiguous")
                                    .count(),
                                2,
                                "{diagnostics:#?}"
                            );
                            if let Some(baseline) = &baseline_diagnostics {
                                assert_eq!(&diagnostics, baseline, "{source_order:?}");
                            } else {
                                baseline_diagnostics = Some(diagnostics);
                            }
                        }
                    }
                }
            }
        }
    }
}
