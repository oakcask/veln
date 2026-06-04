#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectLockfile {
    pub packages: Vec<LockfilePackage>,
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
    Git {
        url: String,
        selector: LockfileGitSelector,
        rev: String,
        subdir: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LockfileGitSelector {
    Rev(String),
    Tag(String),
    Branch(String),
}
