#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use veln_analysis::CapturedDependencyProject;
use veln_language_service::{DirectDependencySnapshot, EffectiveProjectSnapshot};
#[cfg(test)]
use veln_project::PackageSnapshotSource;
use veln_repo_language_reference::{RenderedResource, render_checked_language_reference};

use self::dependencies::{
    DependencyResources, PreparedDependencyResources, captured_dependency_navigation,
};
use self::package_resources::{DocumentationRevisionKey, PackageDocumentation, RetainedPackageKey};
pub(crate) use self::package_resources::{
    PackageSearchCandidate, PackageSearchScope, PublishedResource,
};
pub(crate) use self::standard_library::StandardLibraryResources;
pub(crate) use self::topics::LanguageTopic;
use self::topics::language_topics;
use crate::dependency_snapshot::{DependencyRevisionKey, dependency_revision_key};
use crate::reference_pagination::ReferencePagination;

mod dependencies;
mod package_resources;
mod standard_library;
mod topics;

const RETAINED_PACKAGE_CAPACITY: usize = 256;
const RETAINED_DOCUMENTATION_REVISION_CAPACITY: usize = 256;
const ANALYZED_DEPENDENCY_SCOPE_CAPACITY: usize = 256;
const REJECTED_DEPENDENCY_REVISION_CAPACITY: usize = 256;

struct InitialPackageDocumentation {
    revisions: Vec<(DocumentationRevisionKey, PackageDocumentation)>,
    current: Vec<(RetainedPackageKey, DocumentationRevisionKey)>,
}

impl InitialPackageDocumentation {
    fn new(
        revisions: impl IntoIterator<Item = (DocumentationRevisionKey, PackageDocumentation)>,
        current: impl IntoIterator<Item = (RetainedPackageKey, DocumentationRevisionKey)>,
    ) -> Self {
        Self {
            revisions: revisions.into_iter().collect(),
            current: current.into_iter().collect(),
        }
    }
}

#[derive(Clone)]
struct WorkspaceNavigation {
    workspace_key: Value,
    documentation_revisions: Vec<DocumentationRevisionKey>,
    snapshot: Arc<EffectiveProjectSnapshot>,
}

#[derive(Clone)]
struct AnalyzedDependencies {
    revision: DependencyRevisionKey,
    keys: Vec<RetainedPackageKey>,
    documentation_revisions: Vec<DocumentationRevisionKey>,
    snapshots: Vec<DirectDependencySnapshot>,
}

#[derive(Clone)]
struct AcceptedAnalysis {
    dependencies: Arc<AnalyzedDependencies>,
    workspace_files: Vec<veln_source::SourceFile>,
}

#[derive(Clone, Default)]
struct AnalysisScopeState {
    accepted: Option<AcceptedAnalysis>,
    rejected: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DependencyAnalysisScope {
    Project(String),
    SingleFile { project: String, source: String },
}

impl DependencyAnalysisScope {
    fn project(&self) -> &str {
        match self {
            Self::Project(project) | Self::SingleFile { project, .. } => project,
        }
    }
}

#[derive(Clone)]
struct RejectedDependencyRevisions {
    recent_order: VecDeque<DependencyRevisionKey>,
    recent: BTreeSet<DependencyRevisionKey>,
    saturated: bool,
    #[cfg(test)]
    capacity: usize,
}

impl RejectedDependencyRevisions {
    fn new() -> Self {
        Self {
            recent_order: VecDeque::new(),
            recent: BTreeSet::new(),
            saturated: false,
            #[cfg(test)]
            capacity: REJECTED_DEPENDENCY_REVISION_CAPACITY,
        }
    }

    fn insert(&mut self, revision: DependencyRevisionKey) {
        if !self.recent.insert(revision) {
            return;
        }
        self.recent_order.push_back(revision);
        while self.recent.len() > self.capacity() {
            let evicted = self
                .recent_order
                .pop_front()
                .expect("non-empty rejected dependency revision order");
            self.recent.remove(&evicted);
            self.saturated = true;
        }
    }

    fn contains(&self, revision: DependencyRevisionKey) -> bool {
        self.saturated || self.recent.contains(&revision)
    }

