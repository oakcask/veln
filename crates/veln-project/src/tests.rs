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
fn project_discover_reads_manifest_module_entries() {
    let temp = TempProject::new("manifest-modules");
    temp.write("src/main.veln", "mod app.main\n");
    temp.write("veln.toml", "[modules]\n\"src/main.veln\" = \"app.main\"\n");

    let project = Project::discover(temp.root().to_path_buf(), &[]).unwrap();
    let manifest = project.manifest.expect("manifest should be loaded");

    assert_eq!(manifest.path.as_str(), "veln.toml");
    assert_eq!(manifest.modules.len(), 1);
    assert_eq!(manifest.modules[0].path, "src/main.veln");
    assert_eq!(manifest.modules[0].name, "app.main");
    assert_eq!(manifest.modules[0].path_span.start.line, 2);
    assert_eq!(manifest.modules[0].name_span.start.column, 20);
}

#[test]
fn read_manifest_returns_none_when_manifest_is_absent() {
    let temp = TempProject::new("manifest-absent");

    let manifest = read_manifest(temp.root()).unwrap();

    assert!(manifest.is_none());
}

#[test]
fn read_manifest_tracks_modules_sections_and_ignores_non_entries() {
    let temp = TempProject::new("manifest-sections");
    temp.write(
        "veln.toml",
        concat!(
            "[package]\n",
            "\"ignored.veln\" = \"ignored.module\"\n",
            "[modules]\n",
            "# comment\n",
            "not-an-entry\n",
            "\"src/main.veln\" = \"app.main\"\n",
            "[other]\n",
            "\"ignored-again.veln\" = \"ignored.again\"\n",
            "[modules]\n",
            "  \"src/lib.veln\"   =   \"app.lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.path.as_str(), "veln.toml");
    assert_eq!(manifest.modules.len(), 2);
    assert_eq!(manifest.modules[0].path, "src/main.veln");
    assert_eq!(manifest.modules[0].name, "app.main");
    assert_eq!(manifest.modules[0].path_span.start.line, 6);
    assert_eq!(manifest.modules[0].path_span.start.column, 2);
    assert_eq!(manifest.modules[0].name_span.start.column, 20);
    assert_eq!(manifest.modules[1].path, "src/lib.veln");
    assert_eq!(manifest.modules[1].name, "app.lib");
    assert_eq!(manifest.modules[1].path_span.start.line, 10);
    assert_eq!(manifest.modules[1].path_span.start.column, 4);
    assert_eq!(manifest.modules[1].name_span.start.column, 25);
}

#[test]
fn read_manifest_accepts_crlf_lines_and_trailing_entry_text() {
    let temp = TempProject::new("manifest-crlf");
    temp.write(
        "veln.toml",
        "[modules]\r\n  \"src/main.veln\" = \"app.main\" # owner note\r\n",
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.modules.len(), 1);
    assert_eq!(manifest.modules[0].path, "src/main.veln");
    assert_eq!(manifest.modules[0].name, "app.main");
    assert_eq!(manifest.modules[0].path_span.start.line, 2);
    assert_eq!(manifest.modules[0].path_span.start.column, 4);
    assert_eq!(manifest.modules[0].name_span.start.line, 2);
    assert_eq!(manifest.modules[0].name_span.start.column, 22);
}

#[test]
fn read_manifest_accepts_final_entry_without_newline() {
    let temp = TempProject::new("manifest-final-entry");
    temp.write("veln.toml", "[modules]\n\"src/main.veln\" = \"app.main\"");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.modules.len(), 1);
    assert_eq!(manifest.modules[0].path, "src/main.veln");
    assert_eq!(manifest.modules[0].name, "app.main");
    assert_eq!(manifest.modules[0].path_span.start.line, 2);
    assert_eq!(manifest.modules[0].name_span.start.column, 20);
}

#[test]
fn read_manifest_accepts_modules_header_without_entries() {
    let temp = TempProject::new("manifest-empty-modules");
    temp.write("veln.toml", "[modules]");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.modules.is_empty());
}

#[test]
fn read_manifest_accepts_trailing_text_after_section_headers() {
    let temp = TempProject::new("manifest-section-header-trailing-text");
    temp.write(
        "veln.toml",
        concat!(
            "[package] # ignored section\n",
            "\"ignored.veln\" = \"ignored.module\"\n",
            "[modules] # source modules\n",
            "\"src/main.veln\" = \"app.main\"\n",
            "[other] # ignored again\n",
            "\"ignored-again.veln\" = \"ignored.again\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.modules.len(), 1);
    assert_eq!(manifest.modules[0].path, "src/main.veln");
    assert_eq!(manifest.modules[0].name, "app.main");
    assert_eq!(manifest.modules[0].path_span.start.line, 4);
    assert_eq!(manifest.modules[0].name_span.start.column, 20);
}

#[test]
fn read_manifest_ignores_malformed_module_entries() {
    let temp = TempProject::new("manifest-malformed-entries");
    temp.write(
        "veln.toml",
        concat!(
            "[modules]\n",
            "src/main.veln = \"app.main\"\n",
            "\"src/missing-equals.veln\" \"app.missing_equals\"\n",
            "\"src/missing-name.veln\" = app.missing_name\n",
            "\"src/unclosed.veln = \"app.unclosed\"\n",
            "\"src/lib.veln\" = \"app.lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.modules.len(), 1);
    assert_eq!(manifest.modules[0].path, "src/lib.veln");
    assert_eq!(manifest.modules[0].name, "app.lib");
    assert_eq!(manifest.modules[0].path_span.start.line, 6);
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
