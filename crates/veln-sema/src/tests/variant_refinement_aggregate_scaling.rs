use super::*;
use crate::types::TypeEnvironment;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

fn aggregate_join_scaling_source(variant_count: usize) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("Variant{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("end\ntype Repeated<A>\n  Many(");
    source.push_str(
        &(0..variant_count)
            .map(|_| "A")
            .collect::<Vec<_>>()
            .join(", "),
    );
    source.push_str(")\nend\n");
    for variant in &variants {
        source.push_str(&format!(
            "fn make_{variant}() -> State::{variant}\n  {variant}\nend\n"
        ));
    }
    let calls = variants
        .iter()
        .map(|variant| format!("make_{variant}()"))
        .collect::<Vec<_>>();
    source.push_str(&format!("fn vector()\n  [{}]\nend\n", calls.join(", ")));
    source.push_str(&format!(
        "fn dictionary()\n  {{{}}}\nend\n",
        calls
            .iter()
            .zip(calls.iter().rev())
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    source.push_str(&format!(
        "fn repeated()\n  Many({})\nend\n",
        calls.join(", ")
    ));
    source.push_str(concat!(
        "fn main() -> ()\n",
        "  let items: Vec<State> = vector()\n",
        "  let entries: Dict<State, State> = dictionary()\n",
        "  let payloads: Repeated<State>::Many = repeated()\n",
        "end\n",
    ));
    source
}

fn independent_aggregate_site_scaling_source(variant_count: usize) -> String {
    let mut source = String::from("type State\n");
    for index in 0..variant_count {
        source.push_str(&format!("  Variant{index:04}\n"));
    }
    source.push_str("end\ntype Boxed<A>\n  One(A)\nend\n");
    for index in 0..variant_count {
        let variant = format!("State::Variant{index:04}");
        source.push_str(&format!(
            "fn vector_{index:04}()\n  [{variant}]\nend\n\
             fn dictionary_{index:04}()\n  {{{variant}: {variant}}}\nend\n\
             fn boxed_{index:04}()\n  Boxed::One({variant})\nend\n"
        ));
    }
    source
}

fn aggregate_rejection_scaling_source(variant_count: usize) -> String {
    let mut source = String::from("type State\n");
    for index in 0..variant_count {
        source.push_str(&format!("  Variant{index:04}\n"));
    }
    source.push_str("end\ntype Foreign\n  Other\nend\ntype Repeated<A>\n  Many(");
    source.push_str(
        &(0..=variant_count)
            .map(|_| "A")
            .collect::<Vec<_>>()
            .join(", "),
    );
    source.push_str(")\nend\nfn main() -> ()\n");
    let rejected = (0..variant_count)
        .map(|_| "Foreign::Other")
        .collect::<Vec<_>>();
    source.push_str(&format!(
        "  let vector = [State::Variant0000, {}]\n",
        rejected.join(", ")
    ));
    source.push_str(&format!(
        "  let dictionary = {{State::Variant0000: State::Variant0000, {}}}\n",
        (0..variant_count)
            .map(|_| "Foreign::Other: Foreign::Other")
            .collect::<Vec<_>>()
            .join(", ")
    ));
    source.push_str(&format!(
        "  let repeated = Repeated::Many(State::Variant0000, {})\n",
        rejected.join(", ")
    ));
    source.push_str("end\n");
    source
}

fn wide_generic_constructor_scaling_source(width: usize) -> String {
    let parameters = (0..width)
        .map(|index| format!("A{index}"))
        .collect::<Vec<_>>();
    let payloads = parameters
        .iter()
        .map(|parameter| format!("Vec<{parameter}>"))
        .collect::<Vec<_>>();
    let arguments = (0..width).map(|_| "[1]").collect::<Vec<_>>().join(", ");
    let result_args = (0..width).map(|_| "Int").collect::<Vec<_>>().join(", ");
    format!(
        "type Wide<{}>\n  Made({})\nend\nfn inferred()\n  Made({arguments})\nend\npub fn declared() -> Wide<{result_args}>::Made\n  Made({arguments})\nend\n",
        parameters.join(", "),
        payloads.join(", "),
    )
}

#[test]
fn aggregate_join_work_grows_linearly_through_all_inference_paths() {
    let work = [200, 400, 800, 1600].map(|variant_count| {
        let source = SourceFile::new("main.veln", aggregate_join_scaling_source(variant_count));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        crate::types::reset_variant_canonicalization_lookups();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count}-variant aggregate analysis: {:?}",
            started.elapsed()
        );
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let variant_canonicalization_lookups =
            crate::types::take_variant_canonicalization_lookups();
        let environment = TypeEnvironment::from_module(&module);
        assert_eq!(
            environment.function("vector").unwrap().return_type.render(),
            "Vec<State>"
        );
        assert_eq!(
            environment
                .function("dictionary")
                .unwrap()
                .return_type
                .render(),
            "Dict<State, State>"
        );
        assert_eq!(
            environment
                .function("repeated")
                .unwrap()
                .return_type
                .render(),
            "Repeated<State>::Many"
        );
        (
            crate::aggregate_type_join::take_work(),
            variant_canonicalization_lookups,
        )
    });
    eprintln!("aggregate analysis work at doubled sizes (join, variant lookup): {work:?}");
    assert!(work[0].0 > 0, "the metric must observe aggregate join work");
    assert!(
        work[1].0 <= work[0].0 * 2 + 64
            && work[2].0 <= work[1].0 * 2 + 64
            && work[3].0 <= work[2].0 * 2 + 64,
        "doubling aggregate input must add only linear join work: {work:?}"
    );
    assert!(
        work[0].1 > 0
            && work[1].1 <= work[0].1 * 2 + 64
            && work[2].1 <= work[1].1 * 2 + 64
            && work[3].1 <= work[2].1 * 2 + 64,
        "doubling aggregate input must add only linear variant canonicalization lookups: {work:?}"
    );
}

#[test]
fn aggregate_join_initialization_and_materialization_scale_with_independent_sites() {
    let work = [50, 100, 200, 400].map(|variant_count| {
        let source = SourceFile::new(
            "main.veln",
            independent_aggregate_site_scaling_source(variant_count),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count} variants and {} independent aggregate sites: {:?}",
            variant_count * 3,
            started.elapsed()
        );
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        crate::aggregate_type_join::take_work()
    });
    eprintln!("independent aggregate-site work units at doubled sizes: {work:?}");
    assert!(
        work[0] >= 50 * 3,
        "the metric must include initialization and materialization at each site: {work:?}"
    );
    assert!(
        work[1] <= work[0] * 2 + 64 && work[2] <= work[1] * 2 + 64 && work[3] <= work[2] * 2 + 64,
        "doubling variants and independent sites must add only linear aggregate work: {work:?}"
    );
}

