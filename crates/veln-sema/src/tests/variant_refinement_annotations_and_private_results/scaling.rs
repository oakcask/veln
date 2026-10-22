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
                .filter(|diagnostic| diagnostic.id == "type.variant_union_base")
                .count(),
            count / 2,
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert_eq!(
            work.recovery_comparison,
            count * 5 + 1,
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
fn nested_mixed_base_recovery_work_grows_linearly_with_depth() {
    let measurements = [8, 16, 24].map(|depth| {
        let mut annotation = "Left::A | Right::B".to_string();
        for _ in 0..depth {
            annotation = format!("Box<{annotation}>::Present | Right::B");
        }
        let source = format!(
            "type Left\n  A\nend\n\
             type Right\n  B\nend\n\
             type Box<A>\n  Present(A)\nend\n\
             fn nested(value: {annotation}) -> ()\n  ()\nend\n"
        );

        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_union_base")
                .count(),
            1,
            "depth={depth} diagnostics={diagnostics:#?}"
        );
        eprintln!("nested mixed-base recovery: depth={depth} work={work:?}");
        (
            work.recovery_comparison,
            work.recovery_materialization,
            work.recovery_materialized_type_nodes,
            work.recovery_type_nodes_cloned,
        )
    });

    // Clone nodes are measured by Type::clone itself. This makes the stage
    // sensitive to copies of accumulated child trees, not only copies that a
    // recovery caller remembers to record.
    for stage in [
        |work: (usize, usize, usize, usize)| work.0,
        |work: (usize, usize, usize, usize)| work.1,
        |work: (usize, usize, usize, usize)| work.2,
        |work: (usize, usize, usize, usize)| work.3,
    ] {
        assert_eq!(
            stage(measurements[1]) - stage(measurements[0]),
            stage(measurements[2]) - stage(measurements[1]),
            "nested recovery work must have constant adjacent growth: {measurements:?}"
        );
    }
}

