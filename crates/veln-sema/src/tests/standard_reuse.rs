use std::thread;

use super::*;
use crate::types::TypeEnvironment;
use veln_ast::Visibility;

mod fixture;

use fixture::*;

pub(crate) fn standard_reuse_test_lock_for_sibling() -> std::sync::MutexGuard<'static, ()> {
    standard_reuse_test_lock()
}

#[test]
fn reusable_standard_environment_matches_uncached_analysis_for_table_cases() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();

    for case in [
        app_case(
            "successful project",
            "src/main.veln",
            concat!(
                "fn compute() -> Int effects [prelude::Ask]\n",
                "  perform prelude::Ask::value()\n",
                "end\n",
                "\n",
                "pub fn main(input: ByteView, base: ByteOffset, value: {value: Int}) -> Result<{value: Int}, String>\n",
                "  let observed = handle compute() with prelude::ask(1)\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(observed))\n",
                "  prelude::byte_decode_public_packet(value)\n",
                "end\n",
            ),
        ),
        app_case(
            "type and effect diagnostics",
            "src/main.veln",
            concat!(
                "fn compute() -> Int\n",
                "  perform prelude::Ask::value()\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  prelude::answer(prelude::PayloadShape(1))\n",
                "end\n",
            ),
        ),
        app_case(
            "overlapping app path and declaration names",
            "std/prelude.veln",
            concat!(
                "fn provide(offset: Int) -> Int\n",
                "  offset + 2\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  provide(1)\n",
                "end\n",
            ),
        ),
        app_case(
            "application std path outside reusable bundle",
            "std/helper.veln",
            concat!(
                "fn answer(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  answer(1)\n",
                "end\n",
            ),
        )
        .with_module_name("std::helper"),
        app_case(
            "application module collides with embedded standard module name",
            "std/prelude.veln",
            concat!(
                "fn local_only(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  local_only(1)\n",
                "end\n",
            ),
        )
        .with_module_name("std::prelude"),
    ] {
        let module = merge_modules(vec![standard.clone(), case.module]);
        let uncached = check_project_surface_module(&module);
        let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

        assert_same_analysis(case.name, uncached, cached);
    }
}