#[test]
fn unchanged_aggregate_join_work_grows_linearly_for_rejected_contributions() {
    let work = [50, 100, 200, 400].map(|variant_count| {
        let source = SourceFile::new(
            "main.veln",
            aggregate_rejection_scaling_source(variant_count),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count}-variant rejected aggregate analysis: {:?}",
            started.elapsed()
        );
        assert_eq!(diagnostics.len(), variant_count * 4, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.mismatch"),
            "{diagnostics:#?}"
        );
        let details = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.details.to_json())
            .collect::<Vec<_>>();
        assert!(
            details.iter().all(|details| {
                details.contains("\"expected_type\":\"State::Variant0000\"")
                    && details.contains("\"actual_type\":\"Foreign::Other\"")
            }),
            "{details:#?}"
        );
        for (constraint, expected_count) in [
            ("list_element", variant_count),
            ("dict_key", variant_count),
            ("dict_value", variant_count),
            ("call_argument", variant_count),
        ] {
            assert_eq!(
                details
                    .iter()
                    .filter(|details| details.contains(&format!("\"constraint\":\"{constraint}\"")))
                    .count(),
                expected_count,
                "{constraint}: {details:#?}"
            );
        }
        crate::aggregate_type_join::take_work()
    });
    eprintln!("aggregate rejection join work units at doubled sizes: {work:?}");
    assert!(work[0] > 0, "the metric must observe aggregate join work");
    assert!(
        work[1] <= work[0] * 2 + 64 && work[2] <= work[1] * 2 + 64 && work[3] <= work[2] * 2 + 64,
        "doubling rejected aggregate input must add only linear join work: {work:?}"
    );
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
        let inferred = environment.function("inferred").expect("inferred function");
        assert_eq!(inferred.return_type.render().matches("Int").count(), width);
        (
            crate::aggregate_type_join::take_work(),
            type_parameter_lookups,
        )
    });
    eprintln!("generic constructor work at doubled widths (inference, parameter lookup): {work:?}");
    assert!(
        work[0].0 > 0,
        "the metric must observe constructor inference work"
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
fn aggregate_join_cache_invalidates_after_a_successful_variant_change() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  First\n",
        "  Second\n",
        "  Third\n",
        "end\n",
        "type Foreign\n",
        "  Other\n",
        "end\n",
        "fn foreign() -> Foreign::Other\n",
        "  Other\n",
        "end\n",
        "fn main() -> ()\n",
        "  let values = [Second, foreign(), First, foreign()]\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message.to_string(),
        "expected `State::Second`, but found `Foreign`"
    );
    assert_eq!(
        diagnostics[1].message.to_string(),
        "expected `State::First | State::Second`, but found `Foreign`"
    );
}

