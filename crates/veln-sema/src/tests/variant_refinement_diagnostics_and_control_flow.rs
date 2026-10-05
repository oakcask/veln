use super::*;
use crate::types::TypeEnvironment;
use veln_diagnostics::JsonValue;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

const STATE_DECL: &str = concat!(
    "type State\n",
    "  Ready\n",
    "  Closed\n",
    "  Failed\n",
    "end\n",
);

fn repeated_broad_variant_mismatch_source(variant_count: usize, use_count: usize) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("Variant{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("  Excluded\nend\n");
    let broad = variants
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    source.push_str(&format!(
        "fn accept(value: {broad}) -> ()\n  ()\nend\nfn check(value: State) -> ()\n"
    ));
    for _ in 0..use_count {
        source.push_str("  accept(value)\n");
    }
    source.push_str("end\n");
    source
}

fn distinct_variant_mismatch_source(expected_count: usize, actual_count: usize) -> String {
    let expected_variants = (0..expected_count)
        .map(|index| format!("Expected{index:04}"))
        .collect::<Vec<_>>();
    let actual_variants = (0..actual_count)
        .map(|index| format!("Actual{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in expected_variants.iter().chain(&actual_variants) {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("end\n");
    let broad = expected_variants
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    source.push_str(&format!(
        "fn accept(value: {broad}) -> ()\n  ()\nend\nfn check() -> ()\n"
    ));
    for variant in &actual_variants {
        source.push_str(&format!("  accept({variant})\n"));
    }
    source.push_str("end\n");
    source
}

fn detail_field<'a>(diagnostic: &'a Diagnostic, name: &str) -> &'a JsonValue {
    let JsonValue::Object(entries) = &diagnostic.details else {
        panic!("diagnostic details must be an object")
    };
    entries
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .unwrap_or_else(|| panic!("missing diagnostic detail `{name}`"))
}

#[test]
fn repeated_broad_variant_mismatches_share_retained_diagnostic_facts() {
    for (variant_count, use_count) in [(64, 64), (64, 128), (128, 64), (128, 128)] {
        let diagnostics = diagnostics_for(&repeated_broad_variant_mismatch_source(
            variant_count,
            use_count,
        ));
        assert_eq!(diagnostics.len(), use_count, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.variant_mismatch"),
            "{diagnostics:#?}"
        );
        let first = &diagnostics[0];
        let JsonValue::Shared(first_expected_variants) = detail_field(first, "expected_variants")
        else {
            panic!("expected variants must use shared retained storage")
        };
        let JsonValue::Shared(first_excluded_variants) = detail_field(first, "excluded_variants")
        else {
            panic!("excluded variants must use shared retained storage")
        };
        for diagnostic in diagnostics.iter().skip(1) {
            assert!(diagnostic.message.shares_storage_with(&first.message));
            let JsonValue::Shared(expected_variants) =
                detail_field(diagnostic, "expected_variants")
            else {
                panic!("expected variants must use shared retained storage")
            };
            let JsonValue::Shared(excluded_variants) =
                detail_field(diagnostic, "excluded_variants")
            else {
                panic!("excluded variants must use shared retained storage")
            };
            assert!(std::sync::Arc::ptr_eq(
                expected_variants,
                first_expected_variants
            ));
            assert!(std::sync::Arc::ptr_eq(
                excluded_variants,
                first_excluded_variants
            ));
        }
        for diagnostic in [first, diagnostics.last().unwrap()] {
            let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
            assert!(json.contains("\"form\":\"all_except_expected\""), "{json}");
            assert!(json.contains("State::Variant0000"), "{json}");
            assert!(
                json.contains(&format!("State::Variant{:04}", variant_count - 1)),
                "{json}"
            );
        }
    }
}

