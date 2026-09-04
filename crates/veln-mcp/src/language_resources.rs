use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use veln_analysis::CapturedDependencyProject;
use veln_language_service::{DirectDependencySnapshot, VirtualSourceCatalog};
use veln_project::{
    CapturedPackageSnapshot, PackageIdentity, PackageSnapshotSource,
    capture_embedded_package_snapshot,
};
use veln_repo_language_reference::{RenderedResource, render_checked_language_reference};

const VELN_SOURCE_MEDIA_TYPE: &str = "text/x-veln; charset=utf-8";
const RETAINED_PACKAGE_CAPACITY: usize = 256;

#[derive(Clone)]
pub(crate) struct LanguageResources {
    by_uri: BTreeMap<String, RenderedResource>,
    topics: Vec<LanguageTopic>,
    combined_resources: Vec<PublishedResource>,
    combined_by_uri: BTreeMap<String, PublishedResource>,
    retained_package_keys: BTreeSet<RetainedPackageKey>,
}

impl LanguageResources {
    pub(crate) fn checked() -> Result<Self, String> {
        let rendered = render_checked_language_reference()?;
        let topics = language_topics(&rendered.resources)?;
        let standard_library = StandardLibraryResources::checked()?;
        Self::from_parts(
            rendered.resources,
            topics,
            standard_library.resources,
            [standard_library.key],
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test(resources: Vec<RenderedResource>, topics: Vec<LanguageTopic>) -> Self {
        Self::from_parts(resources, topics, Vec::new(), [])
            .expect("test resources should be unique")
    }

    fn from_parts(
        resources: Vec<RenderedResource>,
        topics: Vec<LanguageTopic>,
        standard_resources: Vec<PublishedResource>,
        retained_package_keys: impl IntoIterator<Item = RetainedPackageKey>,
    ) -> Result<Self, String> {
        let by_uri = resources
            .iter()
            .cloned()
            .map(|resource| (resource.uri.clone(), resource))
            .collect();
        let mut combined_resources = resources
            .iter()
            .map(PublishedResource::from_rendered)
            .chain(standard_resources)
            .collect::<Vec<_>>();
        combined_resources.sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
        let mut combined_by_uri = BTreeMap::new();
        for resource in &combined_resources {
            if combined_by_uri
                .insert(resource.uri.clone(), resource.clone())
                .is_some()
            {
                return Err(format!("duplicate MCP resource URI `{}`", resource.uri));
            }
        }
        Ok(Self {
            by_uri,
            topics,
            combined_resources,
            combined_by_uri,
            retained_package_keys: retained_package_keys.into_iter().collect(),
        })
    }

    pub(crate) fn admit_dependencies(
        &mut self,
        dependencies: &[CapturedDependencyProject],
    ) -> Result<(), ResourceCapacityError> {
        let new_snapshots = dependency_resources(dependencies);
        let new_keys = new_snapshots
            .iter()
            .map(|snapshot| snapshot.key.clone())
            .filter(|key| !self.retained_package_keys.contains(key))
            .collect::<BTreeSet<_>>();
        if self.retained_package_keys.len() + new_keys.len() > RETAINED_PACKAGE_CAPACITY {
            return Err(ResourceCapacityError);
        }
        if new_keys.is_empty() {
            return Ok(());
        }
        for snapshot in new_snapshots {
            if !self.retained_package_keys.insert(snapshot.key) {
                continue;
            }
            for resource in snapshot.resources {
                if self.combined_by_uri.contains_key(&resource.uri) {
                    continue;
                }
                self.combined_by_uri
                    .insert(resource.uri.clone(), resource.clone());
                self.combined_resources.push(resource);
            }
        }
        self.combined_resources
            .sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
        Ok(())
    }

    pub(crate) fn list_result(&self) -> Value {
        json!({
            "resources": self.combined_resources.iter().map(PublishedResource::metadata).collect::<Vec<_>>()
        })
    }

    pub(crate) fn read_result(&self, uri: &str) -> Option<Value> {
        self.combined_by_uri.get(uri).map(|resource| {
            json!({
                "contents": [{
                    "uri": resource.uri,
                    "mimeType": resource.mime_type,
                    "text": resource.text,
                }]
            })
        })
    }

    pub(crate) fn read_doc_result(&self, uri: &str) -> Option<Value> {
        self.by_uri.get(uri).map(|resource| {
            let mut value = json!({
                "uri": resource.uri,
                "name": resource.name,
                "title": resource.title,
                "mimeType": resource.mime_type,
                "text": resource.text,
            });
            if let Some(description) = &resource.description {
                value["description"] = json!(description);
            }
            value
        })
    }

    pub(crate) fn topics(&self) -> &[LanguageTopic] {
        &self.topics
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RetainedPackageKey {
    identity: String,
    digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResourceCapacityError;

struct DependencyResources {
    key: RetainedPackageKey,
    resources: Vec<PublishedResource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublishedResource {
    uri: String,
    name: String,
    title: String,
    description: Option<String>,
    mime_type: &'static str,
    text: String,
}

impl PublishedResource {
    fn from_rendered(resource: &RenderedResource) -> Self {
        Self {
            uri: resource.uri.clone(),
            name: resource.name.clone(),
            title: resource.title.clone(),
            description: resource.description.clone(),
            mime_type: resource.mime_type,
            text: resource.text.clone(),
        }
    }

    pub(crate) fn metadata(&self) -> Value {
        let mut value = json!({
            "uri": self.uri,
            "name": self.name,
            "title": self.title,
            "mimeType": self.mime_type,
        });
        if let Some(description) = &self.description {
            value["description"] = json!(description);
        }
        value
    }
}

#[derive(Debug)]
pub(crate) struct StandardLibraryResources {
    pub(crate) resources: Vec<PublishedResource>,
    key: RetainedPackageKey,
}

impl StandardLibraryResources {
    pub(crate) fn checked() -> Result<Self, String> {
        let bundle = veln_stdlib::package_bundle();
        Self::from_embedded_inputs(
            bundle.manifest,
            bundle
                .files
                .iter()
                .map(|file| PackageSnapshotSource::new(file.path, file.text.as_bytes())),
        )
    }

    fn from_embedded_inputs<'a>(
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
    ) -> Result<Self, String> {
        Self::from_embedded_inputs_with_catalog_builder(manifest, sources, |identity, snapshot| {
            VirtualSourceCatalog::new([(identity, snapshot)])
                .map_err(|error| format!("build embedded standard library source catalog: {error}"))
        })
    }

    fn from_embedded_inputs_with_catalog_builder<'a>(
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
        catalog_builder: impl FnOnce(
            PackageIdentity,
            CapturedPackageSnapshot,
        ) -> Result<VirtualSourceCatalog, String>,
    ) -> Result<Self, String> {
        let snapshot = capture_embedded_package_snapshot(manifest.as_bytes(), sources)
            .map_err(|error| format!("capture embedded standard library snapshot: {error}"))?;
        let manifest = veln_project::parse_manifest_text("veln.toml", manifest);
        DirectDependencySnapshot::from_validated_standard_library(snapshot.clone(), manifest)
            .map_err(|error| format!("validate embedded standard library snapshot: {error}"))?;
        let catalog = catalog_builder(PackageIdentity::embedded_standard(), snapshot.clone())?;
        let mut resources = Vec::with_capacity(snapshot.sources().len());
        for (source_index, source) in snapshot.sources().iter().enumerate() {
            let entry = catalog
                .entry_for_source(0, source_index)
                .ok_or("embedded standard library source catalog is incomplete")?;
            let text = std::str::from_utf8(source.bytes())
                .map_err(|error| {
                    format!(
                        "embedded standard library source `{}` is not valid UTF-8 at byte {}",
                        source.path(),
                        error.valid_up_to()
                    )
                })?
                .to_string();
            resources.push(PublishedResource {
                uri: entry.uri().to_string(),
                name: source.path().to_string(),
                title: format!("Veln standard library source: {}", source.path()),
                description: None,
                mime_type: VELN_SOURCE_MEDIA_TYPE,
                text,
            });
        }
        Ok(Self {
            resources,
            key: RetainedPackageKey {
                identity: PackageIdentity::embedded_standard().as_str().to_string(),
                digest: snapshot.digest().to_string(),
            },
        })
    }
}

fn dependency_resources(dependencies: &[CapturedDependencyProject]) -> Vec<DependencyResources> {
    dependencies
        .iter()
        .filter_map(dependency_resource)
        .collect()
}

fn dependency_resource(dependency: &CapturedDependencyProject) -> Option<DependencyResources> {
    let identity = PackageIdentity::new(&dependency.package).ok()?;
    let project = dependency.project.as_ref()?;
    let manifest = project.manifest.clone()?;
    let sources = project
        .files
        .iter()
        .map(|source| PackageSnapshotSource::new(source.path().as_str(), source.text().as_bytes()));
    let snapshot = capture_embedded_package_snapshot(&manifest.source_bytes, sources).ok()?;
    DirectDependencySnapshot::from_validated_manifest(&identity, snapshot.clone(), manifest)
        .ok()?;
    let catalog = VirtualSourceCatalog::new([(identity.clone(), snapshot.clone())]).ok()?;
    let resources = snapshot
        .sources()
        .iter()
        .enumerate()
        .filter_map(|(source_index, source)| {
            let entry = catalog.entry_for_source(0, source_index)?;
            let text = std::str::from_utf8(source.bytes()).ok()?.to_string();
            Some(PublishedResource {
                uri: entry.uri().to_string(),
                name: source.path().to_string(),
                title: format!(
                    "Veln package source: {}: {}",
                    identity.as_str(),
                    source.path()
                ),
                description: None,
                mime_type: VELN_SOURCE_MEDIA_TYPE,
                text,
            })
        })
        .collect();
    Some(DependencyResources {
        key: RetainedPackageKey {
            identity: identity.as_str().to_string(),
            digest: snapshot.digest().to_string(),
        },
        resources,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LanguageTopic {
    pub(crate) uri: String,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) body: String,
}

fn language_topics(resources: &[RenderedResource]) -> Result<Vec<LanguageTopic>, String> {
    let catalog: Value =
        serde_json::from_str(veln_repo_language_reference::checked_catalog_bytes())
            .map_err(|error| format!("parse checked language-reference catalog: {error}"))?;
    let topics = catalog
        .get("topics")
        .and_then(Value::as_array)
        .ok_or("checked language-reference catalog must contain topics")?;
    let uri_by_name = resources
        .iter()
        .map(|resource| (resource.name.as_str(), resource.uri.as_str()))
        .collect::<BTreeMap<_, _>>();
    topics
        .iter()
        .map(|topic| {
            let id = string_field(topic, "id")?;
            let body = string_array_field(topic, "body")?.join("\n\n");
            Ok(LanguageTopic {
                uri: uri_by_name
                    .get(id)
                    .ok_or("checked topic resource must exist")?
                    .to_string(),
                id: id.to_string(),
                title: string_field(topic, "title")?.to_string(),
                summary: string_field(topic, "summary")?.to_string(),
                keywords: string_array_field(topic, "keywords")?
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                body,
            })
        })
        .collect()
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("checked language-reference topic must contain `{field}`"))
}

fn string_array_field<'a>(value: &'a Value, field: &str) -> Result<Vec<&'a str>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("checked language-reference topic must contain `{field}`"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .ok_or_else(|| format!("checked language-reference `{field}` must contain strings"))
        })
        .collect()
}

#[cfg(test)]
mod standard_library_tests {
    use super::*;