    #[cfg(not(test))]
    fn capacity(&self) -> usize {
        REJECTED_DEPENDENCY_REVISION_CAPACITY
    }

    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.capacity
    }

    #[cfg(test)]
    fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        while self.recent.len() > capacity {
            let evicted = self
                .recent_order
                .pop_front()
                .expect("non-empty rejected dependency revision order");
            self.recent.remove(&evicted);
            self.saturated = true;
        }
    }
}

enum DependencyAnalysisSelection {
    Admitted(AnalyzedDependencies),
    Rejected(Option<AcceptedAnalysis>),
    Unanalyzed,
}

#[cfg(test)]
thread_local! {
    static DEPENDENCY_NAVIGATION_BUILDS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_NAVIGATION_BUILDS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) use self::dependencies::{
    DependencyPreparationWork, dependency_preparation_work, dependency_snapshot_captures,
    reset_dependency_preparation_work, reset_dependency_snapshot_captures,
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
    #[cfg(test)]
    retained_package_capacity: usize,
    package_docs: BTreeMap<DocumentationRevisionKey, PackageDocumentation>,
    current_package_docs: BTreeMap<RetainedPackageKey, DocumentationRevisionKey>,
    current_listed_documentation_uris: BTreeSet<String>,
    #[cfg(test)]
    retained_documentation_revision_capacity: usize,
    #[cfg(test)]
    standard_library_snapshot: Option<DirectDependencySnapshot>,
    standard_library_navigation: EffectiveProjectSnapshot,
    dependency_navigation: Option<(Vec<RetainedPackageKey>, EffectiveProjectSnapshot)>,
    workspace_navigation: Option<WorkspaceNavigation>,
    read_only_workspace_navigation: Option<(Value, Arc<EffectiveProjectSnapshot>)>,
    analysis_scopes: BTreeMap<DependencyAnalysisScope, AnalysisScopeState>,
    analyzed_dependency_scope_order: VecDeque<DependencyAnalysisScope>,
    analysis_scope_history_saturated: bool,
    #[cfg(test)]
    analyzed_dependency_scope_capacity: usize,
    rejected_dependency_revisions: RejectedDependencyRevisions,
    reference_pagination: ReferencePagination,
}

impl LanguageResources {
    pub(crate) fn checked() -> Result<Self, String> {
        static CHECKED: OnceLock<Result<LanguageResources, String>> = OnceLock::new();

        let mut resources = CHECKED.get_or_init(Self::build_checked).clone()?;
        // The checked resource payload is shared as a cache, but continuation
        // state belongs to the server instance that receives the cursor.
        resources.reference_pagination = ReferencePagination::new()?;
        Ok(resources)
    }

    fn build_checked() -> Result<Self, String> {
        let rendered = render_checked_language_reference()?;
        let topics = language_topics(&rendered.resources)?;
        let standard_library = StandardLibraryResources::checked()?;
        let standard_library_key = standard_library.key.clone();
        let standard_library_revision =
            DocumentationRevisionKey::standalone(standard_library.key.clone());
        let navigation_snapshot = standard_library.snapshot.clone();
        Self::from_parts(
            rendered.resources,
            topics,
            standard_library.resources,
            [standard_library_key],
            InitialPackageDocumentation::new(
                [(
                    standard_library_revision.clone(),
                    standard_library.package_documentation,
                )],
                [(standard_library.key, standard_library_revision)],
            ),
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
            InitialPackageDocumentation::new([], []),
            None,
            EffectiveProjectSnapshot::new(Vec::new()),
        )
        .expect("test resources should build")
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
            InitialPackageDocumentation::new(
                self.package_docs
                    .iter()
                    .map(|(key, result)| (key.clone(), result.clone())),
                self.current_package_docs
                    .iter()
                    .map(|(key, revision)| (key.clone(), revision.clone())),
            ),
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
        let retained_package_keys = self.retained_keys_with_standard_library(&standard_library.key);
        let package_documentation = self.documentation_with_standard_library(&standard_library);
        let standard_library_navigation = EffectiveProjectSnapshot::new(Vec::new())
            .with_standard_library(standard_library.snapshot.clone());
        let replacement = Self::from_parts(
            self.by_uri.values().cloned().collect(),
            self.topics.clone(),
            standard_library.resources,
            retained_package_keys,
            package_documentation,
            Some(standard_library.snapshot),
            standard_library_navigation,
        )
        .expect("test standard library resources should be unique");
        *self = replacement;
    }

