use super::*;
use crate::check_project::capture::NavigationScope;
use veln_diagnostics::Diagnostic;
use veln_project::parse_manifest_text;
use veln_source::SourceFile;

#[test]
fn diagnostic_json_preserves_invalid_standard_symbol_case_boundary() {
    let diagnostic = invalid_standard_symbol_case_diagnostic();
    let converted = diagnostic_to_serde(&diagnostic);

    assert_eq!(
        converted,
        json!({
            "id": "toolchain.invalid_symbol_case",
            "severity": "error",
            "kind": "toolchain",
            "message": "compiler-provided function `BadAdapter` from `compiler_adapter` must start with an ASCII lowercase letter",
            "span": null,
            "details": {
                "provider": "compiler_adapter",
                "name": "BadAdapter",
                "name_class": "function",
                "required_initial": "ascii_lowercase"
            },
            "related": []
        })
    );
    assert_ne!(converted["id"], json!("name.invalid_case"));
}

#[test]
fn stable_capture_retries_manifest_source_and_path_set_changes_only_three_times() {
    let cases = [
        (
            "manifest",
            alternating_captures(
                || captured_project(vec![("main.veln", clean_source())], Some("name = \"a\"\n")),
                || captured_project(vec![("main.veln", clean_source())], Some("name = \"b\"\n")),
            ),
        ),
        (
            "source",
            alternating_captures(
                || captured_project(vec![("main.veln", "fn main() -> Int\n  1\nend\n")], None),
                || captured_project(vec![("main.veln", "fn main() -> Int\n  2\nend\n")], None),
            ),
        ),
        (
            "path set",
            alternating_captures(
                || captured_project(vec![("a.veln", clean_source())], None),
                || {
                    captured_project(
                        vec![("a.veln", clean_source()), ("b.veln", clean_source())],
                        None,
                    )
                },
            ),
        ),
        (
            "locally materialized git dependency",
            alternating_captures(
                || {
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", clean_source())],
                            Some("name = \"dep\"\n"),
                        )],
                    )
                },
                || {
                    captured_project_with_dependencies(
                        vec![("main.veln", clean_source())],
                        Some(git_dependency_manifest()),
                        vec![captured_dependency(
                            "dep",
                            "https://example.invalid/dep.git",
                            vec![("lib.veln", "fn answer() -> Int\n  2\nend\n")],
                            Some("name = \"dep\"\n"),
                        )],
                    )
                },
            ),
        ),
    ];

    for (name, captures) in cases {
        let mut captures = captures.into_iter();
        let result = capture_stable_project_with(|| Ok(captures.next().unwrap()));
        assert!(matches!(result, Err(CaptureError::Changed)), "{name}");
        assert!(captures.next().is_none(), "{name}");
    }
}

#[test]
fn navigation_capture_retries_descendant_boundary_changes_as_one_attempt() {
    let mut captures = alternating_captures(
        || {
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"a\"\n"),
            )
        },
        || {
            captured_navigation_source(
                captured_project(vec![("main.veln", clean_source())], Some("")),
                "nested/main.veln",
                navigation_boundary_key("name = \"b\"\n"),
            )
        },
    )
    .into_iter();

    let result = capture_stable_navigation_source_with(|| Ok(captures.next().unwrap()));

    assert!(matches!(result, Err(CaptureError::Changed)));
    assert!(captures.next().is_none());
}

fn alternating_captures<T>(mut first: impl FnMut() -> T, mut second: impl FnMut() -> T) -> Vec<T> {
    (0..SNAPSHOT_ATTEMPTS)
        .flat_map(|_| [first(), second()])
        .collect()
}

fn navigation_boundary_key(boundary_text: &str) -> Value {
    json!({
        "mode": "single_file",
        "source": "nested/main.veln",
        "inspected_project": {
            "root": ".",
            "project": {
                "boundary_manifests": [
                    {"path": "nested/veln.toml", "text": boundary_text}
                ]
            }
        },
        "project": {"files": [{"path": "nested/main.veln", "text": clean_source()}]}
    })
}

#[test]
fn captured_direct_local_dependencies_feed_successful_analysis() {
    let captured = captured_project_with_dependencies(
        vec![(
            "main.veln",
            concat!(
                "use foo from \"github.com/oakcask/foo\"\n\n",
                "pub fn main() -> Int\n",
                "  add_one(1)\n",
                "end\n",
            ),
        )],
        Some("[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n"),
        vec![captured_dependency(
            "github.com/oakcask/foo",
            "vendor/foo",
            vec![(
                "foo.veln",
                "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
            )],
            Some(concat!(
                "[package]\n",
                "name = \"github.com/oakcask/foo\"\n\n",
                "[lib]\n",
                "exports = [\"foo.veln\"]\n",
            )),
        )],
    );

    let diagnostics = checked_project_diagnostics_with_captured_dependencies(
        captured.project,
        DoctestMode::Exclude,
        captured.dependencies,
    );

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

fn captured_project(files: Vec<(&str, &str)>, manifest: Option<&str>) -> CapturedProject {
    captured_project_with_dependencies(files, manifest, Vec::new())
}

fn captured_navigation_source(
    project: CapturedProject,
    source: &str,
    key: Value,
) -> CapturedNavigationSource {
    CapturedNavigationSource {
        project,
        source: source.to_string(),
        scope: NavigationScope::SingleFile {
            project: ".".to_string(),
            source: source.to_string(),
        },
        key,
    }
}

fn captured_project_with_dependencies(
    files: Vec<(&str, &str)>,
    manifest: Option<&str>,
    dependencies: Vec<CapturedDependencyProject>,
) -> CapturedProject {
    let project = Project {
        root: PathBuf::from("."),
        files: files
            .into_iter()
            .map(|(path, text)| SourceFile::new(path, text))
            .collect(),
        manifest: manifest.map(|text| parse_manifest_text("veln.toml", text)),
    };
    let key = synthetic_snapshot_key(&project, &dependencies);
    CapturedProject {
        project,
        dependencies,
        key,
    }
}

fn synthetic_snapshot_key(project: &Project, dependencies: &[CapturedDependencyProject]) -> Value {
    json!({
        "manifest": project.manifest.as_ref().map(|manifest| &manifest.source_bytes),
        "files": project.files.iter().map(|file| {
            json!({"path": file.path().as_str(), "text": file.text()})
        }).collect::<Vec<_>>(),
        "boundary_manifests": [],
        "dependencies": dependencies.iter().map(dependency_snapshot_key).collect::<Vec<_>>(),
    })
}

fn captured_dependency(
    package: &str,
    source: &str,
    files: Vec<(&str, &str)>,
    manifest: Option<&str>,
) -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: package.to_string(),
        source: source.to_string(),
        project: Some(Project {
            root: PathBuf::from(source),
            files: files
                .into_iter()
                .map(|(path, text)| SourceFile::new(path, text))
                .collect(),
            manifest: manifest.map(|text| parse_manifest_text("veln.toml", text)),
        }),
    }
}

fn git_dependency_manifest() -> &'static str {
    "[dependencies.dep]\ngit = \"https://example.invalid/dep.git\"\nrev = \"abc123\"\n"
}

fn clean_source() -> &'static str {
    "fn main() -> Int\n  1\nend\n"
}

fn invalid_standard_symbol_case_diagnostic() -> Diagnostic {
    veln_diagnostics::toolchain_invalid_symbol_case_diagnostic(
        "compiler_adapter",
        "BadAdapter",
        veln_diagnostics::ToolchainSymbolNameClass::Function,
        veln_diagnostics::ToolchainSymbolNameFailureReason::InvalidCase,
    )
}
