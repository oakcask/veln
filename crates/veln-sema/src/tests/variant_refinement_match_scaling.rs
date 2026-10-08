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
        let renders_domain = matches!(
            path,
            CoveragePath::RedundantCatchAll | CoveragePath::Duplicates | CoveragePath::Impossible
        );
        assert!(
            diagnostic_work
                .iter()
                .all(|work| { work.renders == usize::from(renders_domain) }),
            "only a refined match that emits a diagnostic renders its domain: {diagnostic_work:?}"
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

fn residual_catch_all_source(variant_count: usize) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("V{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("  Outside\nend\nfn classify(state: ");
    source.push_str(
        &variants
            .iter()
            .map(|variant| format!("State::{variant}"))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    source.push_str(") -> ()\n  match state\n    _ => ()\n  end\nend\n");
    source
}

fn measure_generated_coverage(
    source: String,
) -> (
    crate::analysis::RefinedMatchCoverageWork,
    crate::analysis::RefinedMatchDiagnosticWork,
    std::time::Duration,
) {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let classify = environment.function("classify").expect("classify function");
    assert!(
        matches!(
            &classify.params[0],
            crate::semantic_model::Type::VariantRefinement { .. }
        ),
        "generated parameter must stay refined: {}",
        classify.params[0].render()
    );
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
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    (
        crate::analysis::take_refined_match_coverage_work(),
        crate::analysis::take_refined_match_diagnostic_work(),
        elapsed,
    )
}

#[test]
fn singleton_match_state_is_independent_of_base_adt_width() {
    let measurements = [100, 200, 400].map(|variant_count| {
        let (work, _, elapsed) =
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
    assert!(
        measurements.iter().all(|work| {
            work.initialized_slots > 0 && work.peak_retained_slots > 0 && work.units > 0
        }),
        "coverage counters must observe singleton match state: {measurements:?}"
    );
}

#[test]
fn singleton_match_setup_grows_with_match_count_not_base_width() {
    let measurements = [100, 200, 400, 800].map(|match_count| {
        let (work, _, elapsed) =
            measure_generated_coverage(singleton_matches_source(400, match_count));
        eprintln!("400-variant ADT with {match_count} singleton matches: {elapsed:?} ({work:?})");
        work
    });

    for adjacent in measurements.windows(2) {
        assert!(
            adjacent[1].initialized_slots > adjacent[0].initialized_slots,
            "more matches must initialize more coverage slots: {measurements:?}"
        );
        assert!(
            adjacent[1].initialized_slots <= adjacent[0].initialized_slots * 2 + 8,
            "doubling match count must add only linear slot initialization: {measurements:?}"
        );
        assert!(
            adjacent[1].units > adjacent[0].units,
            "more matches must perform more coverage work: {measurements:?}"
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
        let (work, _, elapsed) =
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
        let (work, _, elapsed) =
            measure_generated_coverage(nested_singleton_matches_source(400, depth));
        eprintln!("400-variant ADT with depth-{depth} singleton matches: {elapsed:?} ({work:?})");
        work
    });
    for adjacent in depths.windows(2) {
        assert!(
            adjacent[1].peak_retained_slots > adjacent[0].peak_retained_slots,
            "deeper nesting must retain more coverage slots: {depths:?}"
        );
        assert!(
            adjacent[1].peak_retained_slots <= adjacent[0].peak_retained_slots * 2 + 8,
            "doubling nesting depth must add only linear retained slots: {depths:?}"
        );
    }
}

#[test]
fn unchanged_residual_catch_all_shares_domain_storage() {
    for variant_count in [100, 200, 400] {
        let (constructor_work, _, _) = measure_generated_coverage(refined_match_scaling_source(
            variant_count,
            CoveragePath::Complete,
        ));
        let (catch_all_work, diagnostic_work, elapsed) =
            measure_generated_coverage(residual_catch_all_source(variant_count));
        eprintln!("{variant_count}-variant residual catch-all: {elapsed:?} ({catch_all_work:?})");
        assert!(
            catch_all_work.peak_retained_slots < constructor_work.peak_retained_slots,
            "an unchanged residual must share the original refinement instead of retaining ranks: \
             constructor={constructor_work:?}, catch_all={catch_all_work:?}"
        );
        assert!(
            catch_all_work.initialized_slots < constructor_work.initialized_slots,
            "an unchanged residual must not initialize full-width collections: \
             constructor={constructor_work:?}, catch_all={catch_all_work:?}"
        );
        assert_eq!(
            diagnostic_work.renders, 0,
            "accepted catch-all renders no diagnostic"
        );
        assert_eq!(diagnostic_work.refinement_variants, 0);
    }
}

fn nested_residual_catch_all_source(variant_count: usize, depth: usize) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("V{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("  Outside\nend\nfn classify(state: ");
    source.push_str(
        &variants
            .iter()
            .map(|variant| format!("State::{variant}"))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    source.push_str(") -> ()\n");
    let mut scrutinee = "state".to_string();
    for level in 0..depth {
        let binding = format!("remaining{level}");
        source.push_str(&format!("  match {scrutinee}\n    {binding} => "));
        scrutinee = binding;
    }
    source.push_str("()\n");
    for _ in 0..depth {
        source.push_str("  end\n");
    }
    source.push_str("end\n");
    source
}

#[test]
fn nested_wide_residual_matches_have_additive_state_and_work() {
    let measurements = [(100, 8), (200, 16), (400, 32)].map(|(width, depth)| {
        let (coverage, diagnostics, elapsed) =
            measure_generated_coverage(nested_residual_catch_all_source(width, depth));
        eprintln!(
            "{width}-variant refined domain with depth-{depth} residual matches: \
             {elapsed:?} ({coverage:?}, {diagnostics:?})"
        );
        assert_eq!(diagnostics.renders, 0);
        assert_eq!(diagnostics.retained_bytes, 0);
        assert_eq!(diagnostics.refinement_variants, 0);
        assert_eq!(diagnostics.peak_cached_diagnostic_bytes, 0);
        assert_eq!(diagnostics.peak_retained_refinement_variants, 0);
        assert_eq!(diagnostics.peak_shared_domain_handles, depth + 1);
        (coverage, diagnostics)
    });

    for adjacent in measurements.windows(2) {
        let (smaller, _) = adjacent[0];
        let (larger, _) = adjacent[1];
        assert!(
            larger.units <= smaller.units * 2 + 8,
            "doubling width and depth must not multiply setup work: {measurements:?}"
        );
        assert!(
            larger.peak_retained_slots <= smaller.peak_retained_slots * 2 + 8,
            "doubling width and depth must retain only additive match-local state: \
             {measurements:?}"
        );
        assert!(
            larger.initialized_slots <= smaller.initialized_slots * 2 + 8,
            "doubling width and depth must initialize only additive match-local state: \
             {measurements:?}"
        );
    }
}

fn widened_alias_scaling_source(
    variant_count: usize,
    alias_count: usize,
    unrelated_non_adt_count: usize,
    unrelated_adt_count: usize,
) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("V{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("  Outside\nend\n");
    if unrelated_adt_count > 0 {
        source.push_str("type Other\n  OtherValue\nend\n");
    }
    source.push_str("fn classify(state: ");
    source.push_str(
        &variants
            .iter()
            .map(|variant| format!("State::{variant}"))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    source.push_str(") -> ()\n");
    let mut alias = "state".to_string();
    for index in 0..alias_count {
        let next = format!("alias{index}");
        source.push_str(&format!("  let {next}: State = {alias}\n"));
        alias = next;
    }
    for index in 0..unrelated_non_adt_count {
        source.push_str(&format!("  let unrelated{index}: Int = {index}\n"));
    }
    for index in 0..unrelated_adt_count {
        source.push_str(&format!("  let unrelated_adt{index}: Other = OtherValue\n"));
    }
    source.push_str(&format!("  match {alias}\n"));
    for variant in &variants {
        source.push_str(&format!("    {variant} => ()\n"));
    }
    source.push_str("  end\nend\n");
    source
}

fn measure_transparent_alias_and_match_work(
    source: String,
) -> (
    crate::analysis::TransparentAliasWork,
    crate::analysis::RefinedMatchCoverageWork,
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
    crate::analysis::reset_transparent_alias_work();
    crate::analysis::reset_refined_match_coverage_work();
    let diagnostics =
        crate::analysis::check_function_body(function, &environment, &mut variant_diagnostics);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let work = crate::analysis::take_transparent_alias_work();
    assert_eq!(work.retained_groups, 0, "group owners must be released");
    assert_eq!(work.retained_members, 0, "member owners must be released");
    assert_eq!(
        work.active_refinements, 0,
        "refinement frames must be released"
    );
    (work, crate::analysis::take_refined_match_coverage_work())
}

fn measure_transparent_alias_work(source: String) -> crate::analysis::TransparentAliasWork {
    measure_transparent_alias_and_match_work(source).0
}

#[test]
fn widened_alias_group_work_scales_by_independent_dimensions() {
    let aliases = [100, 200, 400]
        .map(|count| measure_transparent_alias_work(widened_alias_scaling_source(32, count, 0, 0)));
    for adjacent in aliases.windows(2) {
        assert!(
            adjacent[1].peak_retained_members <= adjacent[0].peak_retained_members * 2 + 2,
            "alias entries must grow linearly: {aliases:?}"
        );
        assert_eq!(adjacent[0].peak_retained_groups, 1, "{aliases:?}");
        assert_eq!(adjacent[1].peak_retained_groups, 1, "{aliases:?}");
        assert!(
            adjacent[1].group_lookups <= adjacent[0].group_lookups * 2 + 2,
            "alias construction and lookup work must grow linearly: {aliases:?}"
        );
        assert!(
            adjacent[1].member_lookups <= adjacent[0].member_lookups * 2 + 2,
            "alias member lookup work must grow linearly: {aliases:?}"
        );
    }

    let arms = [100, 200, 400]
        .map(|count| measure_transparent_alias_work(widened_alias_scaling_source(count, 16, 0, 0)));
    for adjacent in arms.windows(2) {
        assert!(
            adjacent[1].group_lookups <= adjacent[0].group_lookups * 2 + 8,
            "constructor arms must add only linear group work: {arms:?}"
        );
        assert_eq!(
            adjacent[0].peak_retained_members, adjacent[1].peak_retained_members,
            "domain width must not duplicate alias entries: {arms:?}"
        );
    }

    let unrelated = [100, 200, 400].map(|count| {
        measure_transparent_alias_and_match_work(widened_alias_scaling_source(32, 16, count, 0))
    });
    assert!(unrelated.iter().all(|(aliases, _)| {
        aliases.groups_created == 1
            && aliases.peak_retained_groups == 1
            && aliases.peak_retained_members == 17
    }));
    assert!(
        unrelated.windows(2).all(|adjacent| {
            adjacent[0].0.group_lookups == adjacent[1].0.group_lookups
                && adjacent[0].0.member_lookups == adjacent[1].0.member_lookups
                && adjacent[0].0.peak_retained_groups == adjacent[1].0.peak_retained_groups
                && adjacent[0].0.peak_retained_members == adjacent[1].0.peak_retained_members
                && adjacent[0].1.initialized_slots == adjacent[1].1.initialized_slots
                && adjacent[0].1.units == adjacent[1].1.units
                && adjacent[0].1.peak_retained_slots == adjacent[1].1.peak_retained_slots
                && adjacent[0].1.cloned_labels == adjacent[1].1.cloned_labels
        }),
        "unrelated non-ADT bindings must not add alias-group work or state: {unrelated:?}"
    );

    let unrelated_adts = [100, 200, 400].map(|count| {
        measure_transparent_alias_and_match_work(widened_alias_scaling_source(32, 16, 0, count))
    });
    for (index, count) in [100, 200, 400].into_iter().enumerate() {
        assert_eq!(unrelated_adts[index].0.groups_created, count + 1);
        assert_eq!(unrelated_adts[index].0.peak_retained_groups, count + 1);
        assert_eq!(unrelated_adts[index].0.peak_retained_members, count + 17);
    }
    assert!(
        unrelated_adts.windows(2).all(|adjacent| {
            adjacent[0].0.group_lookups == adjacent[1].0.group_lookups
                && adjacent[0].0.member_lookups == adjacent[1].0.member_lookups
                && adjacent[0].0.peak_active_refinements == adjacent[1].0.peak_active_refinements
                && adjacent[0].1.initialized_slots == adjacent[1].1.initialized_slots
                && adjacent[0].1.units == adjacent[1].1.units
                && adjacent[0].1.peak_retained_slots == adjacent[1].1.peak_retained_slots
                && adjacent[0].1.cloned_labels == adjacent[1].1.cloned_labels
        }),
        "unrelated ADT bindings must not add repeated match or alias-group lookup work: \
         {unrelated_adts:?}"
    );
}

#[test]
fn nested_complete_alias_frames_have_additive_state() {
    let depths = [8, 16, 32]
        .map(|depth| measure_transparent_alias_work(nested_residual_catch_all_source(400, depth)));
    for (index, depth) in [8, 16, 32].into_iter().enumerate() {
        assert_eq!(depths[index].peak_retained_groups, 1, "{depths:?}");
        assert_eq!(depths[index].peak_active_refinements, depth, "{depths:?}");
        assert_eq!(depths[index].peak_retained_members, depth + 1, "{depths:?}");
    }
    for adjacent in depths.windows(2) {
        assert!(
            adjacent[1].group_lookups <= adjacent[0].group_lookups * 2 + 8,
            "nested group work must grow linearly: {depths:?}"
        );
        assert!(
            adjacent[1].peak_active_refinements <= adjacent[0].peak_active_refinements * 2 + 1,
            "nested frame state must grow linearly: {depths:?}"
        );
    }

    let width_and_depth = [(100, 8), (200, 16), (400, 32)].map(|(width, depth)| {
        measure_transparent_alias_work(nested_residual_catch_all_source(width, depth))
    });
    for adjacent in width_and_depth.windows(2) {
        assert!(
            adjacent[1].group_lookups <= adjacent[0].group_lookups * 2 + 8,
            "simultaneous width and depth growth must remain additive: {width_and_depth:?}"
        );
        assert!(
            adjacent[1].peak_retained_members <= adjacent[0].peak_retained_members * 2 + 2,
            "simultaneous width and depth growth must not multiply state: {width_and_depth:?}"
        );
    }
}
