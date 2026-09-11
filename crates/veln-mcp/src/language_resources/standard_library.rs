#[cfg(test)]
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use veln_language_service::{DirectDependencySnapshot, VirtualSourceCatalog};
#[cfg(test)]
use veln_language_service::{
    PackageDocGeneratorContract, PackageDocResult, render_package_documentation,
};
use veln_project::{
    CapturedPackageSnapshot, PackageIdentity, PackageSnapshotSource,
    capture_embedded_package_snapshot,
};

use super::package_resources::{
    CheckedStandardLibraryDocumentation, DeclarationLocationKey, PackageDocumentation,
    PackageSearchCandidate, PackageSearchScope, PublishedResource, RetainedPackageKey,
    VELN_SOURCE_MEDIA_TYPE,
};

#[cfg(test)]
thread_local! {
    static STANDARD_LIBRARY_RESOURCE_BUILDS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn standard_library_resource_builds() -> usize {
    STANDARD_LIBRARY_RESOURCE_BUILDS.get()
}

#[derive(Clone, Debug)]
pub(crate) struct StandardLibraryResources {
    pub(crate) resources: Vec<PublishedResource>,
    pub(super) key: RetainedPackageKey,
    pub(super) snapshot: DirectDependencySnapshot,
    pub(super) package_documentation: PackageDocumentation,
}

impl StandardLibraryResources {
    pub(crate) fn checked() -> Result<Self, String> {
        static CHECKED: OnceLock<Result<StandardLibraryResources, String>> = OnceLock::new();

        CHECKED
            .get_or_init(Self::from_checked_embedded_inputs)
            .clone()
    }

    pub(super) fn from_checked_embedded_inputs() -> Result<Self, String> {
        #[cfg(test)]
        STANDARD_LIBRARY_RESOURCE_BUILDS.set(STANDARD_LIBRARY_RESOURCE_BUILDS.get() + 1);
        let bundle = veln_stdlib::package_bundle();
        let snapshot = capture_checked_snapshot(&bundle)?;
        let manifest = veln_project::parse_manifest_text("veln.toml", bundle.manifest);
        // The checked bundle is bound to these captured bytes by its snapshot digest and includes
        // derived documentation indexes, so MCP startup does not repeat package analysis.
        let checked =
            veln_repo_mcp_standard_library_docs::checked_bundle_for_snapshot(snapshot.digest())?;
        let navigation =
            DirectDependencySnapshot::from_validated_standard_library(snapshot.clone(), manifest)
                .map_err(|error| format!("validate embedded standard library snapshot: {error}"))?;
        let catalog = standard_library_catalog(&snapshot)?;
        let package_documentation =
            checked_documentation(checked.search_candidates, checked.declaration_locations);
        let resources = checked_resources(&snapshot, &catalog, checked.resources)?;
        Ok(Self::new(
            snapshot,
            navigation,
            resources,
            package_documentation,
        ))
    }

    #[cfg(test)]
    pub(super) fn from_embedded_inputs<'a>(
        manifest: &str,
        sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
    ) -> Result<Self, String> {
        Self::from_embedded_inputs_with_catalog_builder(manifest, sources, |identity, snapshot| {
            VirtualSourceCatalog::new([(identity, snapshot)])
                .map_err(|error| format!("build embedded standard library source catalog: {error}"))
        })
    }

    #[cfg(test)]
    pub(super) fn from_embedded_inputs_with_catalog_builder<'a>(
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

    #[cfg(test)]
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
        let navigation =
            DirectDependencySnapshot::from_validated_standard_library(snapshot.clone(), manifest)
                .map_err(|error| format!("validate embedded standard library snapshot: {error}"))?;
        let catalog = catalog_builder(PackageIdentity::embedded_standard(), snapshot.clone())?;
        let mut resources = standard_library_source_resources(&snapshot, &catalog)?;
        resources.extend(documentation_resources);
        Ok(Self::new(
            snapshot,
            navigation,
            resources,
            PackageDocumentation::Ready(package_doc_result),
        ))
    }

    fn new(
        snapshot: CapturedPackageSnapshot,
        navigation: DirectDependencySnapshot,
        resources: Vec<PublishedResource>,
        package_documentation: PackageDocumentation,
    ) -> Self {
        Self {
            resources,
            key: RetainedPackageKey {
                identity: PackageIdentity::embedded_standard().as_str().to_string(),
                digest: snapshot.digest().to_string(),
            },
            snapshot: navigation,
            package_documentation,
        }
    }
}

fn capture_checked_snapshot(
    bundle: &veln_stdlib::StdlibPackage,
) -> Result<CapturedPackageSnapshot, String> {
    capture_embedded_package_snapshot(
        bundle.manifest.as_bytes(),
        bundle
            .files
            .iter()
            .map(|file| PackageSnapshotSource::new(file.path, file.text.as_bytes())),
    )
    .map_err(|error| format!("capture embedded standard library snapshot: {error}"))
}

fn standard_library_catalog(
    snapshot: &CapturedPackageSnapshot,
) -> Result<VirtualSourceCatalog, String> {
    VirtualSourceCatalog::new([(PackageIdentity::embedded_standard(), snapshot.clone())])
        .map_err(|error| format!("build embedded standard library source catalog: {error}"))
}

fn checked_documentation(
    candidates: Vec<veln_repo_mcp_standard_library_docs::CheckedSearchCandidate>,
    locations: Vec<veln_repo_mcp_standard_library_docs::CheckedDeclarationLocation>,
) -> PackageDocumentation {
    let search_candidates = candidates
        .into_iter()
        .map(|candidate| PackageSearchCandidate {
            scope: PackageSearchScope::StandardLibrary,
            uri: candidate.uri,
            identifier: candidate.identifier,
            title: candidate.title,
            name: candidate.name,
            summary: candidate.summary,
            keywords: candidate.keywords,
            signature: candidate.signature,
            documentation: candidate.documentation,
        })
        .collect();
    let declaration_locations = locations
        .into_iter()
        .map(|location| {
            (
                DeclarationLocationKey {
                    source_uri: location.source_uri,
                    line: location.line,
                    column: location.column,
                    offset: location.offset,
                },
                location.declaration_uri,
            )
        })
        .collect::<BTreeMap<_, _>>();
    PackageDocumentation::CheckedStandardLibrary(Arc::new(CheckedStandardLibraryDocumentation {
        search_candidates,
        declaration_locations,
    }))
}

fn checked_resources(
    snapshot: &CapturedPackageSnapshot,
    catalog: &VirtualSourceCatalog,
    documentation: Vec<veln_repo_mcp_standard_library_docs::CheckedResource>,
) -> Result<Vec<PublishedResource>, String> {
    let mut resources = standard_library_source_resources(snapshot, catalog)?;
    resources.extend(
        documentation
            .into_iter()
            .map(PublishedResource::from_checked_package_doc),
    );
    Ok(resources)
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
