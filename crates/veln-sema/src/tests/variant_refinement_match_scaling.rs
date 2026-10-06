use super::*;
use crate::types::TypeEnvironment;
use veln_diagnostics::JsonValue;

#[derive(Clone, Copy, Debug)]
enum CoveragePath {
    Complete,
    RedundantCatchAll,
    Duplicates,
    Impossible,
    Incomplete,
}

fn shared_scrutinee_type(diagnostic: &Diagnostic) -> &std::sync::Arc<JsonValue> {
    let JsonValue::Object(details) = &diagnostic.details else {
        panic!("diagnostic details must be an object")
    };
    let value = details
        .iter()
        .find_map(|(field, value)| (field == "scrutinee_type").then_some(value))
        .expect("scrutinee type detail");
    let JsonValue::Shared(value) = value else {
        panic!("scrutinee type detail must use shared storage")
    };
    value
}

fn refinement_source_message(diagnostic: &Diagnostic) -> &veln_diagnostics::DiagnosticText {
    let JsonValue::Object(entries) = &diagnostic.related[0] else {
        panic!("refinement source must be an object")
    };
    let value = entries
        .iter()
        .find_map(|(field, value)| (field == "message").then_some(value))
        .expect("refinement source message");
    let JsonValue::Text(message) = value else {
        panic!("refinement source message must use shared diagnostic text")
    };
    message
}

fn refined_match_scaling_source(variant_count: usize, path: CoveragePath) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("V{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("  Outside\nend\n");
    source.push_str("fn classify(state: ");
    source.push_str(
        &variants
            .iter()
            .map(|variant| format!("State::{variant}"))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    source.push_str(") -> ()\n  match state\n");
    let covered = if matches!(path, CoveragePath::Incomplete) {
        &variants[..variant_count - 1]
    } else {
        &variants[..]
    };
    for variant in covered {
        source.push_str(&format!("    {variant} => ()\n"));
    }
    if matches!(path, CoveragePath::Duplicates) {
        for variant in &variants {
            source.push_str(&format!("    {variant} => ()\n"));
        }
    }
    if matches!(path, CoveragePath::Impossible) {
        for _ in &variants {
            source.push_str("    Outside => ()\n");
        }
    }
    if matches!(path, CoveragePath::RedundantCatchAll) {
        source.push_str("    _ => ()\n");
    }
    source.push_str("  end\nend\n");
    source
}

fn assert_refined_match_diagnostics(
    path: CoveragePath,
    variant_count: usize,
    diagnostics: &[Diagnostic],
) {
    match path {
        CoveragePath::Complete => assert!(diagnostics.is_empty(), "{diagnostics:#?}"),
        CoveragePath::RedundantCatchAll => {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            assert_eq!(diagnostics[0].id, "type.match_redundant_arm");
            assert!(
                diagnostics[0]
                    .details
                    .to_json()
                    .contains("\"reason\":\"complete_prior_coverage\""),
                "{diagnostics:#?}"
            );
            assert_eq!(diagnostics[0].related.len(), variant_count);
        }
        CoveragePath::Duplicates => {
            assert_eq!(diagnostics.len(), variant_count, "{diagnostics:#?}");
            assert!(diagnostics.iter().all(|diagnostic| {
                diagnostic.id == "type.match_redundant_arm"
                    && diagnostic
                        .details
                        .to_json()
                        .contains("\"reason\":\"duplicate_variant\"")
                    && diagnostic.related.len() == 1
            }));
        }
        CoveragePath::Impossible => {
            assert_eq!(diagnostics.len(), variant_count, "{diagnostics:#?}");
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id == "type.match_impossible_variant")
            );
            assert!(std::sync::Arc::ptr_eq(
                shared_scrutinee_type(&diagnostics[0]),
                shared_scrutinee_type(&diagnostics[variant_count - 1]),
            ));
            assert!(
                refinement_source_message(&diagnostics[0]).shares_storage_with(
                    refinement_source_message(&diagnostics[variant_count - 1])
                )
            );
        }
        CoveragePath::Incomplete => {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            assert_eq!(diagnostics[0].id, "type.match_non_exhaustive");
            assert_eq!(diagnostics[0].related.len(), variant_count);
        }
    }
}

