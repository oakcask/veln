use super::*;
use crate::types::TypeEnvironment;

#[derive(Clone, Copy, Debug)]
enum CoveragePath {
    Complete,
    RedundantCatchAll,
    Duplicates,
    Incomplete,
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
    if matches!(path, CoveragePath::RedundantCatchAll) {
        source.push_str("    _ => ()\n");
    }
    source.push_str("  end\nend\n");
    source
}

fn measure_refined_match_coverage_work(
    variant_count: usize,
    path: CoveragePath,
) -> (usize, std::time::Duration) {
    let source = SourceFile::new(
        "main.veln",
        refined_match_scaling_source(variant_count, path),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let classify = environment.function("classify").expect("classify function");
    assert!(
        matches!(
            &classify.params[0],
            crate::semantic_model::Type::VariantRefinement { variants, .. }
                if variants.len() == variant_count
        ),
        "generated match must retain its refined domain: {}",
        classify.params[0].render()
    );
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
    let work = crate::analysis::take_refined_match_coverage_work();
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
        CoveragePath::Incomplete => {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            assert_eq!(diagnostics[0].id, "type.match_non_exhaustive");
            assert_eq!(diagnostics[0].related.len(), variant_count);
        }
    }
    eprintln!("{variant_count}-variant {path:?} refined match: {elapsed:?} ({work} units)");
    (work, elapsed)
}

#[test]
fn refined_match_coverage_work_grows_linearly() {
    for path in [
        CoveragePath::Complete,
        CoveragePath::RedundantCatchAll,
        CoveragePath::Duplicates,
        CoveragePath::Incomplete,
    ] {
        let measurements = [100, 200, 400]
            .map(|variant_count| measure_refined_match_coverage_work(variant_count, path));
        let work = measurements.map(|(work, _)| work);
        assert!(work[0] > 0, "the metric must observe {path:?} work");
        for adjacent in work.windows(2) {
            assert!(
                adjacent[1] <= adjacent[0] * 2 + 64,
                "doubling the refined domain must add only linear {path:?} work: {work:?}"
            );
        }
    }
}
