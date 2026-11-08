use super::*;

fn wide_generic_constructor_scaling_source(width: usize) -> String {
    let parameters = (0..width)
        .map(|index| format!("A{index}"))
        .collect::<Vec<_>>();
    let payloads = parameters
        .iter()
        .map(|parameter| format!("Duo<{parameter}, {parameter}>"))
        .collect::<Vec<_>>();
    let arguments = (0..width)
        .map(|_| "Paired(1, 1)")
        .collect::<Vec<_>>()
        .join(", ");
    let result_args = (0..width).map(|_| "Int").collect::<Vec<_>>().join(", ");
    format!(
        "type Duo<A, B>\n  Paired(A, B)\nend\npub type Wide<{}>\n  pub Made({})\nend\nfn inferred()\n  Made({arguments})\nend\npub fn declared() -> Wide<{result_args}>::Made\n  Made({arguments})\nend\n",
        parameters.join(", "),
        payloads.join(", "),
    )
}

#[derive(Clone, Copy, Debug)]
enum RepeatedConstraintWorkload {
    Lower,
    Upper,
    Equality,
    Mixed,
}

fn repeated_constraint_scaling_source(
    count: usize,
    workload: RepeatedConstraintWorkload,
) -> String {
    let (payloads, arguments) = match workload {
        RepeatedConstraintWorkload::Lower => (vec!["T"; count], vec!["state"; count]),
        RepeatedConstraintWorkload::Upper => (vec!["fn(T) -> ()"; count], vec!["callback"; count]),
        RepeatedConstraintWorkload::Equality => {
            (vec!["Invariant<T>"; count], vec!["exact_value"; count])
        }
        RepeatedConstraintWorkload::Mixed => (vec!["fn(T) -> T"; count], vec!["identity"; count]),
    };
    format!(
        "type State\n  Ready\n  Closed\nend\n\
         type Invariant<T>\n  InvariantValue(fn(T) -> T)\nend\n\
         type Repeated<T>\n  Made({})\nend\n\
         fn inferred(state: State, callback: fn(State) -> (), identity: fn(State) -> State, exact_value: Invariant<State>)\n  Made({})\nend\n\
         fn declared(state: State, callback: fn(State) -> (), identity: fn(State) -> State, exact_value: Invariant<State>) -> Repeated<State>::Made\n  Made({})\nend\n",
        payloads.join(", "),
        arguments.join(", "),
        arguments.join(", "),
    )
}

fn measure_repeated_constraint_work(count: usize, workload: RepeatedConstraintWorkload) -> usize {
    let source = SourceFile::new(
        "main.veln",
        repeated_constraint_scaling_source(count, workload),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::aggregate_type_join::reset_work();
    crate::aggregate_type_join::reset_bounded_frames();
    crate::type_relations::reset_type_presentation_work();
    let started = std::time::Instant::now();
    let (diagnostics, cloned_type_nodes) =
        crate::semantic_model::measure_type_clone_nodes(|| analyze_surface_module(&module));
    let elapsed = started.elapsed();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let inference_work = crate::aggregate_type_join::take_work();
    let bounded_frames = crate::aggregate_type_join::take_bounded_frames();
    let presentation = crate::type_relations::take_type_presentation_work();
    let presentation_work = presentation.conflict_lookups
        + presentation.conflict_insertions
        + presentation.child_lookups
        + presentation.child_insertions;
    let total = inference_work + cloned_type_nodes + bounded_frames + presentation_work;
    eprintln!(
        "{workload:?} repeated-constraint count {count}: {elapsed:?}, total={total}, inference={inference_work}, clones={cloned_type_nodes}, bounds={bounded_frames}, presentation={presentation_work}",
    );
    let environment = TypeEnvironment::from_module(&module);
    for (function, expected) in [
        ("inferred", "Repeated<State>::Made"),
        ("declared", "Repeated<State>"),
    ] {
        assert_eq!(
            environment.function(function).unwrap().return_type.render(),
            expected,
        );
    }
    total
}

#[test]
fn wide_generic_constructor_inference_work_grows_linearly_in_parameters_and_payloads() {
    let work = [100, 200, 400, 800].map(|width| {
        let source = SourceFile::new("main.veln", wide_generic_constructor_scaling_source(width));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        crate::adt::reset_type_parameter_lookups();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{width}-parameter generic constructor analysis: {:?}",
            started.elapsed()
        );
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let type_parameter_lookups = crate::adt::take_type_parameter_lookups();
        let environment = TypeEnvironment::from_module(&module);
        for function in ["inferred", "declared"] {
            let signature = environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} function"));
            assert_eq!(
                signature.return_type.render().matches("Int").count(),
                width,
                "{function} must exercise every invariant payload"
            );
        }
        (
            crate::aggregate_type_join::take_work(),
            type_parameter_lookups,
        )
    });
    eprintln!(
        "generic constructor work at doubled widths (transactional inference, parameter lookup): {work:?}"
    );
    assert!(
        work.iter()
            .zip([100, 200, 400, 800])
            .all(|((inference_work, _), width)| *inference_work >= width * 10),
        "the metric must include constraint collection, validation, and commit work in ordinary and private inference: {work:?}"
    );
    assert!(
        work[1].0 <= work[0].0 * 2 + 64
            && work[2].0 <= work[1].0 * 2 + 64
            && work[3].0 <= work[2].0 * 2 + 64,
        "doubling generic constructor width must add only linear inference work: {work:?}"
    );
    assert!(
        work[0].1 > 0
            && work[1].1 <= work[0].1 * 2 + 64
            && work[2].1 <= work[1].1 * 2 + 64
            && work[3].1 <= work[2].1 * 2 + 64,
        "doubling generic constructor width must add only linear type-parameter lookups: {work:?}"
    );
}

