use super::*;

#[test]
fn invalid_module_headers_report_source_declaration_diagnostics_and_block_artifacts() {
    for (name, observed, start_column, end_column) in [
        ("App", "ascii_uppercase", 5, 8),
        ("_net", "underscore", 5, 9),
    ] {
        let source = SourceFile::new(
            "main.veln",
            &format!("mod {name}\nfn main() -> ()\n  ()\nend\n"),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        assert_eq!(module.functions[0].module_name, None);

        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            ["name.invalid_case"],
            "{diagnostics:#?}"
        );
        let diagnostic = &diagnostics[0];
        assert_eq!(
            diagnostic.message,
            format!("module name `{name}` must start with an ASCII lowercase letter")
        );
        assert_diagnostic_span(diagnostic, 1, start_column, 1, end_column);
        let details = diagnostic.details.to_json();
        assert!(details.contains("\"phase\":\"name\""), "{details}");
        assert!(details.contains("\"origin\":\"source\""), "{details}");
        assert!(
            details.contains("\"occurrence\":\"declaration\""),
            "{details}"
        );
        assert!(
            details.contains(&format!("\"name\":\"{name}\"")),
            "{details}"
        );
        assert!(details.contains("\"name_class\":\"module\""), "{details}");
        assert!(
            details.contains("\"required_initial\":\"ascii_lowercase\""),
            "{details}"
        );
        assert!(
            details.contains(&format!("\"observed_initial\":\"{observed}\"")),
            "{details}"
        );

        let lowered = lower_checked_surface_module(&module);
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn lowercase_module_header_keeps_checked_artifacts() {
    let source = SourceFile::new(
        "main.veln",
        concat!("mod app\n", "fn main() -> ()\n", "  ()\n", "end\n"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    assert_eq!(module.functions[0].module_name.as_deref(), Some("app"));
    assert!(
        module.invalid_names.is_empty(),
        "{:#?}",
        module.invalid_names
    );

    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}
