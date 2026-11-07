use super::*;

fn deep_record_type(depth: usize, leaf: Type) -> Type {
    (0..depth).fold(leaf, |nested, _| {
        Type::Record(vec![("nested".to_string(), nested)])
    })
}

fn deep_record_leaf(mut ty: &Type, depth: usize) -> &Type {
    for _ in 0..depth {
        let Type::Record(fields) = ty else {
            panic!("expected a record node, found {ty:?}");
        };
        ty = &fields[0].1;
    }
    ty
}

fn record_join_registry() -> TypeEnvironment {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Last\n",
            "  Other\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    TypeEnvironment::from_module(&lower_surface_ast(&parsed.tree))
}

fn record_source_adt_identity(environment: &TypeEnvironment, name: &str) -> String {
    environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == name)
        .unwrap_or_else(|| panic!("missing {name} descriptor"))
        .identity()
}

fn record_state_refinement(identity: &str, name: &str, variant: &str) -> Type {
    Type::resolved_variant_refinement(
        name.to_string(),
        identity.to_string(),
        Vec::new(),
        vec![variant.to_string()],
    )
}

#[derive(Debug)]
struct DeepRecordMeasurement {
    setup: crate::aggregate_type_join::DeepJoinWork,
    acceptance: crate::aggregate_type_join::DeepJoinWork,
    materialization: crate::aggregate_type_join::DeepJoinWork,
}

fn measure_deep_record_join(environment: &TypeEnvironment, depth: usize) -> DeepRecordMeasurement {
    let state_identity = record_source_adt_identity(environment, "State");
    let initial = deep_record_type(
        depth,
        record_state_refinement(&state_identity, "First", "Ready"),
    );
    let second = deep_record_type(
        depth,
        record_state_refinement(&state_identity, "First", "Closed"),
    );

    crate::aggregate_type_join::reset_deep_work();
    let setup_started = std::time::Instant::now();
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("deep record join");
    let setup_elapsed = setup_started.elapsed();
    let setup = crate::aggregate_type_join::take_deep_work();

    crate::aggregate_type_join::reset_deep_work();
    let acceptance_started = std::time::Instant::now();
    assert!(joined.try_join(&environment.adts, &second));
    let acceptance_elapsed = acceptance_started.elapsed();
    let acceptance = crate::aggregate_type_join::take_deep_work();

    crate::aggregate_type_join::reset_deep_work();
    let materialization_started = std::time::Instant::now();
    let materialized = joined.result_type();
    let materialization_elapsed = materialization_started.elapsed();
    let materialization = crate::aggregate_type_join::take_deep_work();
    let Type::VariantRefinement { name, variants, .. } = deep_record_leaf(&materialized, depth)
    else {
        panic!("expected a refined State leaf");
    };
    assert_eq!(name, "First");
    assert_eq!(
        variants.iter().map(String::as_str).collect::<Vec<_>>(),
        ["Ready", "Closed"],
    );

    eprintln!(
        "deep record join depth={depth}: setup={setup_elapsed:?} {setup:?}, acceptance={acceptance_elapsed:?} {acceptance:?}, materialization={materialization_elapsed:?} {materialization:?}",
    );
    crate::semantic_model::drop_type_iteratively(initial);
    crate::semantic_model::drop_type_iteratively(second);
    crate::semantic_model::drop_type_iteratively(materialized);

    DeepRecordMeasurement {
        setup,
        acceptance,
        materialization,
    }
}

#[test]
fn deep_structural_record_join_setup_is_linear_and_transactional() {
    let environment = record_join_registry();
    let depths = [256, 512, 1024];
    let measurements = depths.map(|depth| measure_deep_record_join(&environment, depth));

    for (measurement, depth) in measurements.iter().zip(depths) {
        assert_eq!(measurement.setup.setup_type_nodes, depth + 1);
        assert_eq!(measurement.setup.retained_nodes, depth + 1);
        assert_eq!(measurement.acceptance.setup_type_nodes, 0);
        assert_eq!(measurement.acceptance.node_visits, depth + 1);
        assert_eq!(measurement.acceptance.presentation_nodes, 1);
        assert_eq!(measurement.materialization.materialized_nodes, depth + 1);
    }
    for adjacent in measurements.windows(2) {
        assert!(adjacent[1].setup.setup_type_nodes <= adjacent[0].setup.setup_type_nodes * 2 + 1);
        assert!(adjacent[1].setup.retained_nodes <= adjacent[0].setup.retained_nodes * 2 + 1);
        assert!(adjacent[1].acceptance.node_visits <= adjacent[0].acceptance.node_visits * 2 + 1);
        assert!(
            adjacent[1].materialization.materialized_nodes
                <= adjacent[0].materialization.materialized_nodes * 2 + 1
        );
    }

    assert_deep_record_join_is_transactional(&environment);
    assert_deep_unknown_record_is_rejected_linearly(&environment);
}

fn assert_deep_record_join_is_transactional(environment: &TypeEnvironment) {
    let depth = 512;
    let state_identity = record_source_adt_identity(environment, "State");
    let make =
        |name, leaf| deep_record_type(depth, record_state_refinement(&state_identity, name, leaf));
    let initial = make("First", "Ready");
    let second = make("First", "Closed");
    let rejected = deep_record_type(depth, Type::string());
    let later = make("First", "Other");
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("deep transactional record join");
    assert!(joined.try_join(&environment.adts, &second));

    crate::aggregate_type_join::reset_deep_work();
    assert!(!joined.try_join(&environment.adts, &rejected));
    let failed_work = crate::aggregate_type_join::take_deep_work();
    assert_eq!(failed_work.node_visits, depth + 1);
    assert_eq!(failed_work.presentation_nodes, 0);
    assert_eq!(failed_work.materialized_nodes, 0);
    let after_failure = joined.result_type();
    let Type::VariantRefinement { name, variants, .. } = deep_record_leaf(&after_failure, depth)
    else {
        panic!("expected the retained refined State leaf");
    };
    assert_eq!(name, "First");
    assert_eq!(
        variants.iter().map(String::as_str).collect::<Vec<_>>(),
        ["Ready", "Closed"],
    );

    assert!(joined.try_join(&environment.adts, &later));
    let materialized = joined.result_type();
    let Type::VariantRefinement { name, variants, .. } = deep_record_leaf(&materialized, depth)
    else {
        panic!("expected the final refined State leaf");
    };
    assert_eq!(name, "First");
    assert_eq!(
        variants.iter().map(String::as_str).collect::<Vec<_>>(),
        ["Ready", "Closed", "Other"],
    );

    for ty in [
        initial,
        second,
        rejected,
        later,
        after_failure,
        materialized,
    ] {
        crate::semantic_model::drop_type_iteratively(ty);
    }
}

fn assert_deep_unknown_record_is_rejected_linearly(environment: &TypeEnvironment) {
    let depth = 1024;
    let incomplete = deep_record_type(depth, Type::Unknown);
    crate::aggregate_type_join::reset_deep_work();
    assert!(
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &incomplete)
            .is_none()
    );
    let work = crate::aggregate_type_join::take_deep_work();
    assert_eq!(work.setup_type_nodes, depth + 1);
    assert_eq!(work.retained_nodes, 0);
    crate::semantic_model::drop_type_iteratively(incomplete);
}