    #[test]
    fn standard_library_capture_rejects_invalid_embedded_inputs() {
        let error = StandardLibraryResources::from_embedded_inputs(
            "name = \"std\"\n",
            [PackageSnapshotSource::new(
                "bad/../main.veln",
                b"pub fn main() -> Int\n  1\nend\n",
            )],
        )
        .unwrap_err();

        assert!(error.contains("capture embedded standard library snapshot"));
    }

    #[test]
    fn standard_library_capture_rejects_invalid_manifest_identity() {
        let error = StandardLibraryResources::from_embedded_inputs(
            "name = \"other\"\n",
            [PackageSnapshotSource::new(
                "main.veln",
                b"pub fn main() -> Int\n  1\nend\n",
            )],
        )
        .unwrap_err();

        assert!(error.contains("validate embedded standard library snapshot"));
    }

    #[test]
    fn standard_library_capture_propagates_catalog_construction_failure() {
        let error = StandardLibraryResources::from_embedded_inputs_with_catalog_builder(
            "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
            [PackageSnapshotSource::new(
                "prelude.veln",
                b"pub fn main() -> Int\n  1\nend\n",
            )],
            |_identity, _snapshot| {
                Err(
                    "build embedded standard library source catalog: injected construction failure"
                        .to_string(),
                )
            },
        )
        .unwrap_err();

        assert!(error.contains("build embedded standard library source catalog"));
        assert!(error.contains("injected construction failure"));
    }
}
