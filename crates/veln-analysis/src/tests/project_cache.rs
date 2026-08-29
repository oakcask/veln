use super::*;

#[test]
fn shared_analysis_keeps_diagnostic_json_order_stable_across_projects() {
    let alpha = project(
        "src/alpha/shared.veln",
        concat!(
            "mod alpha.shared\n",
            "pub fn entry() -> Int\n",
            "  \"alpha-only\"\n",
            "end\n",
        ),
    );
    let beta = project(
        "src/beta/shared.veln",
        concat!(
            "mod beta.shared\n",
            "pub fn entry() -> Bool\n",
            "  1\n",
            "end\n",
        ),
    );
    let alpha_isolated = checked_diagnostic_json(alpha.clone());
    let beta_isolated = checked_diagnostic_json(beta.clone());

    assert_project_evidence(
        &alpha_isolated,
        "src/alpha/shared.veln",
        "alpha.shared",
        "expected `Int`, but found `String`",
    );
    assert_project_evidence(
        &beta_isolated,
        "src/beta/shared.veln",
        "beta.shared",
        "expected `Bool`, but found `Int`",
    );
    assert_no_project_leak(
        &alpha_isolated,
        "src/beta/shared.veln",
        "beta.shared",
        "expected `Bool`, but found `Int`",
    );
    assert_no_project_leak(
        &beta_isolated,
        "src/alpha/shared.veln",
        "alpha.shared",
        "expected `Int`, but found `String`",
    );

    for _ in 0..8 {
        assert_eq!(checked_diagnostic_json(alpha.clone()), alpha_isolated);
        assert_eq!(checked_diagnostic_json(beta.clone()), beta_isolated);
    }

    let handles = (0..16)
        .map(|index| {
            let project = if index % 2 == 0 {
                alpha.clone()
            } else {
                beta.clone()
            };
            thread::spawn(move || (index, checked_diagnostic_json(project)))
        })
        .collect::<Vec<_>>();

    for handle in handles {
        let (index, diagnostics) = handle.join().expect("analysis thread should not panic");
        if index % 2 == 0 {
            assert_eq!(diagnostics, alpha_isolated);
            assert_no_project_leak(
                &diagnostics,
                "src/beta/shared.veln",
                "beta.shared",
                "expected `Bool`, but found `Int`",
            );
        } else {
            assert_eq!(diagnostics, beta_isolated);
            assert_no_project_leak(
                &diagnostics,
                "src/alpha/shared.veln",
                "alpha.shared",
                "expected `Int`, but found `String`",
            );
        }
    }
}

#[test]
fn project_analysis_timings_name_pipeline_boundaries() {
    let project = project(
        "src/main.veln",
        concat!("pub fn main() -> Int\n", "  1\n", "end\n"),
    );

    let (analysis, timings) = analyze_project_with_timings(project, DoctestMode::Exclude);
    let (reachable, reachable_timing) =
        analysis.lower_reachable_entry_with_timing("main", FunctionKind::Function);

    assert!(reachable.lowered.diagnostics.is_empty());
    assert_eq!(
        timings
            .iter()
            .map(|timing| timing.stage)
            .collect::<Vec<_>>(),
        vec!["surface_parse_lower", "semantic_environment_check"]
    );
    assert_eq!(reachable_timing.stage, "reachable_entry_lowering");
}

#[test]
fn project_loading_keeps_application_and_selected_standard_inputs_separate() {
    let project = project(
        "src/main.veln",
        concat!(
            "use http2::frame from \"std\"\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{ length : Int, kind : Int, flags : Int, stream_id : Int, payload : ByteView }, String>\n",
            "  http2::frame::decode(view)\n",
            "end\n",
        ),
    );

    let (loaded, diagnostics) = crate::surface::load_surface_modules(&project);
    let (combined, combined_diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(combined_diagnostics.is_empty(), "{combined_diagnostics:#?}");
    assert_eq!(
        standard_declaration_count(&loaded.application),
        0,
        "application input must not own selected standard declarations"
    );
    assert!(
        !loaded.selected_standard_module_names.is_empty(),
        "selected standard input should identify the selected closure"
    );
    assert!(
        loaded
            .selected_standard_module_names
            .contains("std::http2::frame")
    );
    let frame_decode_count = combined
        .functions
        .iter()
        .filter(|function| {
            function.module_name.as_deref() == Some("std::http2::frame")
                && function.name.as_deref() == Some("decode")
        })
        .count();
    assert_eq!(frame_decode_count, 1);
}

#[test]
fn first_application_analysis_uses_embedded_lowered_standard_modules() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let project = project(
        "src/main.veln",
        concat!("pub fn main() -> Int\n", "  1\n", "end\n"),
    );

    let (analysis, standard_work) = crate::surface::embedded_standard_counters::observe(|| {
        crate::analysis::analyze_project_with_test_standard_cache(
            project,
            DoctestMode::Exclude,
            &cache,
        )
    });

    assert!(analysis.checked_diagnostics().is_empty());
    assert_eq!(
        standard_work.runtime_standard_parse_lowers, 0,
        "embedded standard modules must not use the runtime parser or surface lowerer"
    );
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(
        standard_work.materialized_modules.len(),
        analysis.selected_standard_module_names_for_test().len(),
        "embedded standard materialization should stay limited to the selected closure"
    );
    assert!(
        analysis.selected_standard_module_names_for_test().len()
            < veln_stdlib::package_bundle().files.len(),
        "the prelude-only closure should leave unrelated standard modules unmaterialized"
    );
}

