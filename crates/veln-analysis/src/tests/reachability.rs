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
