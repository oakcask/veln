use super::*;

#[test]
fn unresolved_handler_clause_call_does_not_guess_function_role() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "handler ask() for Ask\n",
            "  value() => missing::Thing()\n",
            "end\n",
            "\n",
            "fn body() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  handle body() with ask()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `missing::Thing`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
}

#[test]
fn unresolved_handler_context_refinement_uses_base_resolution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "handler ask(seed: Missing::Thing) for Ask\n",
            "  value() => 0\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let unresolved = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `Missing`"
        })
        .expect("handler context refinement should diagnose its unresolved base");

    assert_diagnostic_span(unresolved, 5, 19, 5, 26);
    assert_eq!(
        unresolved.details.to_json(),
        concat!(
            "{\"phase\":\"name\",\"node_id\":null,",
            "\"symbol\":\"Missing\",\"namespace\":\"type\",",
            "\"resolution_status\":\"unresolved\",\"candidates\":[]}"
        )
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "name.invalid_case"
            && diagnostic.id != "type.invalid_annotation"
            && diagnostic.id != "type.variant_refinement_unknown"
    }));
}
