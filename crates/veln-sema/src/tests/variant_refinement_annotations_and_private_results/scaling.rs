use super::*;

#[test]
fn many_independent_base_failures_have_linear_end_to_end_work() {
    let measurements = [128, 256, 512].map(|count| {
        let mut source = String::from("type Empty\nend\npub fn unrelated_before()\n  ()\nend\n");
        for index in 0..count {
            source.push_str(&format!(
                "fn invalid_{index}(value: empty::Missing) -> ()\n  ()\nend\n\
                 pub fn unrelated_after_{index}()\n  ()\nend\n"
            ));
        }
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            count
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.public_signature_missing")
                .count(),
            count + 1
        );
        assert!(
            work.annotation_collection <= count * 16 + 16,
            "count={count} work={work:?}"
        );
        assert!(
            work.casing_collection <= count * 256 + 20_000,
            "count={count} work={work:?}"
        );
        assert!(
            work.enclosing_context <= count * 8 + 16,
            "count={count} work={work:?}"
        );
        assert!(
            work.diagnostic_ordering <= count * 64 + 10_000,
            "count={count} work={work:?}"
        );
        eprintln!("variant refinement base pipeline: annotations={count} work={work:?}");
        work
    });
    for stage in [
        |work: crate::pipeline::VariantRefinementBaseWork| work.annotation_collection,
        |work: crate::pipeline::VariantRefinementBaseWork| work.casing_collection,
        |work: crate::pipeline::VariantRefinementBaseWork| work.enclosing_context,
        |work: crate::pipeline::VariantRefinementBaseWork| work.diagnostic_ordering,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "stage work must have exact affine growth: measurements={measurements:?}"
        );
    }
}

#[test]
fn many_nested_casing_base_failures_have_linear_end_to_end_work() {
    let measurements = [128, 256, 512].map(|count| {
        let mut source = String::from(
            "type Empty<A>\nend\ntype State\n  Ready\nend\npub fn unrelated_before()\n  ()\nend\n",
        );
        for index in 0..count {
            source.push_str(&format!(
                "fn invalid_{index}(value: Empty<main::state>::Missing) -> ()\n  ()\nend\n\
                 pub fn unrelated_after_{index}()\n  ()\nend\n"
            ));
        }
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.invalid_case")
                .count(),
            count
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            count
        );
        let relevant = diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.id.as_str(),
                    "name.invalid_case" | "type.variant_refinement_base"
                )
            })
            .collect::<Vec<_>>();
        assert!(
            relevant
                .iter()
                .take(count)
                .all(|diagnostic| diagnostic.id == "name.invalid_case")
                && relevant
                    .iter()
                    .skip(count)
                    .all(|diagnostic| diagnostic.id == "type.variant_refinement_base"),
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert!(
            work.annotation_collection <= count * 20 + 20,
            "count={count} work={work:?}"
        );
        assert!(
            work.casing_collection <= count * 512 + 20_000,
            "count={count} work={work:?}"
        );
        assert!(
            work.enclosing_context <= count * 12 + 20,
            "count={count} work={work:?}"
        );
        assert!(
            work.diagnostic_ordering <= count * 80 + 10_000,
            "count={count} work={work:?}"
        );
        eprintln!("nested casing refinement pipeline: annotations={count} work={work:?}");
        work
    });
    for stage in [
        |work: crate::pipeline::VariantRefinementBaseWork| work.annotation_collection,
        |work: crate::pipeline::VariantRefinementBaseWork| work.casing_collection,
        |work: crate::pipeline::VariantRefinementBaseWork| work.enclosing_context,
        |work: crate::pipeline::VariantRefinementBaseWork| work.diagnostic_ordering,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "stage work must have exact affine growth: measurements={measurements:?}"
        );
    }
}

#[test]
fn failed_union_alternatives_keep_linear_recovery_and_merge_work() {
    let measurements = [128, 256, 512].map(|count| {
        let mut source = String::from(concat!(
            "type Empty\nend\n",
            "type State\n  Ready\nend\n",
            "type Other\n  Go\nend\n",
            "fn mixed(value: Empty::Missing",
        ));
        for index in 0..count {
            let alternative = if index % 2 == 0 {
                "State::Ready"
            } else {
                "Other::Go"
            };
            source.push_str(" | ");
            source.push_str(alternative);
        }
        source.push_str(") -> ()\n  ()\nend\n");
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            1,
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
                .count(),
            count / 2,
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert_eq!(
            work.recovery_comparison,
            count * 3 + 1,
            "count={count} work={work:?}"
        );
        eprintln!("variant refinement union recovery: alternatives={count} work={work:?}");
        work
    });

    for stage in [
        |work: crate::pipeline::VariantRefinementBaseWork| work.recovery_comparison,
        |work: crate::pipeline::VariantRefinementBaseWork| work.diagnostic_ordering,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "recovery and merge work must have exact affine growth: {measurements:?}"
        );
    }
}