#[test]
fn valid_nested_recovery_materialization_grows_linearly_with_depth() {
    let measurements = [8, 16, 24].map(|depth| {
        let mut alternative = "State::Ready".to_string();
        for _ in 0..depth {
            alternative = format!("Box<{alternative}>::Present");
        }
        let source = format!(
            "type State\n  Ready\nend\n\
             type Other\n  B\nend\n\
             type Box<A>\n  Present(A)\nend\n\
             fn nested(value: {alternative} | Other::B) -> ()\n  ()\nend\n"
        );

        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_union_base")
                .count(),
            1,
            "depth={depth} diagnostics={diagnostics:#?}"
        );
        eprintln!("valid nested union recovery: depth={depth} work={work:?}");
        (
            work.recovery_comparison,
            work.recovery_materialization,
            work.recovery_materialized_type_nodes,
            work.recovery_type_nodes_cloned,
        )
    });

    // The valid path accumulates one recovered child at each level. Constant
    // adjacent clone growth rejects copying that growing tree at every level.
    for stage in [
        |work: (usize, usize, usize, usize)| work.0,
        |work: (usize, usize, usize, usize)| work.1,
        |work: (usize, usize, usize, usize)| work.2,
        |work: (usize, usize, usize, usize)| work.3,
    ] {
        assert_eq!(
            stage(measurements[1]) - stage(measurements[0]),
            stage(measurements[2]) - stage(measurements[1]),
            "valid nested recovery work must have constant adjacent growth: {measurements:?}"
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
fn reordered_wide_record_union_base_comparison_is_linear() {
    let measurements = [64, 128, 256].map(|count| {
        let fields = (0..count)
            .map(|index| format!("field{index}: Int"))
            .collect::<Vec<_>>();
        let reversed_fields = fields.iter().rev().cloned().collect::<Vec<_>>();
        let source = format!(
            "type Box<A>\n  Present(value: A)\n  Absent(value: A)\nend\n\
             fn valid(value: Box<{{{}}}>::Present | Box<{{{}}}>::Absent) -> ()\n  ()\nend\n",
            fields.join(", "),
            reversed_fields.join(", ")
        );

        crate::type_relations::reset_record_field_match_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::type_relations::take_record_field_match_work();
        assert!(
            diagnostics.is_empty(),
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert!(work >= count * 4, "count={count} work={work}");
        eprintln!("wide record union comparison: fields={count} work={work}");
        work
    });

    let first_growth = measurements[1] - measurements[0];
    let second_growth = measurements[2] - measurements[1];
    assert_eq!(
        second_growth,
        first_growth * 2,
        "record identity comparison must have exact affine growth: {measurements:?}"
    );
}

#[test]
fn reordered_conflicting_record_union_base_rendering_is_linear() {
    let measurements = [64, 128, 256].map(|count| {
        let expected_fields = (0..count)
            .map(|index| format!("field{index}: Int"))
            .collect::<Vec<_>>();
        let actual_fields = (0..count)
            .rev()
            .map(|index| {
                let ty = if index == 0 { "String" } else { "Int" };
                format!("field{index}: {ty}")
            })
            .collect::<Vec<_>>();
        let source = format!(
            "type Box<A>\n  Present(value: A)\n  Absent(value: A)\nend\n\
             fn invalid(value: Box<{{{}}}>::Present | Box<{{{}}}>::Absent) -> ()\n  ()\nend\n",
            expected_fields.join(", "),
            actual_fields.join(", ")
        );

        crate::analysis::reset_diagnostic_record_render_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::analysis::take_diagnostic_record_render_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_union_base")
                .count(),
            1,
            "count={count} diagnostics={diagnostics:#?}"
        );
        assert!(
            work.index_insertions >= count
                && work.pairing_lookups >= count * 2
                && work.result_reordering_lookups >= count,
            "count={count} work={work:?}"
        );
        eprintln!("wide record union diagnostic rendering: fields={count} work={work:?}");
        work
    });

    for stage in [
        |work: crate::analysis::DiagnosticRecordRenderWork| work.index_insertions,
        |work: crate::analysis::DiagnosticRecordRenderWork| work.pairing_lookups,
        |work: crate::analysis::DiagnosticRecordRenderWork| work.result_reordering_lookups,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "diagnostic record rendering must have exact affine growth: {measurements:?}"
        );
    }

    let diagnostics = diagnostics_for(concat!(
        "type Box<A>\n  Present(value: A)\n  Absent(value: A)\nend\n",
        "fn invalid(value: Box<{first: Int, second: Int, third: Int}>::Present | ",
        "Box<{third: Int, second: Int, first: String}>::Absent) -> ()\n",
        "  ()\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_union_base");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"expected_base_type\":\"Box<{first: Int, second: Int, third: Int}>\",",
            "\"actual_base_type\":\"Box<{third: Int, second: Int, first: String}>\"}"
        )
    );
}

