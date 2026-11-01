use veln_diagnostics::diagnostic_to_json;
use veln_project::Project;
use veln_source::SourceFile;

use super::*;
use crate::surface::reachable_entry_module_with_cache;

mod reachable_companion_recovery_tests;

#[test]
fn invalid_standard_registry_failure_surfaces_as_checked_internal_failure() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new("main.veln", clean_source())],
        manifest: None,
    };

    let analysis =
        analyze_project_with_standard_provider(project, DoctestMode::Exclude, None, |_| {
            Err(Box::new(invalid_standard_symbol_case_diagnostic()))
        });

    assert!(analysis.source_diagnostics().is_empty());
    assert!(analysis.semantic_diagnostics().is_empty());
    assert_eq!(
        diagnostic_json(&analysis.checked_diagnostics()),
        vec![concat!(
            "{\"id\":\"toolchain.invalid_symbol_case\",",
            "\"severity\":\"error\",\"kind\":\"toolchain\",",
            "\"message\":\"compiler-provided function `BadAdapter` from `compiler_adapter` ",
            "must start with an ASCII lowercase letter\",",
            "\"span\":null,",
            "\"details\":{\"provider\":\"compiler_adapter\",\"name\":\"BadAdapter\",",
            "\"name_class\":\"function\",\"required_initial\":\"ascii_lowercase\"},",
            "\"related\":[]}"
        )]
    );
}

#[test]
fn combined_source_and_delayed_parse_diagnostics_follow_written_order() {
    let text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "fn rejected(value: state::ready | State::closed) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let analysis = analyze_project(
        Project {
            root: ".".into(),
            files: vec![SourceFile::new("main.veln", text)],
            manifest: None,
        },
        DoctestMode::Exclude,
    );
    let source = analysis.source_diagnostics();
    assert_eq!(source.len(), 1, "{source:#?}");
    assert_eq!(source[0].id, "parse.variant_refinement_type");
    assert_eq!(
        &text[source[0].span.as_ref().unwrap().start.offset
            ..source[0].span.as_ref().unwrap().end.offset],
        "closed"
    );

    for diagnostics in [
        analysis.semantic_diagnostics(),
        analysis.checked_diagnostics(),
    ] {
        let parse = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
            .collect::<Vec<_>>();
        assert_eq!(parse.len(), 2, "{diagnostics:#?}");
        assert_eq!(
            parse
                .iter()
                .map(|diagnostic| {
                    let span = diagnostic.span.as_ref().expect("parse span");
                    &text[span.start.offset..span.end.offset]
                })
                .collect::<Vec<_>>(),
            ["ready", "closed"],
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .take(2)
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            [
                "parse.variant_refinement_type",
                "parse.variant_refinement_type",
            ],
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn separated_reachable_lowering_matches_combined_lowering_outputs() {
    let analysis = analyze_project(
        Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "src/main.veln",
                concat!(
                    "effect Ask\n",
                    "  value() -> Int\n",
                    "end\n",
                    "\n",
                    "fn compute() -> Int effects [Ask]\n",
                    "  perform Ask::value()\n",
                    "end\n",
                    "\n",
                    "handler ask(seed: Int) for Ask\n",
                    "  value() => seed\n",
                    "end\n",
                    "\n",
                    "pub fn main() -> Int\n",
                    "  let observed = handle compute() with ask(1)\n",
                    "  observed + vec_len([1, 2, 3])\n",
                    "end\n",
                ),
            )],
            manifest: None,
        },
        DoctestMode::Exclude,
    );

    let separated = analysis.lower_reachable_entry("main", FunctionKind::Function);
    let combined_module =
        merge_surface_modules_for_test(&analysis.selected_standard, &analysis.module);
    let combined_reachable = reachable_entry_module_with_cache(
        &combined_module,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );
    let standard = standard_environment_for_modules(&analysis.selected_standard_module_names);
    let standard = standard.expect("standard environment");
    let combined_lowered =
        veln_sema::lower_project_reachable_surface_module_with_standard_environment(
            &combined_reachable,
            &standard.environment,
        );

    assert_eq!(
        diagnostic_json(&separated.lowered.diagnostics),
        diagnostic_json(&combined_lowered.diagnostics)
    );
    assert_eq!(separated.lowered.core, combined_lowered.core);
    assert_eq!(separated.lowered.ir, combined_lowered.ir);
    assert_eq!(
        reachable_function_names(&separated.module),
        reachable_function_names(&combined_reachable)
    );
}

fn diagnostic_json(diagnostics: &[veln_diagnostics::Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

fn invalid_standard_symbol_case_diagnostic() -> veln_diagnostics::Diagnostic {
    veln_diagnostics::toolchain_invalid_symbol_case_diagnostic(
        "compiler_adapter",
        "BadAdapter",
        veln_diagnostics::ToolchainSymbolNameClass::Function,
        veln_diagnostics::ToolchainSymbolNameFailureReason::InvalidCase,
    )
}

fn clean_source() -> &'static str {
    "pub fn main() -> Int\n  1\nend\n"
}

fn reachable_function_names(module: &SurfaceModule) -> Vec<(&str, &str)> {
    let mut functions = module
        .functions
        .iter()
        .filter_map(|function| Some((function.module_name.as_deref()?, function.name.as_deref()?)))
        .collect::<Vec<_>>();
    functions.sort_unstable();
    functions
}

fn merge_surface_modules_for_test(
    standard_module: &SurfaceModule,
    application_module: &SurfaceModule,
) -> SurfaceModule {
    let mut merged = standard_module.clone();
    merged.uses.extend(application_module.uses.clone());
    merged.aliases.extend(application_module.aliases.clone());
    merged.effects.extend(application_module.effects.clone());
    merged.handlers.extend(application_module.handlers.clone());
    merged.types.extend(application_module.types.clone());
    merged.schemas.extend(application_module.schemas.clone());
    merged
        .functions
        .extend(application_module.functions.clone());
    if merged.module.is_none() {
        merged.module = application_module.module.clone();
    }
    merged
}
