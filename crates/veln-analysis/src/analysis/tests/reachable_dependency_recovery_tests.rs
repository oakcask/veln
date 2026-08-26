use veln_project::{Project, parse_manifest_text};
use veln_source::SourceFile;

use crate::surface::{CapturedDependencyProject, load_surface_modules_with_captured_dependencies};

fn project_with_dependency(
    source: &str,
    dependency_source: &str,
) -> (Project, Vec<CapturedDependencyProject>) {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new("main.veln", source)],
        manifest: Some(parse_manifest_text(
            "veln.toml",
            concat!(
                "[dependencies.\"example/lib\"]\n",
                "path = \"vendor/lib\"\n",
            ),
        )),
    };
    let dependency_project = Project {
        root: "vendor/lib".into(),
        files: vec![SourceFile::new("math.veln", dependency_source)],
        manifest: Some(parse_manifest_text(
            "vendor/lib/veln.toml",
            concat!(
                "[package]\n",
                "name = \"example/lib\"\n",
                "\n",
                "[lib]\n",
                "exports = [\n",
                "  \"math.veln\",\n",
                "]\n",
            ),
        )),
    };
    let dependencies = vec![CapturedDependencyProject {
        package: "example/lib".to_string(),
        source: "vendor/lib".to_string(),
        project: Some(dependency_project),
    }];
    (project, dependencies)
}

#[test]
fn dependency_recovery_record_does_not_cross_into_consumer() {
    let (project, dependencies) = project_with_dependency(
        concat!(
            "use math from \"example/lib\"\n",
            "\n",
            "fn main() -> Int\n",
            "  math::Bad()\n",
            "end\n",
        ),
        concat!(
            "pub fn good() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub fn Bad() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let (_loaded, diagnostics) =
        load_surface_modules_with_captured_dependencies(&project, &dependencies);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let analysis = super::super::analyze_project_with_captured_dependencies(
        project,
        super::super::DoctestMode::Exclude,
        &dependencies,
    );
    let reachable = analysis.lower_reachable_entry("main", veln_ast::FunctionKind::Function);

    assert!(
        reachable
            .module
            .invalid_names
            .iter()
            .all(|invalid| invalid.name != "Bad"),
        "{:#?}",
        reachable.module.invalid_names
    );
    assert!(
        reachable
            .lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `math::Bad`"),
        "{:#?}",
        reachable.lowered.diagnostics
    );
}

#[test]
fn consumer_recovery_record_does_not_cross_into_dependency() {
    let (project, dependencies) = project_with_dependency(
        concat!(
            "use math from \"example/lib\"\n",
            "\n",
            "fn main() -> Int\n",
            "  math::good()\n",
            "end\n",
            "\n",
            "fn Bad() -> Int\n",
            "  2\n",
            "end\n",
        ),
        concat!("pub fn good() -> Int\n", "  Bad()\n", "end\n",),
    );
    let (_loaded, diagnostics) =
        load_surface_modules_with_captured_dependencies(&project, &dependencies);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let analysis = super::super::analyze_project_with_captured_dependencies(
        project,
        super::super::DoctestMode::Exclude,
        &dependencies,
    );
    let reachable = analysis.lower_reachable_entry("main", veln_ast::FunctionKind::Function);
    let invalid_names = reachable
        .module
        .invalid_names
        .iter()
        .map(|invalid| (invalid.span.file.as_str(), invalid.name.as_str()))
        .collect::<Vec<_>>();

    assert!(
        invalid_names.is_empty(),
        "{:#?}",
        reachable.module.invalid_names
    );
    assert!(
        reachable
            .lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `Bad`"),
        "{:#?}",
        reachable.lowered.diagnostics
    );
}
