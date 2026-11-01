use sha2::{Digest, Sha256};
use veln_analysis::{CapturedDependencyIndex, CapturedDependencyProject};
use veln_language_service::{
    DirectDependencySnapshot, PackageDocDependencySnapshots, PackageDocGeneratorContract,
    PackageDocResult, VirtualSourceCatalog, render_package_documentation,
};
use veln_project::{
    CapturedPackageSnapshot, LowerHexBytes, PackageIdentity, PackageSnapshotSource, Project,
    ProjectManifest, capture_embedded_package_snapshot,
};

use super::package_resources::{
    DocumentationRevisionKey, PublishedResource, RetainedPackageKey, VELN_SOURCE_MEDIA_TYPE,
};

#[cfg(test)]
thread_local! {
    static DEPENDENCY_PREPARATION_WORK: Cell<DependencyPreparationWork> =
        const { Cell::new(DependencyPreparationWork {
            snapshot_captures: 0,
            captured_bytes: 0,
            package_doc_generations: 0,
            semantic_indexed_projects: 0,
            semantic_exact_probes: 0,
            semantic_local_root_probes: 0,
        }) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DependencyPreparationWork {
    pub(crate) snapshot_captures: usize,
    pub(crate) captured_bytes: usize,
    pub(crate) package_doc_generations: usize,
    pub(crate) semantic_indexed_projects: usize,
    pub(crate) semantic_exact_probes: usize,
    pub(crate) semantic_local_root_probes: usize,
}

#[cfg(test)]
pub(crate) fn reset_dependency_preparation_work() {
    DEPENDENCY_PREPARATION_WORK.set(DependencyPreparationWork::default());
}

#[cfg(test)]
pub(crate) fn dependency_preparation_work() -> DependencyPreparationWork {
    DEPENDENCY_PREPARATION_WORK.get()
}

#[cfg(test)]
pub(crate) fn reset_dependency_snapshot_captures() {
    reset_dependency_preparation_work();
}

#[cfg(test)]
pub(crate) fn dependency_snapshot_captures() -> usize {
    dependency_preparation_work().snapshot_captures
}

pub(super) struct DependencyResources {
    pub(super) key: RetainedPackageKey,
    pub(super) revision_key: DocumentationRevisionKey,
    pub(super) resources: Vec<PublishedResource>,
    pub(super) navigation: DirectDependencySnapshot,
    pub(super) package_doc_result: PackageDocResult,
}

struct PreparedDependency<'a> {
    identity: PackageIdentity,
    project: &'a Project,
    manifest: &'a ProjectManifest,
    snapshot: CapturedPackageSnapshot,
    navigation: DirectDependencySnapshot,
}

pub(super) struct PreparedDependencyResources<'a> {
    prepared: Vec<PreparedDependency<'a>>,
    package_doc_dependencies: PackageDocDependencySnapshots,
    dependency_index: CapturedDependencyIndex<'a>,
    context_digest: String,
}

impl<'a> PreparedDependencyResources<'a> {
    pub(super) fn new(dependencies: &'a [CapturedDependencyProject]) -> Self {
        let prepared = dependencies
            .iter()
            .filter_map(prepare_dependency)
            .collect::<Vec<_>>();
        let package_doc_dependencies = PackageDocDependencySnapshots::new(
            prepared
                .iter()
                .map(|dependency| (&dependency.identity, &dependency.snapshot)),
        );
        let context_digest = dependency_context_digest(dependencies);
        let dependency_index = CapturedDependencyIndex::new(dependencies);
        #[cfg(test)]
        DEPENDENCY_PREPARATION_WORK.set({
            let mut work = DEPENDENCY_PREPARATION_WORK.get();
            work.semantic_indexed_projects += dependency_index.work().indexed_projects;
            work
        });
        Self {
            prepared,
            package_doc_dependencies,
            dependency_index,
            context_digest,
        }
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = RetainedPackageKey> + '_ {
        self.prepared.iter().map(|dependency| RetainedPackageKey {
            identity: dependency.identity.as_str().to_string(),
            digest: dependency.snapshot.digest().to_string(),
        })
    }

    pub(super) fn revision_keys(&self) -> impl Iterator<Item = DocumentationRevisionKey> + '_ {
        self.keys().map(|package| DocumentationRevisionKey {
            package,
            context_digest: self.context_digest.clone(),
        })
    }

    pub(super) fn into_resources(self) -> Vec<DependencyResources> {
        let resources = self
            .prepared
            .into_iter()
            .filter_map(|dependency| {
                dependency_resource(
                    dependency,
                    &self.dependency_index,
                    &self.package_doc_dependencies,
                    &self.context_digest,
                )
            })
            .collect();
        #[cfg(test)]
        DEPENDENCY_PREPARATION_WORK.set({
            let semantic = self.dependency_index.work();
            let mut work = DEPENDENCY_PREPARATION_WORK.get();
            work.semantic_exact_probes += semantic.exact_probes;
            work.semantic_local_root_probes += semantic.local_root_probes;
            work
        });
        resources
    }
}

