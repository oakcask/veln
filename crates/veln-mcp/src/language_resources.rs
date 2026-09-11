#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use veln_analysis::CapturedDependencyProject;
use veln_language_service::{DirectDependencySnapshot, EffectiveProjectSnapshot};
#[cfg(test)]
use veln_project::PackageSnapshotSource;
use veln_repo_language_reference::{RenderedResource, render_checked_language_reference};

use self::dependencies::dependency_resources;
use self::package_resources::{PackageDocumentation, RetainedPackageKey};
pub(crate) use self::package_resources::{
    PackageSearchCandidate, PackageSearchScope, PublishedResource,
};
pub(crate) use self::standard_library::StandardLibraryResources;
pub(crate) use self::topics::LanguageTopic;
use self::topics::language_topics;

mod dependencies;
mod package_resources;
mod standard_library;
mod topics;

const RETAINED_PACKAGE_CAPACITY: usize = 256;

#[cfg(test)]
thread_local! {
    static DEPENDENCY_NAVIGATION_BUILDS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_NAVIGATION_BUILDS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) use self::dependencies::{
    dependency_snapshot_captures, reset_dependency_snapshot_captures,
};
#[cfg(test)]
pub(crate) use self::standard_library::standard_library_resource_builds;

#[cfg(test)]
pub(crate) fn reset_dependency_navigation_builds() {
    DEPENDENCY_NAVIGATION_BUILDS.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_navigation_builds() -> usize {
    DEPENDENCY_NAVIGATION_BUILDS.get()
}

#[cfg(test)]
pub(crate) fn reset_workspace_navigation_builds() {
    WORKSPACE_NAVIGATION_BUILDS.set(0);
}

#[cfg(test)]
pub(crate) fn workspace_navigation_builds() -> usize {
    WORKSPACE_NAVIGATION_BUILDS.get()
}

#[derive(Clone)]
pub(crate) struct LanguageResources {
    by_uri: BTreeMap<String, RenderedResource>,
    topics: Vec<LanguageTopic>,
    combined_resources: Vec<PublishedResource>,
    combined_by_uri: BTreeMap<String, PublishedResource>,
    retained_package_keys: BTreeSet<RetainedPackageKey>,
    package_docs: BTreeMap<RetainedPackageKey, PackageDocumentation>,
    standard_library_snapshot: Option<DirectDependencySnapshot>,
    standard_library_navigation: EffectiveProjectSnapshot,
    dependency_navigation: Option<(Vec<RetainedPackageKey>, EffectiveProjectSnapshot)>,
    workspace_navigation: Option<(
        Value,
        Vec<RetainedPackageKey>,
        Arc<EffectiveProjectSnapshot>,
    )>,
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
            [(standard_library.key, standard_library.package_documentation)],
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

    #[cfg(test)]
    pub(crate) fn replace_test_language_resources(
        &mut self,
        resources: Vec<RenderedResource>,
        topics: Vec<LanguageTopic>,
    ) {
        let replacement = Self::from_parts(
            resources,
            topics,
            self.combined_resources
                .iter()
                .filter(|resource| !resource.uri.starts_with("veln-doc:///language/"))
                .cloned()
                .collect(),
            self.retained_package_keys.clone(),
            self.package_docs
                .iter()
                .map(|(key, result)| (key.clone(), result.clone())),
            self.standard_library_snapshot.clone(),
            self.standard_library_navigation.clone(),
        )
        .expect("test resources should be unique");
        *self = replacement;
    }

    #[cfg(test)]
    pub(crate) fn replace_test_standard_library<'a>(
        &mut self,
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
    ) {
        let standard_library = StandardLibraryResources::from_embedded_inputs(manifest, sources)
            .expect("test standard library resources should build");
        let mut retained_package_keys = self
            .retained_package_keys
            .iter()
            .filter(|key| key.identity != "std")
            .cloned()
            .collect::<Vec<_>>();
        retained_package_keys.push(standard_library.key.clone());
        let mut package_docs = self
            .package_docs
            .iter()
            .filter(|(key, _)| key.identity != "std")
            .map(|(key, result)| (key.clone(), result.clone()))
            .collect::<Vec<_>>();
        package_docs.push((
            standard_library.key.clone(),
            standard_library.package_documentation.clone(),
        ));
        let replacement = Self::from_parts(
            self.by_uri.values().cloned().collect(),
            self.topics.clone(),
            standard_library.resources,
            retained_package_keys,
            package_docs,
            Some(standard_library.snapshot.clone()),
            EffectiveProjectSnapshot::new(Vec::new())
                .with_standard_library(standard_library.snapshot),
        )
        .expect("test standard library resources should be unique");
        *self = replacement;
    }

    fn from_parts(
        resources: Vec<RenderedResource>,
        topics: Vec<LanguageTopic>,
        standard_resources: Vec<PublishedResource>,
        retained_package_keys: impl IntoIterator<Item = RetainedPackageKey>,
        package_docs: impl IntoIterator<Item = (RetainedPackageKey, PackageDocumentation)>,
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
            workspace_navigation: None,
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
            self.package_docs.insert(
                key,
                PackageDocumentation::Ready(snapshot.package_doc_result),
            );
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
        if let Some(resource) = self.by_uri.get(uri) {
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
            return Some(value);
        }
        self.combined_by_uri
            .get(uri)
            .filter(|resource| {
                resource.mime_type
                    == veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE
            })
            .map(PublishedResource::doc_tool_result)
    }

    pub(crate) fn topics(&self) -> &[LanguageTopic] {
        &self.topics
    }

    pub(crate) fn package_search_candidates(&self) -> Vec<PackageSearchCandidate> {
        self.package_docs
            .iter()
            .flat_map(|(key, documentation)| documentation.search_candidates(key))
            .collect()
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
        workspace_key: Value,
    ) -> Arc<EffectiveProjectSnapshot> {
        let reuse = self.workspace_navigation.as_ref().is_some_and(
            |(cached_workspace_key, cached_dependency_keys, _)| {
                cached_workspace_key == &workspace_key
                    && cached_dependency_keys == &dependencies.keys
            },
        );
        if reuse {
            return Arc::clone(
                &self
                    .workspace_navigation
                    .as_ref()
                    .expect("workspace navigation was prepared")
                    .2,
            );
        }

        let dependency_keys = dependencies.keys;
        let snapshot = if dependencies.snapshots.is_empty() {
            self.with_standard_library_navigation(files)
        } else {
            let reuse = self
                .dependency_navigation
                .as_ref()
                .is_some_and(|(keys, _)| *keys == dependency_keys);
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
                self.dependency_navigation = Some((dependency_keys.clone(), snapshot));
            }
            self.dependency_navigation
                .as_ref()
                .expect("dependency navigation was prepared")
                .1
                .with_workspace_overlays(files)
        };
        #[cfg(test)]
        WORKSPACE_NAVIGATION_BUILDS.set(WORKSPACE_NAVIGATION_BUILDS.get() + 1);
        let snapshot = Arc::new(snapshot);
        self.workspace_navigation = Some((workspace_key, dependency_keys, Arc::clone(&snapshot)));
        snapshot
    }

    pub(crate) fn package_documentation_uri_for(
        &self,
        location: &veln_language_service::NavigationLocation,
    ) -> Option<&str> {
        self.package_docs
            .values()
            .find_map(|documentation| documentation.declaration_uri_for_location(location))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResourceCapacityError;

#[derive(Debug)]
pub(crate) struct AdmittedDependencies {
    keys: Vec<RetainedPackageKey>,
    snapshots: Vec<DirectDependencySnapshot>,
}

#[cfg(test)]
#[path = "language_resources_tests.rs"]
mod standard_library_tests;
