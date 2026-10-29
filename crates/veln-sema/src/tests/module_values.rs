use super::*;

mod public_annotation_aliases;

fn module_with_values(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Int = {index}\n"))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_with_value_chain(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| {
                if index == 0 {
                    "let value_0: Int = 0\n".to_string()
                } else {
                    format!("let value_{index}: Int = value_{}\n", index - 1)
                }
            })
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_with_value_ring(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "ring.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Int = value_{}\n", (index + 1) % count))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_with_imported_invalidation_chain(count: usize) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for index in 0..count {
        let module_name = format!("chain_{index}");
        let text = if index == 0 {
            "pub let value_0: Missing = 0\n".to_string()
        } else {
            format!(
                "use chain_{}\npub let value_{index}: Int = value_{}\n",
                index - 1,
                index - 1,
            )
        };
        let source = SourceFile::new(format!("chain_{index}.veln"), text);
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module = module.module;
        merged.uses.extend(module.uses);
        merged.module_values.extend(module.module_values);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

fn module_with_imported_invalidation_fanout(
    chain_count: usize,
    unrelated_value_count: usize,
) -> SurfaceModule {
    let mut merged = module_with_imported_invalidation_chain(chain_count);
    let imports = (0..chain_count)
        .map(|index| format!("use chain_{index}\n"))
        .collect::<String>();
    let dependent_values = (0..chain_count)
        .map(|index| format!("let observed_{index}: Int = chain_{index}::value_{index}\n"))
        .collect::<String>();
    let unrelated_values = (0..unrelated_value_count)
        .map(|index| format!("let unrelated_{index}: Int = {index}\n"))
        .collect::<String>();
    let source = SourceFile::new(
        "a_fanout.veln",
        format!("{imports}{dependent_values}{unrelated_values}"),
    );
    let fanout = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "a_fanout".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.uses.extend(fanout.uses);
    merged.module_values.extend(fanout.module_values);
    merged.invalid_names.extend(fanout.invalid_names);
    merged
}

fn module_with_unresolved_value_annotations(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Missing{index} = {index}\n"))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_with_bare_imported_values(count: usize) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for index in 0..count {
        let module_name = format!("values_{index}");
        let source = SourceFile::new(
            format!("values_{index}.veln"),
            format!("pub let value_{index}: Int = {index}\n"),
        );
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module_values.extend(module.module_values);
    }
    let source = SourceFile::new(
        "app.veln",
        (0..count)
            .map(|index| format!("use values_{index}\n"))
            .collect::<String>(),
    );
    let app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "app".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.uses.extend(app.uses);
    merged
}

fn module_with_public_exports_and_importers(
    export_count: usize,
    importer_count: usize,
) -> SurfaceModule {
    let export_source = SourceFile::new(
        "values.veln",
        (0..export_count)
            .map(|index| format!("pub let value_{index}: Int = {index}\n"))
            .collect::<String>(),
    );
    let mut merged = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&export_source).tree,
        "values".to_string(),
        export_source.span(veln_source::TextRange::new(0, 0)),
    );
    for index in 0..importer_count {
        let source = SourceFile::new(format!("owner_{index}.veln"), "use values\n");
        let owner = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            format!("owner_{index}"),
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.uses.extend(owner.uses);
    }
    merged
}

fn module_with_imported_named_annotations(count: usize) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for index in 0..count {
        let module_name = format!("types_{index}");
        let source = SourceFile::new(
            format!("types_{index}.veln"),
            "pub type Item\n  Only\nend\n",
        );
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.types.extend(module.types);
        let consumer_name = format!("app_{index}");
        let consumer_source = SourceFile::new(
            format!("app_{index}.veln"),
            format!("use types_{index}\nlet value_{index}: types_{index}::Item = 0\n"),
        );
        let consumer = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&consumer_source).tree,
            consumer_name,
            consumer_source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module = consumer.module;
        merged.uses.extend(consumer.uses);
        merged.module_values.extend(consumer.module_values);
    }
    merged
}

fn module_with_colliding_qualified_imports(count: usize) -> SurfaceModule {
    let imports = (0..count)
        .map(|index| format!("use shared from \"package_{index}\"\n"))
        .collect::<String>();
    let owner_source = SourceFile::new("app.veln", imports);
    let mut merged = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&owner_source).tree,
        "owner/pkg".to_string(),
        "app".to_string(),
        owner_source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut merged.uses {
        use_decl.name = "shared".to_string();
    }
    for index in 0..count {
        let declaration_count = if index == 0 { 2 } else { 1 };
        for declaration_index in 0..declaration_count {
            let source = SourceFile::new(
                format!("package_{index}/shared_{declaration_index}.veln"),
                "pub let answer: Int = 1\n",
            );
            let mut module = veln_ast::lower_surface_ast_with_package_module_identity(
                &parse(&source).tree,
                format!("package_{index}"),
                "shared".to_string(),
                source.span(veln_source::TextRange::new(0, 0)),
            );
            for value in &mut module.module_values {
                value.module_name = Some("shared".to_string());
            }
            merged.module_values.extend(module.module_values);
        }
    }
    merged
}

fn colliding_qualified_import_work(
    count: usize,
    lookup_count: usize,
) -> crate::types::QualifiedModuleValueImportWork {
    let module = module_with_colliding_qualified_imports(count);
    crate::types::reset_qualified_module_value_import_work();
    let environment = crate::types::TypeEnvironment::from_module(&module);
    for _ in 0..lookup_count {
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["shared".to_string(), "answer".to_string()],
                Some("owner/pkg::app"),
                Some("owner/pkg"),
            ),
            crate::types::ModuleValueLookup::Ambiguous
        ));
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["shared".to_string(), "missing".to_string()],
                Some("owner/pkg::app"),
                Some("owner/pkg"),
            ),
            crate::types::ModuleValueLookup::Ambiguous
        ));
    }
    crate::types::take_qualified_module_value_import_work()
}

