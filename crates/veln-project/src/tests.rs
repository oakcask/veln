use super::*;
use crate::manifest::read_manifest;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn keeps_explicit_files_sorted_and_unique() {
    let root = PathBuf::from(".");
    let paths = discover_source_paths(
        &root,
        &[
            PathBuf::from("b.veln"),
            PathBuf::from("a.veln"),
            PathBuf::from("a.veln"),
        ],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![PathBuf::from("./a.veln"), PathBuf::from("./b.veln")]
    );
}

#[test]
fn discovers_veln_files_recursively_and_skips_ignored_directories() {
    let temp = TempProject::new("recursive-discovery");
    temp.write("src/main.veln", "main");
    temp.write("src/nested/lib.veln", "lib");
    temp.write("src/readme.txt", "not source");
    temp.write("target/generated.veln", "ignored");
    temp.write(".git/hooks/hook.veln", "ignored");

    let paths = discover_source_paths(temp.root(), &[]).unwrap();

    assert_eq!(
        paths,
        vec![temp.path("src/main.veln"), temp.path("src/nested/lib.veln"),]
    );
}

#[test]
fn discovers_veln_files_from_explicit_directories() {
    let temp = TempProject::new("directory-input");
    temp.write("src/main.veln", "main");
    temp.write("tests/case.veln", "case");
    temp.write("tests/case.txt", "ignored");

    let paths = discover_source_paths(temp.root(), &[PathBuf::from("tests")]).unwrap();

    assert_eq!(paths, vec![temp.path("tests/case.veln")]);
}

#[test]
fn explicit_directories_skip_ignored_subdirectories() {
    let temp = TempProject::new("directory-input-ignored-subdirs");
    temp.write("tests/case.veln", "case");
    temp.write("tests/target/generated.veln", "ignored");
    temp.write("tests/.git/hooks/hook.veln", "ignored");

    let paths = discover_source_paths(temp.root(), &[PathBuf::from("tests")]).unwrap();

    assert_eq!(paths, vec![temp.path("tests/case.veln")]);
}

#[test]
fn deduplicates_overlapping_explicit_directory_inputs() {
    let temp = TempProject::new("overlapping-directory-inputs");
    temp.write("tests/unit/a.veln", "a");
    temp.write("tests/unit/b.veln", "b");
    temp.write("tests/integration/c.veln", "c");

    let paths = discover_source_paths(
        temp.root(),
        &[PathBuf::from("tests"), PathBuf::from("tests/unit")],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![
            temp.path("tests/integration/c.veln"),
            temp.path("tests/unit/a.veln"),
            temp.path("tests/unit/b.veln"),
        ]
    );
}

#[test]
fn discovers_veln_files_from_absolute_directory_inputs() {
    let temp = TempProject::new("absolute-directory-input");
    temp.write("src/main.veln", "main");
    temp.write("tests/case.veln", "case");
    temp.write("tests/case.txt", "ignored");

    let paths = discover_source_paths(temp.root(), &[temp.path("tests")]).unwrap();

    assert_eq!(paths, vec![temp.path("tests/case.veln")]);
}

#[test]
fn keeps_explicit_non_veln_files() {
    let temp = TempProject::new("explicit-non-veln");
    temp.write("notes.txt", "notes");

    let paths = discover_source_paths(temp.root(), &[PathBuf::from("notes.txt")]).unwrap();

    assert_eq!(paths, vec![temp.path("notes.txt")]);
}

#[test]
fn keeps_absolute_explicit_files_sorted_and_unique() {
    let temp = TempProject::new("absolute-file-input");
    temp.write("src/a.veln", "a");
    temp.write("src/b.veln", "b");

    let paths = discover_source_paths(
        temp.root(),
        &[
            temp.path("src/b.veln"),
            temp.path("src/a.veln"),
            temp.path("src/a.veln"),
        ],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![temp.path("src/a.veln"), temp.path("src/b.veln")]
    );
}

#[test]
fn deduplicates_mixed_relative_and_absolute_file_inputs() {
    let temp = TempProject::new("mixed-file-input");
    temp.write("src/a.veln", "a");
    temp.write("src/b.veln", "b");

    let paths = discover_source_paths(
        temp.root(),
        &[
            PathBuf::from("src/a.veln"),
            temp.path("src/a.veln"),
            PathBuf::from("src/b.veln"),
        ],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![temp.path("src/a.veln"), temp.path("src/b.veln")]
    );
}

#[test]
fn project_discover_reads_sources_with_project_relative_paths() {
    let temp = TempProject::new("project-discover");
    temp.write("src/b.veln", "second");
    temp.write("src/a.veln", "first");

    let project = Project::discover(temp.root().to_path_buf(), &[]).unwrap();

    assert_eq!(project.root, temp.root().to_path_buf());
    let files = project
        .files
        .iter()
        .map(|file| (file.path().as_str().to_string(), file.text().to_string()))
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        vec![
            ("src/a.veln".to_string(), "first".to_string()),
            ("src/b.veln".to_string(), "second".to_string()),
        ]
    );
}