fn prepare_dependency(dependency: &CapturedDependencyProject) -> Option<PreparedDependency<'_>> {
    let identity = PackageIdentity::new(&dependency.package).ok()?;
    let project = dependency.project.as_ref()?;
    let manifest = project.manifest.as_ref()?;
    let snapshot = captured_dependency_snapshot(project, &manifest.source_bytes)?;
    let navigation = DirectDependencySnapshot::from_validated_manifest(
        &identity,
        snapshot.clone(),
        manifest.clone(),
    )
    .ok()?;
    Some(PreparedDependency {
        identity,
        project,
        manifest,
        snapshot,
        navigation,
    })
}

fn dependency_resource(
    dependency: PreparedDependency<'_>,
    dependency_index: &CapturedDependencyIndex<'_>,
    package_doc_dependencies: &PackageDocDependencySnapshots,
    context_digest: &str,
) -> Option<DependencyResources> {
    let PreparedDependency {
        identity,
        project,
        manifest,
        snapshot,
        navigation,
    } = dependency;
    #[cfg(test)]
    DEPENDENCY_PREPARATION_WORK.set({
        let mut work = DEPENDENCY_PREPARATION_WORK.get();
        work.package_doc_generations += 1;
        work
    });
    let package_doc_result = PackageDocResult::generate_with_dependency_index_and_snapshots(
        &identity,
        &snapshot,
        manifest,
        &project.root,
        dependency_index,
        package_doc_dependencies,
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
        revision_key: DocumentationRevisionKey {
            package: RetainedPackageKey {
                identity: identity.as_str().to_string(),
                digest: snapshot.digest().to_string(),
            },
            context_digest: context_digest.to_string(),
        },
        resources,
        navigation,
        package_doc_result,
    })
}

fn dependency_context_digest(dependencies: &[CapturedDependencyProject]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"veln-mcp-package-documentation-context/v1\0");
    hash_context_field(
        &mut hasher,
        veln_repo_mcp_standard_library_docs::GENERATOR_CONTRACT.as_bytes(),
    );
    hash_context_count(&mut hasher, dependencies.len());
    for dependency in dependencies {
        hash_context_field(&mut hasher, dependency.package.as_bytes());
        hash_context_field(&mut hasher, dependency.source.as_bytes());
        match &dependency.project {
            None => hasher.update([0]),
            Some(project) => {
                hasher.update([1]);
                match &project.manifest {
                    None => hasher.update([0]),
                    Some(manifest) => {
                        hasher.update([1]);
                        hash_context_field(&mut hasher, &manifest.source_bytes);
                    }
                }
                hash_context_count(&mut hasher, project.files.len());
                for source in &project.files {
                    hash_context_field(&mut hasher, source.path().as_str().as_bytes());
                    hash_context_field(&mut hasher, source.text().as_bytes());
                }
            }
        }
    }
    let digest = hasher.finalize();
    format!("{:x}", LowerHexBytes(&digest))
}

fn hash_context_field(hasher: &mut Sha256, bytes: &[u8]) {
    hash_context_count(hasher, bytes.len());
    hasher.update(bytes);
}

fn hash_context_count(hasher: &mut Sha256, count: usize) {
    hasher.update(
        u64::try_from(count)
            .expect("captured dependency context length fits u64")
            .to_be_bytes(),
    );
}

pub(super) fn captured_dependency_navigation(
    dependency: &CapturedDependencyProject,
) -> Option<DirectDependencySnapshot> {
    let identity = PackageIdentity::new(&dependency.package).ok()?;
    let project = dependency.project.as_ref()?;
    let manifest = project.manifest.clone()?;
    let snapshot = captured_dependency_snapshot(project, &manifest.source_bytes)?;
    DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest).ok()
}

fn captured_dependency_snapshot(
    project: &veln_project::Project,
    manifest_source: &[u8],
) -> Option<CapturedPackageSnapshot> {
    let sources = project
        .files
        .iter()
        .map(|source| PackageSnapshotSource::new(source.path().as_str(), source.text().as_bytes()));
    let snapshot = capture_embedded_package_snapshot(manifest_source, sources).ok()?;
    #[cfg(test)]
    DEPENDENCY_PREPARATION_WORK.set({
        let mut work = DEPENDENCY_PREPARATION_WORK.get();
        work.snapshot_captures += 1;
        work.captured_bytes += snapshot.manifest_bytes().len()
            + snapshot
                .sources()
                .iter()
                .map(|source| source.bytes().len())
                .sum::<usize>();
        work
    });
    Some(snapshot)
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
#[cfg(test)]
use std::cell::Cell;