    #[cfg(test)]
    fn retained_keys_with_standard_library(
        &self,
        standard_library_key: &RetainedPackageKey,
    ) -> Vec<RetainedPackageKey> {
        self.retained_package_keys
            .iter()
            .filter(|key| key.identity != "std")
            .cloned()
            .chain([standard_library_key.clone()])
            .collect()
    }

    #[cfg(test)]
    fn documentation_with_standard_library(
        &self,
        standard_library: &StandardLibraryResources,
    ) -> InitialPackageDocumentation {
        let mut package_docs = self
            .package_docs
            .iter()
            .filter(|(key, _)| key.package.identity != "std")
            .map(|(key, result)| (key.clone(), result.clone()))
            .collect::<Vec<_>>();
        let mut current_package_docs = self
            .current_package_docs
            .iter()
            .filter(|(key, _)| key.identity != "std")
            .map(|(key, revision)| (key.clone(), revision.clone()))
            .collect::<Vec<_>>();
        let standard_library_revision =
            DocumentationRevisionKey::standalone(standard_library.key.clone());
        package_docs.push((
            standard_library_revision.clone(),
            standard_library.package_documentation.clone(),
        ));
        current_package_docs.push((standard_library.key.clone(), standard_library_revision));
        InitialPackageDocumentation::new(package_docs, current_package_docs)
    }

    fn from_parts(
        resources: Vec<RenderedResource>,
        topics: Vec<LanguageTopic>,
        standard_resources: Vec<PublishedResource>,
        retained_package_keys: impl IntoIterator<Item = RetainedPackageKey>,
        package_documentation: InitialPackageDocumentation,
        standard_library_snapshot: Option<DirectDependencySnapshot>,
        standard_library_navigation: EffectiveProjectSnapshot,
    ) -> Result<Self, String> {
        let by_uri = resources
            .iter()
            .cloned()
            .map(|resource| (resource.uri.clone(), resource))
            .collect();
        let (combined_resources, combined_by_uri) =
            Self::index_resources(&resources, standard_resources)?;
        #[cfg(not(test))]
        let _ = standard_library_snapshot;
        let current_listed_documentation_uris =
            Self::listed_documentation_uris(&combined_resources);
        Ok(Self {
            by_uri,
            topics,
            combined_resources,
            combined_by_uri,
            retained_package_keys: retained_package_keys.into_iter().collect(),
            #[cfg(test)]
            retained_package_capacity: RETAINED_PACKAGE_CAPACITY,
            package_docs: package_documentation.revisions.into_iter().collect(),
            current_package_docs: package_documentation.current.into_iter().collect(),
            current_listed_documentation_uris,
            #[cfg(test)]
            retained_documentation_revision_capacity: RETAINED_DOCUMENTATION_REVISION_CAPACITY,
            #[cfg(test)]
            standard_library_snapshot,
            standard_library_navigation,
            dependency_navigation: None,
            workspace_navigation: None,
            read_only_workspace_navigation: None,
            analysis_scopes: BTreeMap::new(),
            analyzed_dependency_scope_order: VecDeque::new(),
            analysis_scope_history_saturated: false,
            #[cfg(test)]
            analyzed_dependency_scope_capacity: ANALYZED_DEPENDENCY_SCOPE_CAPACITY,
            rejected_dependency_revisions: RejectedDependencyRevisions::new(),
            reference_pagination: ReferencePagination::new()?,
        })
    }

    fn index_resources(
        resources: &[RenderedResource],
        standard_resources: Vec<PublishedResource>,
    ) -> Result<(Vec<PublishedResource>, BTreeMap<String, PublishedResource>), String> {
        let mut combined = resources
            .iter()
            .map(PublishedResource::from_rendered)
            .chain(standard_resources)
            .collect::<Vec<_>>();
        combined.sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
        let mut by_uri = BTreeMap::new();
        for resource in &combined {
            if by_uri
                .insert(resource.uri.clone(), resource.clone())
                .is_some()
            {
                return Err(format!("duplicate MCP resource URI `{}`", resource.uri));
            }
        }
        Ok((combined, by_uri))
    }

