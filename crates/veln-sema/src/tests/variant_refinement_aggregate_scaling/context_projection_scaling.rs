use super::*;

fn nested_context_classification_source(depth: usize) -> String {
    let nested_type = (0..depth).fold("Result<Int, String>".to_string(), |ty, _| {
        format!("Option<{ty}>")
    });
    let nested_value = (0..depth).fold("Ok(1)".to_string(), |value, _| format!("Some({value})"));
    format!(
        "fn inferred(first: {nested_type})\n  [first, {nested_value}]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: {nested_type}) -> ()\n  needs_int([first, {nested_value}])\nend\n"
    )
}

fn distinct_payload_parameter_context_source(width: usize) -> String {
    let parameters = (0..width)
        .map(|index| format!("A{index}"))
        .collect::<Vec<_>>();
    let ints = (0..width).map(|_| "Int").collect::<Vec<_>>();
    let values = (0..width)
        .map(|index| index.to_string())
        .collect::<Vec<_>>();
    let parameters = parameters.join(", ");
    let ints = ints.join(", ");
    let values = values.join(", ");
    format!(
        "type Bundle<{parameters}>\n  Packed({parameters})\nend\n\
         type Carrier<{parameters}>\n  Made(Bundle<{parameters}>)\nend\n\
         fn inferred(first: Carrier<{ints}>)\n  [first, Made(Packed({values}))]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: Carrier<{ints}>) -> ()\n  needs_int([first, Made(Packed({values}))])\nend\n"
    )
}

fn concrete_payload_wrapper_context_source(depth: usize) -> String {
    let nested_type = (0..depth).fold("T".to_string(), |ty, _| format!("Option<{ty}>"));
    let nested_value = (0..depth).fold("1".to_string(), |value, _| format!("Some({value})"));
    format!(
        "type Wrap<T>\n  Built({nested_type})\nend\n\
         fn inferred(first: Wrap<Int>)\n  [first, Built({nested_value})]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: Wrap<Int>) -> ()\n  needs_int([first, Built({nested_value})])\nend\n"
    )
}

fn wide_record_type(width: usize) -> String {
    let fields = (0..width)
        .map(|index| format!("field_{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{fields}}}")
}

fn record_list_context_source(member_count: usize, context_width: usize) -> String {
    let record = wide_record_type(context_width);
    let members = (0..member_count)
        .map(|_| "{}")
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "fn inferred(first: Vec<{record}>)\n  [first, [{members}]]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: Vec<{record}>) -> ()\n  needs_int([first, [{members}]])\nend\n"
    )
}

fn record_dict_context_source(member_count: usize, context_width: usize) -> String {
    let record = wide_record_type(context_width);
    let members = (0..member_count)
        .map(|index| format!("\"key_{index}\": {{}}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "fn inferred(first: Dict<String, {record}>)\n  [first, {{{members}}}]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: Dict<String, {record}>) -> ()\n  needs_int([first, {{{members}}}])\nend\n"
    )
}

fn wide_function_type(width: usize) -> String {
    let parameters = (0..width).map(|_| "Int").collect::<Vec<_>>().join(", ");
    format!("fn({parameters}) -> Int")
}

fn wide_function_declaration(width: usize) -> String {
    let parameters = (0..width)
        .map(|index| format!("value_{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("fn callback({parameters}) -> Int\n  0\nend\n")
}

fn function_list_context_source(member_count: usize, context_width: usize) -> String {
    let function_type = wide_function_type(context_width);
    let callback = wide_function_declaration(context_width);
    let members = (0..member_count)
        .map(|_| "callback")
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{callback}\
         fn inferred(first: Vec<{function_type}>)\n  [first, [{members}]]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: Vec<{function_type}>) -> ()\n  needs_int([first, [{members}]])\nend\n"
    )
}

fn function_dict_context_source(member_count: usize, context_width: usize) -> String {
    let function_type = wide_function_type(context_width);
    let callback = wide_function_declaration(context_width);
    let members = (0..member_count)
        .map(|index| format!("\"key_{index}\": callback"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{callback}\
         fn inferred(first: Dict<String, {function_type}>)\n  [first, {{{members}}}]\nend\n\
         fn needs_int(value: Int) -> ()\n  ()\nend\n\
         fn checked(first: Dict<String, {function_type}>) -> ()\n  needs_int([first, {{{members}}}])\nend\n"
    )
}

type ContextProjectionWork = crate::aggregate_type_join::ContextProjectionWork;

fn measure_context_projection(label: &str, source_text: String) -> [ContextProjectionWork; 2] {
    measure_context_projection_with_mismatches(label, source_text, Some(1))
}

fn measure_context_projection_with_mismatches(
    label: &str,
    source_text: String,
    expected_mismatches: Option<usize>,
) -> [ContextProjectionWork; 2] {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    crate::aggregate_type_join::reset_context_classification_work();
    let private_started = std::time::Instant::now();
    let environment = TypeEnvironment::from_module(&module);
    let private_elapsed = private_started.elapsed();
    let private_work = crate::aggregate_type_join::take_context_classification_work();

    let checked = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("checked"))
        .expect("checked function");
    crate::aggregate_type_join::reset_context_classification_work();
    let ordinary_started = std::time::Instant::now();
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    let diagnostics =
        crate::analysis::check_function_body(checked, &environment, &mut variant_diagnostics);
    let ordinary_elapsed = ordinary_started.elapsed();
    let mismatch_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .count();
    if let Some(expected) = expected_mismatches {
        assert_eq!(mismatch_count, expected, "{diagnostics:#?}");
    } else {
        assert!(mismatch_count >= 1, "{diagnostics:#?}");
    }
    let ordinary_work = crate::aggregate_type_join::take_context_classification_work();
    eprintln!(
        "{label}: private {private_elapsed:?} {private_work:?}, ordinary {ordinary_elapsed:?} {ordinary_work:?}"
    );
    [private_work, ordinary_work]
}