#[test]
fn ambiguous_exact_module_value_import_is_not_selected_by_member_presence() {
    for imports in [
        "use shared from \"one/pkg\"\nuse shared from \"two/pkg\"\n",
        "use shared from \"two/pkg\"\nuse shared from \"one/pkg\"\n",
    ] {
        let owner_source = SourceFile::new("app.veln", imports);
        let mut merged = veln_ast::lower_surface_ast_with_package_module_identity(
            &parse(&owner_source).tree,
            "app/pkg".to_string(),
            "app".to_string(),
            owner_source.span(veln_source::TextRange::new(0, 0)),
        );
        for use_decl in &mut merged.uses {
            use_decl.name = "shared".to_string();
        }
        for (package, declaration) in [
            ("one/pkg", "pub let answer: Int = 1\n"),
            ("two/pkg", "pub let other: Int = 2\n"),
        ] {
            let source = SourceFile::new(format!("{package}/shared.veln"), declaration);
            let module = veln_ast::lower_surface_ast_with_package_module_identity(
                &parse(&source).tree,
                package.to_string(),
                "shared".to_string(),
                source.span(veln_source::TextRange::new(0, 0)),
            );
            merged.module_values.extend(module.module_values);
        }

        let environment = crate::types::TypeEnvironment::from_module(&merged);
        for member in ["answer", "other", "missing"] {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["shared".to_string(), member.to_string()],
                    Some("app/pkg::app"),
                    Some("app/pkg"),
                ),
                crate::types::ModuleValueLookup::Ambiguous
            ));
        }
    }
}

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
#[ignore = "supporting wall-clock evidence; deterministic scaling is asserted separately"]
fn module_value_gate_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_values(count);
        let diagnostics = analyze_surface_module(&module);
        let started = std::time::Instant::now();
        let lowered = lower_analyzed_surface_module(&module, diagnostics);
        let elapsed = started.elapsed();

        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
        eprintln!("module value gate: {count} declarations in {elapsed:?}");
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

        assert_eq!(diagnostics.len(), count, "{diagnostics:#?}");
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

        assert_eq!(diagnostics.len(), count, "{diagnostics:#?}");
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
        assert!(
            (count / 2..=count + 1).contains(&work.availability_waves),
            "{work:#?}"
        );
        assert!(
            work.full_exposure_rebuild_entries <= count * 2 + count.saturating_sub(1) + 4,
            "{work:#?}"
        );
        assert!(work.availability_updates <= count + 1, "{work:#?}");
        assert!(work.peak_pending_owners <= count, "{work:#?}");
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
fn module_value_gate_work_grows_linearly_with_declarations() {
    fn work_for(count: usize) -> crate::pipeline::ModuleValueGateWork {
        let module = module_with_values(count);
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(diagnostics.len(), count, "{diagnostics:#?}");

        reset_module_value_gate_work();
        let lowered = lower_analyzed_surface_module(&module, diagnostics);
        let work = take_module_value_gate_work();

        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
        assert_eq!(work.indexed_values, count, "{work:#?}");
        assert_eq!(work.scanned_diagnostics, count, "{work:#?}");
        assert_eq!(work.span_index_lookups, count, "{work:#?}");
        assert_eq!(work.emitted_canonical_diagnostics, count, "{work:#?}");
        work
    }

    const N: usize = 64;
    let work_n = work_for(N);
    let work_2n = work_for(2 * N);
    assert_eq!(work_2n.total(), 2 * work_n.total());
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

#[test]
fn module_values_block_checked_core_and_typed_ir_at_the_shared_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "let backup: Int = 7\n",
            "let final_value: Int = 9\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    assert_eq!(module.module_values.len(), 3);
    assert_eq!(module.functions.len(), 1);

    let lowered = lower_checked_surface_module(&module);
    assert_module_value_gate_diagnostics(&lowered);

    let unrelated = Diagnostic::new(
        "caller.unrelated",
        veln_diagnostics::Severity::Warning,
        DiagnosticKind::Module,
        "caller-owned unrelated warning",
        None,
        veln_diagnostics::JsonValue::Null,
    );
    let lowered_with_incomplete_diagnostics =
        lower_analyzed_surface_module(&module, vec![unrelated.clone()]);
    assert_incomplete_diagnostics_are_completed(&lowered_with_incomplete_diagnostics);
    assert_colliding_gate_diagnostics_are_replaced(&module, unrelated);
}

fn assert_module_value_gate_diagnostics(lowered: &LoweredSurfaceModule) {
    let diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 3, "{:#?}", lowered.diagnostics);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "module value declaration `answer` is not executable",
            "module value declaration `backup` is not executable",
            "module value declaration `final_value` is not executable",
        ]
    );
    let diagnostic = diagnostics[0];
    assert_eq!(
        diagnostic.message,
        "module value declaration `answer` is not executable"
    );
    assert_eq!(diagnostic.kind, DiagnosticKind::Module);
    assert_eq!(diagnostic.severity, veln_diagnostics::Severity::Error);
    assert_eq!(
        diagnostic.details,
        veln_diagnostics::JsonValue::object([
            ("phase", veln_diagnostics::JsonValue::string("module")),
            ("field", veln_diagnostics::JsonValue::string("module_value"),),
            ("name", veln_diagnostics::JsonValue::string("answer")),
        ])
    );
    assert_eq!(
        diagnostic.related,
        vec![veln_diagnostics::JsonValue::object([(
            "message",
            veln_diagnostics::JsonValue::string(
                "The declaration is semantically checked, but checked-core lowering and code generation do not support module values yet.",
            ),
        )])]
    );
    assert_diagnostic_span(diagnostic, 1, 1, 2, 1);
    assert_diagnostic_span(diagnostics[1], 2, 1, 3, 1);
    assert_diagnostic_span(diagnostics[2], 3, 1, 4, 1);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

fn assert_incomplete_diagnostics_are_completed(lowered: &LoweredSurfaceModule) {
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
            .count(),
        3
    );
    assert_eq!(lowered.diagnostics[0].id, "caller.unrelated");
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