#[test]
fn structural_sibling_recovery_work_is_linear() {
    let measurements = [64, 128, 256].map(|count| {
        let mut source = String::from(concat!(
            "type Empty\nend\n",
            "fn mixed(value: {failed: Empty::Missing",
        ));
        for index in 0..count {
            source.push_str(&format!(", item{index}: Vec<Int, Int>"));
        }
        source.push_str("}) -> ()\n  ()\nend\n");
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            1,
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
                .count(),
            count,
            "count={count} diagnostics={diagnostics:#?}"
        );
        eprintln!("structural sibling recovery: fields={count} work={work:?}");
        work
    });

    for stage in [
        |work: crate::pipeline::VariantRefinementBaseWork| work.recovery_comparison,
        |work: crate::pipeline::VariantRefinementBaseWork| work.diagnostic_ordering,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "structural recovery and merge work must have exact affine growth: {measurements:?}"
        );
    }
}

#[test]
fn declaration_quarantine_work_is_linear_across_consumers() {
    let measurements = [16, 32, 64].map(|count| {
        let mut source = String::from("type Empty\nend\n");
        for index in 0..count {
            source.push_str(&format!(
                "type Carrier{index}\n  Built{index}(invalid: Empty::Missing, sibling: Int)\nend\n\
                 pub type CarrierAlias{index} = Carrier{index}\n\
                 pub type CarrierTransitive{index} = CarrierAlias{index}\n\
                 fn construct_alias_{index}() -> Carrier{index}\n\
                   CarrierTransitive{index}::Built{index}(1, 2)\n\
                 end\n\
                 fn ordinary_{index}(invalid: Empty::Missing, sibling: String) -> Int\n\
                   sibling\n\
                 end\n\
                 effect Effect{index}\n  apply(invalid: Empty::Missing, sibling: Int) -> Int\nend\n\
                 handler handler_{index}(invalid: Empty::Missing, sibling: Int) for Effect{index}\n\
                   apply(invalid, sibling) => sibling\nend\n"
            ));
        }
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            count * 4,
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.mismatch"),
            "count={count} diagnostics={diagnostics:#?}"
        );
        eprintln!("variant refinement declaration quarantine: declarations={count} work={work:?}");
        work.diagnostic_ordering
    });

    let first_growth = measurements[1] - measurements[0];
    let second_growth = measurements[2] - measurements[1];
    assert_eq!(
        second_growth,
        first_growth * 2,
        "quarantine work must have exact affine growth: measurements={measurements:?}"
    );
}

#[test]
fn public_visibility_work_is_linear_for_wide_unions() {
    let work = [16, 32, 64].map(|count| {
        let mut source = String::from("pub type State\n");
        for index in 0..count {
            source.push_str(&format!("  Hidden{index}\n"));
        }
        source.push_str("end\npub fn expose(value: ");
        for index in 0..count {
            if index > 0 {
                source.push_str(" | ");
            }
            source.push_str(&format!("State::Hidden{index}"));
        }
        source.push_str(") -> ()\n  ()\nend\n");

        crate::analysis::reset_visibility_work();
        crate::adt::registry::descriptor_lookup_counters::reset();
        let diagnostics = diagnostics_for(&source);
        let visibility_work = crate::analysis::take_visibility_work();
        let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
        let work = visibility_work + descriptor_work;
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
                .count(),
            count,
            "count={count} diagnostics={diagnostics:#?}"
        );
        eprintln!(
            "variant refinement visibility union: alternatives={count} \
             visibility_work={visibility_work} descriptor_work={descriptor_work} work={work}"
        );
        work
    });

    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert_eq!(second_growth, first_growth * 2, "work={work:?}");
}

#[test]
fn public_visibility_work_is_linear_for_many_declarations() {
    let work = [16, 32, 64].map(|count| {
        let mut source = String::new();
        for index in 0..count {
            source.push_str(&format!(
                "pub type State{index}\n  Hidden\nend\n\
                 pub fn expose_{index}(value: State{index}::Hidden) -> ()\n  ()\nend\n"
            ));
        }

        crate::analysis::reset_visibility_work();
        crate::adt::registry::descriptor_lookup_counters::reset();
        let diagnostics = diagnostics_for(&source);
        let visibility_work = crate::analysis::take_visibility_work();
        let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
        let work = visibility_work + descriptor_work;
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
                .count(),
            count,
            "count={count} diagnostics={diagnostics:#?}"
        );
        eprintln!(
            "variant refinement visibility declarations: declarations={count} \
             visibility_work={visibility_work} descriptor_work={descriptor_work} work={work}"
        );
        work
    });

    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert_eq!(second_growth, first_growth * 2, "work={work:?}");
}

