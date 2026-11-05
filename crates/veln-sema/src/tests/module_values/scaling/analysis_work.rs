use super::super::*;
use super::fixtures_and_import_resolution::*;

#[test]
#[ignore = "supporting wall-clock evidence; deterministic lookup work is asserted separately"]
fn bare_imported_module_value_lookup_reports_adjacent_size_timings() {
    for count in [150, 300, 600, 1_200] {
        let module = module_with_bare_imported_values(count);
        let started = std::time::Instant::now();
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
        eprintln!(
            "bare imported module value lookup: {count} imports and references in {:?}",
            started.elapsed()
        );
    }
}

fn bare_module_value_exposure_construction_work(
    exports: usize,
    importers: usize,
) -> crate::types::BareModuleValueExposureWork {
    let module = module_with_public_exports_and_importers(exports, importers);
    crate::types::reset_bare_module_value_exposure_work();
    let _environment = crate::types::TypeEnvironment::from_module(&module);
    let work = crate::types::take_bare_module_value_exposure_work();
    assert!(work.index_entries >= exports, "{work:#?}");
    assert_eq!(work.materialized_exposures, 0, "{work:#?}");
    assert_eq!(work.candidate_target_checks, 0, "{work:#?}");
    assert_eq!(work.cache_probes, 0, "{work:#?}");
    assert_eq!(work.cache_entries, 0, "{work:#?}");
    assert_eq!(work.selections, 0, "{work:#?}");
    assert_eq!(work.result_selections, 0, "{work:#?}");
    assert!(
        work.reverse_index_entries >= importers * 2 && work.reverse_index_entries <= importers * 8,
        "{work:#?}"
    );
    work
}

fn assert_materialized_bare_module_value_exposure_work(
    work: &crate::types::BareModuleValueExposureWork,
) {
    assert_eq!(work.materialized_exposures, 1, "{work:#?}");
    assert_eq!(work.candidate_target_checks, 1, "{work:#?}");
    assert_eq!(work.cache_probes, 1, "{work:#?}");
    assert_eq!(work.cache_entries, 1, "{work:#?}");
    assert_eq!(work.selections, 1, "{work:#?}");
    assert_eq!(work.result_selections, 1, "{work:#?}");
}

fn assert_cached_bare_module_value_exposure_work(work: &crate::types::BareModuleValueExposureWork) {
    assert_eq!(work.materialized_exposures, 0, "{work:#?}");
    assert_eq!(work.candidate_target_checks, 0, "{work:#?}");
    assert_eq!(work.cache_probes, 1, "{work:#?}");
    assert_eq!(work.cache_entries, 0, "{work:#?}");
    assert_eq!(work.selections, 0, "{work:#?}");
    assert_eq!(work.result_selections, 1, "{work:#?}");
}

