use super::*;

fn diagnostics(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn callsite_modifier_introduces_source_location_binding() {
    let diagnostics = diagnostics(concat!(
        "pub fn start_line() -> Int callsite\n",
        "  callsite.start_line\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_return_inference_sees_the_callsite_binding() {
    let diagnostics = diagnostics("fn start_line() callsite\n  callsite.start_line\nend\n");

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn unresolved_callsite_reference_suggests_the_modifier() {
    let diagnostics = diagnostics("pub fn location() -> SourceLocation\n  callsite\nend\n");

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
        .expect("missing modifier diagnostic");
    assert_diagnostic_span(diagnostic, 2, 3, 2, 11);
    assert!(diagnostic.related.iter().any(|related| {
        related
            .to_json()
            .contains("Add `callsite` after the function's optional effects clause.")
    }));
}

#[test]
fn callsite_bindings_cannot_shadow_the_builtin() {
    let diagnostics = diagnostics(concat!(
        "fn parameter(callsite: Int) -> Int callsite\n",
        "  0\n",
        "end\n",
        "fn result() -> callsite: Int callsite\n",
        "  0\n",
        "end\n",
        "fn local() -> Int callsite\n",
        "  let callsite: Int = 1\n",
        "  0\n",
        "end\n",
        "fn pattern() -> Int callsite\n",
        "  match true\n",
        "    callsite => 0\n",
        "  end\n",
        "end\n",
    ));

    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.callsite_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 4, "{diagnostics:#?}");
    assert_diagnostic_span(shadows[0], 1, 14, 1, 22);
    assert_diagnostic_span(shadows[1], 4, 16, 4, 24);
    assert_diagnostic_span(shadows[2], 8, 7, 8, 15);
    assert_diagnostic_span(shadows[3], 13, 5, 13, 13);
    assert!(shadows.iter().all(|diagnostic| {
        diagnostic
            .related
            .iter()
            .any(|related| related.to_json().contains("Rename this binding"))
    }));
}

#[test]
fn source_location_is_the_exact_standard_record_shape() {
    let diagnostics = diagnostics(concat!(
        "pub fn fields(value: SourceLocation) -> Int\n",
        "  value.start_line + value.start_column + value.start_offset + value.end_line + value.end_column + value.end_offset\n",
        "end\n",
        "pub fn package_name(value: SourceLocation) -> String\n",
        "  value.package\n",
        "end\n",
        "pub fn module_name(value: SourceLocation) -> String\n",
        "  value.module\n",
        "end\n",
        "pub fn file_name(value: SourceLocation) -> String\n",
        "  value.file\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}
