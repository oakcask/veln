#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde_json::{Value, json};
use veln_analysis::CapturedDependencyProject;
use veln_language_service::{
    DirectDependencySnapshot, EffectiveProjectSnapshot, PackageDocGeneratorContract,
    PackageDocResult, RenderedPackageDocResource, VirtualSourceCatalog,
    render_package_documentation,
};
use veln_project::{
    CapturedPackageSnapshot, PackageIdentity, PackageSnapshotSource,
    capture_embedded_package_snapshot,
};
use veln_repo_language_reference::{RenderedResource, render_checked_language_reference};

const VELN_SOURCE_MEDIA_TYPE: &str = "text/x-veln; charset=utf-8";
const RETAINED_PACKAGE_CAPACITY: usize = 256;

#[cfg(test)]
thread_local! {
    static DEPENDENCY_SNAPSHOT_CAPTURES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_NAVIGATION_BUILDS: Cell<usize> = const { Cell::new(0) };
    static STANDARD_LIBRARY_RESOURCE_BUILDS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn record_dependency_snapshot_capture() {
    DEPENDENCY_SNAPSHOT_CAPTURES.set(DEPENDENCY_SNAPSHOT_CAPTURES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_dependency_snapshot_captures() {
    DEPENDENCY_SNAPSHOT_CAPTURES.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_snapshot_captures() -> usize {
    DEPENDENCY_SNAPSHOT_CAPTURES.get()
}

#[cfg(test)]
pub(crate) fn reset_dependency_navigation_builds() {
    DEPENDENCY_NAVIGATION_BUILDS.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_navigation_builds() -> usize {
    DEPENDENCY_NAVIGATION_BUILDS.get()
}

#[cfg(test)]
pub(crate) fn standard_library_resource_builds() -> usize {
    STANDARD_LIBRARY_RESOURCE_BUILDS.get()
}

#[derive(Clone)]
pub(crate) struct LanguageResources {
    by_uri: BTreeMap<String, RenderedResource>,
    topics: Vec<LanguageTopic>,
    combined_resources: Vec<PublishedResource>,
    combined_by_uri: BTreeMap<String, PublishedResource>,
    retained_package_keys: BTreeSet<RetainedPackageKey>,
    package_docs: BTreeMap<RetainedPackageKey, PackageDocResult>,
    standard_library_snapshot: Option<DirectDependencySnapshot>,
    standard_library_navigation: EffectiveProjectSnapshot,
    dependency_navigation: Option<(Vec<RetainedPackageKey>, EffectiveProjectSnapshot)>,
}

impl LanguageResources {
    pub(crate) fn checked() -> Result<Self, String> {
        static CHECKED: OnceLock<Result<LanguageResources, String>> = OnceLock::new();

        CHECKED.get_or_init(Self::build_checked).clone()
    }

    fn build_checked() -> Result<Self, String> {
        let rendered = render_checked_language_reference()?;
        let topics = language_topics(&rendered.resources)?;
        let standard_library = StandardLibraryResources::checked()?;
        let standard_library_key = standard_library.key.clone();
        let navigation_snapshot = standard_library.snapshot.clone();
        Self::from_parts(
            rendered.resources,
            topics,
            standard_library.resources,
            [standard_library_key],
            [(standard_library.key, standard_library.package_doc_result)],
            Some(standard_library.snapshot),
            EffectiveProjectSnapshot::new(Vec::new()).with_standard_library(navigation_snapshot),
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test(resources: Vec<RenderedResource>, topics: Vec<LanguageTopic>) -> Self {
        Self::from_parts(
            resources,
            topics,
            Vec::new(),
            [],
            [],
            None,
            EffectiveProjectSnapshot::new(Vec::new()),
        )
        .expect("test resources should be unique")
    }

    fn from_parts(
        resources: Vec<RenderedResource>,
        topics: Vec<LanguageTopic>,
        standard_resources: Vec<PublishedResource>,
        retained_package_keys: impl IntoIterator<Item = RetainedPackageKey>,
        package_docs: impl IntoIterator<Item = (RetainedPackageKey, PackageDocResult)>,
        standard_library_snapshot: Option<DirectDependencySnapshot>,
        standard_library_navigation: EffectiveProjectSnapshot,
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
            package_docs: package_docs.into_iter().collect(),
            standard_library_snapshot,
            standard_library_navigation,
            dependency_navigation: None,
        })
    }

    pub(crate) fn admit_dependencies(
        &mut self,
        dependencies: &[CapturedDependencyProject],
    ) -> Result<AdmittedDependencies, ResourceCapacityError> {
        let new_snapshots = dependency_resources(dependencies);
        let keys = new_snapshots
            .iter()
            .map(|snapshot| snapshot.key.clone())
            .collect::<Vec<_>>();
        let new_keys = new_snapshots
            .iter()
            .map(|snapshot| snapshot.key.clone())
            .filter(|key| !self.retained_package_keys.contains(key))
            .collect::<BTreeSet<_>>();
        if self.retained_package_keys.len() + new_keys.len() > RETAINED_PACKAGE_CAPACITY {
            return Err(ResourceCapacityError);
        }
        let mut navigation_snapshots = Vec::with_capacity(new_snapshots.len());
        for snapshot in new_snapshots {
            navigation_snapshots.push(snapshot.navigation);
            let key = snapshot.key.clone();
            if !self.retained_package_keys.insert(key.clone()) {
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
            self.package_docs.insert(key, snapshot.package_doc_result);
        }
        self.combined_resources
            .sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
        Ok(AdmittedDependencies {
            keys,
            snapshots: navigation_snapshots,
        })
    }

    pub(crate) fn list_result(&self) -> Value {
        json!({
            "resources": self.combined_resources.iter().filter(|resource| resource.listed).map(PublishedResource::metadata).collect::<Vec<_>>()
        })
    }

    pub(crate) fn resource_templates_result(&self) -> Value {
        json!({
            "resourceTemplates": [
                {
                    "uriTemplate": "veln-doc:///package/{package}/snapshot/{snapshot_digest}/documentation/{documentation_digest}/module/{module_id}",
                    "name": "package-documentation-module",
                    "title": "Veln package documentation module",
                    "mimeType": veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
                },
                {
                    "uriTemplate": "veln-doc:///package/{package}/snapshot/{snapshot_digest}/documentation/{documentation_digest}/declaration/{declaration_id}",
                    "name": "package-documentation-declaration",
                    "title": "Veln package documentation declaration",
                    "mimeType": veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
                },
            ]
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

    pub(crate) fn standard_library_snapshot(&self) -> Option<DirectDependencySnapshot> {
        self.standard_library_snapshot.clone()
    }

    pub(crate) fn with_standard_library_navigation(
        &self,
        files: Vec<veln_source::SourceFile>,
    ) -> EffectiveProjectSnapshot {
        self.standard_library_navigation
            .with_workspace_overlays(files)
    }

    pub(crate) fn with_dependency_navigation(
        &mut self,
        files: Vec<veln_source::SourceFile>,
        dependencies: AdmittedDependencies,
    ) -> EffectiveProjectSnapshot {
        if dependencies.snapshots.is_empty() {
            return self.with_standard_library_navigation(files);
        }
        let reuse = self
            .dependency_navigation
            .as_ref()
            .is_some_and(|(keys, _)| *keys == dependencies.keys);
        if !reuse {
            #[cfg(test)]
            DEPENDENCY_NAVIGATION_BUILDS.set(DEPENDENCY_NAVIGATION_BUILDS.get() + 1);
            let mut snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                Vec::new(),
                dependencies.snapshots,
            );
            if let Some(standard_library) = self.standard_library_snapshot() {
                snapshot = snapshot.with_standard_library(standard_library);
            }
            self.dependency_navigation = Some((dependencies.keys, snapshot));
        }
        self.dependency_navigation
            .as_ref()
            .expect("dependency navigation was prepared")
            .1
            .with_workspace_overlays(files)
    }

    pub(crate) fn package_documentation_uri_for(
        &self,
        location: &veln_language_service::NavigationLocation,
    ) -> Option<&str> {
        self.package_docs
            .values()
            .find_map(|package_doc| package_doc.declaration_uri_for_location(location))
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RetainedPackageKey {
    identity: String,
    digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResourceCapacityError;

#[derive(Debug)]
pub(crate) struct AdmittedDependencies {
    keys: Vec<RetainedPackageKey>,
    snapshots: Vec<DirectDependencySnapshot>,
}

struct DependencyResources {
    key: RetainedPackageKey,
    resources: Vec<PublishedResource>,
    navigation: DirectDependencySnapshot,
    package_doc_result: PackageDocResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PublishedResource {
    uri: String,
    name: String,
    title: String,
    description: Option<String>,
    mime_type: &'static str,
    text: String,
    pub(crate) listed: bool,
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
            listed: true,
        }
    }

    fn from_package_doc(resource: &RenderedPackageDocResource) -> Self {
        Self {
            uri: resource.uri.clone(),
            name: resource.name.clone(),
            title: resource.title.clone(),
            description: resource.description.clone(),
            mime_type: resource.mime_type,
            text: resource.text.clone(),
            listed: resource.listed,
        }
    }

    fn from_checked_package_doc(
        resource: veln_repo_mcp_standard_library_docs::CheckedResource,
    ) -> Self {
        Self {
            uri: resource.uri,
            name: resource.name,
            title: resource.title,
            description: resource.description,
            mime_type: veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
            text: resource.text,
            listed: resource.listed,
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

#[derive(Clone, Debug)]
pub(crate) struct StandardLibraryResources {
    pub(crate) resources: Vec<PublishedResource>,
    key: RetainedPackageKey,
    snapshot: DirectDependencySnapshot,
    package_doc_result: PackageDocResult,
}

impl StandardLibraryResources {
    pub(crate) fn checked() -> Result<Self, String> {
        static CHECKED: OnceLock<Result<StandardLibraryResources, String>> = OnceLock::new();

        CHECKED
            .get_or_init(Self::from_checked_embedded_inputs)
            .clone()
    }

    fn from_checked_embedded_inputs() -> Result<Self, String> {
        #[cfg(test)]
        STANDARD_LIBRARY_RESOURCE_BUILDS.set(STANDARD_LIBRARY_RESOURCE_BUILDS.get() + 1);
        let bundle = veln_stdlib::package_bundle();
        Self::from_embedded_inputs_with_builders(
            bundle.manifest,
            bundle
                .files
                .iter()
                .map(|file| PackageSnapshotSource::new(file.path, file.text.as_bytes())),
            |identity, snapshot| {
                VirtualSourceCatalog::new([(identity, snapshot)]).map_err(|error| {
                    format!("build embedded standard library source catalog: {error}")
                })
            },
            |snapshot, _manifest, package_doc_result| {
                veln_repo_mcp_standard_library_docs::checked_bundle_for_snapshot(snapshot.digest())
                    .and_then(|bundle| {
                        if bundle.metadata.documentation_digest != package_doc_result.doc_digest()
                        {
                            return Err(format!(
                                "regenerate the checked MCP standard-library package-documentation resources for documentation `{}`; checked documentation is `{}`",
                                package_doc_result.doc_digest(),
                                bundle.metadata.documentation_digest
                            ));
                        }
                        Ok(
                            bundle
                                .resources
                                .into_iter()
                                .map(PublishedResource::from_checked_package_doc)
                                .collect(),
                        )
                    })
            },
        )
    }

    #[cfg(test)]
    fn from_embedded_inputs<'a>(
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
    ) -> Result<Self, String> {
        Self::from_embedded_inputs_with_catalog_builder(manifest, sources, |identity, snapshot| {
            VirtualSourceCatalog::new([(identity, snapshot)])
                .map_err(|error| format!("build embedded standard library source catalog: {error}"))
        })
    }

    #[cfg(test)]
    fn from_embedded_inputs_with_catalog_builder<'a>(
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
        catalog_builder: impl FnOnce(
            PackageIdentity,
            CapturedPackageSnapshot,
        ) -> Result<VirtualSourceCatalog, String>,
    ) -> Result<Self, String> {
        Self::from_embedded_inputs_with_builders(
            manifest,
            sources,
            catalog_builder,
            |_, _, package_doc_result| {
                Ok(render_package_documentation(package_doc_result)
                    .iter()
                    .map(PublishedResource::from_package_doc)
                    .collect())
            },
        )
    }

    fn from_embedded_inputs_with_builders<'a>(
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
        catalog_builder: impl FnOnce(
            PackageIdentity,
            CapturedPackageSnapshot,
        ) -> Result<VirtualSourceCatalog, String>,
        documentation_builder: impl FnOnce(
            &CapturedPackageSnapshot,
            &veln_project::ProjectManifest,
            &PackageDocResult,
        ) -> Result<Vec<PublishedResource>, String>,
    ) -> Result<Self, String> {
        let snapshot = capture_embedded_package_snapshot(manifest.as_bytes(), sources)
            .map_err(|error| format!("capture embedded standard library snapshot: {error}"))?;
        let manifest = veln_project::parse_manifest_text("veln.toml", manifest);
        let package_doc_result = PackageDocResult::generate(
            &PackageIdentity::embedded_standard(),
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new(
                veln_repo_mcp_standard_library_docs::GENERATOR_CONTRACT,
            ),
        );
        let documentation_resources =
            documentation_builder(&snapshot, &manifest, &package_doc_result)?;
        let navigation_snapshot =
            DirectDependencySnapshot::from_validated_standard_library(snapshot.clone(), manifest)
                .map_err(|error| format!("validate embedded standard library snapshot: {error}"))?;
        let catalog = catalog_builder(PackageIdentity::embedded_standard(), snapshot.clone())?;
        let mut resources = standard_library_source_resources(&snapshot, &catalog)?;
        resources.extend(documentation_resources);
        Ok(Self {
            resources,
            key: RetainedPackageKey {
                identity: PackageIdentity::embedded_standard().as_str().to_string(),
                digest: snapshot.digest().to_string(),
            },
            snapshot: navigation_snapshot,
            package_doc_result,
        })
    }
}

fn standard_library_source_resources(
    snapshot: &CapturedPackageSnapshot,
    catalog: &VirtualSourceCatalog,
) -> Result<Vec<PublishedResource>, String> {
    snapshot
        .sources()
        .iter()
        .enumerate()
        .map(|(source_index, source)| {
            let entry = catalog
                .entry_for_source(0, source_index)
                .ok_or("embedded standard library source catalog is incomplete")?;
            let text = embedded_source_text(source.path(), source.bytes())?;
            Ok(PublishedResource {
                uri: entry.uri().to_string(),
                name: source.path().to_string(),
                title: format!("Veln standard library source: {}", source.path()),
                description: None,
                mime_type: VELN_SOURCE_MEDIA_TYPE,
                text,
                listed: true,
            })
        })
        .collect()
}

fn embedded_source_text(path: &str, bytes: &[u8]) -> Result<String, String> {
    std::str::from_utf8(bytes)
        .map_err(|error| {
            format!(
                "embedded standard library source `{}` is not valid UTF-8 at byte {}",
                path,
                error.valid_up_to()
            )
        })
        .map(str::to_string)
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
    #[cfg(test)]
    record_dependency_snapshot_capture();
    let snapshot = captured_dependency_snapshot(project, &manifest.source_bytes)?;
    let navigation = DirectDependencySnapshot::from_validated_manifest(
        &identity,
        snapshot.clone(),
        manifest.clone(),
    )
    .ok()?;
    let package_doc_result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new(veln_repo_mcp_standard_library_docs::GENERATOR_CONTRACT),
    );
    let catalog = VirtualSourceCatalog::new([(identity.clone(), snapshot.clone())]).ok()?;
    let mut resources = dependency_source_resources(&identity, &snapshot, &catalog);
    resources.extend(
        render_package_documentation(&package_doc_result)
            .iter()
            .map(PublishedResource::from_package_doc),
    );
    Some(DependencyResources {
        key: RetainedPackageKey {
            identity: identity.as_str().to_string(),
            digest: snapshot.digest().to_string(),
        },
        resources,
        navigation,
        package_doc_result,
    })
}

fn captured_dependency_snapshot(
    project: &veln_project::Project,
    manifest_source: &[u8],
) -> Option<CapturedPackageSnapshot> {
    let sources = project
        .files
        .iter()
        .map(|source| PackageSnapshotSource::new(source.path().as_str(), source.text().as_bytes()));
    capture_embedded_package_snapshot(manifest_source, sources).ok()
}

fn dependency_source_resources(
    identity: &PackageIdentity,
    snapshot: &CapturedPackageSnapshot,
    catalog: &VirtualSourceCatalog,
) -> Vec<PublishedResource> {
    snapshot
        .sources()
        .iter()
        .enumerate()
        .filter_map(|(source_index, source)| {
            dependency_source_resource(
                identity,
                catalog,
                source_index,
                source.path(),
                source.bytes(),
            )
        })
        .collect()
}

fn dependency_source_resource(
    identity: &PackageIdentity,
    catalog: &VirtualSourceCatalog,
    source_index: usize,
    source_path: &str,
    source_bytes: &[u8],
) -> Option<PublishedResource> {
    let entry = catalog.entry_for_source(0, source_index)?;
    let text = std::str::from_utf8(source_bytes).ok()?.to_string();
    Some(PublishedResource {
        uri: entry.uri().to_string(),
        name: source_path.to_string(),
        title: format!(
            "Veln package source: {}: {}",
            identity.as_str(),
            source_path
        ),
        description: None,
        mime_type: VELN_SOURCE_MEDIA_TYPE,
        text,
        listed: true,
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
    let catalog = checked_language_catalog()?;
    let topics = checked_language_topics(&catalog)?;
    let uri_by_name = resources
        .iter()
        .map(|resource| (resource.name.as_str(), resource.uri.as_str()))
        .collect::<BTreeMap<_, _>>();
    topics
        .iter()
        .map(|topic| language_topic(topic, &uri_by_name))
        .collect()
}

fn checked_language_catalog() -> Result<Value, String> {
    serde_json::from_str(veln_repo_language_reference::checked_catalog_bytes())
        .map_err(|error| format!("parse checked language-reference catalog: {error}"))
}

fn checked_language_topics(catalog: &Value) -> Result<&Vec<Value>, String> {
    catalog
        .get("topics")
        .and_then(Value::as_array)
        .ok_or_else(|| "checked language-reference catalog must contain topics".to_string())
}

fn language_topic(
    topic: &Value,
    uri_by_name: &BTreeMap<&str, &str>,
) -> Result<LanguageTopic, String> {
    let id = string_field(topic, "id")?;
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
        body: string_array_field(topic, "body")?.join("\n\n"),
    })
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
#[path = "language_resources_tests.rs"]
mod standard_library_tests;