#[test]
fn repeated_constructor_constraint_work_grows_linearly_per_parameter() {
    for workload in [
        RepeatedConstraintWorkload::Lower,
        RepeatedConstraintWorkload::Upper,
        RepeatedConstraintWorkload::Equality,
        RepeatedConstraintWorkload::Mixed,
    ] {
        let work = [32, 64, 128].map(|count| measure_repeated_constraint_work(count, workload));
        assert!(
            work.iter()
                .zip([32, 64, 128])
                .all(|(work, count)| *work >= count * 8),
            "the guard must observe retained-type clones, constraint comparisons, bound updates, validation, and presentation work for {workload:?}: {work:?}",
        );
        assert!(
            work[1] <= work[0] * 2 + 128 && work[2] <= work[1] * 2 + 128,
            "doubling repeated {workload:?} constraints for one parameter must add only linear work: {work:?}",
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum UnavailableConstraintWorkload {
    Lower,
    Upper,
}

fn unavailable_constraint_scaling_source(
    count: usize,
    workload: UnavailableConstraintWorkload,
) -> String {
    let declarations = (0..count)
        .map(|index| format!("type Bound{index}\n  Value{index}\nend\n"))
        .collect::<String>();
    let callbacks = matches!(workload, UnavailableConstraintWorkload::Upper)
        .then(|| {
            (0..count)
                .map(|index| format!("fn accepts_{index}(value: Bound{index}) -> ()\n  ()\nend\n"))
                .collect::<String>()
        })
        .unwrap_or_default();
    let payload = match workload {
        UnavailableConstraintWorkload::Lower => "{value: T}",
        UnavailableConstraintWorkload::Upper => "{callback: fn(T) -> ()}",
    };
    let payloads = vec![payload; count].join(", ");
    let arguments = (0..count)
        .map(|index| match workload {
            UnavailableConstraintWorkload::Lower => format!("{{value: Value{index}}}"),
            UnavailableConstraintWorkload::Upper => {
                format!("{{callback: accepts_{index}}}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{declarations}{callbacks}type Repeated<T>\n  Made({payloads})\nend\nfn inferred()\n  Made({arguments})\nend\nfn ordinary() -> ()\n  let unavailable = Made({arguments})\n  ()\nend\n"
    )
}

fn measure_unavailable_constraint_work(
    count: usize,
    workload: UnavailableConstraintWorkload,
) -> usize {
    let source = SourceFile::new(
        "main.veln",
        unavailable_constraint_scaling_source(count, workload),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::aggregate_type_join::reset_work();
    crate::aggregate_type_join::reset_bounded_frames();
    crate::type_relations::reset_type_presentation_work();
    let (diagnostics, cloned_type_nodes) =
        crate::semantic_model::measure_type_clone_nodes(|| analyze_surface_module(&module));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.inference_ambiguous"),
        "{diagnostics:#?}"
    );
    let inference_work = crate::aggregate_type_join::take_work();
    let bounded_frames = crate::aggregate_type_join::take_bounded_frames();
    let presentation = crate::type_relations::take_type_presentation_work();
    let presentation_work = presentation.conflict_lookups
        + presentation.conflict_insertions
        + presentation.child_lookups
        + presentation.child_insertions;
    let total = inference_work + cloned_type_nodes + bounded_frames + presentation_work;
    eprintln!(
        "{workload:?} unavailable count {count}: total={total}, inference={inference_work}, clones={cloned_type_nodes}, bounds={bounded_frames}, presentation={presentation_work}"
    );
    total
}

#[test]
fn pairwise_incomparable_constructor_constraints_grow_linearly_when_unavailable() {
    for workload in [
        UnavailableConstraintWorkload::Lower,
        UnavailableConstraintWorkload::Upper,
    ] {
        let work = [16, 32, 64].map(|count| measure_unavailable_constraint_work(count, workload));
        assert!(
            work[1] <= work[0] * 2 + 128 && work[2] <= work[1] * 2 + 128,
            "doubling pairwise-incomparable {workload:?} constraints must add only linear work: {work:?}",
        );
    }
}
