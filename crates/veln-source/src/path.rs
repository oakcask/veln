use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath(Arc<str>);

impl SourcePath {
    pub fn new(path: impl Into<String>) -> Self {
        let mut path = path.into().replace('\\', "/");
        while let Some(stripped) = path.strip_prefix("./") {
            path = stripped.to_string();
        }
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn virtual_source(
        source: &SourcePath,
        identity: impl AsRef<str>,
    ) -> Result<Self, VirtualSourcePathError> {
        validate_relative_source_path(source.as_str())?;
        let identity = identity.as_ref();
        if identity.is_empty()
            || identity == "."
            || identity == ".."
            || identity.contains(['/', '\\', '#', ':'])
        {
            return Err(VirtualSourcePathError::InvalidIdentity);
        }
        Ok(Self::new(format!("{}#{identity}", source.as_str())))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualSourcePathError {
    SourceMustBeRelative,
    SourceMustBeCanonical,
    InvalidIdentity,
}

impl fmt::Display for VirtualSourcePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SourceMustBeRelative => "virtual source origin must be package-relative",
            Self::SourceMustBeCanonical => {
                "virtual source origin must use canonical relative segments"
            }
            Self::InvalidIdentity => "virtual source identity must be one non-empty path segment",
        };
        formatter.write_str(message)
    }
}

impl Error for VirtualSourcePathError {}

fn validate_relative_source_path(path: &str) -> Result<(), VirtualSourcePathError> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized.as_bytes().get(1) == Some(&b':')
    {
        return Err(VirtualSourcePathError::SourceMustBeRelative);
    }
    if normalized.is_empty()
        || normalized.contains(':')
        || normalized.contains('#')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(VirtualSourcePathError::SourceMustBeCanonical);
    }
    Ok(())
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