    fn listed_documentation_uris(resources: &[PublishedResource]) -> BTreeSet<String> {
        resources
            .iter()
            .filter(|resource| resource.listed && resource.uri.starts_with("veln-doc:///package/"))
            .map(|resource| resource.uri.clone())
            .collect()
    }

    pub(crate) fn reference_pagination(&mut self) -> &mut ReferencePagination {
        &mut self.reference_pagination
    }

    #[cfg(test)]
    pub(crate) fn set_test_remaining_package_capacity(&mut self, remaining: usize) {
        self.retained_package_capacity = self
            .retained_package_keys
            .len()
            .checked_add(remaining)
            .expect("test package capacity should fit usize");
    }

    #[cfg(test)]
    pub(crate) fn set_test_remaining_documentation_revision_capacity(&mut self, remaining: usize) {
        self.retained_documentation_revision_capacity = self
            .package_docs
            .len()
            .checked_add(remaining)
            .expect("test documentation revision capacity should fit usize");
    }

    #[cfg(test)]
    pub(crate) fn set_test_rejected_dependency_revision_capacity(&mut self, capacity: usize) {
        self.rejected_dependency_revisions.set_capacity(capacity);
    }

    #[cfg(test)]
    pub(crate) fn set_test_analyzed_dependency_scope_capacity(&mut self, capacity: usize) {
        self.analyzed_dependency_scope_capacity = capacity;
        self.evict_analyzed_dependency_scopes();
    }

    #[cfg(test)]
    pub(crate) fn retained_analyzed_dependency_scopes(&self) -> usize {
        self.analysis_scopes.len()
    }

    #[cfg(test)]
    pub(crate) fn retained_analyzed_dependency_payloads(&self) -> usize {
        self.analysis_scopes
            .values()
            .filter_map(|state| state.accepted.as_ref())
            .map(|accepted| Arc::as_ptr(&accepted.dependencies))
            .collect::<BTreeSet<_>>()
            .len()
    }

    #[cfg(test)]
    pub(crate) fn retained_rejected_dependency_revisions(&self) -> usize {
        self.rejected_dependency_revisions.recent.len()
    }

    #[cfg(test)]
    fn retained_package_capacity(&self) -> usize {
        self.retained_package_capacity
    }

    #[cfg(test)]
    fn retained_documentation_revision_capacity(&self) -> usize {
        self.retained_documentation_revision_capacity
    }

    #[cfg(not(test))]
    fn retained_documentation_revision_capacity(&self) -> usize {
        RETAINED_DOCUMENTATION_REVISION_CAPACITY
    }

    #[cfg(not(test))]
    fn retained_package_capacity(&self) -> usize {
        RETAINED_PACKAGE_CAPACITY
    }

    pub(crate) fn admit_dependencies(
        &mut self,
        dependencies: &[CapturedDependencyProject],
    ) -> Result<AdmittedDependencies, ResourceCapacityError> {
        let prepared = PreparedDependencyResources::new(dependencies);
        let keys = prepared.keys().collect::<Vec<_>>();
        let revision_keys = prepared.revision_keys().collect::<Vec<_>>();
        self.ensure_dependency_capacity(&keys, &revision_keys)?;
        let snapshots = prepared
            .into_resources()
            .into_iter()
            .map(|snapshot| self.retain_dependency(snapshot))
            .collect();
        self.combined_resources
            .sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
        Ok(AdmittedDependencies {
            keys,
            revision_keys,
            snapshots,
        })
    }

