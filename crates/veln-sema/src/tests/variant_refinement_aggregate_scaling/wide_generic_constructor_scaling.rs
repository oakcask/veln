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
            .all(|((inference_work, _), width)| *inference_work >= width * 30),
        "the metric must include invariant contribution materialization, validation, and commit work in ordinary and private inference: {work:?}"
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
