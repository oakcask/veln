use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use veln_analysis::CapturedDependencyProject;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DependencyRevisionKey([u8; 32]);

pub(crate) fn dependency_revision_key(
    dependencies: &[CapturedDependencyProject],
) -> DependencyRevisionKey {
    let mut hasher = Sha256::new();
    hasher.update(b"veln-mcp-dependency-revision/v1\0");
    hash_count(&mut hasher, dependencies.len());
    for dependency in dependencies {
        hash_field(&mut hasher, dependency.package.as_bytes());
        hash_field(&mut hasher, dependency.source.as_bytes());
        match &dependency.project {
            None => hasher.update([0]),
            Some(project) => {
                hasher.update([1]);
                match &project.manifest {
                    None => hasher.update([0]),
                    Some(manifest) => {
                        hasher.update([1]);
                        hash_field(&mut hasher, &manifest.source_bytes);
                    }
                }
                hash_count(&mut hasher, project.files.len());
                for file in &project.files {
                    hash_field(&mut hasher, file.path().as_str().as_bytes());
                    hash_field(&mut hasher, file.text().as_bytes());
                }
            }
        }
    }
    DependencyRevisionKey(hasher.finalize().into())
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hash_count(hasher, bytes.len());
    hasher.update(bytes);
}

fn hash_count(hasher: &mut Sha256, count: usize) {
    hasher.update(
        u64::try_from(count)
            .expect("captured dependency revision length fits u64")
            .to_be_bytes(),
    );
}

pub(crate) fn dependency_snapshot_key(snapshot: &CapturedDependencyProject) -> Value {
    let Some(project) = &snapshot.project else {
        return json!({
            "package": snapshot.package,
            "root": snapshot.source,
            "unavailable": true,
        });
    };
    json!({
        "package": snapshot.package,
        "root": snapshot.source,
        "manifest": project.manifest.as_ref().map(|manifest| &manifest.source_bytes),
        "files": project.files.iter().map(|file| {
            json!({"path": file.path().as_str(), "text": file.text()})
        }).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use veln_project::{Project, parse_manifest_text};
    use veln_source::SourceFile;

    #[test]
    fn dependency_revision_key_has_fixed_size() {
        assert_eq!(std::mem::size_of::<DependencyRevisionKey>(), 32);
    }

    #[test]
    fn dependency_revision_key_changes_with_captured_content() {
        let first = captured_dependency("first");
        let second = captured_dependency("second");

        assert_ne!(
            dependency_revision_key(&[first]),
            dependency_revision_key(&[second])
        );
    }

    fn captured_dependency(body: &str) -> CapturedDependencyProject {
        CapturedDependencyProject {
            package: "example/dep".to_string(),
            source: "vendor/dep".to_string(),
            project: Some(Project {
                root: PathBuf::new(),
                files: vec![SourceFile::new("api.veln", body)],
                manifest: Some(parse_manifest_text(
                    "veln.toml",
                    "[package]\nname = \"example/dep\"\n[lib]\nexports = [\"api.veln\"]\n",
                )),
            }),
        }
    }
}