    pub(crate) fn admit_analyzed_dependencies(
        &mut self,
        scope: DependencyAnalysisScope,
        workspace_files: Vec<veln_source::SourceFile>,
        dependencies: &[CapturedDependencyProject],
    ) -> Result<(), ResourceCapacityError> {
        let revision = dependency_revision_key(dependencies);
        let analyzed = if let Some(analyzed) = self
            .analysis_scopes
            .values()
            .filter_map(|state| state.accepted.as_ref())
            .map(|accepted| &accepted.dependencies)
            .find(|analyzed| analyzed.revision == revision)
            .cloned()
        {
            self.select_analyzed_dependency_documentation(&analyzed);
            analyzed
        } else {
            let admitted = self.admit_dependencies(dependencies)?;
            Arc::new(AnalyzedDependencies {
                revision,
                keys: admitted.keys,
                documentation_revisions: admitted.revision_keys,
                snapshots: admitted.snapshots,
            })
        };
        self.analysis_scopes.insert(
            scope.clone(),
            AnalysisScopeState {
                accepted: Some(AcceptedAnalysis {
                    dependencies: analyzed,
                    workspace_files,
                }),
                rejected: false,
            },
        );
        self.analyzed_dependency_scope_order
            .retain(|retained| retained != &scope);
        self.analyzed_dependency_scope_order.push_back(scope);
        self.evict_analyzed_dependency_scopes();
        self.workspace_navigation = None;
        self.read_only_workspace_navigation = None;
        Ok(())
    }

    fn select_analyzed_dependency_documentation(&mut self, analyzed: &AnalyzedDependencies) {
        for (key, revision) in analyzed
            .keys
            .iter()
            .cloned()
            .zip(analyzed.documentation_revisions.iter().cloned())
        {
            if let Some(previous_revision) = self.current_package_docs.insert(key, revision.clone())
                && let Some(uri) = self
                    .package_docs
                    .get(&previous_revision)
                    .and_then(PackageDocumentation::listed_uri)
            {
                self.current_listed_documentation_uris.remove(uri);
            }
            if let Some(uri) = self
                .package_docs
                .get(&revision)
                .and_then(PackageDocumentation::listed_uri)
            {
                self.current_listed_documentation_uris
                    .insert(uri.to_string());
            }
        }
    }

    pub(crate) fn retain_analyzed_dependency_projects(&mut self, projects: &[String]) {
        let projects = projects.iter().map(String::as_str).collect::<BTreeSet<_>>();
        let previous_len = self.analysis_scopes.len();
        self.analysis_scopes
            .retain(|scope, _| projects.contains(scope.project()));
        self.analyzed_dependency_scope_order
            .retain(|scope| self.analysis_scopes.contains_key(scope));
        if self.analysis_scopes.len() != previous_len {
            self.workspace_navigation = None;
            self.read_only_workspace_navigation = None;
        }
    }

    fn evict_analyzed_dependency_scopes(&mut self) {
        while self.analysis_scopes.len() > self.analyzed_dependency_scope_capacity() {
            let evicted = self
                .analyzed_dependency_scope_order
                .pop_front()
                .expect("non-empty analyzed dependency scope order");
            self.analysis_scopes.remove(&evicted);
            self.analysis_scope_history_saturated = true;
        }
    }

    #[cfg(not(test))]
    fn analyzed_dependency_scope_capacity(&self) -> usize {
        ANALYZED_DEPENDENCY_SCOPE_CAPACITY
    }

    #[cfg(test)]
    fn analyzed_dependency_scope_capacity(&self) -> usize {
        self.analyzed_dependency_scope_capacity
    }

    pub(crate) fn reject_dependency_analysis(
        &mut self,
        scope: DependencyAnalysisScope,
        dependencies: &[CapturedDependencyProject],
    ) {
        self.analysis_scopes
            .entry(scope.clone())
            .or_default()
            .rejected = true;
        self.rejected_dependency_revisions
            .insert(dependency_revision_key(dependencies));
        self.analyzed_dependency_scope_order
            .retain(|retained| retained != &scope);
        self.analyzed_dependency_scope_order.push_back(scope);
        self.evict_analyzed_dependency_scopes();
        self.workspace_navigation = None;
        self.read_only_workspace_navigation = None;
    }

    fn ensure_dependency_capacity(
        &self,
        keys: &[RetainedPackageKey],
        revision_keys: &[DocumentationRevisionKey],
    ) -> Result<(), ResourceCapacityError> {
        let new_keys = keys
            .iter()
            .filter(|key| !self.retained_package_keys.contains(key))
            .collect::<BTreeSet<_>>();
        if self.retained_package_keys.len() + new_keys.len() > self.retained_package_capacity() {
            return Err(ResourceCapacityError);
        }
        let new_revision_keys = revision_keys
            .iter()
            .filter(|key| !self.package_docs.contains_key(key))
            .collect::<BTreeSet<_>>();
        if self.package_docs.len() + new_revision_keys.len()
            > self.retained_documentation_revision_capacity()
        {
            return Err(ResourceCapacityError);
        }
        Ok(())
    }