fn assert_colliding_gate_diagnostics_are_replaced(module: &SurfaceModule, unrelated: Diagnostic) {
    let colliding_warning = |index: usize| {
        Diagnostic::new(
            "module.value_not_executable",
            veln_diagnostics::Severity::Warning,
            DiagnosticKind::Module,
            "caller-owned warning",
            Some(module.module_values[index].span.clone()),
            veln_diagnostics::JsonValue::Null,
        )
    };
    let lowered_with_colliding_warning = lower_analyzed_surface_module(
        module,
        vec![
            unrelated,
            colliding_warning(1),
            colliding_warning(1),
            colliding_warning(0),
        ],
    );
    let blockers = lowered_with_colliding_warning
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
        .collect::<Vec<_>>();
    assert_eq!(
        blockers.len(),
        3,
        "{:#?}",
        lowered_with_colliding_warning.diagnostics
    );
    assert_eq!(
        blockers
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "module value declaration `backup` is not executable",
            "module value declaration `answer` is not executable",
            "module value declaration `final_value` is not executable",
        ]
    );
    assert!(
        blockers
            .iter()
            .all(|diagnostic| diagnostic.severity == veln_diagnostics::Severity::Error)
    );
    assert_eq!(
        lowered_with_colliding_warning
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "caller.unrelated")
            .count(),
        1
    );
    assert!(lowered_with_colliding_warning.core.is_none());
    assert!(lowered_with_colliding_warning.ir.is_none());
}

#[test]
fn module_value_initializers_accept_finite_data_and_forward_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "  Active(count: Int)\n",
            "end\n",
            "let defaults: { state: State, attempts: Int } = { state: initial, attempts: -3 }\n",
            "let initial: State = Active(3)\n",
            "fn attempts() -> Int\n",
            "  defaults.attempts\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "module.value_not_executable"
                || diagnostic.severity != veln_diagnostics::Severity::Error
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_initializers_accept_the_complete_finite_data_surface() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "  Active(count: Int)\n",
            "end\n",
            "let boolean: Bool = true\n",
            "let text: String = \"ready\"\n",
            "let nothing: () = ()\n",
            "let grouped: Bool = (boolean)\n",
            "let idle: State = Idle\n",
            "let active: State = Active(later)\n",
            "let optional: Option<Int> = Some(later)\n",
            "let outcome: Result<Int, String> = Ok(later)\n",
            "let items: List<Int> = Cons(later, Nil)\n",
            "let nested: { state: State, data: { optional: Option<Int>, items: List<Int> } } = { state: Active(later), data: { optional: Some(later), items: Cons(later, Nil) } }\n",
            "let later: Int = 3\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == veln_diagnostics::Severity::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), module.module_values.len(), "{diagnostics:#?}");
    assert!(
        errors
            .iter()
            .all(|diagnostic| diagnostic.id == "module.value_not_executable"),
        "{diagnostics:#?}"
    );
}

#[test]
fn contextual_empty_dictionaries_are_not_admitted_as_empty_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Container\n",
            "  Wrap(items: Dict<Int, Int>)\n",
            "end\n",
            "pub type ContainerAlias = Container\n",
            "let direct: Dict<Int, Int> = {}\n",
            "let nested: { items: Dict<Int, Int> } = { items: {} }\n",
            "let payload: Container = Wrap({})\n",
            "let mismatched_payload: Int = Wrap({})\n",
            "let aliased: ContainerAlias = Wrap({})\n",
            "let structural: {} = {}\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 5, "{diagnostics:#?}");
    assert_eq!(
        unsupported
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().expect("unsupported span");
                (span.start.line, span.end.offset - span.start.offset)
            })
            .collect::<Vec<_>>(),
        vec![(5, 2), (6, 2), (7, 2), (8, 2), (9, 2)]
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "value.module_initializer_unsupported"
                || diagnostic.span.as_ref() != Some(&module.module_values[5].initializer.span)
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_same_module_values_resolve_across_declaring_sources() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new(
                "first.veln",
                concat!(
                    "let bare: Int = later\n",
                    "let leaf: Int = config::later\n",
                    "let complete: Int = app::config::later\n",
                ),
            ),
        ),
        (
            "app::config",
            SourceFile::new("later.veln", "let later: Int = 2\n"),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "module.value_not_executable"
                || diagnostic.severity != veln_diagnostics::Severity::Error
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_import_precedes_same_module_leaf_value_qualification() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use config\n",
                    "let answer: Int = config::answer()\n",
                    "fn calls_import() -> Int\n",
                    "  config::answer()\n",
                    "end\n",
                ),
            ),
        ),
        (
            "config",
            SourceFile::new(
                "config.veln",
                concat!("pub fn answer() -> Int\n", "  1\n", "end\n"),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "type.not_callable" | "value.module_initializer_cycle" | "name.unresolved"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn same_module_leaf_qualification_rejects_unwritten_prefixes() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new("first.veln", "let first: Int = bogus::config::later\n"),
        ),
        (
            "app::config",
            SourceFile::new("later.veln", "let later: Int = 2\n"),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `bogus::config::later`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_initializers_report_smallest_unsupported_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compute() -> Int\n",
            "  1\n",
            "end\n",
            "let computed: Int = compute()\n",
            "let nested: { value: Int } = { value: 1 + 2 }\n",
            "let floating: Float = 1.0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert_diagnostic_span(unsupported[0], 4, 21, 4, 30);
    assert_diagnostic_span(unsupported[1], 5, 39, 5, 44);
    assert_diagnostic_span(unsupported[2], 6, 23, 6, 26);
    for (index, (diagnostic, name)) in unsupported
        .iter()
        .zip(["computed", "nested", "floating"])
        .enumerate()
    {
        let detail = |field: &str| {
            diagnostic.details.as_object().and_then(|entries| {
                entries
                    .iter()
                    .find(|(entry, _)| entry == field)
                    .and_then(|(_, value)| value.as_text())
            })
        };
        assert_eq!(detail("phase"), Some("type_check"));
        assert_eq!(detail("name"), Some(name));
        assert_eq!(
            diagnostic.related,
            vec![
                veln_diagnostics::JsonValue::object([
                    (
                        "kind",
                        veln_diagnostics::JsonValue::string("module_value_declaration"),
                    ),
                    (
                        "message",
                        veln_diagnostics::JsonValue::string(format!(
                            "Module value `{name}` is declared here."
                        )),
                    ),
                    (
                        "span",
                        crate::diagnostics::span_json(&module.module_values[index].span),
                    ),
                ]),
                veln_diagnostics::JsonValue::object([
                    (
                        "kind",
                        veln_diagnostics::JsonValue::string("permitted_initializers"),
                    ),
                    (
                        "message",
                        veln_diagnostics::JsonValue::string(
                            "Use literals, records, ADT constructors, or eligible same-module or imported module values.",
                        ),
                    ),
                ]),
            ]
        );
    }
}

#[test]
fn imported_module_values_follow_bare_qualified_and_alias_exposure() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "let bare: Int = answer\n",
                    "let qualified: Int = values::answer\n",
                    "fn read() -> Int\n",
                    "  values::answer\n",
                    "end\n",
                ),
            ),
        ),
        (
            "aliased",
            SourceFile::new(
                "aliased.veln",
                concat!(
                    "use values as config\n",
                    "let qualified: Int = config::answer\n",
                    "fn bare_is_hidden() -> Int\n",
                    "  answer\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `answer`"
        }),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "value.module_initializer_unsupported"
                && diagnostic.id != "type.mismatch"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn bare_and_qualified_paths_select_the_same_module_value_declaration() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        ("app", SourceFile::new("app.veln", "use values\n")),
    ]);
    let environment = crate::types::TypeEnvironment::from_module(&module);
    let bare =
        match environment.module_value_path_in_owner(&["answer".to_string()], Some("app"), None) {
            crate::types::ModuleValueLookup::Available(value) => value,
            _ => panic!("expected bare declaration"),
        };
    let qualified = match environment.module_value_path_in_owner(
        &["values".to_string(), "answer".to_string()],
        Some("app"),
        None,
    ) {
        crate::types::ModuleValueLookup::Available(value) => value,
        _ => panic!("expected qualified declaration"),
    };

    assert!(std::ptr::eq(bare, qualified));
}