fn same_leaf_qualified_import_project(count: usize) -> veln_ast::SurfaceModule {
    let mut modules = Vec::new();
    let mut consumer_source = String::new();
    for index in 0..count {
        let module_name = format!("dependency{index}");
        modules.push((
            module_name.clone(),
            SourceFile::new(
                format!("{module_name}.veln"),
                "pub type State\n  Hidden\nend\n",
            ),
        ));
        consumer_source.push_str(&format!("use {module_name}\n"));
    }
    for index in 0..count {
        consumer_source.push_str(&format!(
            "pub fn expose_{index}(value: dependency{index}::State::Hidden) -> ()\n  ()\nend\n"
        ));
    }
    let consumer = SourceFile::new("consumer.veln", consumer_source);
    let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&consumer).tree,
        "consumer".to_string(),
        consumer.span(veln_source::TextRange::new(0, 0)),
    );
    modules.push(("consumer".to_string(), consumer));
    let sources = modules
        .iter()
        .map(|(name, source)| (name.as_str(), source.clone()))
        .collect();
    let mut project = merged_modules_with_identities(sources);
    project.module = consumer_identity.module;
    project
}

fn same_leaf_qualified_import_work(count: usize) -> usize {
    let project = same_leaf_qualified_import_project(count);
    crate::analysis::reset_visibility_work();
    crate::adt::registry::descriptor_lookup_counters::reset();
    crate::adt::reset_import_path_comparisons();
    let diagnostics = analyze_surface_module(&project);
    let visibility_work = crate::analysis::take_visibility_work();
    let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
    let import_path_work = crate::adt::import_path_comparisons();
    let work = visibility_work + descriptor_work + import_path_work;
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count(),
        count,
        "count={count} diagnostics={diagnostics:#?}"
    );
    assert!(import_path_work > 0, "the metric must observe import paths");
    eprintln!(
        "qualified same-leaf refinement visibility: declarations={count} \
         visibility_work={visibility_work} descriptor_work={descriptor_work} \
         import_path_work={import_path_work} work={work}"
    );
    work
}

#[test]
fn public_visibility_work_is_linear_for_same_leaf_qualified_imports() {
    let work = [16, 32, 64].map(same_leaf_qualified_import_work);

    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert_eq!(second_growth, first_growth * 2, "work={work:?}");
}

#[test]
fn nested_refinement_canonicalization_handles_increasing_depths() {
    fn canonicalization_work(depth: usize) -> usize {
        let mut annotation = "Int".to_string();
        for _ in 0..depth {
            annotation = format!("Choice<{annotation}>::Present");
        }
        let source = format!(
            "type Choice<A>\n  Present(A)\n  Absent\nend\nfn keep(value: {annotation}) -> {annotation}\n  value\nend\n"
        );
        crate::types::reset_type_canonicalization_visits();
        let diagnostics = diagnostics_for(&source);
        assert!(diagnostics.is_empty(), "depth {depth}: {diagnostics:#?}");
        crate::types::take_type_canonicalization_visits()
    }

    let work = [16, 32, 64].map(canonicalization_work);
    eprintln!("type canonicalization visits at depths 16, 32, and 64: {work:?}");
    assert!(work[0] > 0, "the metric must observe canonicalization work");
    assert!(
        work[1] <= work[0] * 2 + 32 && work[2] <= work[1] * 2 + 32,
        "doubling refinement depth must add only linear canonicalization work: {work:?}"
    );
}

#[test]
fn unresolved_nested_annotations_do_not_repeat_canonicalization() {
    fn canonicalization_work(depth: usize) -> usize {
        let mut annotation = "Int".to_string();
        for _ in 0..depth {
            annotation = format!("missing::Outer<{annotation}>");
        }
        let source = format!("fn keep(value: {annotation}) -> ()\n  ()\nend\n");
        crate::types::reset_type_canonicalization_visits();
        let diagnostics = diagnostics_for(&source);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.variant_mismatch"),
            "depth {depth}: {diagnostics:#?}"
        );
        crate::types::take_type_canonicalization_visits()
    }

    let work = [16, 32, 64].map(canonicalization_work);
    eprintln!("invalid annotation canonicalization visits at depths 16, 32, and 64: {work:?}");
    assert!(work[0] > 0, "the metric must observe canonicalization work");
    assert!(
        work[0] == work[1] && work[1] == work[2],
        "an invalid outer annotation must stop validation before child recanonicalization: {work:?}"
    );
}
