use std::io;
use std::path::PathBuf;

use veln_source::SourceFile;

use crate::{ProjectManifest, companion_analysis_inputs, manifest::read_manifest};

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
    pub manifest: Option<ProjectManifest>,
}

impl Project {
    pub fn discover(root: impl Into<PathBuf>, inputs: &[PathBuf]) -> io::Result<Self> {
        let root = root.into();
        let paths = companion_analysis_inputs(&root, inputs)?;
        let mut files = Vec::new();
        for path in paths {
            files.push(SourceFile::read(&root, &path)?);
        }
        let manifest = read_manifest(&root)?;
        Ok(Self {
            root,
            files,
            manifest,
        })
    }
}