#[test]
fn bare_module_value_exposure_construction_is_additive_and_lookup_is_lazy() {
    let base = bare_module_value_exposure_construction_work(32, 32);
    let more_exports = bare_module_value_exposure_construction_work(64, 32);
    let more_importers = bare_module_value_exposure_construction_work(32, 64);
    let both = bare_module_value_exposure_construction_work(64, 64);
    assert_eq!(
        more_exports.reverse_index_entries,
        base.reverse_index_entries
    );
    assert_eq!(more_exports.index_entries, base.index_entries * 2);
    assert_eq!(
        more_importers.reverse_index_entries,
        base.reverse_index_entries * 2
    );
    assert_eq!(
        both.reverse_index_entries,
        more_importers.reverse_index_entries
    );

    let module = module_with_public_exports_and_importers(64, 64);
    crate::types::reset_bare_module_value_exposure_work();
    let environment = crate::types::TypeEnvironment::from_module(&module);
    assert!(matches!(
        environment.module_value_path_in_owner(&["value_0".to_string()], Some("owner_0"), None,),
        crate::types::ModuleValueLookup::Available(_)
    ));
    let first = crate::types::take_bare_module_value_exposure_work();
    assert_materialized_bare_module_value_exposure_work(&first);
    assert!(
        first.reverse_index_entries >= 128 && first.reverse_index_entries <= 512,
        "{first:#?}"
    );

    crate::types::reset_bare_module_value_exposure_work();
    assert!(matches!(
        environment.module_value_path_in_owner(&["value_0".to_string()], Some("owner_0"), None,),
        crate::types::ModuleValueLookup::Available(_)
    ));
    let cached = crate::types::take_bare_module_value_exposure_work();
    assert_cached_bare_module_value_exposure_work(&cached);

    assert!(matches!(
        environment.module_value_path_in_owner(&["value_1".to_string()], Some("owner_0"), None,),
        crate::types::ModuleValueLookup::Available(_)
    ));
    let second = crate::types::take_bare_module_value_exposure_work();
    assert_materialized_bare_module_value_exposure_work(&second);
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic import work is asserted separately"]
fn imported_named_annotation_analysis_reports_adjacent_size_timings() {
    for count in [100, 200, 400, 800] {
        let module = module_with_imported_named_annotations(count);
        let environment = crate::types::TypeEnvironment::from_module(&module);
        let started = std::time::Instant::now();
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
        eprintln!(
            "imported named annotations: {count} imports and annotations in {:?}",
            started.elapsed()
        );
    }
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic collision work is asserted separately"]
fn qualified_module_value_colliding_imports_report_adjacent_size_timings() {
    for count in [128, 256, 512, 1_024] {
        let started = std::time::Instant::now();
        let work = colliding_qualified_import_work(count, count);
        eprintln!(
            "qualified module value collisions: {count} imports and references in {:?}, work={work:?}",
            started.elapsed()
        );

        let started = std::time::Instant::now();
        let control = colliding_qualified_import_work(count, 1);
        eprintln!(
            "qualified module value collision control: {count} imports and one reference in {:?}, work={control:?}",
            started.elapsed()
        );
    }
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic semantics are asserted separately"]
fn module_value_analysis_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_values(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        eprintln!("module value analysis: {count} declarations in {elapsed:?}");
    }
}

#[test]
#[ignore = "supporting wall-clock evidence for failure-heavy annotation scaling"]
fn module_value_unresolved_annotation_analysis_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_unresolved_value_annotations(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert!(diagnostics.len() >= count, "{diagnostics:#?}");
        eprintln!("module value unresolved annotations: {count} declarations in {elapsed:?}");
    }
}

#[test]
#[ignore = "supporting wall-clock evidence for dependency graph scaling"]
fn module_value_dependency_analysis_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_value_chain(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        eprintln!("module value dependencies: {count} declarations in {elapsed:?}");
    }
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic availability work is asserted separately"]
fn imported_module_value_invalidation_chain_reports_adjacent_size_timings() {
    for count in [128, 256, 512] {
        let module = module_with_imported_invalidation_fanout(count, count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert!(diagnostics.len() >= count, "{diagnostics:#?}");
        eprintln!(
            "imported module value invalidation fanout: {count} chain and {count} unrelated declarations in {elapsed:?}"
        );
    }
}

#[test]
fn imported_module_value_invalidation_work_is_adjacent_linear() {
    fn work_for(count: usize) -> crate::analysis::ModuleValueAvailabilityWork {
        let module = module_with_imported_invalidation_chain(count);
        reset_module_value_availability_work();
        let diagnostics = analyze_surface_module(&module);
        let work = take_module_value_availability_work();

        assert!(diagnostics.len() >= count, "{diagnostics:#?}");
        assert!(work.semantic_visits <= count * 4 + 4, "{work:#?}");
        assert!(work.owner_index_entries <= count * 2 + 2, "{work:#?}");
        assert!(work.owner_recheck_lookups <= count + 1, "{work:#?}");
        assert!(work.retained_history_insertions <= count + 1, "{work:#?}");
        assert!(work.availability_state_steps <= count * 4 + 4, "{work:#?}");
        assert_eq!(work.availability_waves, 1, "{work:#?}");
        assert!(
            work.full_exposure_rebuild_entries <= count * 2 + count.saturating_sub(1) + 4,
            "{work:#?}"
        );
        assert!(work.availability_updates <= count + 1, "{work:#?}");
        assert!(work.peak_pending_owners <= count * 2, "{work:#?}");
        assert!(
            work.peak_retained_availability_identities <= count * 2,
            "{work:#?}"
        );
        work
    }

    let smaller = work_for(64);
    let larger = work_for(128);
    assert!(
        larger.semantic_visits <= smaller.semantic_visits * 2 + 4,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.full_exposure_rebuild_entries <= smaller.full_exposure_rebuild_entries * 2 + 4,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.owner_index_entries <= smaller.owner_index_entries * 2 + 4,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.owner_recheck_lookups <= smaller.owner_recheck_lookups * 2 + 1,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.retained_history_insertions <= smaller.retained_history_insertions * 2 + 1,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.availability_state_steps <= smaller.availability_state_steps * 2 + 4,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.availability_waves <= smaller.availability_waves * 2 + 1,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.availability_updates <= smaller.availability_updates * 2 + 1,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.peak_pending_owners <= smaller.peak_pending_owners * 2 + 2,
        "{smaller:#?} -> {larger:#?}"
    );
    assert!(
        larger.peak_retained_availability_identities
            <= smaller.peak_retained_availability_identities * 2,
        "{smaller:#?} -> {larger:#?}"
    );
}

#[test]
fn module_value_cycle_component_storage_is_adjacent_linear() {
    fn work_for(count: usize) -> crate::analysis::ModuleValueAvailabilityWork {
        let module = module_with_value_ring(count);
        reset_module_value_availability_work();
        let diagnostics = analyze_surface_module(&module);
        let work = take_module_value_availability_work();

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert_eq!(work.retained_cyclic_component_identities, count * 2);
        work
    }

    let smaller = work_for(256);
    let larger = work_for(512);
    assert_eq!(
        larger.retained_cyclic_component_identities,
        smaller.retained_cyclic_component_identities * 2,
        "{smaller:#?} -> {larger:#?}"
    );
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic cycle storage is asserted separately"]
fn module_value_cycle_component_storage_reports_adjacent_size_timings() {
    for count in [512, 1_024, 2_048] {
        let module = module_with_value_ring(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        eprintln!("module value cycle: {count} declarations in {elapsed:?}");
    }
}

#[test]
fn imported_invalidation_rechecks_only_dependent_declarations() {
    fn work_for(
        chain_count: usize,
        unrelated_value_count: usize,
    ) -> crate::analysis::ModuleValueAvailabilityWork {
        let module = module_with_imported_invalidation_fanout(chain_count, unrelated_value_count);
        reset_module_value_availability_work();
        let diagnostics = analyze_surface_module(&module);
        let work = take_module_value_availability_work();

        let declaration_count = chain_count * 2 + unrelated_value_count;
        let dependency_edges = chain_count.saturating_sub(1) + chain_count;
        let compressed_dependency_entries = dependency_edges + chain_count;
        assert!(diagnostics.len() >= chain_count * 2, "{diagnostics:#?}");
        assert!(
            work.semantic_visits <= declaration_count * 2 + dependency_edges + 4,
            "{work:#?}"
        );
        assert!(
            work.dependency_index_entries <= compressed_dependency_entries + 1,
            "{work:#?}"
        );
        assert!(
            work.declaration_recheck_lookups <= dependency_edges + 1,
            "{work:#?}"
        );
        work
    }

    let base = work_for(32, 32);
    let more_chain = work_for(64, 32);
    let more_unrelated = work_for(32, 64);
    let both = work_for(64, 64);

    assert!(
        more_unrelated.declaration_recheck_lookups <= base.declaration_recheck_lookups + 1,
        "{base:#?} -> {more_unrelated:#?}"
    );
    assert!(
        both.declaration_recheck_lookups <= more_chain.declaration_recheck_lookups + 1,
        "{more_chain:#?} -> {both:#?}"
    );
    assert!(
        more_chain.declaration_recheck_lookups <= base.declaration_recheck_lookups * 2 + 2,
        "{base:#?} -> {more_chain:#?}"
    );
}

#[test]
fn module_value_lowering_import_preparation_is_additive_in_values_and_imports() {
    fn work_for(
        value_count: usize,
        import_count: usize,
    ) -> crate::lowering::ModuleValueImportPreparationWork {
        let module = module_with_literal_values_and_imports(value_count, import_count);
        assert_eq!(module.module_values.len(), value_count);
        assert_eq!(module.uses.len(), import_count);
        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != veln_diagnostics::Severity::Error),
            "{diagnostics:#?}"
        );

        crate::lowering::reset_module_value_import_preparation_work();
        let lowered = lower_analyzed_surface_module(&module, diagnostics);
        let work = crate::lowering::take_module_value_import_preparation_work();

        let core = lowered
            .core
            .expect("literal values should reach checked core");
        let ir = lowered.ir.expect("literal values should reach typed IR");
        assert_eq!(core.module_values.len(), value_count);
        assert_eq!(ir.module_values.len(), value_count);
        assert!(core.module_values.iter().any(|value| {
            value.identity.package.as_deref() == Some("owner_0/pkg")
                && value.identity.module.as_deref() == Some("owner_0/pkg::shared")
        }));
        assert!(core.module_values.iter().any(|value| {
            value.identity.package.as_deref() == Some("owner_1/pkg")
                && value.identity.module.as_deref() == Some("owner_1/pkg::shared")
        }));
        assert_eq!(work.scanned_imports, import_count, "{work:#?}");
        assert_eq!(work.retained_import_clones, import_count, "{work:#?}");
        assert_eq!(work.owner_lookups, value_count, "{work:#?}");
        work
    }

    const N: usize = 32;
    let base = work_for(N, N);
    let more_values = work_for(2 * N, N);
    let more_imports = work_for(N, 2 * N);
    let both = work_for(2 * N, 2 * N);

    assert_eq!(more_values.total() - base.total(), N);
    assert_eq!(more_imports.total() - base.total(), 2 * N);
    assert_eq!(
        both.total(),
        more_values.total() + more_imports.total() - base.total()
    );
}

#[test]
fn module_value_annotation_dedup_work_grows_linearly_with_failures() {
    fn work_for(count: usize) -> (usize, usize) {
        let module = module_with_unresolved_value_annotations(count);
        reset_module_value_annotation_dedup_work();
        let diagnostics = analyze_surface_module(&module);
        let work = take_module_value_annotation_dedup_work();

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            count,
            "{diagnostics:#?}"
        );
        assert_eq!(work.path_lookups, count * 2, "{work:#?}");
        assert_eq!(work.indexed_diagnostics, count * 2, "{work:#?}");
        (work.path_lookups, work.indexed_diagnostics)
    }

    const N: usize = 64;
    let work_n = work_for(N);
    let work_2n = work_for(2 * N);
    assert_eq!(work_2n.0, 2 * work_n.0);
    assert_eq!(work_2n.1, 2 * work_n.1);
}
