use super::*;
use crate::types::TypeEnvironment;

fn analyze_structural_record_case(
    source_text: &'static str,
) -> (veln_ast::SurfaceModule, Vec<Diagnostic>) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    (module, diagnostics)
}

fn assert_function_result(environment: &TypeEnvironment, function: &str, expected: &str) {
    assert_eq!(
        environment
            .function(function)
            .unwrap_or_else(|| panic!("missing function {function}"))
            .return_type
            .render(),
        expected,
        "unexpected inferred result for {function}",
    );
}

#[test]
fn structural_record_joins_cover_each_shared_inference_route_and_order() {
    let source_text =
        include_str!("../../../../examples/specification/check/structural-record-joins/main.veln");
    let (module, diagnostics) = analyze_structural_record_case(source_text);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    let record = "{detail: {state: State}, label: String}";

    for function in ["collection_forward", "collection_reverse"] {
        assert_function_result(&environment, function, &format!("Vec<{record}>"));
    }
    assert_function_result(
        &environment,
        "exact_leaf",
        "Vec<{callback: fn(Int) -> String, state: State}>",
    );
    for function in ["payload_forward", "payload_reverse"] {
        assert_function_result(&environment, function, &format!("Pair<{record}>::Paired"));
    }
    for function in [
        "if_forward",
        "if_reverse",
        "match_forward",
        "match_reverse",
        "private_forward",
        "private_reverse",
    ] {
        assert_function_result(&environment, function, record);
    }
}

#[test]
fn failed_nested_record_join_is_transactional_in_each_shared_inference_route() {
    let source_text = include_str!(
        "../../../../examples/specification/check/structural-record-joins-diagnostics/main.veln"
    );
    let (module, diagnostics) = analyze_structural_record_case(source_text);
    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}",
    );
    let rejected_spans = diagnostics
        .iter()
        .filter(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("mismatch span");
            &source_text[span.start.offset..span.end.offset] == "rejected"
        })
        .count();
    assert_eq!(rejected_spans, diagnostics.len(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let record = "{detail: {guard: Option<Int>}, state: First::Ready | First::Closed}";
    assert_function_result(
        &environment,
        "collection_transactional",
        &format!("Vec<{record}>"),
    );
    assert_function_result(
        &environment,
        "payload_transactional",
        &format!("Triple<{record}>::Grouped"),
    );
    assert_function_result(&environment, "private_transactional", record);
}
