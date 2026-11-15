use serde_json::{Value, json};
use veln_analysis::CapturedDependencyProject;

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