#[test]
fn qualified_module_value_exact_path_no_fallback_is_stable() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let alpha_exact = (
            "alpha::exact",
            SourceFile::new("alpha/exact.veln", "pub let other: Int = 1\n"),
        );
        let beta_exact = (
            "beta::exact",
            SourceFile::new("beta/exact.veln", "pub let answer: Int = 1\n"),
        );
        let imports = if reverse_imports {
            "use beta::exact\nuse alpha::exact\n"
        } else {
            "use alpha::exact\nuse beta::exact\n"
        };
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!("{imports}fn exact() -> Int\n  alpha::exact::answer\nend\n"),
            ),
        );
        let sources = if reverse_sources {
            vec![beta_exact, alpha_exact, app]
        } else {
            vec![alpha_exact, beta_exact, app]
        };
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| {
                matches!(diagnostic.id.as_str(), "name.ambiguous" | "name.unresolved")
            })
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(expected.len(), 1, "{expected:#?}");
    assert!(expected.iter().any(|(id, message)| {
        id == "name.unresolved" && message.contains("alpha::exact::answer")
    }));
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
}

#[test]
fn quarantined_imports_do_not_create_qualified_module_value_ambiguity() {
    fn assert_case(
        first_import: &str,
        second_import: &str,
        qualified_value: &str,
        reverse_sources: bool,
    ) {
        let values = (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        );
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let answer: Int = 1\n"),
        );
        let beta = (
            "beta",
            SourceFile::new("beta.veln", "pub let answer: Int = 2\n"),
        );
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!(
                    "{first_import}\n{second_import}\nfn read() -> Int\n  {qualified_value}\nend\n"
                ),
            ),
        );
        let sources = if reverse_sources {
            vec![app, beta, alpha, values]
        } else {
            vec![values, alpha, beta, app]
        };
        let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "{diagnostics:#?}"
        );
    }

    for reverse_sources in [false, true] {
        assert_case(
            "use values",
            "use values as config",
            "values::answer",
            reverse_sources,
        );
        assert_case(
            "use values as config",
            "use values",
            "values::answer",
            reverse_sources,
        );
        assert_case(
            "use alpha as shared",
            "use beta as shared",
            "shared::answer",
            reverse_sources,
        );
        assert_case(
            "use beta as shared",
            "use alpha as shared",
            "shared::answer",
            reverse_sources,
        );
    }
}

fn analyze_quarantined_owner_module_value_fallback(
    reverse_sources: bool,
    reverse_imports: bool,
) -> Vec<(String, veln_diagnostics::DiagnosticText)> {
    let imports = if reverse_imports {
        concat!(
            "use values as config from \"dep/pkg\"\n",
            "use values from \"dep/pkg\"\n",
        )
    } else {
        concat!(
            "use values from \"dep/pkg\"\n",
            "use values as config from \"dep/pkg\"\n",
        )
    };
    let owner_source = SourceFile::new(
        "values.veln",
        format!("{imports}let answer: Int = 1\nfn read() -> Int\n  values::answer\nend\n"),
    );
    let owner = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&owner_source).tree,
        "owner/pkg".to_string(),
        "values".to_string(),
        owner_source.span(veln_source::TextRange::new(0, 0)),
    );
    let dependency_source = SourceFile::new(
        "vendor/dep/values.veln",
        "pub let answer: String = \"dep\"\n",
    );
    let dependency = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&dependency_source).tree,
        "dep/pkg".to_string(),
        "values".to_string(),
        dependency_source.span(veln_source::TextRange::new(0, 0)),
    );
    let (mut merged, other) = if reverse_sources {
        (dependency, owner)
    } else {
        (owner, dependency)
    };
    merged.uses.extend(other.uses);
    merged.module_values.extend(other.module_values);
    merged.functions.extend(other.functions);

    analyze_surface_module(&merged)
        .into_iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.duplicate" | "name.unresolved" | "name.ambiguous" | "type.mismatch"
            )
        })
        .map(|diagnostic| (diagnostic.id, diagnostic.message))
        .collect::<Vec<_>>()
}

