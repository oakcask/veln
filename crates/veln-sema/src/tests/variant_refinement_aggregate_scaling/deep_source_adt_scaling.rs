use super::*;

fn deep_source_adt_registry() -> TypeEnvironment {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "type Positive<T>\n",
            "  Present(T)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    TypeEnvironment::from_module(&lower_surface_ast(&parsed.tree))
}

fn source_adt_identity(environment: &TypeEnvironment, name: &str) -> String {
    environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == name)
        .unwrap_or_else(|| panic!("missing {name} descriptor"))
        .identity()
}

fn deep_covariant_type(
    depth: usize,
    positive_identity: &str,
    wrapper_name: &str,
    leaf: Type,
) -> Type {
    (0..depth).fold(leaf, |nested, _| {
        Type::resolved_named(
            wrapper_name.to_string(),
            positive_identity.to_string(),
            vec![nested],
        )
    })
}

fn state_refinement(identity: &str, name: &str, variant: &str) -> Type {
    Type::resolved_variant_refinement(
        name.to_string(),
        identity.to_string(),
        Vec::new(),
        vec![variant.to_string()],
    )
}

fn deep_leaf(mut ty: &Type, depth: usize) -> &Type {
    for _ in 0..depth {
        let Type::Named { args, .. } = ty else {
            panic!("expected a named covariant wrapper, found {ty:?}");
        };
        ty = &args[0];
    }
    ty
}

#[derive(Debug)]
struct DeepJoinMeasurement {
    retained_nodes: usize,
    acceptance: crate::aggregate_type_join::DeepJoinWork,
    acceptance_clones: usize,
    materialization: crate::aggregate_type_join::DeepJoinWork,
    materialization_clones: usize,
}

struct TimedDeepJoinWork {
    work: crate::aggregate_type_join::DeepJoinWork,
    clones: usize,
    elapsed: std::time::Duration,
}

fn deep_source_adt_join_inputs(
    environment: &TypeEnvironment,
    depth: usize,
    equal_leaf: bool,
) -> (Type, Type) {
    let state_identity = source_adt_identity(environment, "State");
    let positive_identity = source_adt_identity(environment, "Positive");
    let initial = deep_covariant_type(
        depth,
        &positive_identity,
        "FirstPositive",
        state_refinement(&state_identity, "FirstState", "Ready"),
    );
    let right = deep_covariant_type(
        depth,
        &positive_identity,
        "SecondPositive",
        state_refinement(
            &state_identity,
            "SecondState",
            if equal_leaf { "Ready" } else { "Closed" },
        ),
    );
    (initial, right)
}

fn measure_deep_join_acceptance(
    environment: &TypeEnvironment,
    joined: &mut crate::aggregate_type_join::AggregateTypeJoin,
    right: &Type,
) -> TimedDeepJoinWork {
    crate::aggregate_type_join::reset_deep_work();
    let started = std::time::Instant::now();
    let (accepted, clones) = crate::semantic_model::measure_type_clone_nodes(|| {
        joined.try_join(&environment.adts, right)
    });
    let work = crate::aggregate_type_join::take_deep_work();
    assert!(accepted);
    TimedDeepJoinWork {
        work,
        clones,
        elapsed: started.elapsed(),
    }
}

fn measure_deep_join_materialization(
    joined: &crate::aggregate_type_join::AggregateTypeJoin,
) -> (Type, TimedDeepJoinWork) {
    crate::aggregate_type_join::reset_deep_work();
    let started = std::time::Instant::now();
    let (materialized, clones) =
        crate::semantic_model::measure_type_clone_nodes(|| joined.result_type());
    let work = crate::aggregate_type_join::take_deep_work();
    (
        materialized,
        TimedDeepJoinWork {
            work,
            clones,
            elapsed: started.elapsed(),
        },
    )
}

fn measure_deep_source_adt_join(
    environment: &TypeEnvironment,
    depth: usize,
    equal_leaf: bool,
) -> DeepJoinMeasurement {
    let (initial, right) = deep_source_adt_join_inputs(environment, depth, equal_leaf);

    crate::aggregate_type_join::reset_deep_work();
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("deep source ADT join");
    let retained_nodes = crate::aggregate_type_join::take_deep_work().retained_nodes;

    let acceptance = measure_deep_join_acceptance(environment, &mut joined, &right);
    let (materialized, materialization) = measure_deep_join_materialization(&joined);

    let Type::VariantRefinement { name, variants, .. } = deep_leaf(&materialized, depth) else {
        panic!("expected a refined State leaf");
    };
    assert_eq!(name, "State");
    assert_eq!(variants.len(), usize::from(!equal_leaf) + 1);
    eprintln!(
        "deep source-ADT join depth={depth} equal_leaf={equal_leaf}: acceptance={:?} {:?} clones={}, materialization={:?} {:?} clones={}",
        acceptance.elapsed,
        acceptance.work,
        acceptance.clones,
        materialization.elapsed,
        materialization.work,
        materialization.clones,
    );

    DeepJoinMeasurement {
        retained_nodes,
        acceptance: acceptance.work,
        acceptance_clones: acceptance.clones,
        materialization: materialization.work,
        materialization_clones: materialization.clones,
    }
}

