use super::super::import_recovery::module_with_bare_module_value_ambiguity;
use super::super::*;
use super::fixtures_and_import_resolution::{
    colliding_qualified_import_work, module_with_bare_imported_values,
    module_with_imported_named_annotations,
};

#[test]
fn bare_imported_module_value_lookup_work_is_bounded_per_reference() {
    fn lookup_work_for(count: usize) -> crate::types::BareModuleValueExposureWork {
        let module = module_with_bare_imported_values(count);
        crate::types::reset_bare_module_value_exposure_work();
        let environment = crate::types::TypeEnvironment::from_module(&module);
        for index in 0..count {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &[format!("value_{index}")],
                    Some("app"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));
        }
        crate::types::take_bare_module_value_exposure_work()
    }

    fn dependency_work_for(count: usize) -> crate::types::BareModuleValueExposureWork {
        let module = module_with_bare_imported_values(count);
        crate::types::reset_bare_module_value_exposure_work();
        let environment = crate::types::TypeEnvironment::from_module(&module);
        for index in 0..count {
            let Some(crate::types::ModuleValueDependency::BareExposure(exposure)) = environment
                .module_value_dependency_in_owner(&[format!("value_{index}")], Some("app"), None)
            else {
                panic!("expected bare imported dependency")
            };
            assert_eq!(
                environment
                    .module_value_candidates_for_bare_exposure(&exposure)
                    .len(),
                1
            );
        }
        crate::types::take_bare_module_value_exposure_work()
    }

    for measure in [lookup_work_for, dependency_work_for] {
        let smaller = measure(64);
        let larger = measure(128);
        assert!(smaller.index_entries >= 64, "{smaller:#?}");
        assert!(smaller.reverse_index_entries >= 128, "{smaller:#?}");
        assert_eq!(smaller.candidate_target_checks, 64, "{smaller:#?}");
        assert_eq!(smaller.materialized_exposures, 64, "{smaller:#?}");
        assert_eq!(smaller.selections, 64, "{smaller:#?}");
        assert_eq!(larger.index_entries, smaller.index_entries * 2);
        assert_eq!(
            larger.reverse_index_entries,
            smaller.reverse_index_entries * 2
        );
        assert_eq!(
            larger.candidate_target_checks,
            smaller.candidate_target_checks * 2
        );
        assert_eq!(
            larger.materialized_exposures,
            smaller.materialized_exposures * 2
        );
        assert_eq!(larger.selections, smaller.selections * 2);
        assert_eq!(larger.cache_probes, smaller.cache_probes * 2);
        assert_eq!(larger.cache_entries, smaller.cache_entries * 2);
        assert_eq!(larger.result_selections, smaller.result_selections * 2);
    }
}

#[test]
fn cached_bare_lookup_does_not_copy_retained_same_name_candidates() {
    fn lookup_work_for(
        candidate_count: usize,
        reference_count: usize,
    ) -> crate::types::BareModuleValueExposureWork {
        let module = module_with_bare_module_value_ambiguity(candidate_count, 0);
        let mut environment = crate::types::TypeEnvironment::from_module(&module);
        for index in 1..candidate_count {
            let identity = (None, Some(format!("values_{index}")), "shared".to_string());
            assert!(
                !environment
                    .set_module_value_availability(&identity, false)
                    .is_empty()
            );
        }
        assert!(matches!(
            environment.module_value_path_in_owner(&["shared".to_string()], Some("app"), None),
            crate::types::ModuleValueLookup::Available(_)
        ));
        let Some(crate::types::ModuleValueDependency::BareExposure(exposure)) = environment
            .module_value_dependency_in_owner(&["shared".to_string()], Some("app"), None)
        else {
            panic!("expected bare imported dependency")
        };
        assert_eq!(
            environment
                .module_value_candidates_for_bare_exposure(&exposure)
                .len(),
            candidate_count
        );

        crate::types::reset_bare_module_value_exposure_work();
        for _ in 0..reference_count {
            assert!(matches!(
                environment.module_value_path_in_owner(&["shared".to_string()], Some("app"), None,),
                crate::types::ModuleValueLookup::Available(_)
            ));
        }
        let work = crate::types::take_bare_module_value_exposure_work();
        assert_eq!(work.cache_probes, reference_count, "{work:#?}");
        assert_eq!(work.result_selections, reference_count, "{work:#?}");
        assert_eq!(work.materialized_exposures, 0, "{work:#?}");
        assert_eq!(work.selections, 0, "{work:#?}");
        work
    }

    let base = lookup_work_for(16, 24);
    let more_candidates = lookup_work_for(32, 24);
    let more_references = lookup_work_for(16, 48);
    let both = lookup_work_for(32, 48);
    for work in [base, more_candidates, more_references, both] {
        assert_eq!(work.exposure_identity_copies, 0, "{work:#?}");
        assert_eq!(work.candidate_target_checks, 0, "{work:#?}");
        assert_eq!(work.cache_entries, 0, "{work:#?}");
    }
    assert_eq!(more_candidates.cache_probes, base.cache_probes);
    assert_eq!(more_candidates.result_selections, base.result_selections);
    assert_eq!(more_references.cache_probes, base.cache_probes * 2);
    assert_eq!(
        more_references.result_selections,
        base.result_selections * 2
    );
    assert_eq!(both.cache_probes, more_references.cache_probes);
    assert_eq!(both.result_selections, more_references.result_selections);
    assert!(
        both.exposure_identity_copies + base.exposure_identity_copies
            <= more_candidates.exposure_identity_copies + more_references.exposure_identity_copies,
        "{base:#?}; {more_candidates:#?}; {more_references:#?}; {both:#?}"
    );
}

