use super::*;

pub(super) fn unresolved_local_import_diagnostics(
    module: &SurfaceModule,
    derived_modules: &[(String, SourceFile)],
    rejected_derived_modules: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    module
        .uses
        .iter()
        .filter(|use_decl| {
            use_decl.package.is_none()
                && use_decl.origin == UseOrigin::Source
                && (use_decl.name.contains("::")
                    || rejected_derived_modules.contains(&use_decl.name))
                && !derived_modules
                    .iter()
                    .any(|(module_name, _)| module_name == &use_decl.name)
        })
        .map(|use_decl| unresolved_local_import_diagnostic(use_decl, derived_modules))
        .collect()
}

fn unresolved_local_import_diagnostic(
    use_decl: &UseDecl,
    derived_modules: &[(String, SourceFile)],
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.unresolved_import",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "local import `{}` has no matching selected source file",
            use_decl.name
        ),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_path")),
            ("module_path", JsonValue::string(use_decl.name.clone())),
        ]),
    );
    for (module_name, source) in derived_modules {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("selected_source_module")),
            (
                "message",
                JsonValue::string(format!(
                    "Selected source `{}` derives `{module_name}`.",
                    source.path().as_str()
                )),
            ),
        ]));
    }
    diagnostic
}