#[test]
fn record_field_identity_matching_preserves_first_duplicate() {
    let expected = vec![
        ("field".to_string(), Type::int()),
        ("field".to_string(), Type::int()),
    ];
    let actual = vec![
        ("field".to_string(), Type::int()),
        ("field".to_string(), Type::string()),
    ];

    assert!(crate::type_relations::record_fields_match_by_name(
        &expected,
        &actual,
        |expected, actual| expected == actual,
    ));
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
fn dependent_public_adt_quarantine_work_grows_linearly() {
    let work = [16, 32, 64].map(|count| {
        let mut source = String::from("pub type State\n  Hidden\nend\n");
        for index in 0..count {
            let dependency = if index == 0 {
                "State".to_string()
            } else {
                format!("Link{}", index - 1)
            };
            source.push_str(&format!(
                "pub type Link{index}\n  pub Wrapped(value: {dependency}::Wrapped)\nend\n"
            ));
        }
        source = source.replacen("State::Wrapped", "State::Hidden", 1);

        crate::analysis::reset_visibility_work();
        crate::adt::registry::descriptor_lookup_counters::reset();
        crate::adt::registry::variant_lookup_counters::reset();
        let diagnostics = diagnostics_for(&source);
        let visibility_work = crate::analysis::take_visibility_work();
        let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
        let variant_work = crate::adt::registry::variant_lookup_counters::index_lookups();
        let work = visibility_work + descriptor_work + variant_work;
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
                .count(),
            count,
            "count={count} diagnostics={diagnostics:#?}"
        );
        eprintln!(
            "dependent public ADT quarantine: declarations={count} \
             visibility_work={visibility_work} descriptor_work={descriptor_work} \
             variant_work={variant_work} work={work}"
        );
        work
    });

    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert_eq!(second_growth, first_growth * 2, "work={work:?}");
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
        crate::adt::registry::variant_lookup_counters::reset();
        let diagnostics = diagnostics_for(&source);
        let visibility_work = crate::analysis::take_visibility_work();
        let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
        let variant_work = crate::adt::registry::variant_lookup_counters::index_lookups();
        let work = visibility_work + descriptor_work + variant_work;
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
             visibility_work={visibility_work} descriptor_work={descriptor_work} \
             variant_work={variant_work} work={work}"
        );
        work
    });

    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert_eq!(second_growth, first_growth * 2, "work={work:?}");
}