#[test]
fn quarantined_exact_import_blocks_same_owner_module_value_fallback() {
    let expected = analyze_quarantined_owner_module_value_fallback(false, false);
    assert_eq!(
        expected
            .iter()
            .filter(|(id, _)| id == "name.duplicate")
            .count(),
        1,
        "{expected:#?}"
    );
    assert_eq!(
        expected
            .iter()
            .filter(|(id, message)| id == "name.unresolved" && message.contains("values::answer"))
            .count(),
        1,
        "{expected:#?}"
    );
    assert!(
        expected
            .iter()
            .all(|(id, _)| !matches!(id.as_str(), "name.ambiguous" | "type.mismatch")),
        "{expected:#?}"
    );
    assert_eq!(
        analyze_quarantined_owner_module_value_fallback(true, false),
        expected
    );
    assert_eq!(
        analyze_quarantined_owner_module_value_fallback(false, true),
        expected
    );
    assert_eq!(
        analyze_quarantined_owner_module_value_fallback(true, true),
        expected
    );
}

#[test]
fn reusable_standard_base_preserves_bare_prelude_module_values() {
    let standard_source = SourceFile::new("prelude.veln", "pub let answer: Int = 42\n");
    let standard = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&standard_source).tree,
        veln_stdlib::PACKAGE_NAME.to_string(),
        "prelude".to_string(),
        standard_source.span(veln_source::TextRange::new(0, 0)),
    );
    let base = crate::types::TypeEnvironment::from_module(&standard);

    let app_source = SourceFile::new("app.veln", "use prelude from \"std\"\n");
    let mut app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&app_source).tree,
        "app".to_string(),
        app_source.span(veln_source::TextRange::new(0, 0)),
    );
    app.uses[0].name = "std::prelude".to_string();
    let environment = crate::types::TypeEnvironment::from_module_with_base_for_test(&app, &base);

    assert!(matches!(
        environment.module_value_path_in_owner(&["answer".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Available(_)
    ));
}

#[test]
fn imported_module_value_ambiguity_includes_function_data_collisions() {
    for function in [
        "pub fn shared() -> Int\n  2\nend\n",
        concat!(
            "pub fn original() -> Int\n",
            "  2\n",
            "end\n",
            "pub fn shared = original\n",
        ),
    ] {
        let module = merged_modules_with_identities(vec![
            (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
            ),
            ("beta", SourceFile::new("beta.veln", function)),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    concat!(
                        "use alpha\n",
                        "use beta\n",
                        "fn read() -> Int\n",
                        "  shared\n",
                        "end\n",
                    ),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.ambiguous"),
            "{diagnostics:#?}"
        );
    }
}

fn module_with_bare_module_value_ambiguity(
    import_count: usize,
    read_count: usize,
) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for index in 0..import_count {
        let module_name = format!("values_{index}");
        let source = SourceFile::new(format!("values_{index}.veln"), "pub let shared: Int = 1\n");
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module_values.extend(module.module_values);
    }
    let imports = (0..import_count)
        .map(|index| format!("use values_{index}\n"))
        .collect::<String>();
    let reads = (0..read_count)
        .map(|index| format!("fn read_{index}() -> Int\n  shared\nend\n"))
        .collect::<String>();
    let source = SourceFile::new("app.veln", format!("{imports}{reads}"));
    let app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "app".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.module = app.module;
    merged.uses.extend(app.uses);
    merged.functions.extend(app.functions);
    merged
}

#[test]
fn bare_module_value_ambiguity_provenance_work_is_linear_and_shared_across_reads() {
    fn measure(
        import_count: usize,
        read_count: usize,
    ) -> crate::types::ModuleValueAmbiguityProvenanceWork {
        let module = module_with_bare_module_value_ambiguity(import_count, read_count);
        crate::types::reset_module_value_ambiguity_provenance_work();
        let diagnostics = analyze_surface_module(&module);
        let ambiguities = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .collect::<Vec<_>>();
        assert_eq!(ambiguities.len(), read_count, "{diagnostics:#?}");
        assert!(
            ambiguities
                .iter()
                .all(|diagnostic| diagnostic.related.len() == import_count),
            "{diagnostics:#?}"
        );
        crate::types::take_module_value_ambiguity_provenance_work()
    }

    let smaller = measure(64, 1);
    let larger = measure(128, 1);
    let repeated = measure(128, 4);

    assert!(smaller.index_construction_visits >= 64, "{smaller:#?}");
    assert_eq!(
        larger.index_construction_visits,
        smaller.index_construction_visits * 2
    );
    assert_eq!(smaller.candidate_provenance_visits, 64, "{smaller:#?}");
    assert_eq!(larger.candidate_provenance_visits, 128, "{larger:#?}");
    assert_eq!(
        repeated.index_construction_visits, larger.index_construction_visits,
        "index construction must not repeat for each ambiguous read"
    );
    assert_eq!(repeated.candidate_provenance_visits, 128 * 4);
}

#[test]
fn same_owner_function_shadows_bare_imported_module_value_in_value_and_call_positions() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "fn answer() -> Int\n",
                    "  7\n",
                    "end\n",
                    "fn callback() -> fn() -> Int\n",
                    "  answer\n",
                    "end\n",
                    "fn call() -> Int\n",
                    "  answer()\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "name.ambiguous" | "name.unresolved" | "type.not_callable" | "type.mismatch"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn imported_module_value_call_target_does_not_fall_through_to_imported_function() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "functions",
            SourceFile::new("functions.veln", "pub fn answer() -> Int\n  7\nend\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                "use values\nuse functions\nfn call() -> Int\n  values::answer()\nend\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.not_callable")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.ambiguous"),
        "{diagnostics:#?}"
    );
}

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

