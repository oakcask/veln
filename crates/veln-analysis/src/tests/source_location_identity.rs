use super::*;
use veln_core::{CoreExprKind, CoreFunction, CoreStmtKind};

#[test]
fn generated_source_locations_are_identical_after_source_tree_relocation() {
    let first = TempProject::new("generated-source-location-first");
    let second = TempProject::new("generated-source-location-second");
    first.write("library.veln", "# logical library source\n");
    second.write("library.veln", "# logical library source\n");

    let first_location = generated_location(&first);
    let second_location = generated_location(&second);

    assert_eq!(first_location, second_location);
    assert_eq!(
        first_location[2],
        ("file", "library.veln#instrumentation-1.veln".to_string())
    );
    let rendered = format!("{first_location:?}{second_location:?}");
    assert!(!rendered.contains(&first.root().to_string_lossy().to_string()));
    assert!(!rendered.contains(&second.root().to_string_lossy().to_string()));
}

fn generated_location(project: &TempProject) -> Vec<(&'static str, String)> {
    let original = SourceFile::read(project.root(), &project.root().join("library.veln"))
        .expect("logical library source should load relative to its root");
    let virtual_path = SourcePath::virtual_source(original.path(), "instrumentation-1.veln")
        .expect("library source should produce a canonical virtual identity");
    let generated = SourceFile::generated(
        virtual_path,
        concat!(
            "fn capture() -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "\n",
            "pub fn main() -> SourceLocation\n",
            "  capture()\n",
            "end\n",
        ),
        Some(original.path().clone()),
    );
    let analysis = analyze_project(
        Project {
            root: project.root().to_path_buf(),
            files: vec![generated],
            manifest: None,
        },
        DoctestMode::Exclude,
    );
    let diagnostics = analysis.checked_diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);
    assert!(reachable.lowered.diagnostics.is_empty());
    let function = reachable
        .lowered
        .core
        .as_ref()
        .expect("generated source should lower to core")
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("generated entry should be reachable");
    source_location_fields(function)
}

#[test]
fn source_locations_are_identical_after_project_relocation() {
    let first = TempProject::new("source-location-relocation-first");
    let second = TempProject::new("source-location-relocation-second");
    write_collision_project(&first);
    write_collision_project(&second);

    let first_locations = dependency_locations(&first);
    let second_locations = dependency_locations(&second);

    assert_eq!(first_locations, second_locations);
    assert_eq!(first_locations[0][0].0, "package");
    assert_eq!(first_locations[0][0].1, "example/alpha");
    assert_eq!(first_locations[1][0].0, "package");
    assert_eq!(first_locations[1][0].1, "example/beta");
    assert_eq!(first_locations[0][1..], first_locations[1][1..]);
    assert_eq!(first_locations[0][1].0, "module");
    assert_eq!(first_locations[0][1].1, "shared");
    assert_eq!(first_locations[0][2].0, "file");
    assert_eq!(first_locations[0][2].1, "shared.veln");

    let rendered = format!("{first_locations:?}{second_locations:?}");
    assert!(!rendered.contains(&first.root().to_string_lossy().to_string()));
    assert!(!rendered.contains(&second.root().to_string_lossy().to_string()));
}

fn write_collision_project(project: &TempProject) {
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/alpha\"]\n",
            "path = \"vendor/alpha\"\n",
            "\n",
            "[dependencies.\"example/beta\"]\n",
            "path = \"vendor/beta\"\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use alpha_wrapper\n",
            "use beta_wrapper\n",
            "\n",
            "pub fn main() -> Int\n",
            "  let first = alpha_wrapper::from_alpha()\n",
            "  let second = beta_wrapper::from_beta()\n",
            "  first.start_line + second.start_line\n",
            "end\n",
        ),
    );
    project.write(
        "alpha_wrapper.veln",
        concat!(
            "use shared from \"example/alpha\"\n",
            "\n",
            "pub fn from_alpha() -> SourceLocation\n",
            "  shared::alpha_location()\n",
            "end\n",
        ),
    );
    project.write(
        "beta_wrapper.veln",
        concat!(
            "use shared from \"example/beta\"\n",
            "\n",
            "pub fn from_beta() -> SourceLocation\n",
            "  shared::beta__location()\n",
            "end\n",
        ),
    );
    write_dependency(project, "alpha");
    write_dependency(project, "beta");
}

fn write_dependency(project: &TempProject, name: &str) {
    project.write(
        &format!("vendor/{name}/veln.toml"),
        &format!("[package]\nname = \"example/{name}\"\n\n[lib]\nexports = [\"shared.veln\"]\n"),
    );
    project.write(
        &format!("vendor/{name}/shared.veln"),
        &if name == "alpha" {
            "fn capture_alpha() -> SourceLocation callsite\n  callsite\nend\n\npub fn alpha_location() -> SourceLocation\n  capture_alpha()\nend\n".to_string()
        } else {
            "fn capture_beta_() -> SourceLocation callsite\n  callsite\nend\n\npub fn beta__location() -> SourceLocation\n  capture_beta_()\nend\n".to_string()
        },
    );
}

fn dependency_locations(project: &TempProject) -> Vec<Vec<(&'static str, String)>> {
    let discovered = Project::discover(
        project.root().to_path_buf(),
        &[
            PathBuf::from("main.veln"),
            PathBuf::from("alpha_wrapper.veln"),
            PathBuf::from("beta_wrapper.veln"),
        ],
    )
    .expect("equivalent relocation project should be discovered");
    let analysis = analyze_project(discovered, DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);
    assert!(reachable.lowered.diagnostics.is_empty());
    let core = reachable
        .lowered
        .core
        .as_ref()
        .expect("relocation project should lower to core");

    ["alpha_location", "beta__location"]
        .into_iter()
        .map(|name| {
            let function = core
                .functions
                .iter()
                .find(|function| function.name == name)
                .expect("dependency location function should be reachable");
            source_location_fields(function)
        })
        .collect()
}

fn source_location_fields(function: &CoreFunction) -> Vec<(&'static str, String)> {
    let statement = function
        .body
        .statements
        .first()
        .expect("location function should contain its capture call");
    let expr = match &statement.kind {
        CoreStmtKind::Expr { expr } | CoreStmtKind::Return { expr } => expr,
        kind => panic!("location function should contain its capture expression: {kind:?}"),
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("location function should lower its capture call");
    };
    let CoreExprKind::Record(fields) = &args
        .last()
        .expect("capture call should receive hidden source location")
        .kind
    else {
        panic!("hidden source location should lower as a record");
    };
    fields
        .iter()
        .map(|field| {
            let value = match &field.expr.kind {
                CoreExprKind::StringLiteral(value) | CoreExprKind::IntLiteral(value) => {
                    value.clone()
                }
                kind => panic!("unexpected SourceLocation field value: {kind:?}"),
            };
            (source_location_field_name(&field.name), value)
        })
        .collect()
}

fn source_location_field_name(name: &str) -> &'static str {
    match name {
        "package" => "package",
        "module" => "module",
        "file" => "file",
        "start_line" => "start_line",
        "start_column" => "start_column",
        "start_offset" => "start_offset",
        "end_line" => "end_line",
        "end_column" => "end_column",
        "end_offset" => "end_offset",
        _ => panic!("unexpected SourceLocation field: {name}"),
    }
}