#[test]
fn project_discover_reads_explicit_files_with_project_relative_paths() {
    let temp = TempProject::new("project-discover-explicit-files");
    temp.write("examples/b.veln", "second");
    temp.write("examples/a.veln", "first");

    let project = Project::discover(
        temp.root().to_path_buf(),
        &[
            temp.path("examples/b.veln"),
            PathBuf::from("examples/a.veln"),
        ],
    )
    .unwrap();

    let files = project
        .files
        .iter()
        .map(|file| (file.path().as_str().to_string(), file.text().to_string()))
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        vec![
            ("examples/a.veln".to_string(), "first".to_string()),
            ("examples/b.veln".to_string(), "second".to_string()),
        ]
    );
}

#[test]
fn project_discover_reads_manifest_with_explicit_inputs() {
    let temp = TempProject::new("project-discover-explicit-manifest");
    temp.write("src/main.veln", "mod app.main\n");
    temp.write("src/extra.veln", "mod app.extra\n");
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

    let project = Project::discover(
        temp.root().to_path_buf(),
        &[PathBuf::from("src/extra.veln")],
    )
    .unwrap();

    let files = project
        .files
        .iter()
        .map(|file| file.path().as_str().to_string())
        .collect::<Vec<_>>();
    let manifest = project.manifest.expect("manifest should be loaded");
    assert_eq!(files, vec!["src/extra.veln".to_string()]);
    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
}

#[test]
fn project_discover_reports_missing_explicit_file() {
    let temp = TempProject::new("project-discover-missing-explicit-file");

    let error = Project::discover(temp.root().to_path_buf(), &[PathBuf::from("missing.veln")])
        .expect_err("missing explicit file should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn project_discover_reports_missing_absolute_explicit_file() {
    let temp = TempProject::new("project-discover-missing-absolute-explicit-file");

    let error = Project::discover(temp.root().to_path_buf(), &[temp.path("missing.veln")])
        .expect_err("missing explicit file should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn project_discover_reads_manifest_lib_exports() {
    let temp = TempProject::new("manifest-lib-exports");
    temp.write("src/main.veln", "mod app.main\n");
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

    let project = Project::discover(temp.root().to_path_buf(), &[]).unwrap();
    let manifest = project.manifest.expect("manifest should be loaded");

    assert_eq!(manifest.path.as_str(), "veln.toml");
    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 2);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 13);
}

#[test]
fn read_manifest_returns_none_when_manifest_is_absent() {
    let temp = TempProject::new("manifest-absent");

    let manifest = read_manifest(temp.root()).unwrap();

    assert!(manifest.is_none());
}

#[test]
fn read_manifest_tracks_lib_exports_and_ignores_other_sections() {
    let temp = TempProject::new("manifest-lib-sections");
    temp.write(
        "veln.toml",
        concat!(
            "[package]\n",
            "\"ignored.veln\" = \"ignored.module\"\n",
            "[lib]\n",
            "# comment\n",
            "not-an-entry\n",
            "exports = [\"src/main.veln\"]\n",
            "[other]\n",
            "\"ignored-again.veln\" = \"ignored.again\"\n",
            "[lib]\n",
            "exports = [\n",
            "  \"src/lib.veln\",\n",
            "]\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.path.as_str(), "veln.toml");
    assert_eq!(manifest.lib.exports.len(), 2);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 6);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 13);
    assert_eq!(manifest.lib.exports[1].path, "src/lib.veln");
    assert_eq!(manifest.lib.exports[1].path_span.start.line, 11);
    assert_eq!(manifest.lib.exports[1].path_span.start.column, 4);
}