#[test]
fn imported_module_value_ambiguity_is_independent_of_source_and_import_order() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
        );
        let beta = (
            "beta",
            SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
        );
        let imports = if reverse_imports {
            "use beta\nuse alpha\n"
        } else {
            "use alpha\nuse beta\n"
        };
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!("{imports}fn read() -> Int\n  shared\nend\n"),
            ),
        );
        let sources = if reverse_sources {
            vec![beta, alpha, app]
        } else {
            vec![alpha, beta, app]
        };
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(expected.len(), 1, "{expected:#?}");
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
}

#[test]
fn invalid_local_module_value_shadows_an_imported_value() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "let answer: Missing = 0\n",
                    "let selected: Int = answer\n",
                    "fn read() -> Int\n",
                    "  selected\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.mismatch"
            || !diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.start.line == 3)
    }));
}

#[test]
fn invalid_imported_module_value_does_not_reserve_a_bare_exposure() {
    for reverse_sources in [false, true] {
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
        );
        let app = (
            "app",
            SourceFile::new("app.veln", "use alpha\nfn read() -> Int\n  shared\nend\n"),
        );
        let sources = if reverse_sources {
            vec![app, alpha]
        } else {
            vec![alpha, app]
        };

        let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved value `shared`"
            }),
            "reverse_sources={reverse_sources}: {diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "reverse_sources={reverse_sources}: {diagnostics:#?}"
        );
    }
}

#[test]
fn callable_collisions_stay_unavailable_after_imported_availability_rechecks() {
    for callable in [
        "pub fn clash() -> Int\n  1\nend\n",
        concat!(
            "pub fn original() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn clash = original\n",
        ),
    ] {
        let module = merged_modules_with_identities(vec![
            ("b", SourceFile::new("b.veln", "pub let bad: Missing = 0\n")),
            (
                "a",
                SourceFile::new(
                    "a.veln",
                    format!(
                        "use b\n{callable}pub let clash: Int = 2\npub let recheck: Int = b::bad\n"
                    ),
                ),
            ),
            (
                "c",
                SourceFile::new(
                    "c.veln",
                    concat!(
                        "use a\n",
                        "fn bare() -> Int\n",
                        "  clash()\n",
                        "end\n",
                        "fn qualified() -> Int\n",
                        "  a::clash()\n",
                        "end\n",
                    ),
                ),
            ),
        ]);
        let mut environment = crate::types::TypeEnvironment::from_module(&module);
        let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
        let analysis = crate::analysis::check_module_values(
            &module,
            &mut environment,
            &mut variant_diagnostics,
        );

        assert!(
            analysis
                .invalid
                .contains(&(None, Some("a".to_string()), "clash".to_string())),
            "{:#?}",
            analysis.invalid
        );
        assert!(matches!(
            environment.module_value_path_in_owner(&["clash".to_string()], Some("c"), None),
            crate::types::ModuleValueLookup::Missing
        ));
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["a".to_string(), "clash".to_string()],
                Some("c"),
                None,
            ),
            crate::types::ModuleValueLookup::Missing
        ));

        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "name.ambiguous"
                    && diagnostic.id != "type.not_callable"
                    && !(diagnostic.id == "name.unresolved" && diagnostic.message.contains("clash"))
            }),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn unpublished_functions_and_aliases_do_not_collide_with_imported_values() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let functions = (
                "functions",
                SourceFile::new(
                    "functions.veln",
                    concat!(
                        "pub type State\n",
                        "  Hidden\n",
                        "end\n",
                        "pub fn shared(value: State::Hidden) -> Int\n",
                        "  1\n",
                        "end\n",
                        "pub fn aliased = shared\n",
                    ),
                ),
            );
            let values = (
                "values",
                SourceFile::new(
                    "values.veln",
                    concat!("pub let shared: Int = 2\n", "pub let aliased: Int = 3\n",),
                ),
            );
            let imports = if reverse_imports {
                "use values\nuse functions\n"
            } else {
                "use functions\nuse values\n"
            };
            let app = (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "{imports}let selected: Int = shared\nlet selected_alias: Int = aliased\n"
                    ),
                ),
            );
            let sources = if reverse_sources {
                vec![app, values, functions]
            } else {
                vec![functions, values, app]
            };
            let module = merged_modules_with_identities(sources);
            let environment = crate::types::TypeEnvironment::from_module(&module);

            for name in ["shared", "aliased"] {
                assert!(matches!(
                    environment.module_value_path_in_owner(&[name.to_string()], Some("app"), None,),
                    crate::types::ModuleValueLookup::Available(_)
                ));
            }

            let diagnostics = analyze_surface_module(&module);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| { diagnostic.id == "type.variant_refinement_private" }),
                "{diagnostics:#?}"
            );
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && (diagnostic.message.contains("shared")
                                || diagnostic.message.contains("aliased")))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn valid_imported_value_survives_an_invalid_namesake_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let alpha = (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            );
            let beta = (
                "beta",
                SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
            );
            let imports = if reverse_imports {
                "use beta\nuse alpha\n"
            } else {
                "use alpha\nuse beta\n"
            };
            let app = (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn read() -> Int\n  shared\nend\n"),
                ),
            );
            let sources = if reverse_sources {
                vec![beta, alpha, app]
            } else {
                vec![alpha, beta, app]
            };

            let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message == "unresolved value `shared`")
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn module_value_initializers_observe_final_imported_availability_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let alpha = (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            );
            let beta = (
                "beta",
                SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
            );
            let imports = if reverse_imports {
                "use beta\nuse alpha\n"
            } else {
                "use alpha\nuse beta\n"
            };
            let app = (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "{imports}let selected: Int = shared\nfn read() -> Int\n  selected\nend\n"
                    ),
                ),
            );
            let sources = if reverse_sources {
                vec![app, beta, alpha]
            } else {
                vec![alpha, beta, app]
            };

            let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("shared"))
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("selected"))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }

    for reference in ["shared", "alpha::shared"] {
        let module = merged_modules_with_identities(vec![
            (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "use alpha\nlet selected: Int = {reference}\nfn read() -> Int\n  selected\nend\n"
                    ),
                ),
            ),
        ]);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == format!("unresolved value `{reference}`")
            }),
            "reference={reference}: {diagnostics:#?}"
        );
    }
}

