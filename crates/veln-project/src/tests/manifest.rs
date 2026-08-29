use super::*;

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
fn read_manifest_tracks_git_dependency_metadata() {
    let temp = TempProject::new("manifest-git-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "git = \"https://example.invalid/foo.git\"\n",
            "tag = \"v1.2.0\"\n",
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "branch = \"main\"\n",
            "subdir = \"packages/bar\"\n",
            "[dependencies.\"github.com/oakcask/baz\"]\n",
            "git = \"https://example.invalid/baz.git\"\n",
            "rev = \"0123456789abcdef\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let foo = &manifest.dependencies[0];
    assert_eq!(foo.package, "github.com/oakcask/foo");
    assert_eq!(
        foo.git
            .as_ref()
            .expect("foo should have a git source")
            .value,
        "https://example.invalid/foo.git"
    );
    assert_eq!(foo.selectors.len(), 1);
    assert_eq!(foo.selectors[0].kind, ManifestDependencySelectorKind::Tag);
    assert_eq!(foo.selectors[0].field.value, "v1.2.0");
    assert_eq!(foo.selectors[0].field.key_span.start.line, 3);
    assert!(foo.subdir.is_none());

    let bar = &manifest.dependencies[1];
    assert_eq!(
        bar.selectors[0].kind,
        ManifestDependencySelectorKind::Branch
    );
    assert_eq!(bar.selectors[0].field.value, "main");
    assert_eq!(
        bar.subdir.as_ref().expect("bar should have a subdir").value,
        "packages/bar"
    );

    let baz = &manifest.dependencies[2];
    assert_eq!(baz.selectors[0].kind, ManifestDependencySelectorKind::Rev);
    assert_eq!(baz.selectors[0].field.value, "0123456789abcdef");
}

#[test]
fn read_manifest_tracks_vendor_dependency_metadata() {
    let temp = TempProject::new("manifest-vendor-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/vendor-lib\"]\n",
            "vendor = \"vendor/vendor-lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let dependency = &manifest.dependencies[0];
    assert_eq!(dependency.package, "github.com/oakcask/vendor-lib");
    let vendor = dependency
        .vendor
        .as_ref()
        .expect("dependency should have a vendor source");
    assert_eq!(vendor.key, "vendor");
    assert_eq!(vendor.value, "vendor/vendor-lib");
    assert_eq!(vendor.value_span.start.line, 2);
    assert_eq!(vendor.value_span.start.column, 11);
}

#[test]
fn read_manifest_tracks_mirror_dependency_metadata() {
    let temp = TempProject::new("manifest-mirror-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/mirror-lib\"]\n",
            "mirror = \"mirror/github.com/oakcask/mirror-lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let dependency = &manifest.dependencies[0];
    assert_eq!(dependency.package, "github.com/oakcask/mirror-lib");
    let mirror = dependency
        .mirror
        .as_ref()
        .expect("dependency should have a mirror source");
    assert_eq!(mirror.key, "mirror");
    assert_eq!(mirror.value, "mirror/github.com/oakcask/mirror-lib");
    assert_eq!(mirror.key_span.start.line, 2);
    assert_eq!(mirror.value_span.start.column, 11);
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
