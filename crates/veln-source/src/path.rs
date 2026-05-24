#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath(String);

impl SourcePath {
    pub fn new(path: impl Into<String>) -> Self {
        let mut path = path.into().replace('\\', "/");
        while let Some(stripped) = path.strip_prefix("./") {
            path = stripped.to_string();
        }
        Self(path)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SourcePath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SourcePath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