#[test]
fn deep_source_adt_covariant_join_work_is_linear_and_transactional() {
    let environment = deep_source_adt_registry();
    let depths = [512, 1024, 2048];
    let differing = depths.map(|depth| measure_deep_source_adt_join(&environment, depth, false));
    let equal = depths.map(|depth| measure_deep_source_adt_join(&environment, depth, true));

    for measurements in [&differing, &equal] {
        assert_deep_join_measurements_are_linear(depths, measurements);
    }

    assert_deep_source_adt_join_is_transactional(&environment);
}

fn assert_deep_join_measurements_are_linear(
    depths: [usize; 3],
    measurements: &[DeepJoinMeasurement; 3],
) {
    for (measurement, depth) in measurements.iter().zip(depths) {
        assert_deep_join_work(measurement, depth);
    }
    for adjacent in measurements.windows(2) {
        assert_adjacent_deep_join_work_is_linear(&adjacent[0], &adjacent[1]);
    }
}

fn assert_deep_join_work(measurement: &DeepJoinMeasurement, depth: usize) {
    assert_eq!(measurement.retained_nodes, depth + 1);
    assert_eq!(measurement.acceptance.node_visits, depth + 1);
    assert_eq!(measurement.acceptance.retained_nodes, depth + 1);
    assert_eq!(measurement.acceptance.presentation_nodes, depth + 1);
    assert_eq!(measurement.acceptance.materialized_nodes, 0);
    assert_eq!(measurement.acceptance_clones, 0);
    assert_eq!(measurement.materialization.node_visits, 0);
    assert_eq!(measurement.materialization.presentation_nodes, 0);
    assert_eq!(measurement.materialization.materialized_nodes, depth + 1);
    assert_eq!(measurement.materialization_clones, 0);
}

fn assert_adjacent_deep_join_work_is_linear(
    smaller: &DeepJoinMeasurement,
    larger: &DeepJoinMeasurement,
) {
    assert!(larger.retained_nodes <= smaller.retained_nodes * 2 + 1);
    assert!(larger.acceptance.node_visits <= smaller.acceptance.node_visits * 2 + 1);
    assert!(larger.acceptance.presentation_nodes <= smaller.acceptance.presentation_nodes * 2 + 1);
    assert!(
        larger.materialization.materialized_nodes
            <= smaller.materialization.materialized_nodes * 2 + 1
    );
}

struct DeepTransactionalJoinTypes {
    initial: Type,
    second: Type,
    rejected: Type,
    later: Type,
}

fn deep_transactional_join_types(
    environment: &TypeEnvironment,
    depth: usize,
) -> DeepTransactionalJoinTypes {
    let state_identity = source_adt_identity(environment, "State");
    let positive_identity = source_adt_identity(environment, "Positive");
    let wrap = |name, leaf| deep_covariant_type(depth, &positive_identity, name, leaf);
    DeepTransactionalJoinTypes {
        initial: wrap(
            "FirstPositive",
            state_refinement(&state_identity, "FirstState", "Ready"),
        ),
        second: wrap(
            "SecondPositive",
            state_refinement(&state_identity, "SecondState", "Closed"),
        ),
        rejected: wrap("RejectedPositive", Type::string()),
        later: wrap(
            "FirstPositive",
            state_refinement(&state_identity, "FirstState", "Failed"),
        ),
    }
}

fn assert_deep_source_adt_join_is_transactional(environment: &TypeEnvironment) {
    let depth = 256;
    let types = deep_transactional_join_types(environment, depth);
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &types.initial)
            .expect("deep transactional join");
    assert!(joined.try_join(&environment.adts, &types.second));
    let before_failure = joined.result_type();
    crate::aggregate_type_join::reset_deep_work();
    assert!(!joined.try_join(&environment.adts, &types.rejected));
    let failed_work = crate::aggregate_type_join::take_deep_work();
    assert_eq!(failed_work.node_visits, depth + 1);
    assert_eq!(failed_work.presentation_nodes, 0);
    assert_eq!(failed_work.materialized_nodes, 0);
    let after_failure = joined.result_type();
    assert_eq!(before_failure, after_failure);
    assert!(joined.try_join(&environment.adts, &types.later));
    let materialized = joined.result_type();
    let Type::Named { name, args, .. } = deep_leaf(&materialized, depth) else {
        panic!("expected the complete State domain after rollback");
    };
    assert_eq!(name, "State", "alias conflict must remain canonical");
    assert!(args.is_empty());
}