    fn retain_dependency(&mut self, snapshot: DependencyResources) -> DirectDependencySnapshot {
        let key = snapshot.key.clone();
        let is_new_package = self.retained_package_keys.insert(key.clone());
        for resource in snapshot.resources {
            if (is_new_package || !resource.uri.starts_with("veln-pkg:"))
                && !self.combined_by_uri.contains_key(&resource.uri)
            {
                self.combined_by_uri
                    .insert(resource.uri.clone(), resource.clone());
                self.combined_resources.push(resource);
            }
        }
        self.select_dependency_documentation(
            key,
            snapshot.revision_key,
            PackageDocumentation::Ready(snapshot.package_doc_result),
        );
        snapshot.navigation
    }

    fn select_dependency_documentation(
        &mut self,
        key: RetainedPackageKey,
        revision_key: DocumentationRevisionKey,
        documentation: PackageDocumentation,
    ) {
        if let Some(previous_revision) = self.current_package_docs.insert(key, revision_key.clone())
            && let Some(uri) = self
                .package_docs
                .get(&previous_revision)
                .and_then(PackageDocumentation::listed_uri)
        {
            self.current_listed_documentation_uris.remove(uri);
        }
        if let Some(uri) = documentation.listed_uri() {
            self.current_listed_documentation_uris
                .insert(uri.to_string());
        }
        self.package_docs.insert(revision_key, documentation);
    }