#[test]
fn reusable_source_backed_base_is_quarantined_before_return_inference() {
    let _guard = standard_reuse_test_lock();
    let standard = module_with_identity(
        "prelude.veln",
        "mod std::prelude\npub type SharedEmpty\nend\n",
        "std::prelude",
    );
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let application = app_case(
        "source-backed refinement base",
        "src/main.veln",
        concat!(
            "mod app\n",
            "fn broken(invalid: prelude::SharedEmpty::Missing, sibling: String) -> Int\n",
            "  0\n",
            "end\n",
            "fn inferred()\n",
            "  broken(1, \"x\")\n",
            "end\n",
            "fn needs_string(value: String) -> ()\n",
            "  ()\n",
            "end\n",
            "fn user() -> ()\n",
            "  needs_string(inferred())\n",
            "end\n",
        ),
    );
    let selected_standard_modules = std::iter::once("std::prelude".to_string()).collect();
    let environment = TypeEnvironment::from_application_module_with_standard_module_names(
        &application.module,
        &selected_standard_modules,
        &reusable,
    );
    let descriptor = environment
        .adts
        .unique_descriptor_for_type_path_any_arity(
            "prelude::SharedEmpty",
            Some("app"),
            &environment.uses,
        )
        .expect("selected reusable type descriptor");
    assert!(
        environment.adts.declaration_span(descriptor).is_some(),
        "selected reusable type must retain its source declaration span"
    );
    assert!(
        environment
            .variant_refinement_base_failure("prelude::SharedEmpty", 0, Some("app"))
            .is_some(),
        "selected reusable type must retain its base failure"
    );
    let cached = check_project_surface_module_with_standard_modules_environment(
        &application.module,
        &selected_standard_modules,
        &reusable,
    );

    let base_diagnostics = cached
        .0
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .collect::<Vec<_>>();
    assert_eq!(base_diagnostics.len(), 1, "{:#?}", cached.0);
    let base_json = veln_diagnostics::diagnostic_to_json(base_diagnostics[0]).to_json();
    assert!(
        base_json.contains(
            "\"details\":{\"written_type\":\"prelude::SharedEmpty\",\"reason\":\"not_adt\"}"
        ),
        "{base_json}"
    );
    assert!(
        base_json.contains("\"kind\":\"base_type_declaration\"")
            && base_json.contains("\"resolved_identity\":\"SharedEmpty\"")
            && base_json.contains("\"file\":\"prelude.veln\""),
        "{base_json}"
    );
    assert!(
        cached
            .0
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{:#?}",
        cached.0
    );
}

#[test]
fn reusable_standard_environment_matches_uncached_analysis_for_standard_imports() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_modules_with_imports();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let module = merge_modules(vec![
        standard,
        app_case(
            "standard import",
            "src/main.veln",
            concat!(
                "pub fn main(value: prelude::ImportedPayload) -> prelude::ImportedPayload\n",
                "  value\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("standard import", uncached, cached);
}

#[test]
fn reusable_standard_environment_uses_only_loaded_standard_modules() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_modules_with_unused_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let loaded_standard = standard_module();
    let selected_modules = std::iter::once("std::prelude".to_string()).collect();
    let selected_environment = reusable.environment_for_modules_for_test(&selected_modules);
    assert_eq!(
        selected_environment.standard_function_modules_for_test(),
        selected_modules
    );

    let module = merge_modules(vec![
        loaded_standard,
        app_case(
            "loaded prelude only",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("loaded prelude only", uncached, cached);
}

#[test]
fn reusable_standard_environment_does_not_expose_unloaded_standard_functions() {
    let _guard = standard_reuse_test_lock();
    let reusable =
        prepare_reusable_standard_surface_module_environment(&standard_modules_with_extra_module())
            .with_current_identity_for_test();
    let module = merge_modules(vec![
        standard_module(),
        app_case(
            "unloaded extra module",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  extra::extra_answer(1)\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert!(
        diagnostic_json(&cached.0)
            .iter()
            .any(|diagnostic| diagnostic.contains("extra_answer")),
        "the unloaded standard function should remain unresolved"
    );
    assert_same_analysis("unloaded extra module", uncached, cached);
}

#[test]
fn reusable_standard_environment_selects_loaded_standard_module_sets() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_modules_with_extra_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    assert_eq!(reusable.prepared_environment_count_for_test(), 1);

    let prelude_only = merge_modules(vec![
        standard_module(),
        app_case(
            "prelude only",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);
    let prelude_and_extra = merge_modules(vec![
        standard_module(),
        extra_standard_module(),
        app_case(
            "prelude and extra",
            "src/main.veln",
            concat!(
                "use std::extra\n",
                "\n",
                "pub fn main() -> Int\n",
                "  extra::extra_answer(1)\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached_prelude_only = check_project_surface_module(&prelude_only);
    let cached_prelude_only =
        check_project_surface_module_with_standard_environment(&prelude_only, &reusable);

    assert_same_analysis("prelude only", uncached_prelude_only, cached_prelude_only);

    let uncached_prelude_and_extra = check_project_surface_module(&prelude_and_extra);
    let cached_prelude_and_extra =
        check_project_surface_module_with_standard_environment(&prelude_and_extra, &reusable);

    assert_same_analysis(
        "prelude and extra",
        uncached_prelude_and_extra,
        cached_prelude_and_extra,
    );

    let _ = check_project_surface_module_with_standard_environment(&prelude_and_extra, &reusable);
    assert_eq!(reusable.prepared_environment_count_for_test(), 1);
}

#[test]
fn reusable_standard_environment_keeps_selected_facts_constant_for_unrelated_standard_modules() {
    let _guard = standard_reuse_test_lock();
    let selected_modules = std::iter::once("std::prelude".to_string()).collect();

    let base_standard = standard_modules_with_extra_module();
    let base_reusable = prepare_reusable_standard_surface_module_environment(&base_standard)
        .with_current_identity_for_test();
    let base_environment = base_reusable.environment_for_modules_for_test(&selected_modules);

    let expanded_standard = merge_modules(vec![
        standard_modules_with_extra_module(),
        unrelated_annotated_standard_module(128),
    ]);
    let expanded_reusable =
        prepare_reusable_standard_surface_module_environment(&expanded_standard)
            .with_current_identity_for_test();
    let expanded_environment =
        expanded_reusable.environment_for_modules_for_test(&selected_modules);

    assert_eq!(
        expanded_environment.standard_function_modules_for_test(),
        base_environment.standard_function_modules_for_test()
    );
    assert_eq!(
        expanded_reusable.selected_declaration_count_for_test(&selected_modules),
        base_reusable.selected_declaration_count_for_test(&selected_modules)
    );

    let module = merge_modules(vec![
        standard_module(),
        app_case(
            "prelude selected",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let base_checked =
        check_project_surface_module_with_standard_environment(&module, &base_reusable);
    let expanded_checked =
        check_project_surface_module_with_standard_environment(&module, &expanded_reusable);

    assert_same_analysis("prelude selected", base_checked, expanded_checked);
}

#[test]
fn reusable_standard_environment_identity_mismatch_uses_uncached_analysis() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard);
    let module = merge_modules(vec![
        standard,
        app_case(
            "identity mismatch",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("identity mismatch", uncached, cached);
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 0);
}

#[test]
fn embedded_reusable_standard_environment_detects_current_identity() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = crate::types::embedded_standard_surface_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard);

    assert!(reusable.has_current_identity_for_test());
    assert_eq!(
        crate::standard_reuse_counters::embedded_standard_lowered_decodes(),
        2 * veln_stdlib::package_bundle().lowered_files.len(),
        "materialization and comparison should each decode the build-time modules"
    );
}

#[test]
fn embedded_reusable_standard_environment_has_current_identity_and_reuses_application_facts() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = crate::types::embedded_standard_surface_module();
    let reusable = prepare_current_reusable_standard_surface_module_environment(&standard);
    assert!(reusable.has_current_identity_for_test());
    assert_eq!(
        crate::standard_reuse_counters::embedded_standard_lowered_decodes(),
        veln_stdlib::package_bundle().lowered_files.len(),
        "embedded materialization should decode each build-time module once"
    );

    let module = merge_modules(vec![
        standard,
        app_case(
            "embedded identity",
            "src/main.veln",
            concat!("pub fn main() -> Int\n", "  1\n", "end\n",),
        )
        .module,
    ]);

    let _ = check_project_surface_module_with_standard_environment(&module, &reusable);
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 1);
}

#[test]
fn reusable_standard_environment_is_prepared_once_for_repeated_and_concurrent_projects() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );

    let alpha = merge_modules(vec![
        standard.clone(),
        app_case(
            "alpha",
            "src/shared.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);
    let beta = merge_modules(vec![
        standard.clone(),
        app_case(
            "beta",
            "src/shared.veln",
            concat!(
                "pub fn main() -> Bool\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let alpha_expected = checked_json(&alpha, &reusable);
    let beta_expected = checked_json(&beta, &reusable);
    assert_ne!(alpha_expected, beta_expected);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 2);
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );

    for module in [alpha.clone(), beta.clone(), alpha.clone(), beta.clone()] {
        let _ = check_project_surface_module_with_standard_environment(&module, &reusable);
    }
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 6);
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );

    thread::scope(|scope| {
        let handles = (0..12)
            .map(|index| {
                let module = if index % 2 == 0 {
                    alpha.clone()
                } else {
                    beta.clone()
                };
                let reusable = &reusable;
                scope.spawn(move || (index, checked_json(&module, reusable)))
            })
            .collect::<Vec<_>>();

        for handle in handles {
            let (index, diagnostics) = handle.join().expect("analysis should not panic");
            if index % 2 == 0 {
                assert_eq!(diagnostics, alpha_expected);
            } else {
                assert_eq!(diagnostics, beta_expected);
            }
        }
    });
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 18);
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );
}

#[test]
fn reachable_project_lowering_with_reusable_standard_environment_keeps_standard_bodies() {
    let _guard = standard_reuse_test_lock();
    let mut standard = standard_module();
    for decl in &mut standard.types {
        if decl.name.as_deref() == Some("PayloadShape") {
            decl.visibility = Visibility::Public;
        }
    }
    for decl in &mut standard.schemas {
        if decl.name.as_deref() == Some("Packet") {
            decl.visibility = Visibility::Public;
        }
    }
    for decl in &mut standard.functions {
        if decl.name.as_deref() == Some("identity") {
            decl.visibility = Visibility::Public;
        }
    }
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let app = app_case(
        "reachable standard bodies",
        "src/main.veln",
        concat!(
            "mod app.main\n",
            "\n",
            "fn compute() -> Int effects [prelude::Ask]\n",
            "  perform prelude::Ask::value()\n",
            "end\n",
            "\n",
            "pub fn main(input: ByteView, base: ByteOffset, payload: prelude::SharedPayload) -> DecodeStep<{value: Int}>\n",
            "  let observed = handle compute() with prelude::ask(1)\n",
            "  let boxed = payload\n",
            "  let readiness: DecodeReadiness = NeedEnd\n",
            "  NeedMore(readiness)\n",
            "end\n",
        ),
    )
    .module;
    let module_header = app.module.clone();
    let mut module = merge_modules(vec![standard, app]);
    module.module = module_header;

    let lowered =
        lower_project_reachable_surface_module_with_standard_environment(&module, &reusable);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("reachable core should lower");
    assert!(
        core.effects.iter().any(|effect| effect.name == "Ask"),
        "standard effect should be present: {:#?}",
        core.effects
    );
    assert_core_function(core, "__veln_std$prelude$provide");
    assert!(
        core.functions.iter().any(|function| {
            function.name == "main"
                && function.body.iter().any(|stmt| {
                    matches!(
                        &stmt.kind,
                        CoreStmtKind::Let { expr, .. }
                            if matches!(
                                &expr.kind,
                                CoreExprKind::Handle { providers, .. }
                                    if providers.iter().any(|provider| {
                                        provider.function
                                            == "__veln_std$prelude$__handler_3$ask_5$value"
                                    })
                            )
                    )
                })
        }),
        "application entry should retain the standard handler provider: {:#?}",
        core.functions
    );
    assert!(
        core.functions.iter().any(|function| {
            function.name == "main"
                && function
                    .params
                    .iter()
                    .any(|param| format!("{:?}", param.ty).contains("SharedPayload"))
        }),
        "application entry should retain the standard type: {:#?}",
        core.functions
    );

    let ir = lowered.ir.as_ref().expect("reachable IR should lower");
    assert!(
        ir.schema_decoders.iter().any(|schema| {
            schema.schema_name == "Packet" && schema.function_name == "byte_decode_packet"
        }),
        "standard schema decoder should be present: {:#?}",
        ir.schema_decoders
    );
}