fn assert_shared_refined_parameter(parameter: &crate::semantic_model::Type, variant_count: usize) {
    assert!(
        matches!(
            parameter,
            crate::semantic_model::Type::VariantRefinement { variants, .. }
                if variants.len() == variant_count
        ),
        "generated match must retain its refined domain: {}",
        parameter.render()
    );
    let cloned_parameter = parameter.clone();
    let (
        crate::semantic_model::Type::VariantRefinement {
            variants: original_variants,
            ..
        },
        crate::semantic_model::Type::VariantRefinement {
            variants: cloned_variants,
            ..
        },
    ) = (parameter, &cloned_parameter)
    else {
        unreachable!("generated parameter is a refined domain")
    };
    assert!(std::sync::Arc::ptr_eq(original_variants, cloned_variants));
}

fn measure_refined_match_coverage_work(
    variant_count: usize,
    path: CoveragePath,
) -> (
    usize,
    crate::analysis::RefinedMatchDiagnosticWork,
    std::time::Duration,
) {
    let source = SourceFile::new(
        "main.veln",
        refined_match_scaling_source(variant_count, path),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let classify = environment.function("classify").expect("classify function");
    assert_shared_refined_parameter(&classify.params[0], variant_count);
    let function = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("classify"))
        .expect("classify declaration");
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    crate::analysis::reset_refined_match_coverage_work();
    crate::analysis::reset_refined_match_diagnostic_work();
    let started = std::time::Instant::now();
    let diagnostics =
        crate::analysis::check_function_body(function, &environment, &mut variant_diagnostics);
    let elapsed = started.elapsed();
    let coverage_work = crate::analysis::take_refined_match_coverage_work();
    let diagnostic_work = crate::analysis::take_refined_match_diagnostic_work();
    assert_refined_match_diagnostics(path, variant_count, &diagnostics);
    eprintln!(
        "{variant_count}-variant {path:?} refined match: {elapsed:?} \
         ({coverage_work:?}, {diagnostic_work:?})"
    );
    (coverage_work.units, diagnostic_work, elapsed)
}

#[test]
fn refined_match_coverage_work_grows_linearly() {
    for path in [
        CoveragePath::Complete,
        CoveragePath::RedundantCatchAll,
        CoveragePath::Duplicates,
        CoveragePath::Impossible,
        CoveragePath::Incomplete,
    ] {
        let measurements = [100, 200, 400]
            .map(|variant_count| measure_refined_match_coverage_work(variant_count, path));
        let work = measurements.map(|(work, _, _)| work);
        assert!(work[0] > 0, "the metric must observe {path:?} work");
        for adjacent in work.windows(2) {
            assert!(
                adjacent[1] <= adjacent[0] * 2 + 64,
                "doubling the refined domain must add only linear {path:?} work: {work:?}"
            );
        }
        let diagnostic_work = measurements.map(|(_, work, _)| work);
        assert!(
            diagnostic_work.iter().all(|work| work.renders == 1),
            "each refined match must render its domain once: {diagnostic_work:?}"
        );
        for adjacent in diagnostic_work.windows(2) {
            assert!(
                adjacent[1].retained_bytes <= adjacent[0].retained_bytes * 2 + 64,
                "doubling the refined domain must retain only linear diagnostic text: \
                 {diagnostic_work:?}"
            );
            assert!(
                adjacent[1].refinement_variants <= adjacent[0].refinement_variants * 2 + 2,
                "doubling the refined domain must materialize only linear refinement variants: \
                 {diagnostic_work:?}"
            );
        }
    }
}

fn singleton_matches_source(variant_count: usize, match_count: usize) -> String {
    let mut source = String::from("type State\n");
    for index in 0..variant_count {
        source.push_str(&format!("  V{index:04}\n"));
    }
    source.push_str("end\nfn classify(state: State::V0000) -> ()\n");
    for _ in 0..match_count {
        source.push_str("  match state\n    V0000 => ()\n  end\n");
    }
    source.push_str("end\n");
    source
}

