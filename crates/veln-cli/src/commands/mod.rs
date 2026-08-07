pub(crate) mod check;
pub(crate) mod doc;
pub(crate) mod explain;
pub(crate) mod fmt;
pub(crate) mod metrics;
pub(crate) mod package;
pub(crate) mod repair;
pub(crate) mod run;
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
        let invocation_dir = env::current_dir().map_err(|error| error.to_string())?;
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
