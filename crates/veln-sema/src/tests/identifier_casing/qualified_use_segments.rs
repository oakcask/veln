use super::*;

#[test]
fn unresolved_qualified_call_does_not_guess_module_segment_role() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Int\n", "  Foo::bar()\n", "end\n"),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `Foo::bar`"
    }));
}

#[test]
fn unique_recovered_type_qualified_constructor_reports_type_segment() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Sample\n",
            "  Made(Int)\n",
            "end\n",
            "fn main() -> Sample\n",
            "  sample::Made(1)\n",
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
                    == "type name `sample` must start with an ASCII uppercase letter"
        })
        .expect("recovered type segment casing diagnostic");
    assert_diagnostic_span(invalid, 5, 3, 5, 9);
    let details = invalid.details.to_json();
    assert!(
        details.contains("\"occurrence\":\"path_segment\""),
        "{details}"
    );
    assert!(details.contains("\"name_class\":\"type\""), "{details}");
    assert!(details.contains("\"segment_index\":0"), "{details}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn generated_unresolved_qualified_calls_do_not_create_segment_casing_work() {
    for count in [64, 128] {
        let source = SourceFile::new("main.veln", generated_unresolved_qualified_calls(count));
        let module = lower_surface_ast(&parse(&source).tree);
        let diagnostics = analyze_surface_module(&module);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.invalid_case")
                .count(),
            0,
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            count,
            "{diagnostics:#?}"
        );
    }
}

fn generated_unresolved_qualified_calls(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        source.push_str(&format!(
            "fn case_{index}() -> Int\n  Missing::bad::Value()\nend\n"
        ));
    }
    source
}