#[test]
fn delayed_bare_import_invalidation_quarantines_dependents_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let owner_imports = if reverse_imports {
                "use beta\nuse alpha\n"
            } else {
                "use alpha\nuse beta\n"
            };
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
                    SourceFile::new(
                        "beta.veln",
                        "pub let shared: Int = 1\npub let shared: Int = 2\n",
                    ),
                ),
                (
                    "owner",
                    SourceFile::new(
                        "owner.veln",
                        format!(
                            "{owner_imports}pub let selected: Int = shared\npub let healthy: Int = 3\n"
                        ),
                    ),
                ),
                (
                    "consumer",
                    SourceFile::new(
                        "consumer.veln",
                        concat!(
                            "use owner\n",
                            "fn read_selected() -> Int\n",
                            "  selected\n",
                            "end\n",
                            "fn read_healthy() -> Int\n",
                            "  healthy\n",
                            "end\n",
                        ),
                    ),
                ),
            ];
            if reverse_sources {
                sources.reverse();
            }
            let module = merged_modules_with_identities(sources);
            let mut environment = crate::types::TypeEnvironment::from_module(&module);
            let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
            let analysis = crate::analysis::check_module_values(
                &module,
                &mut environment,
                &mut variant_diagnostics,
            );
            let selected = (None, Some("owner".to_string()), "selected".to_string());

            assert!(
                analysis.invalid.contains(&selected),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {:#?}",
                analysis.invalid
            );
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["selected".to_string()],
                    Some("consumer"),
                    None,
                ),
                crate::types::ModuleValueLookup::Missing
            ));
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["healthy".to_string()],
                    Some("consumer"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));

            let diagnostics = analyze_surface_module(&module);
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.id == "name.unresolved"
                        && diagnostic.message == "unresolved value `selected`"
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("healthy"))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}

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