fn wide_structural_visibility_work(count: usize, boundary: &str) -> usize {
    let mut fields = String::new();
    for index in 0..count {
        if index > 0 {
            fields.push_str(", ");
        }
        fields.push_str(&format!("field{index}: State::Hidden"));
    }
    let signature = match boundary {
        "parameter" => format!("pub fn expose(value: {{{fields}}}) -> ()\n  ()\nend\n"),
        "result" => format!("pub fn expose() -> {{{fields}}}\n  panic(\"unused\")\nend\n"),
        _ => unreachable!("known test boundary"),
    };
    let source = SourceFile::new(
        "main.veln",
        format!("pub type State\n  Hidden\nend\n{signature}"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    crate::analysis::reset_visibility_work();
    let environment = TypeEnvironment::from_module(&module);
    assert!(
        !environment.function_is_published_for_test("expose"),
        "boundary={boundary} count={count}"
    );
    let diagnostics = analyze_surface_module(&module);
    let candidate_work = crate::analysis::take_visibility_candidate_work();
    let visibility_work = crate::analysis::take_visibility_work();
    assert!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count()
            == count,
        "boundary={boundary} count={count} diagnostics={diagnostics:#?}"
    );
    assert!(
        candidate_work.root_classification > 0,
        "an empty exposure result must still account for root classification: {candidate_work:?}"
    );
    assert!(
        candidate_work.candidate_construction >= count
            && candidate_work.candidate_construction.is_multiple_of(count),
        "each visibility pass must construct one candidate per structural field: {candidate_work:?}"
    );
    assert!(
        candidate_work.occurrence_lookup == 0,
        "structural roots must be rejected before occurrence lookup: {candidate_work:?}"
    );
    eprintln!(
        "wide structural refinement visibility: boundary={boundary} fields={count} \
         candidate_work={candidate_work:?} visibility_work={visibility_work}"
    );
    candidate_work.root_classification
        + candidate_work.structural_parses
        + candidate_work.candidate_construction
        + candidate_work.occurrence_lookup
        + candidate_work.record_path_nodes
        + candidate_work.structural_path_nodes
        + candidate_work.materialized_structural_path_segments
        + visibility_work
}

#[test]
fn nested_record_visibility_keeps_linear_shared_path_nodes() {
    let measurements = [64, 65, 66].map(|depth| {
        let mut annotation = String::with_capacity(depth * 37 + 3);
        for _ in 0..depth {
            annotation.push_str("{state: State::Ready, next: ");
        }
        annotation.push_str("Int");
        for _ in 0..depth {
            annotation.push('}');
        }
        let source = SourceFile::new(
            "main.veln",
            format!(
                "pub type State\n  pub Ready\nend\npub fn expose(value: {annotation}) -> ()\n  ()\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::analysis::reset_visibility_work();
        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
            "{diagnostics:#?}"
        );
        let work = crate::analysis::take_visibility_candidate_work();
        assert!(work.record_path_nodes > 0, "{work:?}");
        work.record_path_nodes
    });
    assert_eq!(
        measurements[1] - measurements[0],
        measurements[2] - measurements[1],
        "semantic record path nodes must have constant adjacent growth: {measurements:?}"
    );
}

#[test]
fn public_parameter_structural_visibility_work_has_bounded_growth() {
    let work = [256, 512, 1024].map(|count| wide_structural_visibility_work(count, "parameter"));
    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert!(second_growth <= first_growth * 3, "work={work:?}");
}

#[test]
fn public_result_structural_visibility_work_has_bounded_growth() {
    let work = [256, 512, 1024].map(|count| wide_structural_visibility_work(count, "result"));
    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert!(second_growth <= first_growth * 3, "work={work:?}");
}

fn wide_named_argument_visibility_work(count: usize, public: bool, variant: &str) -> usize {
    let fields = (0..count)
        .map(|index| format!("field{index}: Box<State::{variant}>"))
        .collect::<Vec<_>>()
        .join(", ");
    let visibility = if public { "pub " } else { "" };
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub type State\n  Hidden\n  pub Ready\nend\n\
             pub type Box<A>\n  pub Boxed(A)\nend\n\
             {visibility}fn expose(value: {{{fields}}}) -> ()\n  ()\nend\n"
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    crate::analysis::reset_visibility_work();
    let started = std::time::Instant::now();
    let diagnostics = analyze_surface_module(&module);
    let elapsed = started.elapsed();
    let candidate_work = crate::analysis::take_visibility_candidate_work();
    let visibility_work = crate::analysis::take_visibility_work();
    let expected = usize::from(public && variant == "Hidden") * count;
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count(),
        expected,
        "public={public} variant={variant} count={count}"
    );
    eprintln!(
        "nested named visibility: fields={count} public={public} variant={variant} elapsed={elapsed:?}"
    );
    candidate_work.root_classification
        + candidate_work.structural_parses
        + candidate_work.candidate_construction
        + candidate_work.occurrence_lookup
        + candidate_work.record_path_nodes
        + candidate_work.structural_path_nodes
        + candidate_work.materialized_structural_path_segments
        + visibility_work
}

#[test]
fn nested_named_argument_visibility_indexing_has_affine_wide_growth() {
    for (public, variant) in [(true, "Hidden"), (true, "Ready"), (false, "Hidden")] {
        let work = [500, 750, 1_000]
            .map(|count| wide_named_argument_visibility_work(count, public, variant));
        assert_eq!(
            work[1] - work[0],
            work[2] - work[1],
            "public={public} variant={variant} work={work:?}"
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct NamedDepthVisibilityMeasurement {
    work: usize,
    peak_path_depth: usize,
    structural_parses: usize,
    structural_path_nodes: usize,
    materialized_path_segments: usize,
    private_diagnostics: usize,
}

fn nested_named_annotation(depth: usize, leaf: &str) -> String {
    let mut annotation = String::with_capacity(depth * 5 + leaf.len());
    for _ in 0..depth {
        annotation.push_str("Box<");
    }
    annotation.push_str(leaf);
    for _ in 0..depth {
        annotation.push('>');
    }
    annotation
}

fn named_depth_visibility_measurement(
    depth: usize,
    leaf: &str,
    public: bool,
) -> NamedDepthVisibilityMeasurement {
    let visibility = if public { "pub " } else { "" };
    let annotation = nested_named_annotation(depth, leaf);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub type State\n  Hidden\n  pub Ready\nend\n\
             pub type Box<A>\n  pub Boxed(A)\nend\n\
             {visibility}fn expose(value: {annotation}) -> ()\n  ()\nend\n"
        ),
    );
    let parse_started = std::time::Instant::now();
    let parsed = parse(&source);
    let parse_elapsed = parse_started.elapsed();
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lower_started = std::time::Instant::now();
    let module = lower_surface_ast(&parsed.tree);
    let lower_elapsed = lower_started.elapsed();

    crate::analysis::reset_visibility_work();
    let analyze_started = std::time::Instant::now();
    let diagnostics = analyze_surface_module(&module);
    let analyze_elapsed = analyze_started.elapsed();
    let candidate_work = crate::analysis::take_visibility_candidate_work();
    let visibility_work = crate::analysis::take_visibility_work();
    let private_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .count();
    let work = candidate_work.root_classification
        + candidate_work.structural_parses
        + candidate_work.candidate_construction
        + candidate_work.occurrence_lookup
        + candidate_work.record_path_nodes
        + candidate_work.structural_path_nodes
        + candidate_work.materialized_structural_path_segments
        + visibility_work;
    eprintln!(
        "named visibility depth: depth={depth} leaf={leaf} public={public} \
         parse={parse_elapsed:?} lower={lower_elapsed:?} analyze={analyze_elapsed:?} \
         candidate_work={candidate_work:?} visibility_work={visibility_work} work={work}"
    );
    NamedDepthVisibilityMeasurement {
        work,
        peak_path_depth: candidate_work.peak_structural_path_depth,
        structural_parses: candidate_work.structural_parses,
        structural_path_nodes: candidate_work.structural_path_nodes,
        materialized_path_segments: candidate_work.materialized_structural_path_segments,
        private_diagnostics,
    }
}

#[test]
fn named_visibility_depth_work_is_linear_with_fixed_candidates() {
    for leaf in ["State::Ready", "State::Hidden"] {
        let measurements =
            [64, 128, 256].map(|depth| named_depth_visibility_measurement(depth, leaf, true));
        for (measurement, depth) in measurements.iter().zip([64, 128, 256]) {
            assert_eq!(
                measurement.private_diagnostics,
                usize::from(leaf == "State::Hidden"),
                "leaf={leaf} measurements={measurements:?}"
            );
            assert!(
                measurement.peak_path_depth >= depth,
                "leaf={leaf} measurements={measurements:?}"
            );
            assert!(
                measurement.structural_path_nodes >= depth,
                "leaf={leaf} measurements={measurements:?}"
            );
            if leaf == "State::Hidden" {
                assert!(
                    measurement.materialized_path_segments >= depth
                        && measurement.materialized_path_segments.is_multiple_of(depth),
                    "leaf={leaf} measurements={measurements:?}"
                );
            } else {
                assert_eq!(
                    measurement.materialized_path_segments, 0,
                    "leaf={leaf} measurements={measurements:?}"
                );
            }
        }
        assert!(
            measurements[1].work <= measurements[0].work * 2 + 32
                && measurements[2].work <= measurements[1].work * 2 + 32,
            "leaf={leaf} measurements={measurements:?}"
        );
    }

    let private = named_depth_visibility_measurement(256, "State::Hidden", false);
    assert_eq!(private.private_diagnostics, 0, "{private:?}");
    assert_eq!(private.structural_parses, 0, "{private:?}");
    assert_eq!(private.structural_path_nodes, 0, "{private:?}");
    assert_eq!(private.materialized_path_segments, 0, "{private:?}");
    assert_eq!(private.peak_path_depth, 0, "{private:?}");
}

#[derive(Clone, Copy, Debug)]
struct CombinedDepthWidthVisibilityMeasurement {
    outer_span_selection: usize,
    materialized_path_segments: usize,
    private_diagnostics: usize,
}

fn combined_depth_width_visibility_measurement(
    depth: usize,
    width: usize,
    variant: &str,
) -> CombinedDepthWidthVisibilityMeasurement {
    let fields = (0..width)
        .map(|index| format!("field{index}: State::{variant}"))
        .collect::<Vec<_>>()
        .join(", ");
    let annotation = nested_named_annotation(depth, &format!("{{{fields}}}"));
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub type State\n  Hidden\n  pub Ready\nend\n\
             pub type Box<A>\n  pub Boxed(A)\nend\n\
             pub fn expose(value: {annotation}) -> ()\n  ()\nend\n"
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    crate::analysis::reset_visibility_work();
    let diagnostics = analyze_surface_module(&module);
    let work = crate::analysis::take_visibility_candidate_work();
    let private_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .count();
    let measurement = CombinedDepthWidthVisibilityMeasurement {
        outer_span_selection: work.outer_span_selection,
        materialized_path_segments: work.materialized_structural_path_segments,
        private_diagnostics,
    };
    eprintln!(
        "combined refinement visibility: depth={depth} width={width} variant={variant} \
         measurement={measurement:?}"
    );
    measurement
}

#[test]
fn outer_structural_span_selection_is_linear_when_depth_and_width_grow_together() {
    for variant in ["Ready", "Hidden"] {
        let measurements = [16, 32, 64]
            .map(|size| combined_depth_width_visibility_measurement(size, size, variant));
        assert_eq!(
            measurements.map(|measurement| measurement.private_diagnostics),
            if variant == "Hidden" {
                [16, 32, 64]
            } else {
                [0, 0, 0]
            },
            "variant={variant} measurements={measurements:?}"
        );
        for (measurement, size) in measurements.iter().zip([16, 32, 64]) {
            assert!(
                measurement.outer_span_selection <= 20 * (size + size) + 64,
                "outer-span selection must stay linear in depth plus width: \
                 variant={variant} measurements={measurements:?}"
            );
        }
        if variant == "Hidden" {
            for (measurement, size) in measurements.iter().zip([16, 32, 64]) {
                assert!(
                    measurement.materialized_path_segments >= size * size,
                    "required diagnostic paths must remain complete: \
                     variant={variant} measurements={measurements:?}"
                );
            }
        } else {
            assert_eq!(
                measurements.map(|measurement| measurement.materialized_path_segments),
                [0, 0, 0],
                "variant={variant} measurements={measurements:?}"
            );
        }
    }
}

#[test]
fn refinement_free_named_depth_bypasses_structural_visibility_walk() {
    let measurements =
        [128, 256, 512].map(|depth| named_depth_visibility_measurement(depth, "Int", true));
    for measurement in measurements {
        assert_eq!(measurement.private_diagnostics, 0, "{measurement:?}");
        assert_eq!(measurement.structural_parses, 0, "{measurement:?}");
        assert_eq!(measurement.peak_path_depth, 0, "{measurement:?}");
    }
    assert!(
        measurements[1].work <= measurements[0].work * 2 + 32
            && measurements[2].work <= measurements[1].work * 2 + 32,
        "measurements={measurements:?}"
    );
}

#[test]
fn maximum_supported_refinement_depth_completes_visibility_analysis() {
    let depth = veln_syntax::MAX_VARIANT_REFINEMENT_NESTING;
    let mut annotation = "State::Ready".to_string();
    for _ in 0..depth {
        annotation = format!("Layer<{annotation}>::Wrapped");
    }
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub type State\n  pub Ready\nend\n\
             pub type Layer<A>\n  pub Wrapped(A)\nend\n\
             pub fn expose(value: {annotation}) -> ()\n  ()\nend\n"
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::analysis::reset_visibility_work();
    let diagnostics = analyze_surface_module(&module);
    let candidate_work = crate::analysis::take_visibility_candidate_work();
    let visibility_work = crate::analysis::take_visibility_work();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(
        candidate_work.peak_structural_path_depth >= depth,
        "depth={depth} work={candidate_work:?}"
    );
    eprintln!(
        "maximum refinement visibility depth: depth={depth} \
         candidate_work={candidate_work:?} visibility_work={visibility_work}"
    );
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
        crate::adt::registry::variant_lookup_counters::reset();
        let diagnostics = diagnostics_for(&source);
        let visibility_work = crate::analysis::take_visibility_work();
        let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
        let variant_work = crate::adt::registry::variant_lookup_counters::index_lookups();
        let work = visibility_work + descriptor_work + variant_work;
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
             visibility_work={visibility_work} descriptor_work={descriptor_work} \
             variant_work={variant_work} work={work}"
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
    crate::adt::registry::variant_lookup_counters::reset();
    crate::adt::reset_import_path_comparisons();
    let diagnostics = analyze_surface_module(&project);
    let visibility_work = crate::analysis::take_visibility_work();
    let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
    let variant_work = crate::adt::registry::variant_lookup_counters::index_lookups();
    let import_path_work = crate::adt::import_path_comparisons();
    let work = visibility_work + descriptor_work + variant_work + import_path_work;
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
         variant_work={variant_work} import_path_work={import_path_work} work={work}"
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

#[derive(Clone, Copy, Debug)]
struct QualifiedPathDepthWork {
    publication: usize,
    diagnostics: usize,
    checked_lowering: usize,
}

fn recovered_qualified_path_depth_work(depth: usize) -> QualifiedPathDepthWork {
    let module = recovered_qualified_path_module(depth);
    let work = QualifiedPathDepthWork {
        publication: recovered_qualified_publication_work(&module, depth),
        diagnostics: recovered_qualified_diagnostic_work(&module, depth),
        checked_lowering: recovered_qualified_lowering_work(&module, depth),
    };
    eprintln!("recovered qualified refinement: depth={depth} work={work:?}");
    work
}

fn recovered_qualified_path_module(depth: usize) -> veln_ast::SurfaceModule {
    let written_segments = std::iter::repeat_n("Q", depth).collect::<Vec<_>>();
    let mut module_segments = written_segments.clone();
    module_segments[depth / 2] = "q";
    let qualifiers = written_segments.join("::");
    let module_name = module_segments.join("::");
    let source = SourceFile::new(
        "consumer.veln",
        format!(
            "pub type State\n  Hidden\nend\n\
             pub fn expose(value: {qualifiers}::State::Hidden) -> ()\n  ()\nend\n"
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    veln_ast::lower_surface_ast_with_module_identity(
        &parsed.tree,
        module_name,
        source.span(veln_source::TextRange::new(0, 0)),
    )
}

fn recovered_qualified_publication_work(module: &veln_ast::SurfaceModule, depth: usize) -> usize {
    crate::adt::reset_qualified_path_work();
    let environment = TypeEnvironment::from_module(module);
    assert!(
        !environment.function_is_published_for_test("expose"),
        "depth={depth}"
    );
    crate::adt::qualified_path_work()
}

fn recovered_qualified_diagnostic_work(module: &veln_ast::SurfaceModule, depth: usize) -> usize {
    crate::adt::reset_qualified_path_work();
    let diagnostics = analyze_surface_module(module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.variant_refinement_private"),
        "depth={depth} diagnostics={diagnostics:#?}"
    );
    crate::adt::qualified_path_work()
}

fn recovered_qualified_lowering_work(module: &veln_ast::SurfaceModule, depth: usize) -> usize {
    crate::adt::reset_qualified_path_work();
    let lowered = lower_checked_surface_module(module);
    assert!(lowered.core.is_none(), "depth={depth}");
    assert!(lowered.ir.is_none(), "depth={depth}");
    crate::adt::qualified_path_work()
}

#[test]
fn qualified_path_recovery_work_is_linear_across_visibility_consumers() {
    let measurements = [64, 128, 256].map(recovered_qualified_path_depth_work);
    for stage in [
        |work: QualifiedPathDepthWork| work.publication,
        |work: QualifiedPathDepthWork| work.diagnostics,
        |work: QualifiedPathDepthWork| work.checked_lowering,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "qualified path work must have exact affine growth: {measurements:?}"
        );
    }
}

fn same_leaf_unqualified_import_project(count: usize) -> veln_ast::SurfaceModule {
    let mut sources = same_leaf_unqualified_import_sources(count);
    let consumer = same_leaf_unqualified_import_consumer(count);
    let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&consumer).tree,
        "consumer".to_string(),
        consumer.span(veln_source::TextRange::new(0, 0)),
    );
    sources.push(("consumer".to_string(), consumer));
    let source_refs = sources
        .iter()
        .map(|(name, source)| (name.as_str(), source.clone()))
        .collect();
    let mut project = merged_modules_with_identities(source_refs);
    project.module = consumer_identity.module;
    project
}

fn same_leaf_unqualified_import_sources(count: usize) -> Vec<(String, SourceFile)> {
    let mut sources = Vec::new();
    for index in 0..count {
        let dependency = format!("dependency{index}");
        sources.push((
            dependency.clone(),
            SourceFile::new(
                format!("{dependency}.veln"),
                "pub type State<A>\n  Hidden\nend\n",
            ),
        ));
        let unrelated = format!("unrelated{index}");
        sources.push((
            unrelated.clone(),
            SourceFile::new(
                format!("{unrelated}.veln"),
                format!("mod {unrelated}\nuse {dependency}\n"),
            ),
        ));
    }
    sources
}

fn same_leaf_unqualified_import_consumer(count: usize) -> SourceFile {
    let mut consumer_source = String::from("mod consumer\nuse dependency0\n");
    for index in 0..count {
        consumer_source.push_str(&format!(
            "pub fn exact_{index}(value: State<Int>::Hidden) -> ()\n  ()\nend\n\
             pub fn any_{index}(value: State::Hidden) -> ()\n  ()\nend\n"
        ));
    }
    SourceFile::new("consumer.veln", consumer_source)
}

fn same_leaf_unqualified_import_work(count: usize) -> usize {
    let project = same_leaf_unqualified_import_project(count);
    crate::analysis::reset_visibility_work();
    crate::adt::registry::descriptor_lookup_counters::reset();
    crate::adt::registry::variant_lookup_counters::reset();
    crate::adt::reset_import_path_comparisons();
    let environment = TypeEnvironment::from_module(&project);
    crate::analysis::reset_visibility_work();
    crate::adt::registry::descriptor_lookup_counters::reset();
    crate::adt::registry::variant_lookup_counters::reset();
    let diagnostics = project
        .functions
        .iter()
        .flat_map(|function| {
            crate::analysis::check_public_variant_refinement_visibility(function, &environment)
        })
        .collect::<Vec<_>>();
    let visibility_work = crate::analysis::take_visibility_work();
    let descriptor_work = crate::adt::registry::descriptor_lookup_counters::candidate_scans();
    let variant_work = crate::adt::registry::variant_lookup_counters::index_lookups();
    let import_work = crate::adt::import_path_comparisons();
    let work = visibility_work + descriptor_work + variant_work + import_work;
    assert_unqualified_import_diagnostics(&diagnostics, count, import_work);
    eprintln!(
        "unqualified same-leaf refinement visibility: width={count} \
         visibility_work={visibility_work} descriptor_work={descriptor_work} \
         variant_work={variant_work} import_work={import_work} work={work}"
    );
    work
}

fn assert_unqualified_import_diagnostics(
    diagnostics: &[Diagnostic],
    count: usize,
    import_work: usize,
) {
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), count * 2, "{diagnostics:#?}");
    let written = visibility
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    for written_type in ["State<Int>::Hidden", "State::Hidden"] {
        assert_eq!(
            written
                .iter()
                .filter(|json| json.contains(&format!("\"written_type\":\"{written_type}\"")))
                .count(),
            count
        );
    }
    assert!(
        import_work >= count * 4,
        "index construction and exact/any lookup must be counted: count={count} import_work={import_work}"
    );
}

#[test]
fn public_visibility_unqualified_import_work_is_linear_for_exact_and_any_arity() {
    let work = [16, 32, 64].map(same_leaf_unqualified_import_work);

    let first_growth = work[1] - work[0];
    let second_growth = work[2] - work[1];
    assert_eq!(second_growth, first_growth * 2, "work={work:?}");
}

mod canonicalization;