#[test]
fn failed_payloads_do_not_contribute_to_private_or_body_recovery_joins() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "type Triple<A>\n",
            "  Made(A, A, A)\n",
            "end\n",
            "fn recovered()\n",
            "  Triple::Made(Ready, begin\n",
            "    let bad: Int = \"bad\"\n",
            "    Closed\n",
            "  end, Failed)\n",
            "end\n",
            "fn recovered_vector()\n",
            "  [Ready, begin\n",
            "    let bad: Int = \"bad\"\n",
            "    Closed\n",
            "  end, Failed]\n",
            "end\n",
            "fn recovered_dict_values()\n",
            "  {1: Ready, 2: begin\n",
            "    let bad: Int = \"bad\"\n",
            "    Closed\n",
            "  end, 3: Failed}\n",
            "end\n",
            "fn accept_triple(value: Triple<State::Ready | State::Failed>::Made) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_vector(value: Vec<State::Ready | State::Failed>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_dict_values(value: Dict<Int, State::Ready | State::Failed>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept_triple(recovered())\n",
            "  accept_vector(recovered_vector())\n",
            "  accept_dict_values(recovered_dict_values())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    for diagnostic in &diagnostics {
        assert_eq!(diagnostic.id, "type.mismatch", "{diagnostics:#?}");
        assert_eq!(
            diagnostic.message.to_string(),
            "expected `Int`, but found `String`"
        );
        let span = diagnostic.span.as_ref().expect("initializer mismatch span");
        assert_eq!(
            &source.text()[span.start.offset..span.end.offset],
            "\"bad\""
        );
    }

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("recovered")
            .expect("private omitted result should be published")
            .return_type
            .render(),
        "Triple<State::Ready | State::Failed>::Made"
    );
    for (function, expected) in [
        ("recovered_vector", "Vec<State::Ready | State::Failed>"),
        (
            "recovered_dict_values",
            "Dict<Int, State::Ready | State::Failed>",
        ),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected
        );
    }
}

#[test]
fn rejected_expressions_never_contribute_to_private_aggregate_results() {
    let failures = [
        (
            "annotated local",
            concat!(
                "keep_closed(begin\n",
                "    let bad: Int = \"bad\"\n",
                "    Closed(1)\n",
                "  end)"
            ),
        ),
        ("declared call argument", "takes_int(\"bad\")"),
        ("constructor payload", "Closed(\"bad\")"),
    ];
    let aggregates = [
        (
            "vector element",
            "[Ready, {bad}, Failed]",
            "Vec<State::Ready | State::Failed>",
        ),
        (
            "dictionary key",
            "{{State::Ready: 1, {bad}: 2, State::Failed: 3}}",
            "Dict<State::Ready | State::Failed, Int>",
        ),
        (
            "dictionary value",
            "{{1: Ready, 2: {bad}, 3: Failed}}",
            "Dict<Int, State::Ready | State::Failed>",
        ),
        (
            "generic payload",
            "Triple::Made(Ready, {bad}, Failed)",
            "Triple<State::Ready | State::Failed>::Made",
        ),
    ];

    for (failure_name, rejected) in failures {
        for (aggregate_name, template, expected) in aggregates {
            if failure_name == "annotated local" && aggregate_name == "dictionary key" {
                continue;
            }
            let expression = template.replace("{bad}", rejected);
            let source = SourceFile::new(
                "main.veln",
                format!(
                    concat!(
                        "type State\n",
                        "  Ready\n",
                        "  Closed(Int)\n",
                        "  Failed\n",
                        "end\n",
                        "type Triple<A>\n",
                        "  Made(A, A, A)\n",
                        "end\n",
                        "fn takes_int(value: Int) -> State::Closed\n",
                        "  Closed(value)\n",
                        "end\n",
                        "fn keep_closed(value: State::Closed) -> State::Closed\n",
                        "  value\n",
                        "end\n",
                        "fn recovered()\n",
                        "  {}\n",
                        "end\n",
                        "fn accept(value: {}) -> ()\n",
                        "  ()\n",
                        "end\n",
                        "fn main() -> ()\n",
                        "  accept(recovered())\n",
                        "end\n",
                    ),
                    expression, expected,
                ),
            );
            let parsed = parse(&source);
            assert!(
                parsed.diagnostics.is_empty(),
                "{failure_name}, {aggregate_name}: {:#?}",
                parsed.diagnostics
            );
            let module = lower_surface_ast(&parsed.tree);
            let diagnostics = analyze_surface_module(&module);
            assert_eq!(
                diagnostics.len(),
                1,
                "{failure_name}, {aggregate_name}: {diagnostics:#?}"
            );
            assert_eq!(
                diagnostics[0].message.to_string(),
                "expected `Int`, but found `String`",
                "{failure_name}, {aggregate_name}: {diagnostics:#?}"
            );
            assert_eq!(
                TypeEnvironment::from_module(&module)
                    .function("recovered")
                    .expect("private omitted result should be published")
                    .return_type
                    .render(),
                expected,
                "{failure_name}, {aggregate_name}"
            );
        }
    }
}
