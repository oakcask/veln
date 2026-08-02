use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::discover_source_paths;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectLockfile {
    pub packages: Vec<LockfilePackage>,
}

impl ProjectLockfile {
    pub fn render(&self) -> String {
        let mut packages = self.packages.clone();
        packages.sort_by(|left, right| left.name.cmp(&right.name));

        let mut out = String::new();
        for (index, package) in packages.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str("[[package]]\n");
            out.push_str(&format!(
                "name = \"{}\"\n",
                escape_toml_string(&package.name)
            ));
            out.push_str(&format!("source = {}\n", package.source.render()));
            out.push_str(&format!(
                "checksum = \"{}\"\n",
                escape_toml_string(&package.checksum)
            ));
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockfilePackage {
    pub name: String,
    pub source: LockfileSource,
    pub checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LockfileSource {
    Path {
        path: String,
    },
    Vendor {
        path: String,
    },
    Mirror {
        path: String,
    },
    Git {
        url: String,
        selector: LockfileGitSelector,
        rev: String,
        subdir: Option<String>,
    },
}

impl LockfileSource {
    fn render(&self) -> String {
        match self {
            Self::Path { path } => {
                format!(
                    "{{ kind = \"path\", path = \"{}\" }}",
                    escape_toml_string(path)
                )
            }
            Self::Vendor { path } => {
                format!(
                    "{{ kind = \"vendor\", path = \"{}\" }}",
                    escape_toml_string(path)
                )
            }
            Self::Mirror { path } => {
                format!(
                    "{{ kind = \"mirror\", path = \"{}\" }}",
                    escape_toml_string(path)
                )
            }
            Self::Git {
                url,
                selector,
                rev,
                subdir,
            } => {
                let mut fields = vec![
                    "kind = \"git\"".to_string(),
                    format!("url = \"{}\"", escape_toml_string(url)),
                    format!("selector = {}", selector.render()),
                    format!("rev = \"{}\"", escape_toml_string(rev)),
                ];
                if let Some(subdir) = subdir {
                    fields.push(format!("subdir = \"{}\"", escape_toml_string(subdir)));
                }
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LockfileGitSelector {
    Rev(String),
    Tag(String),
    Branch(String),
}

impl LockfileGitSelector {
    fn render(&self) -> String {
        match self {
            Self::Rev(value) => format!("{{ rev = \"{}\" }}", escape_toml_string(value)),
            Self::Tag(value) => format!("{{ tag = \"{}\" }}", escape_toml_string(value)),
            Self::Branch(value) => {
                format!("{{ branch = \"{}\" }}", escape_toml_string(value))
            }
        }
    }
}

pub fn source_tree_checksum(root: &Path) -> io::Result<String> {
    let paths = discover_source_paths(root, &[])?;
    let mut entries = paths
        .into_iter()
        .map(|path| (package_relative_path(root, &path), path))
        .collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut hash = Sha256::new();
    hash.update(b"veln-source-tree/v1\n");
    for (relative, path) in entries {
        let contents = fs::read(&path)?;
        hash.update((relative.len() as u64).to_be_bytes());
        hash.update(relative.as_bytes());
        hash.update((contents.len() as u64).to_be_bytes());
        hash.update(&contents);
    }
    let digest = hash.finalize();
    Ok(format!("sha256:{:x}", LowerHexBytes(&digest)))
}

pub struct LowerHexBytes<'a>(pub &'a [u8]);

impl fmt::LowerHex for LowerHexBytes<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

pub fn write_lockfile(root: &Path, lockfile: &ProjectLockfile) -> io::Result<()> {
    fs::write(root.join("veln.lock"), lockfile.render())
}

pub fn normalize_lockfile_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn package_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    path_to_lockfile_string(relative)
}

fn path_to_lockfile_string(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        parts.push(component.as_os_str().to_string_lossy());
    }
    parts.join("/")
}

fn escape_toml_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out
}
