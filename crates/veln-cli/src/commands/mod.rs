pub(crate) mod check;
pub(crate) mod doc;
pub(crate) mod explain;
pub(crate) mod fmt;
pub(crate) mod metrics;
pub(crate) mod package;
pub(crate) mod repair;
pub(crate) mod run;
mod run_report;
pub(crate) mod test;
pub(crate) mod test_scheduler;

use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct CommandAnalysisStart {
    invocation_dir: PathBuf,
    pub(crate) package_root: PathBuf,
}

impl CommandAnalysisStart {
    pub(crate) fn select() -> Result<Self, String> {
        let invocation_dir = env::current_dir()
            .and_then(|dir| dir.canonicalize())
            .map_err(|error| error.to_string())?;
        let package_root = veln_project::select_package_root(&invocation_dir)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            invocation_dir,
            package_root,
        })
    }

    pub(crate) fn resolve_inputs(&self, inputs: Vec<PathBuf>) -> Vec<PathBuf> {
        if self.invocation_dir == self.package_root {
            return inputs;
        }
        inputs
            .into_iter()
            .map(|input| {
                if input.is_absolute() {
                    input
                } else {
                    self.invocation_dir.join(input)
                }
            })
            .collect()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn relative_inputs_resolve_in_selected_root_path_form() {
        let temp = TempProject::new("command-analysis-start-path-form");
        temp.write("veln.toml", "[package]\nname = \"path_form\"\n");
        temp.write("src/main.veln", "fn main() -> i32 = 1\n");

        let start = CommandAnalysisStart {
            invocation_dir: temp.path("src").canonicalize().unwrap(),
            package_root: temp.root().canonicalize().unwrap(),
        };

        let inputs = start.resolve_inputs(vec![PathBuf::from("main.veln")]);
        assert_eq!(inputs.len(), 1);
        assert!(
            inputs[0].strip_prefix(&start.package_root).is_ok(),
            "resolved input `{}` should share the package root path form `{}`",
            inputs[0].display(),
            start.package_root.display()
        );
    }

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root =
                env::temp_dir().join(format!("veln-cli-{name}-{}-{nanos}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn root(&self) -> &PathBuf {
            &self.root
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.path(relative);
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
}
