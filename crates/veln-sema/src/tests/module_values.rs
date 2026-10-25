use super::*;

#[test]
fn module_values_block_checked_core_and_typed_ir_at_the_shared_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    assert_eq!(module.module_values.len(), 1);
    assert_eq!(module.functions.len(), 1);

    let lowered = lower_checked_surface_module(&module);

    let diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "{:#?}", lowered.diagnostics);
    let diagnostic = diagnostics[0];
    assert_eq!(
        diagnostic.message,
        "module value declaration `answer` is not executable"
    );
    assert_eq!(diagnostic.kind, DiagnosticKind::Module);
    assert_eq!(diagnostic.severity, veln_diagnostics::Severity::Error);
    assert_eq!(
        diagnostic.details,
        veln_diagnostics::JsonValue::object([
            ("phase", veln_diagnostics::JsonValue::string("module")),
            ("field", veln_diagnostics::JsonValue::string("module_value"),),
            ("name", veln_diagnostics::JsonValue::string("answer")),
        ])
    );
    assert_eq!(
        diagnostic.related,
        vec![veln_diagnostics::JsonValue::object([(
            "message",
            veln_diagnostics::JsonValue::string(
                "The declaration is available to parsing and formatting, but semantic analysis and code generation do not support module values yet.",
            ),
        )])]
    );
    assert_diagnostic_span(diagnostic, 1, 1, 2, 1);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());

    let lowered_with_incomplete_diagnostics = lower_analyzed_surface_module(&module, Vec::new());
    assert_eq!(
        lowered_with_incomplete_diagnostics
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
            .count(),
        1
    );
    assert!(lowered_with_incomplete_diagnostics.core.is_none());
    assert!(lowered_with_incomplete_diagnostics.ir.is_none());
}
