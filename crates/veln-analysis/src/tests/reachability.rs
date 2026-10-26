use super::*;

#[test]
fn reachable_entry_lowering_keeps_application_reachability_project_local() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let alpha = project(
        "src/main.veln",
        concat!(
            "fn alpha_helper() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  alpha_helper()\n",
            "end\n",
        ),
    );
    let beta = project(
        "src/main.veln",
        concat!(
            "fn beta_helper() -> Int\n",
            "  2\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  beta_helper()\n",
            "end\n",
        ),
    );

    let alpha = crate::analysis::analyze_project_with_test_standard_cache(
        alpha,
        DoctestMode::Exclude,
        &cache,
    );
    let beta = crate::analysis::analyze_project_with_test_standard_cache(
        beta,
        DoctestMode::Exclude,
        &cache,
    );

    let alpha_reachable = alpha.lower_reachable_entry("main", FunctionKind::Function);
    let beta_reachable = beta.lower_reachable_entry("main", FunctionKind::Function);

    assert!(alpha_reachable.lowered.diagnostics.is_empty());
    assert!(beta_reachable.lowered.diagnostics.is_empty());
    assert_eq!(
        lowered_function_names(&alpha_reachable),
        ["alpha_helper", "main"]
    );
    assert_eq!(
        lowered_function_names(&beta_reachable),
        ["beta_helper", "main"]
    );
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 2);
}

#[test]
fn shared_analysis_keeps_generated_doctest_sources_project_local() {
    let cache = crate::analysis::TestStandardEnvironmentCache::new();
    let alpha = project(
        "src/alpha.veln",
        concat!(
            "## ```veln\n",
            "## let value: Int = \"alpha-only\"\n",
            "## ```\n",
            "pub fn documented() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let beta = project(
        "src/beta.veln",
        concat!(
            "## ```veln\n",
            "## let value: Bool = 1\n",
            "## ```\n",
            "pub fn documented() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let alpha_diagnostics =
        checked_diagnostic_json_with_cache_and_mode(alpha.clone(), DoctestMode::Include, &cache);
    let beta_diagnostics =
        checked_diagnostic_json_with_cache_and_mode(beta.clone(), DoctestMode::Include, &cache);

    assert_eq!(diagnostic_ids(&alpha_diagnostics), ["type.mismatch"]);
    assert_eq!(diagnostic_ids(&beta_diagnostics), ["type.mismatch"]);
    assert_diagnostics_contain(
        &alpha_diagnostics,
        "src/alpha.veln#doctest-1_test.veln",
        "expected `Int`, but found `String`",
    );
    assert_diagnostics_contain(
        &beta_diagnostics,
        "src/beta.veln#doctest-1_test.veln",
        "expected `Bool`, but found `Int`",
    );
    assert_no_project_leak(
        &alpha_diagnostics,
        "src/beta.veln#doctest-1_test.veln",
        "src/beta.veln",
        "expected `Bool`, but found `Int`",
    );
    assert_no_project_leak(
        &beta_diagnostics,
        "src/alpha.veln#doctest-1_test.veln",
        "src/alpha.veln",
        "expected `Int`, but found `String`",
    );
    assert!(checked_diagnostic_json_with_cache(alpha, &cache).is_empty());
    assert!(checked_diagnostic_json_with_cache(beta, &cache).is_empty());
    assert_eq!(cache.standard_prepares(), 1);
    assert_eq!(cache.application_analyses(), 4);
}

#[test]
fn reachable_lowering_does_not_diagnose_blocked_exact_alias_fallback_body() {
    for imports in [
        [
            "use foo as first\n",
            "use foo as second\n",
            "use bar as foo\n",
        ],
        [
            "use bar as foo\n",
            "use foo as second\n",
            "use foo as first\n",
        ],
    ] {
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files: vec![
                    SourceFile::new(
                        "main.veln",
                        format!(
                            "{}{}{}pub fn main() -> Int\n  foo::hit()\nend\n",
                            imports[0], imports[1], imports[2]
                        ),
                    ),
                    SourceFile::new("foo.veln", "pub fn hit() -> Int\n  1\nend\n"),
                    SourceFile::new("bar.veln", "pub fn hit() -> Int\n  missing()\nend\n"),
                ],
                manifest: None,
            },
            DoctestMode::Exclude,
        );
        let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);
        let diagnostics = &reachable.lowered.diagnostics;

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            ["name.duplicate", "name.unresolved"],
            "imports={imports:?}, diagnostics={diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("missing")),
            "imports={imports:?}, diagnostics={diagnostics:#?}"
        );
        assert_eq!(
            reachable
                .module
                .functions
                .iter()
                .filter_map(|function| {
                    Some((function.module_name.as_deref()?, function.name.as_deref()?))
                })
                .collect::<Vec<_>>(),
            [("main", "main")]
        );
    }
}
