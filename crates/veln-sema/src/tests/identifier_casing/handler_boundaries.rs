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
            "handler ask() handles Ask\n",
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
fn handler_context_parameter_type_path_validates_module_segment() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "handler ask(seed: Missing::Thing) handles Ask\n",
            "  value() => 0\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let invalid = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message
                    == "module name `Missing` must start with an ASCII lowercase letter"
        })
        .expect("handler context parameter type should validate the module segment");

    assert_diagnostic_span(invalid, 5, 19, 5, 26);
    let details = invalid.details.to_json();
    assert!(
        details.contains("\"occurrence\":\"path_segment\""),
        "{details}"
    );
    assert!(details.contains("\"name_class\":\"module\""), "{details}");
    assert!(details.contains("\"segment_index\":0"), "{details}");
}