#[test]
fn distinct_variant_mismatches_retain_linear_diagnostic_cache_keys() {
    for (expected_count, actual_count) in [(64, 64), (128, 128)] {
        let source = SourceFile::new(
            "main.veln",
            distinct_variant_mismatch_source(expected_count, actual_count),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::analysis::reset_retained_variant_diagnostic_key_variants();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();
        let retained = crate::analysis::take_retained_variant_diagnostic_key_variants();
        eprintln!(
            "{expected_count} expected and {actual_count} distinct actual variants: {elapsed:?}, {retained} retained cache-key variants"
        );

        assert_eq!(diagnostics.len(), actual_count, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.variant_mismatch"),
            "{diagnostics:#?}"
        );
        assert_eq!(
            retained,
            expected_count + actual_count,
            "one broad expected key and each singleton actual key should be retained once"
        );
    }
}

fn large_variant_union_source(variant_count: usize) -> String {
    let variants = (0..=variant_count)
        .map(|index| format!("Variant{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("end\n");
    let broad = variants[..variant_count]
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let narrow = variants[..variant_count - 1]
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    source.push_str(&format!(
        "fn accept(value: {narrow}) -> ()\n  ()\nend\nfn check(value: {broad}) -> ()\n  accept(value)\nend\n"
    ));
    source
}

#[test]
fn large_variant_union_semantic_analysis_work_grows_linearly() {
    let work = [128, 256, 512].map(|variant_count| {
        let source = SourceFile::new("main.veln", large_variant_union_source(variant_count));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::type_relations::reset_variant_set_lookups();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count}-variant semantic analysis: {:?}",
            started.elapsed()
        );
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].id, "type.variant_mismatch");
        crate::type_relations::take_variant_set_lookups()
    });
    assert!(work[0] > 0, "the metric must observe variant-set work");
    assert!(
        work[1] <= work[0] * 2 + 32 && work[2] <= work[1] * 2 + 32,
        "doubling the variant count must add only linear set work: {work:?}"
    );
}

#[test]
fn source_annotations_canonicalize_variant_sets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "fn finite(value: State::Closed | State::Ready | State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn complete(value: State::Failed | State::Ready | State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    assert_eq!(
        environment.function("finite").unwrap().params[0].render(),
        "State::Ready | State::Closed"
    );
    assert_eq!(
        environment.function("complete").unwrap().params[0].render(),
        "State"
    );
}

#[test]
fn nested_mismatch_diagnostic_output_grows_linearly_with_variant_count() {
    fn diagnostic_bytes(variant_count: usize) -> (usize, usize) {
        let mut source = String::from("type State\n");
        for index in 0..variant_count {
            source.push_str(&format!("  Variant{index:03}\n"));
        }
        source.push_str("end\n");
        for index in 0..variant_count {
            source.push_str(&format!(
                "fn reject{index:03}(value: {{state: State::Variant{index:03}}}) -> ()\n"
            ));
            source.push_str("  let rejected: {state: State} = value\n");
            source.push_str("end\n");
        }
        let diagnostics = diagnostics_for(&source);
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .collect::<Vec<_>>();
        assert_eq!(mismatches.len(), diagnostics.len(), "{diagnostics:#?}");
        let bytes = mismatches
            .iter()
            .map(|diagnostic| {
                veln_diagnostics::diagnostic_to_json(diagnostic)
                    .to_json()
                    .len()
            })
            .sum();
        (mismatches.len(), bytes)
    }

    let (small_count, small_bytes) = diagnostic_bytes(64);
    let (large_count, large_bytes) = diagnostic_bytes(128);
    eprintln!(
        "nested mismatch diagnostic bytes at 64 and 128 variants: {small_bytes}, {large_bytes}"
    );
    assert_eq!((small_count, large_count), (64, 128));
    assert!(
        large_bytes * 2 <= small_bytes * 5,
        "diagnostic bytes grew too quickly: {small_bytes} -> {large_bytes}"
    );
}

#[test]
fn final_if_checks_every_branch_and_untyped_expressions_do_not_cascade() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch() -> State::Ready\n",
            "  if true\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn unresolved() -> State::Ready\n",
            "  missing\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}

#[test]
fn untyped_final_if_branch_does_not_report_a_variant_mismatch() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch() -> State::Ready\n",
            "  if true\n",
            "    missing\n",
            "  else\n",
            "    Ready\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.unresolved", "{diagnostics:#?}");
}

#[test]
fn final_match_checks_every_typed_arm_even_for_a_refined_scrutinee() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch(value: State::Ready) -> State::Ready\n",
            "  match value\n",
            "    Ready => Ready\n",
            "    Closed => Closed\n",
            "    Failed => Ready\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn untyped_final_match_arm_does_not_report_a_variant_mismatch() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch(value: State) -> State::Ready\n",
            "  match value\n",
            "    Ready => missing\n",
            "    Closed => Ready\n",
            "    Failed => Ready\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.unresolved", "{diagnostics:#?}");
}