fn assert_projection_axes_are_additive(
    label: &str,
    source: fn(usize, usize) -> String,
    expected_mismatches: Option<usize>,
    expect_shared_results: bool,
) {
    let axes = [
        [(4, 16), (8, 16), (16, 16)],
        [(16, 4), (16, 8), (16, 16)],
        [(4, 4), (8, 8), (16, 16)],
    ];
    for (axis, sizes) in ["members", "context", "combined"].into_iter().zip(axes) {
        let measurements = sizes.map(|(members, width)| {
            measure_context_projection_with_mismatches(
                &format!("{label} {axis} members-{members} width-{width}"),
                source(members, width),
                expected_mismatches,
            )
        });
        for pipeline in 0..2 {
            assert!(
                measurements[0][pipeline].type_nodes_cloned >= sizes[0].1,
                "owned context clones must be observed: {measurements:?}",
            );
            if expect_shared_results {
                assert!(
                    measurements[0][pipeline].shared_context_reuses >= sizes[0].0 - 1,
                    "pointer-identical projected results must be reused: {measurements:?}",
                );
            }
            for adjacent in measurements.windows(2) {
                assert!(
                    adjacent[1][pipeline].total_relevant_work()
                        <= adjacent[0][pipeline].total_relevant_work() * 2 + 128,
                    "{axis} growth must add projection work: {measurements:?}",
                );
                assert!(
                    adjacent[1][pipeline].max_retained_state
                        <= adjacent[0][pipeline].max_retained_state * 2 + 16,
                    "{axis} growth must add retained state: {measurements:?}",
                );
                assert!(
                    adjacent[1][pipeline].structural_merge_probes
                        <= adjacent[0][pipeline].structural_merge_probes * 2 + 16,
                    "{axis} growth must not multiply structural merge work: {measurements:?}",
                );
            }
        }
    }
}

fn assert_total_projection_work_is_linear(
    measurements: &[[ContextProjectionWork; 2]],
    minimum_work: usize,
    work_allowance: usize,
    retained_allowance: usize,
) {
    for pipeline in 0..2 {
        assert!(
            measurements[0][pipeline].total_relevant_work() > minimum_work,
            "the metric must observe each inference pipeline: {measurements:?}",
        );
        for adjacent in measurements.windows(2) {
            assert!(
                adjacent[1][pipeline].total_relevant_work()
                    <= adjacent[0][pipeline].total_relevant_work() * 2 + work_allowance,
                "adjacent inputs must add only linear projection work: {measurements:?}",
            );
            assert!(
                adjacent[1][pipeline].max_retained_state
                    <= adjacent[0][pipeline].max_retained_state * 2 + retained_allowance,
                "adjacent inputs must retain only linear projection state: {measurements:?}",
            );
        }
    }
}

fn assert_edge_projection_work_is_linear(
    measurements: &[[ContextProjectionWork; 2]],
    minimum_edges: usize,
) {
    for pipeline in 0..2 {
        assert!(
            measurements[0][pipeline].expression_or_parameter_edges_scanned >= minimum_edges,
            "the metric must observe every payload edge in each pipeline: {measurements:?}",
        );
    }
    assert_total_projection_work_is_linear(measurements, 0, 128, 4);
}

#[test]
fn aggregate_context_classification_grows_linearly_with_adjacent_depths() {
    let measurements = [32, 64, 128].map(|depth| {
        measure_context_projection(
            &format!("depth-{depth} projected aggregate context"),
            nested_context_classification_source(depth),
        )
    });
    assert_total_projection_work_is_linear(&measurements, 32, 64, 2);
}

#[test]
fn aggregate_context_projection_grows_linearly_with_distinct_payload_parameters() {
    let measurements = [16, 32, 64].map(|width| {
        measure_context_projection(
            &format!("width-{width} distinct payload parameters"),
            distinct_payload_parameter_context_source(width),
        )
    });
    assert_edge_projection_work_is_linear(&measurements, 16);
}

#[test]
fn concrete_payload_wrapper_projection_grows_linearly_in_both_pipelines() {
    let measurements = [16, 32, 64].map(|depth| {
        measure_context_projection(
            &format!("concrete wrapper depth-{depth}"),
            concrete_payload_wrapper_context_source(depth),
        )
    });
    assert_edge_projection_work_is_linear(&measurements, 16);
}

#[test]
fn record_list_projection_is_additive_in_member_and_context_width() {
    assert_projection_axes_are_additive("record list", record_list_context_source, None, true);
}

#[test]
fn record_dict_projection_is_additive_in_member_and_context_width() {
    assert_projection_axes_are_additive(
        "record dictionary",
        record_dict_context_source,
        None,
        true,
    );
}

#[test]
fn function_list_projection_is_additive_in_member_and_context_width() {
    assert_projection_axes_are_additive(
        "function list",
        function_list_context_source,
        Some(1),
        false,
    );
}

#[test]
fn function_dict_projection_is_additive_in_member_and_context_width() {
    assert_projection_axes_are_additive(
        "function dictionary",
        function_dict_context_source,
        Some(1),
        false,
    );
}
