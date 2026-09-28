use super::super::selection_plan;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use veln_test::expand_test_targets;

#[test]
fn explicit_standard_companion_preserves_full_package_analysis() {
    let project = TempProject::new("standard-companion-selection");
    project.write("veln.toml", "[package]\nname = \"std\"\n");
    project.write("math.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write(
        "math.test.veln",
        "use math\ntest value_is_one() -> Bool\n  math::value() == 1\nend\n",
    );
    let targets = vec![PathBuf::from("math.test.veln")];
    let expansion = expand_test_targets(&project.root, &targets);

    let plan = selection_plan(&project.root, &targets, true, &expansion)
        .expect("standard companion selection should succeed");

    assert!(
        plan.analysis_targets.is_empty(),
        "the standard package must retain full analysis"
    );
    assert_eq!(
        plan.selected_roots,
        Some(["math.test.veln".to_string()].into_iter().collect())
    );
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "veln-cli-selection-test-{name}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test project directory should be created");
        Self { root }
    }

    fn write(&self, path: &str, text: &str) {
        fs::write(self.root.join(path), text).expect("fixture should be written");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