    pub(crate) fn list_result(&self) -> Value {
        json!({
            "resources": self.combined_resources.iter().filter(|resource| {
                resource.listed && (!resource.uri.starts_with("veln-doc:///package/")
                    || self.current_listed_documentation_uris.contains(&resource.uri))
            }).map(PublishedResource::metadata).collect::<Vec<_>>()
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
        self.current_package_docs
            .iter()
            .filter_map(|(key, revision)| {
                self.package_docs
                    .get(revision)
                    .map(|documentation| documentation.search_candidates(key))
            })
            .flatten()
            .collect()
    }

    #[cfg(test)]
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

    pub(crate) fn navigation_snapshot(
        &mut self,
        files: Vec<veln_source::SourceFile>,
        captured_dependencies: &[CapturedDependencyProject],
        analysis_scope: DependencyAnalysisScope,
        workspace_key: Value,
    ) -> Result<Arc<EffectiveProjectSnapshot>, ResourceCapacityError> {
        // The stable capture key includes both workspace files and dependency
        // contents, so a match also proves that their resources were admitted.
        if let Some(cached) = &self.workspace_navigation
            && cached.workspace_key == workspace_key
        {
            return Ok(Arc::clone(&cached.snapshot));
        }

        let (dependencies, files) =
            match self.dependency_analysis_selection(&analysis_scope, captured_dependencies) {
                DependencyAnalysisSelection::Admitted(dependencies) => (
                    AdmittedDependencies {
                        keys: dependencies.keys,
                        revision_keys: dependencies.documentation_revisions,
                        snapshots: dependencies.snapshots,
                    },
                    files,
                ),
                DependencyAnalysisSelection::Rejected(Some(accepted)) => (
                    AdmittedDependencies {
                        keys: accepted.dependencies.keys.clone(),
                        revision_keys: accepted.dependencies.documentation_revisions.clone(),
                        snapshots: accepted.dependencies.snapshots.clone(),
                    },
                    accepted.workspace_files,
                ),
                DependencyAnalysisSelection::Rejected(None) => (
                    AdmittedDependencies {
                        keys: Vec::new(),
                        revision_keys: Vec::new(),
                        snapshots: Vec::new(),
                    },
                    Vec::new(),
                ),
                DependencyAnalysisSelection::Unanalyzed => {
                    (self.admit_dependencies(captured_dependencies)?, files)
                }
            };
        let dependency_keys = dependencies.keys;
        let documentation_revisions = dependencies.revision_keys;
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
                let snapshot = self
                    .standard_library_navigation
                    .with_direct_dependency_layer(dependencies.snapshots);
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
        self.workspace_navigation = Some(WorkspaceNavigation {
            workspace_key,
            documentation_revisions,
            snapshot: Arc::clone(&snapshot),
        });
        Ok(snapshot)
    }

    pub(crate) fn read_only_navigation_snapshot(
        &mut self,
        files: Vec<veln_source::SourceFile>,
        captured_dependencies: &[CapturedDependencyProject],
        analysis_scope: DependencyAnalysisScope,
        workspace_key: Value,
    ) -> Arc<EffectiveProjectSnapshot> {
        if let Some((cached_workspace_key, snapshot)) = &self.read_only_workspace_navigation
            && cached_workspace_key == &workspace_key
        {
            return Arc::clone(snapshot);
        }

        let (dependencies, files) = match self
            .dependency_analysis_selection(&analysis_scope, captured_dependencies)
        {
            DependencyAnalysisSelection::Admitted(dependencies) => (dependencies.snapshots, files),
            DependencyAnalysisSelection::Rejected(Some(accepted)) => (
                accepted.dependencies.snapshots.clone(),
                accepted.workspace_files,
            ),
            DependencyAnalysisSelection::Rejected(None) => (Vec::new(), Vec::new()),
            DependencyAnalysisSelection::Unanalyzed => (
                captured_dependencies
                    .iter()
                    .filter_map(captured_dependency_navigation)
                    .collect::<Vec<_>>(),
                files,
            ),
        };
        let snapshot = if dependencies.is_empty() {
            self.with_standard_library_navigation(files)
        } else {
            self.standard_library_navigation
                .with_direct_dependency_layer(dependencies)
                .with_workspace_overlays(files)
        };
        #[cfg(test)]
        WORKSPACE_NAVIGATION_BUILDS.set(WORKSPACE_NAVIGATION_BUILDS.get() + 1);
        let snapshot = Arc::new(snapshot);
        self.read_only_workspace_navigation = Some((workspace_key, Arc::clone(&snapshot)));
        snapshot
    }

    pub(crate) fn package_documentation_uri_for(
        &self,
        location: &veln_language_service::NavigationLocation,
    ) -> Option<&str> {
        let workspace_revisions = self
            .workspace_navigation
            .as_ref()
            .map(|navigation| navigation.documentation_revisions.as_slice())
            .unwrap_or_default();
        workspace_revisions
            .iter()
            .filter_map(|revision| self.package_docs.get(revision))
            .chain(
                self.current_package_docs
                    .iter()
                    .filter(|(key, _)| key.identity == "std")
                    .filter_map(|(_, revision)| self.package_docs.get(revision)),
            )
            .find_map(|documentation| documentation.declaration_uri_for_location(location))
    }

    fn dependency_analysis_selection(
        &self,
        scope: &DependencyAnalysisScope,
        dependencies: &[CapturedDependencyProject],
    ) -> DependencyAnalysisSelection {
        let revision = dependency_revision_key(dependencies);
        if let Some(state) = self.analysis_scopes.get(scope) {
            if state.rejected {
                return DependencyAnalysisSelection::Rejected(state.accepted.clone());
            }
            if let Some(accepted) = &state.accepted
                && accepted.dependencies.revision == revision
            {
                return DependencyAnalysisSelection::Admitted(
                    accepted.dependencies.as_ref().clone(),
                );
            }
            if self.rejected_dependency_revisions.contains(revision) {
                return DependencyAnalysisSelection::Rejected(state.accepted.clone());
            }
        } else if self.analysis_scope_history_saturated
            || self.rejected_dependency_revisions.contains(revision)
        {
            return DependencyAnalysisSelection::Rejected(None);
        }
        DependencyAnalysisSelection::Unanalyzed
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResourceCapacityError;

#[derive(Debug)]
pub(crate) struct AdmittedDependencies {
    keys: Vec<RetainedPackageKey>,
    revision_keys: Vec<DocumentationRevisionKey>,
    snapshots: Vec<DirectDependencySnapshot>,
}

#[cfg(test)]
#[path = "language_resources_tests.rs"]
mod standard_library_tests;
