use std::io;
use std::path::PathBuf;

use veln_source::SourceFile;

use crate::discover_source_paths;

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
}

impl Project {
    pub fn discover(root: impl Into<PathBuf>, inputs: &[PathBuf]) -> io::Result<Self> {
        let root = root.into();
        let paths = discover_source_paths(&root, inputs)?;
        let mut files = Vec::new();
        for path in paths {
            files.push(SourceFile::read(&root, &path)?);
        }
        Ok(Self { root, files })
    }
}
