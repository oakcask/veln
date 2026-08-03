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