#[test]
fn read_manifest_tracks_package_and_tool_string_fields() {
    let temp = TempProject::new("manifest-package-tool-fields");
    temp.write(
        "veln.toml",
        concat!(
            "[package]\n",
            "name = \"demo\"\n",
            "version = \"0.1.0\"\n",
            "ignored = 1\n",
            "[tool.docs]\n",
            "template = \"reference\"\n",
            "[tool.docs]\n",
            "output = \"docs/api.md\"\n",
            "[lib]\n",
            "exports = [\"src/main.veln\"]\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.package.fields.len(), 2);
    assert_eq!(manifest.package.fields[0].key, "name");
    assert_eq!(manifest.package.fields[0].value, "demo");
    assert_eq!(manifest.package.fields[0].key_span.start.line, 2);
    assert_eq!(manifest.package.fields[0].value_span.start.column, 9);
    assert_eq!(manifest.package.fields[1].key, "version");
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "docs");
    assert_eq!(manifest.tools[0].fields.len(), 2);
    assert_eq!(manifest.tools[0].fields[0].key, "template");
    assert_eq!(manifest.tools[0].fields[0].value, "reference");
    assert_eq!(manifest.tools[0].fields[1].key, "output");
    assert_eq!(manifest.tools[0].fields[1].value, "docs/api.md");
    assert_eq!(manifest.lib.exports.len(), 1);
}

#[test]
fn read_manifest_tracks_path_dependencies() {
    let temp = TempProject::new("manifest-path-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "path = \"vendor/foo\"\n",
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"https://example.invalid/bar.git\"\n",
            "path = \"vendor/bar\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.dependencies.len(), 2);
    assert_eq!(manifest.dependencies[0].package, "github.com/oakcask/foo");
    assert_eq!(manifest.dependencies[0].package_span.start.line, 1);
    assert_eq!(manifest.dependencies[0].package_span.start.column, 16);
    let foo_path = manifest.dependencies[0]
        .path
        .as_ref()
        .expect("foo dependency should have a path");
    assert_eq!(foo_path.key, "path");
    assert_eq!(foo_path.value, "vendor/foo");
    assert_eq!(foo_path.value_span.start.line, 2);
    assert_eq!(foo_path.value_span.start.column, 9);
    assert_eq!(manifest.dependencies[1].package, "github.com/oakcask/bar");
    assert_eq!(
        manifest.dependencies[1]
            .path
            .as_ref()
            .expect("bar dependency should have a path")
            .value,
        "vendor/bar"
    );
}

#[test]
fn read_manifest_accepts_crlf_export_arrays_and_trailing_text() {
    let temp = TempProject::new("manifest-export-crlf");
    temp.write(
        "veln.toml",
        "[lib]\r\n  exports = [\"src/main.veln\"] # owner note\r\n",
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 2);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 15);
}

#[test]
fn read_manifest_accepts_final_export_without_newline() {
    let temp = TempProject::new("manifest-final-export");
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 2);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 13);
}

#[test]
fn read_manifest_tracks_export_path_span_ends() {
    let temp = TempProject::new("manifest-export-span-ends");
    temp.write("veln.toml", "[lib]\n  exports = [\"src/main.veln\"]\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let export = &manifest.lib.exports[0];
    assert_eq!(export.path_span.start.line, 2);
    assert_eq!(export.path_span.start.column, 15);
    assert_eq!(export.path_span.end.line, 2);
    assert_eq!(export.path_span.end.column, 28);
}

#[test]
fn read_manifest_tracks_empty_export_path_span() {
    let temp = TempProject::new("manifest-empty-export-path");
    temp.write("veln.toml", "[lib]\nexports = [\"\"]\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let export = &manifest.lib.exports[0];
    assert_eq!(export.path, "");
    assert_eq!(export.path_span.start.line, 2);
    assert_eq!(export.path_span.start.column, 13);
    assert_eq!(export.path_span.end.line, 2);
    assert_eq!(export.path_span.end.column, 13);
}

#[test]
fn read_manifest_accepts_empty_exports_array() {
    let temp = TempProject::new("manifest-empty-exports");
    temp.write("veln.toml", "[lib]\nexports = []\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.lib.exports.is_empty());
}

#[test]
fn read_manifest_tracks_modules_as_unsupported_section() {
    let temp = TempProject::new("manifest-unsupported-modules");
    temp.write("veln.toml", "[modules]\n\"main.veln\" = \"app.main\"\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.lib.exports.is_empty());
    assert_eq!(manifest.unsupported_sections.len(), 1);
    assert_eq!(manifest.unsupported_sections[0].name, "modules");
    assert_eq!(manifest.unsupported_sections[0].span.start.line, 1);
    assert_eq!(manifest.unsupported_sections[0].span.start.column, 2);
}

#[test]
fn read_manifest_accepts_modules_header_without_entries_as_unsupported() {
    let temp = TempProject::new("manifest-empty-unsupported-modules");
    temp.write("veln.toml", "[modules]");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.lib.exports.is_empty());
    assert_eq!(manifest.unsupported_sections.len(), 1);
}

#[test]
fn read_manifest_accepts_trailing_text_after_section_headers() {
    let temp = TempProject::new("manifest-section-header-trailing-text");
    temp.write(
        "veln.toml",
        concat!(
            "[package] # ignored section\n",
            "\"ignored.veln\" = \"ignored.module\"\n",
            "[lib] # source exports\n",
            "exports = [\"src/main.veln\"]\n",
            "[other] # ignored again\n",
            "\"ignored-again.veln\" = \"ignored.again\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 4);
}

#[test]
fn read_manifest_ignores_malformed_export_entries() {
    let temp = TempProject::new("manifest-malformed-exports");
    temp.write(
        "veln.toml",
        concat!(
            "[lib]\n",
            "exports = [\n",
            "src/main.veln,\n",
            "\"src/unclosed.veln,\n",
            "\"src/lib.veln\",\n",
            "]\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/lib.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 5);
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-project-test-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.path(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
