use veln_project::Project;
use veln_source::SourceFile;

use super::load_surface_module;

#[test]
fn invalid_source_path_casing_quarantines_lowered_declarations() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("app.veln", "fn valid() -> Int\n  1\nend\n"),
            SourceFile::new(
                "Bad.veln",
                concat!(
                    "mod app\n",
                    "\n",
                    "pub fn leaked() -> Int\n",
                    "  1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"
                && diagnostic.message
                    == "module name `Bad` must start with an ASCII lowercase letter"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "module.source_mod"),
        "{diagnostics:#?}"
    );
    assert!(
        module
            .functions
            .iter()
            .any(|function| function.name.as_deref() == Some("valid"))
    );
    assert!(
        module
            .functions
            .iter()
            .all(|function| function.name.as_deref() != Some("leaked")),
        "{:#?}",
        module.functions
    );
}
