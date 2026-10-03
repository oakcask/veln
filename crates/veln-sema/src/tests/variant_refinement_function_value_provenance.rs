use super::*;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

const STATE_DECL: &str = concat!("type State\n", "  Ready\n", "  Closed\n", "end\n",);

fn expected_type_origin_json(diagnostic: &Diagnostic) -> String {
    diagnostic
        .related
        .iter()
        .map(veln_diagnostics::JsonValue::to_json)
        .find(|related| related.contains("\"kind\":\"expected_type_origin\""))
        .expect("variant mismatch should report the expected type origin")
}

#[test]
fn function_typed_local_call_reports_the_local_annotation_as_expected_origin() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn ignore_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  let callback: fn(State::Ready) -> () = ignore_ready\n",
            "  callback(Closed)\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let mismatch = &diagnostics[0];
    assert_eq!(mismatch.id, "type.variant_mismatch", "{diagnostics:#?}");
    let origin = expected_type_origin_json(mismatch);
    assert!(
        origin.contains("Type annotation declared here."),
        "{origin}"
    );
    assert!(origin.contains("\"start\":{\"line\":9"), "{origin}");
    assert!(!origin.contains("\"start\":{\"line\":10"), "{origin}");
}

#[test]
fn function_typed_parameter_call_reports_the_parameter_declaration_as_expected_origin() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn invoke(callback: fn(State::Ready) -> ()) -> ()\n",
            "  callback(Closed)\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let mismatch = &diagnostics[0];
    assert_eq!(mismatch.id, "type.variant_mismatch", "{diagnostics:#?}");
    let origin = expected_type_origin_json(mismatch);
    assert!(origin.contains("Parameter type declared here."), "{origin}");
    assert!(origin.contains("\"start\":{\"line\":5"), "{origin}");
    assert!(!origin.contains("\"start\":{\"line\":6"), "{origin}");
}
