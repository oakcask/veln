use std::fs;

#[allow(dead_code)]
#[path = "../build.rs"]
mod build_script;

#[test]
fn production_bundle_collector_excludes_test_filename_classes() {
    let root =
        std::env::temp_dir().join(format!("veln-stdlib-build-policy-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("nested")).expect("test source root should be created");
    fs::write(root.join("main.veln"), "pub fn main() -> ()\n  ()\nend\n")
        .expect("production source should be written");
    fs::write(
        root.join("main_test.veln"),
        "test integration() -> ()\nend\n",
    )
    .expect("integration test source should be written");
    fs::write(
        root.join("nested").join("main.test.veln"),
        "test companion() -> ()\nend\n",
    )
    .expect("companion test source should be written");

    let mut paths = Vec::new();
    build_script::collect_veln_sources(&root, root.as_path(), &mut paths);
    let _ = fs::remove_dir_all(&root);
    paths.sort();

    assert_eq!(paths, vec!["main.veln"]);
}

#[test]
fn production_distribution_policy_excludes_each_test_suffix() {
    assert!(build_script::is_distribution_source("main.veln"));
    assert!(!build_script::is_distribution_source("main_test.veln"));
    assert!(!build_script::is_distribution_source("main.test.veln"));
}

#[test]
fn generated_bundle_keeps_source_and_lowered_tables_in_sync() {
    let root = std::env::temp_dir().join(format!(
        "veln-stdlib-generated-bundle-{}",
        std::process::id()
    ));
    let (source_root, output) = write_bundle_fixture(&root);

    build_script::build_standard_library_bundle(&source_root, &output);

    assert_generated_tables(&output);
    assert_generated_lowered_module(&output);
    fs::remove_dir_all(&root).expect("test fixture should be removed");
}

fn write_bundle_fixture(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let source_root = root.join("veln");
    let output = root.join("out");
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(&source_root).expect("test source root should be created");
    fs::create_dir_all(&output).expect("test output directory should be created");
    fs::write(
        source_root.join("veln.toml"),
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"main.veln\"]\n",
    )
    .expect("test manifest should be written");
    fs::write(
        source_root.join("main.veln"),
        "pub fn main() -> ()\n  ()\nend\n",
    )
    .expect("test source should be written");
    (source_root, output)
}

fn assert_generated_tables(output: &std::path::Path) {
    let generated = fs::read_to_string(output.join("stdlib_bundle.rs"))
        .expect("generated bundle source should be readable");
    assert!(generated.contains("static EXPORTS: &[&str] = &[\n    \"main.veln\",\n];"));
    assert!(generated.contains("StdlibFile { path: \"main.veln\""));
    assert!(generated.contains("StdlibLoweredFile { path: \"main.veln\""));
}

fn assert_generated_lowered_module(output: &std::path::Path) {
    let encoded = fs::read(output.join("lowered/main.veln.bin"))
        .expect("generated lowered module should be readable");
    let decoded =
        veln_ast::decode_surface_module(&encoded).expect("generated lowered module should decode");
    assert_eq!(decoded.functions.len(), 1);
    assert_eq!(decoded.functions[0].name.as_deref(), Some("main"));
}
