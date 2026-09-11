use veln_analysis::CapturedDependencyProject;
use veln_language_service::{
    DirectDependencySnapshot, PackageDocGeneratorContract, PackageDocResult, VirtualSourceCatalog,
    render_package_documentation,
};
use veln_project::{
    CapturedPackageSnapshot, PackageIdentity, PackageSnapshotSource,
    capture_embedded_package_snapshot,
};

use super::package_resources::{PublishedResource, RetainedPackageKey, VELN_SOURCE_MEDIA_TYPE};

#[cfg(test)]
thread_local! {
    static DEPENDENCY_SNAPSHOT_CAPTURES: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_dependency_snapshot_captures() {
    DEPENDENCY_SNAPSHOT_CAPTURES.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_snapshot_captures() -> usize {
    DEPENDENCY_SNAPSHOT_CAPTURES.get()
}

pub(super) struct DependencyResources {
    pub(super) key: RetainedPackageKey,
    pub(super) resources: Vec<PublishedResource>,
    pub(super) navigation: DirectDependencySnapshot,
    pub(super) package_doc_result: PackageDocResult,
}

pub(super) fn dependency_resources(
    dependencies: &[CapturedDependencyProject],
) -> Vec<DependencyResources> {
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
    DEPENDENCY_SNAPSHOT_CAPTURES.set(DEPENDENCY_SNAPSHOT_CAPTURES.get() + 1);
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
#[cfg(test)]
use std::cell::Cell;