#[test]
fn cached_bare_lookup_retains_candidates_across_availability_transitions() {
    let candidate_count = 8;
    let module = module_with_bare_module_value_ambiguity(candidate_count, 0);
    let mut environment = crate::types::TypeEnvironment::from_module(&module);
    let identity = |index| (None, Some(format!("values_{index}")), "shared".to_string());
    for index in 0..candidate_count {
        assert!(
            !environment
                .set_module_value_availability(&identity(index), false)
                .is_empty()
        );
    }
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Missing
    ));
    crate::types::reset_bare_module_value_exposure_work();
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Missing
    ));
    let cached_missing = crate::types::take_bare_module_value_exposure_work();
    assert_eq!(
        cached_missing.exposure_identity_copies, 0,
        "{cached_missing:#?}"
    );
    assert_eq!(
        cached_missing.candidate_target_checks, 0,
        "{cached_missing:#?}"
    );

    assert!(
        !environment
            .set_module_value_availability(&identity(0), true)
            .is_empty()
    );
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Available(_)
    ));
    assert!(
        !environment
            .set_module_value_availability(&identity(1), true)
            .is_empty()
    );
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Ambiguous
    ));
    let Some(crate::types::ModuleValueDependency::BareExposure(exposure)) =
        environment.module_value_dependency_in_owner(&["shared".to_string()], Some("app"), None)
    else {
        panic!("expected bare imported dependency")
    };
    assert_eq!(
        environment
            .module_value_candidates_for_bare_exposure(&exposure)
            .len(),
        candidate_count
    );
}

#[test]
fn bare_module_value_index_counts_misses_and_refreshes_availability() {
    let module = merged_modules_with_identities(vec![
        (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
        ),
        (
            "unused",
            SourceFile::new("unused.veln", "pub let absent: Int = 3\n"),
        ),
        (
            "owner",
            SourceFile::new("owner.veln", "use alpha\nuse beta\n"),
        ),
    ]);
    let mut environment = crate::types::TypeEnvironment::from_module(&module);
    crate::types::reset_bare_module_value_exposure_work();

    assert!(matches!(
        environment.module_value_path_in_owner(&["absent".to_string()], Some("owner"), None),
        crate::types::ModuleValueLookup::Missing
    ));
    let miss = crate::types::take_bare_module_value_exposure_work();
    assert_eq!(miss.candidate_target_checks, 1, "{miss:#?}");
    assert_eq!(miss.cache_probes, 1, "{miss:#?}");
    assert_eq!(miss.selections, 1, "{miss:#?}");
    assert_eq!(miss.result_selections, 1, "{miss:#?}");

    crate::types::reset_bare_module_value_exposure_work();
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("owner"), None),
        crate::types::ModuleValueLookup::Ambiguous
    ));
    let alpha = (None, Some("alpha".to_string()), "shared".to_string());
    assert!(
        !environment
            .set_module_value_availability(&alpha, false)
            .is_empty()
    );
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("owner"), None),
        crate::types::ModuleValueLookup::Available(_)
    ));
    let Some(crate::types::ModuleValueDependency::BareExposure(exposure)) =
        environment.module_value_dependency_in_owner(&["shared".to_string()], Some("owner"), None)
    else {
        panic!("expected bare imported dependency")
    };
    assert_eq!(
        environment
            .module_value_candidates_for_bare_exposure(&exposure)
            .len(),
        2
    );
    assert!(
        !environment
            .set_module_value_availability(&alpha, true)
            .is_empty()
    );
    assert!(matches!(
        environment.module_value_path_in_owner(&["shared".to_string()], Some("owner"), None),
        crate::types::ModuleValueLookup::Ambiguous
    ));
    let refreshed = crate::types::take_bare_module_value_exposure_work();
    assert_eq!(refreshed.cache_probes, 3, "{refreshed:#?}");
    assert_eq!(refreshed.cache_entries, 3, "{refreshed:#?}");
    assert_eq!(refreshed.selections, 4, "{refreshed:#?}");
    assert_eq!(refreshed.result_selections, 3, "{refreshed:#?}");
    assert_eq!(refreshed.candidate_target_checks, 8, "{refreshed:#?}");
}