#[test]
fn rediscovered_project_analysis_uses_changed_source_text_and_manifest_data() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let temp = TempProject::new("analysis-rediscovery-isolation");
    temp.write(
        "src/main.veln",
        concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
    );
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

    let baseline = checked_discovered_diagnostic_json_with_cache(&temp, &[], &cache);

    assert!(baseline.is_empty(), "{baseline:#?}");

    temp.write(
        "src/main.veln",
        concat!("pub fn entry() -> Bool\n", "  1\n", "end\n"),
    );
    temp.write("veln.toml", "[lib]\nexports = [\"src/other.veln\"]\n");

    let changed = checked_discovered_diagnostic_json_with_cache(&temp, &[], &cache);

    assert_eq!(
        diagnostic_ids(&changed),
        ["manifest.missing_export", "type.mismatch"],
        "{changed:#?}"
    );
    assert!(
        changed
            .iter()
            .any(|diagnostic| diagnostic.contains("src/other.veln")),
        "{changed:#?}"
    );
    assert!(
        changed
            .iter()
            .any(|diagnostic| diagnostic.contains("expected `Bool`, but found `Int`")),
        "{changed:#?}"
    );

    temp.write(
        "src/main.veln",
        concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
    );
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

    let restored = checked_discovered_diagnostic_json_with_cache(&temp, &[], &cache);

    assert!(restored.is_empty(), "{restored:#?}");
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 3);
}

#[test]
fn rediscovered_project_analysis_uses_changed_package_and_command_inputs() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let temp = TempProject::new("analysis-package-and-input-isolation");
    temp.write(
        "src/good.veln",
        concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
    );
    temp.write(
        "src/bad.veln",
        concat!("pub fn entry() -> Bool\n", "  1\n", "end\n"),
    );
    temp.write(
        "src/main.veln",
        concat!(
            "use exported from \"example/pkg\"\n",
            "\n",
            "pub fn entry() -> Int\n",
            "  exported::value()\n",
            "end\n",
        ),
    );
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/pkg\"]\n",
            "path = \"vendor/pkg\"\n",
        ),
    );
    temp.write(
        "vendor/pkg/veln.toml",
        concat!(
            "[package]\n",
            "name = \"example/pkg\"\n",
            "\n",
            "[lib]\n",
            "exports = [\"exported.veln\"]\n",
        ),
    );
    temp.write(
        "vendor/pkg/exported.veln",
        concat!("pub fn value() -> Int\n", "  1\n", "end\n"),
    );

    let good_input = checked_discovered_diagnostic_json_with_cache(
        &temp,
        &[PathBuf::from("src/good.veln")],
        &cache,
    );
    let bad_input = checked_discovered_diagnostic_json_with_cache(
        &temp,
        &[PathBuf::from("src/bad.veln")],
        &cache,
    );
    let package_baseline = checked_discovered_diagnostic_json_with_cache(
        &temp,
        &[PathBuf::from("src/main.veln")],
        &cache,
    );

    assert!(good_input.is_empty(), "{good_input:#?}");
    assert_eq!(diagnostic_ids(&bad_input), ["type.mismatch"]);
    assert!(package_baseline.is_empty(), "{package_baseline:#?}");

    temp.write(
        "vendor/pkg/exported.veln",
        concat!("pub fn value() -> Bool\n", "  true\n", "end\n"),
    );

    let package_changed = checked_discovered_diagnostic_json_with_cache(
        &temp,
        &[PathBuf::from("src/main.veln")],
        &cache,
    );

    assert_eq!(diagnostic_ids(&package_changed), ["type.mismatch"]);
    assert!(
        package_changed
            .iter()
            .any(|diagnostic| diagnostic.contains("expected `Int`, but found `Bool`")),
        "{package_changed:#?}"
    );
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 4);
}

#[test]
fn shared_analysis_keeps_local_std_prefixed_application_modules_fresh() {
    let project = project(
        "std/helper.veln",
        concat!(
            "fn answer(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn entry() -> Int\n",
            "  answer(1)\n",
            "end\n",
        ),
    );

    let diagnostics = checked_diagnostic_json(project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn shared_analysis_keeps_embedded_standard_module_name_collisions_fresh() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let project = project(
        "std/prelude.veln",
        concat!(
            "fn local_only(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn entry() -> Int\n",
            "  local_only(1)\n",
            "end\n",
        ),
    );

    let diagnostics = checked_diagnostic_json_with_cache(project, &cache);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 1);
}

#[test]
fn shared_analysis_prepares_standard_once_and_rebuilds_each_application() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let alpha = project(
        "src/shared.veln",
        concat!("pub fn entry() -> Int\n", "  1\n", "end\n",),
    );
    let beta = project(
        "std/helper.veln",
        concat!(
            "fn answer(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn entry() -> Bool\n",
            "  answer(1)\n",
            "end\n",
        ),
    );

    let alpha_expected = checked_diagnostic_json_with_cache(alpha.clone(), &cache);
    let beta_expected = checked_diagnostic_json_with_cache(beta.clone(), &cache);

    assert!(alpha_expected.is_empty(), "{alpha_expected:#?}");
    assert_eq!(diagnostic_ids(&beta_expected), ["type.mismatch"]);
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 2);

    let handles = thread::scope(|scope| {
        (0..12)
            .map(|index| {
                let project = if index % 2 == 0 {
                    alpha.clone()
                } else {
                    beta.clone()
                };
                let cache = &cache;
                scope.spawn(move || (index, checked_diagnostic_json_with_cache(project, cache)))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("analysis should not panic"))
            .collect::<Vec<_>>()
    });

    for (index, diagnostics) in handles {
        if index % 2 == 0 {
            assert_eq!(diagnostics, alpha_expected);
        } else {
            assert_eq!(diagnostics, beta_expected);
        }
    }
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 14);
}
