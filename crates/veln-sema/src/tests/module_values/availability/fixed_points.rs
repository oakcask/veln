use std::collections::BTreeSet;

use super::super::*;

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

fn three_way_competing_fixed_points_module() -> SurfaceModule {
    merged_modules_with_identities(vec![
        (
            "a",
            SourceFile::new(
                "a.veln",
                "use b\nuse fallback_b\npub let a_value: Int = b_value\n",
            ),
        ),
        (
            "b",
            SourceFile::new(
                "b.veln",
                "use c\nuse fallback_c\npub let b_value: Int = c_value\n",
            ),
        ),
        (
            "c",
            SourceFile::new(
                "c.veln",
                "use a\nuse fallback_a\npub let c_value: Int = a_value\n",
            ),
        ),
        (
            "fallback_a",
            SourceFile::new("fallback_a.veln", "pub let a_value: Int = 1\n"),
        ),
        (
            "fallback_b",
            SourceFile::new("fallback_b.veln", "pub let b_value: Int = 2\n"),
        ),
        (
            "fallback_c",
            SourceFile::new("fallback_c.veln", "pub let c_value: Int = 3\n"),
        ),
    ])
}

fn intrinsic_failure_in_cyclic_candidate_component(
    source_order: [usize; 3],
    reverse_imports: bool,
) -> SurfaceModule {
    let b_imports = if reverse_imports {
        "use ca\nuse a\n"
    } else {
        "use a\nuse ca\n"
    };
    let sources = [
        (
            "a",
            SourceFile::new("a.veln", "use b\npub let a_value: Missing = b_value\n"),
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
    ];
    merged_modules_with_identities(
        source_order
            .into_iter()
            .map(|index| (sources[index].0, sources[index].1.clone()))
            .collect(),
    )
}

#[test]
fn unchanged_intrinsic_failure_does_not_reject_its_cyclic_candidate_component() {
    let identity = |module: &str, name: &str| (None, Some(module.to_string()), name.to_string());
    let expected_invalid = [identity("a", "a_value")]
        .into_iter()
        .collect::<BTreeSet<_>>();

    for first in 0..3 {
        for second in 0..3 {
            for third in 0..3 {
                let source_order = [first, second, third];
                if source_order.iter().copied().collect::<BTreeSet<_>>().len() != 3 {
                    continue;
                }
                for reverse_imports in [false, true] {
                    let module = intrinsic_failure_in_cyclic_candidate_component(
                        source_order,
                        reverse_imports,
                    );
                    let mut environment = crate::types::TypeEnvironment::from_module(&module);
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
                        ("b", "b_value", true),
                        ("ca", "a_value", true),
                    ] {
                        let lookup = environment.module_value_path_in_owner(
                            &[value_name.to_string()],
                            Some(module_name),
                            None,
                        );
                        assert_eq!(
                            matches!(lookup, crate::types::ModuleValueLookup::Available(_)),
                            available,
                            "unexpected availability for {module_name}::{value_name}"
                        );
                    }
                    assert_eq!(analysis.diagnostics.len(), 2, "{:#?}", analysis.diagnostics);
                    assert!(analysis.diagnostics.iter().any(|diagnostic| {
                        diagnostic.id == "name.unresolved"
                            && diagnostic.message == "unresolved type `Missing`"
                    }));
                    assert!(analysis.diagnostics.iter().any(|diagnostic| {
                        diagnostic.id == "type.mismatch"
                            && diagnostic.message == "expected `Missing`, but found `Int`"
                    }));
                }
            }
        }
    }
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

#[test]
fn three_way_competing_imported_fixed_point_rejects_the_complete_component() {
    let module = three_way_competing_fixed_points_module();
    let mut environment = crate::types::TypeEnvironment::from_module(&module);
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    let analysis =
        crate::analysis::check_module_values(&module, &mut environment, &mut variant_diagnostics);
    let identity = |module: &str, name: &str| (None, Some(module.to_string()), name.to_string());

    assert_eq!(
        analysis.invalid,
        [
            identity("a", "a_value"),
            identity("b", "b_value"),
            identity("c", "c_value"),
        ]
        .into_iter()
        .collect()
    );
    for (module_name, value_name) in [
        ("fallback_a", "a_value"),
        ("fallback_b", "b_value"),
        ("fallback_c", "c_value"),
    ] {
        assert!(matches!(
            environment.module_value_path_in_owner(
                &[value_name.to_string()],
                Some(module_name),
                None,
            ),
            crate::types::ModuleValueLookup::Available(_)
        ));
    }
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .count(),
        3,
        "{:#?}",
        analysis.diagnostics
    );
}