#[test]
fn qualified_module_value_import_work_includes_construction_and_lookup() {
    fn work_for(count: usize) -> crate::types::QualifiedModuleValueImportWork {
        let module = module_with_bare_imported_values(count);
        crate::types::reset_qualified_module_value_import_work();
        let environment = crate::types::TypeEnvironment::from_module(&module);
        for index in 0..count {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &[format!("values_{index}"), format!("value_{index}"),],
                    Some("app"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));
        }
        crate::types::take_qualified_module_value_import_work()
    }

    let smaller = work_for(64);
    let larger = work_for(128);
    assert!(smaller.index_build >= 64, "{smaller:?}");
    assert!(smaller.target_classification >= 64, "{smaller:?}");
    assert_eq!(smaller.member_materialization, 0, "{smaller:?}");
    assert_eq!(smaller.same_owner_blocking, 64);
    assert_eq!(smaller.selection, 64 * 3);
    assert_eq!(smaller.visibility, 64);
    assert!(larger.index_build <= smaller.index_build * 2 + 4);
    assert!(larger.target_classification <= smaller.target_classification * 2 + 4);
    assert!(larger.member_materialization <= smaller.member_materialization * 2 + 4);
    assert_eq!(larger.same_owner_blocking, smaller.same_owner_blocking * 2);
    assert_eq!(larger.selection, smaller.selection * 2);
    assert_eq!(larger.visibility, smaller.visibility * 2);
}

#[test]
fn colliding_qualified_module_value_import_work_is_bounded_per_reference() {
    let smaller = colliding_qualified_import_work(64, 64);
    let larger = colliding_qualified_import_work(128, 128);
    let smaller_control = colliding_qualified_import_work(64, 1);
    let larger_control = colliding_qualified_import_work(128, 1);

    assert!(
        larger.index_build <= smaller.index_build * 2 + 8,
        "{smaller:?} -> {larger:?}"
    );
    assert!(
        larger.target_classification <= smaller.target_classification * 2 + 8,
        "{smaller:?} -> {larger:?}"
    );
    assert!(
        larger.member_materialization <= smaller.member_materialization * 2 + 8,
        "{smaller:?} -> {larger:?}"
    );
    assert_eq!(larger.same_owner_blocking, smaller.same_owner_blocking * 2);
    assert_eq!(larger.selection, smaller.selection * 2);
    assert_eq!(larger.visibility, 0);

    assert_eq!(smaller_control.same_owner_blocking, 2);
    assert_eq!(larger_control.same_owner_blocking, 2);
    assert_eq!(smaller_control.selection, 4);
    assert_eq!(larger_control.selection, 4);
    assert_eq!(smaller_control.visibility, 0);
    assert_eq!(larger_control.visibility, 0);
}

#[test]
fn imported_named_annotation_lookup_work_is_adjacent_linear() {
    fn work_for(count: usize) -> (usize, usize) {
        let module = module_with_imported_named_annotations(count);
        let environment = crate::types::TypeEnvironment::from_module(&module);
        crate::adt::reset_import_path_comparisons();
        crate::adt::reset_qualified_path_work();
        let diagnostics = crate::pipeline::analyze_surface_module_with_environment_for_test(
            &module,
            &environment,
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "name.unresolved" || !diagnostic.message.contains("Item")
            }),
            "{diagnostics:#?}"
        );
        (
            crate::adt::import_path_comparisons(),
            crate::adt::qualified_path_work(),
        )
    }

    let work = [64, 128, 256].map(work_for);
    for (count, pair) in [64, 128, 256].into_iter().zip(work) {
        assert!(
            pair.0 >= count,
            "closedness lookups must be counted: {pair:?}"
        );
    }
    for adjacent in work.windows(2) {
        let [smaller, larger] = adjacent else {
            unreachable!();
        };
        assert!(
            larger.0 <= smaller.0 * 2 + 16,
            "import work must be adjacent-linear: {smaller:?} -> {larger:?}"
        );
        assert!(
            larger.1 <= smaller.1 * 3,
            "qualified path byte work must remain bounded: {smaller:?} -> {larger:?}"
        );
    }
}