#[test]
fn bare_imported_cycle_edges_preserve_package_identity() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
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
                    veln_ast::lower_surface_ast_with_package_module_identity(
                        &parsed.tree,
                        package.to_string(),
                        module_name.to_string(),
                        source.span(veln_source::TextRange::new(0, 0)),
                    )
                })
                .collect::<Vec<_>>();
            let mut module = modules.remove(0);
            for source_module in modules {
                module.uses.extend(source_module.uses);
                module.module_values.extend(source_module.module_values);
            }

            let mut environment = crate::types::TypeEnvironment::from_module(&module);
            let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
            let analysis = crate::analysis::check_module_values(
                &module,
                &mut environment,
                &mut variant_diagnostics,
            );

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
                    analysis.invalid.contains(&identity),
                    "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {:#?}",
                    analysis.invalid
                );
            }
            assert!(!analysis.invalid.contains(&(
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
    }
}

#[test]
fn invalid_imported_value_does_not_collide_with_a_valid_imported_function() {
    for reverse_imports in [false, true] {
        let imports = if reverse_imports {
            "use functions\nuse values\n"
        } else {
            "use values\nuse functions\n"
        };
        let module = merged_modules_with_identities(vec![
            (
                "values",
                SourceFile::new("values.veln", "pub let shared: Missing = 0\n"),
            ),
            (
                "functions",
                SourceFile::new("functions.veln", "pub fn shared() -> Int\n  2\nend\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn read() -> Int\n  shared()\nend\n"),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().all(|diagnostic| {
                !matches!(
                    diagnostic.id.as_str(),
                    "name.ambiguous" | "type.not_callable"
                ) && !(diagnostic.id == "name.unresolved" && diagnostic.message.contains("shared"))
            }),
            "reverse_imports={reverse_imports}: {diagnostics:#?}"
        );
    }
}

#[test]
fn exact_companion_module_value_can_read_private_module_values_but_other_sources_cannot() {
    let module = merged_modules_with_identities(vec![
        (
            "math",
            SourceFile::new("math.veln", "let secret: Int = 42\n"),
        ),
        (
            "math.test",
            SourceFile::new(
                "math.test.veln",
                "use math\nlet observed: Int = math::secret\n",
            ),
        ),
        (
            "other",
            SourceFile::new("other.veln", "let marker: Int = 0\n"),
        ),
        (
            "other.test",
            SourceFile::new(
                "other.test.veln",
                "use math\nlet wrong_target: Int = math::secret\n",
            ),
        ),
        (
            "consumer",
            SourceFile::new(
                "consumer.veln",
                "use math\nlet ordinary: Int = math::secret\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.span.as_ref().is_none_or(|span| {
                span.file.as_str() != "math.test.veln" || diagnostic.id != "name.unresolved"
            })
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_companion_cannot_read_an_unavailable_private_module_value() {
    let module = merged_modules_with_identities(vec![
        (
            "math",
            SourceFile::new("math.veln", "let secret: Missing = 0\n"),
        ),
        (
            "math.test",
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "pub let observed: Int = math::secret\n",
                    "fn read_observed() -> String\n",
                    "  observed\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    let mut environment = crate::types::TypeEnvironment::from_module(&module);
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    let analysis =
        crate::analysis::check_module_values(&module, &mut environment, &mut variant_diagnostics);

    assert_eq!(
        analysis.invalid,
        [
            (None, Some("math".to_string()), "secret".to_string()),
            (None, Some("math.test".to_string()), "observed".to_string(),),
        ]
        .into_iter()
        .collect(),
    );
    assert!(matches!(
        environment.module_value_path_in_owner(
            &["math".to_string(), "secret".to_string()],
            Some("math.test"),
            None,
        ),
        crate::types::ModuleValueLookup::Missing
    ));
    assert!(matches!(
        environment.module_value_path_in_owner(&["observed".to_string()], Some("math.test"), None,),
        crate::types::ModuleValueLookup::Unavailable
    ));

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `math::secret`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.span.as_ref().is_none_or(|span| {
                span.file.as_str() != "math.test.veln" || diagnostic.id != "type.mismatch"
            })
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_module_value_annotations_reject_private_named_types_and_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Secret\n",
            "  pub Value\n",
            "end\n",
            "pub type State\n",
            "  pub Visible\n",
            "  Hidden\n",
            "end\n",
            "pub let named: Secret = Value\n",
            "pub let refined: State::Hidden = Hidden\n",
            "pub let hidden_initializer: State = Hidden\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn public_module_value_visibility_recurses_through_annotation_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Secret\n",
            "  pub Value\n",
            "end\n",
            "pub type State\n",
            "  Hidden\n",
            "  Other\n",
            "end\n",
            "pub type Box<T>\n",
            "  pub Box(value: T)\n",
            "end\n",
            "pub type Choice<T>\n",
            "  pub Left\n",
            "  pub Right\n",
            "end\n",
            "pub let named_generic: Box<Secret> = 0\n",
            "pub let named_function: fn(Secret) -> Secret = 0\n",
            "pub let named_record: {field: Secret} = 0\n",
            "pub let named_union: Choice<Int>::Left | Choice<Secret>::Right = 0\n",
            "pub let refined_generic: Box<State::Hidden> = 0\n",
            "pub let refined_function: fn(State::Hidden) -> State::Hidden = 0\n",
            "pub let refined_record: {field: State::Hidden} = 0\n",
            "pub let refined_union: State::Hidden | State::Other = 0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let private_named_types = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
        .collect::<Vec<_>>();
    assert_eq!(private_named_types.len(), 4, "{diagnostics:#?}");
    let union_private_type = private_named_types
        .iter()
        .find(|diagnostic| {
            let detail = |field: &str| {
                diagnostic.details.as_object().and_then(|entries| {
                    entries
                        .iter()
                        .find(|(entry, _)| entry == field)
                        .and_then(|(_, value)| value.as_text())
                })
            };
            detail("name") == Some("named_union") && detail("written_type") == Some("Secret")
        })
        .expect("private named type in the later union alternative");
    assert_diagnostic_span(union_private_type, 18, 49, 18, 55);
    let refinements = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(refinements.len(), 6, "{diagnostics:#?}");
    let rendered = format!("{refinements:#?}");
    for segment in [
        "type_argument",
        "function_parameter",
        "function_result",
        "record_field",
        "union_alternative",
    ] {
        assert!(rendered.contains(segment), "missing {segment}: {rendered}");
    }
}

#[test]
fn non_name_calls_are_rejected_and_quarantined_at_the_call_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Boxed\n",
            "  Box(value: Int)\n",
            "end\n",
            "let direct: Int = (1)()\n",
            "let nested: { value: Int } = { value: ((2))(3) }\n",
            "let payload: Boxed = Box((4)())\n",
            "let dependent: Int = direct\n",
            "let independent: Int = 1\n",
            "fn reads_direct() -> String\n",
            "  direct\n",
            "end\n",
            "fn reads_dependent() -> String\n",
            "  dependent\n",
            "end\n",
            "fn reads_independent() -> String\n",
            "  independent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert_diagnostic_span(unsupported[0], 4, 19, 4, 24);
    assert_diagnostic_span(unsupported[1], 5, 39, 5, 47);
    assert_diagnostic_span(unsupported[2], 6, 26, 6, 31);
    for (diagnostic, declaration) in unsupported.iter().zip(&module.module_values[..3]) {
        assert_eq!(
            diagnostic.related.first(),
            Some(&veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("module_value_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(format!(
                        "Module value `{}` is declared here.",
                        declaration.name.as_deref().expect("module value name")
                    )),
                ),
                ("span", crate::diagnostics::span_json(&declaration.span),),
            ]))
        );
    }

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(16)
    );
}

#[test]
fn module_value_annotations_are_checked_even_when_unused() {
    let source = SourceFile::new("main.veln", "let unused: String = 42\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `String`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_annotations_must_be_closed_and_quarantine_dependents() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let open: Option<T> = None\n",
            "let dependent: Option<T> = open\n",
            "fn read() -> String\n",
            "  dependent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unresolved_type_parameters = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .count();

    assert_eq!(unresolved_type_parameters, 2, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.mismatch" || !diagnostic.message.contains("found `Option<T>`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_function_annotations_validate_effect_rows_and_labels() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let unbound: fn() -> Int effects [...E] = 1\n",
            "let unknown: fn() -> Int effects [MissingAudit] = 1\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let unbound = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.row_unbound")
        .unwrap_or_else(|| panic!("expected unbound effect row: {diagnostics:#?}"));
    let unbound_span = unbound.span.as_ref().expect("unbound effect row span");
    assert_eq!(
        (unbound_span.start.line, unbound_span.start.column),
        (1, 35)
    );
    assert_eq!((unbound_span.end.line, unbound_span.end.column), (1, 39));

    let unknown = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.unknown")
        .unwrap_or_else(|| panic!("expected unknown effect label: {diagnostics:#?}"));
    let unknown_span = unknown.span.as_ref().expect("unknown effect label span");
    assert_eq!(
        (unknown_span.start.line, unknown_span.start.column),
        (2, 35)
    );
    assert_eq!((unknown_span.end.line, unknown_span.end.column), (2, 47));
}

#[test]
fn module_value_annotations_and_initializers_use_existing_type_name_rules() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "end\n",
            "let good: State::Idle = State::Idle\n",
            "let wrong: Option<Int>::Absent = None\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.invalid_case"
                || !diagnostic.message.contains("module name `State`")
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.variant_refinement_unknown"
                && diagnostic.message.contains("variant `Absent`")
        }),
        "{diagnostics:#?}"
    );
}

mod availability;
mod invariants;