fn nested_singleton_matches_source(variant_count: usize, depth: usize) -> String {
    let mut source = String::from("type State\n");
    for index in 0..variant_count {
        source.push_str(&format!("  V{index:04}\n"));
    }
    source.push_str("end\nfn classify(state: State::V0000) -> ()\n");
    source.push_str("  match state\n");
    for level in 0..depth {
        if level + 1 == depth {
            source.push_str("    V0000 => ()\n");
        } else {
            source.push_str("    V0000 => match state\n");
        }
    }
    for _ in 0..depth {
        source.push_str("  end\n");
    }
    source.push_str("end\n");
    source
}

fn measure_generated_coverage(
    source: String,
) -> (
    crate::analysis::RefinedMatchCoverageWork,
    std::time::Duration,
) {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let function = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("classify"))
        .expect("classify declaration");
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    crate::analysis::reset_refined_match_coverage_work();
    let started = std::time::Instant::now();
    let diagnostics =
        crate::analysis::check_function_body(function, &environment, &mut variant_diagnostics);
    let elapsed = started.elapsed();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    (crate::analysis::take_refined_match_coverage_work(), elapsed)
}

#[test]
fn singleton_match_state_is_independent_of_base_adt_width() {
    let measurements = [100, 200, 400].map(|variant_count| {
        let (work, elapsed) =
            measure_generated_coverage(singleton_matches_source(variant_count, 64));
        eprintln!("{variant_count}-variant ADT with 64 singleton matches: {elapsed:?} ({work:?})");
        work
    });

    assert!(
        measurements
            .windows(2)
            .all(
                |adjacent| adjacent[0].initialized_slots == adjacent[1].initialized_slots
                    && adjacent[0].units == adjacent[1].units
                    && adjacent[0].peak_retained_slots == adjacent[1].peak_retained_slots
                    && adjacent[0].cloned_labels == adjacent[1].cloned_labels
            ),
        "base ADT width must not change narrow-match state: {measurements:?}"
    );
    assert!(
        measurements.iter().all(|work| work.cloned_labels == 0),
        "complete singleton matches must share coverage labels: {measurements:?}"
    );
}

#[test]
fn singleton_match_setup_grows_with_match_count_not_base_width() {
    let measurements = [100, 200, 400, 800].map(|size| {
        let (work, elapsed) = measure_generated_coverage(singleton_matches_source(size, size));
        eprintln!("{size}-variant ADT with {size} singleton matches: {elapsed:?} ({work:?})");
        work
    });

    for adjacent in measurements.windows(2) {
        assert!(
            adjacent[1].initialized_slots <= adjacent[0].initialized_slots * 2 + 8,
            "doubling match count must add only linear slot initialization: {measurements:?}"
        );
        assert!(
            adjacent[1].units <= adjacent[0].units * 2 + 8,
            "doubling match count must add only linear coverage work: {measurements:?}"
        );
    }
    assert!(
        measurements
            .iter()
            .all(|work| work.peak_retained_slots == measurements[0].peak_retained_slots),
        "sequential singleton matches retain one narrow coverage state: {measurements:?}"
    );
}

#[test]
fn nested_singleton_match_retention_grows_with_depth_not_base_width() {
    let widths = [100, 200, 400].map(|variant_count| {
        let (work, elapsed) =
            measure_generated_coverage(nested_singleton_matches_source(variant_count, 12));
        eprintln!(
            "{variant_count}-variant ADT with depth-12 singleton matches: {elapsed:?} ({work:?})"
        );
        work
    });
    assert!(
        widths
            .windows(2)
            .all(|adjacent| adjacent[0].peak_retained_slots == adjacent[1].peak_retained_slots),
        "base width must not change retained nested coverage state: {widths:?}"
    );

    let depths = [8, 16, 32].map(|depth| {
        let (work, elapsed) =
            measure_generated_coverage(nested_singleton_matches_source(400, depth));
        eprintln!("400-variant ADT with depth-{depth} singleton matches: {elapsed:?} ({work:?})");
        work
    });
    for adjacent in depths.windows(2) {
        assert!(
            adjacent[1].peak_retained_slots <= adjacent[0].peak_retained_slots * 2 + 8,
            "doubling nesting depth must add only linear retained slots: {depths:?}"
        );
    }
}
