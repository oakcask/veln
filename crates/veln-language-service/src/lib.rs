//! Editor- and transport-neutral definition and reference services for Veln.

mod navigation;
mod package_documentation;
mod uri_encoding;
mod virtual_source;

use uri_encoding::encoded_uri_segment;

pub use navigation::{
    NavigationLocation, NavigationResult, NavigationSource, RenameAffectedScope, RenameFailure,
    RenameFailureKind, RenameNameClass, RenameRequiredInitial, SelectedSymbol, SourcePosition,
    SymbolKind, navigate, validate_rename, validate_rename_in_snapshot,
};
pub use package_documentation::{
    PackageDocAlias, PackageDocCatalog, PackageDocDeclaration, PackageDocDiagnostic,
    PackageDocDiagnosticSpan, PackageDocDoctest, PackageDocExpectedOutput,
    PackageDocFunctionContract, PackageDocGeneration, PackageDocGenerationStatus,
    PackageDocGeneratorContract, PackageDocMetadata, PackageDocModule, PackageDocReference,
    PackageDocResult, PackageDocResultKind, PackageDocTypeConstructor,
};
pub use virtual_source::{VirtualSourceCatalog, VirtualSourceCatalogError, VirtualSourceEntry};

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, OnceLock};

use navigation::SymbolIndex;
use veln_project::{
    CapturedPackageSnapshot, CapturedPackageSource, PackageIdentity, ProjectManifest,
};
use veln_source::{SourceFile, SourcePath};

#[derive(Clone, Debug)]
pub struct EffectiveProjectSnapshot {
    sources: Vec<SourceFile>,
    direct_dependencies: Vec<DirectDependencySnapshot>,
    standard_library: Option<DirectDependencySnapshot>,
    navigation_index: OnceLock<Arc<SymbolIndex>>,
}

impl EffectiveProjectSnapshot {
    pub fn new(sources: Vec<SourceFile>) -> Self {
        Self {
            sources,
            direct_dependencies: Vec::new(),
            standard_library: None,
            navigation_index: OnceLock::new(),
        }
    }

    pub fn with_direct_dependencies(
        sources: Vec<SourceFile>,
        direct_dependencies: Vec<DirectDependencySnapshot>,
    ) -> Self {
        Self {
            sources,
            direct_dependencies,
            standard_library: None,
            navigation_index: OnceLock::new(),
        }
    }

    pub fn with_standard_library(mut self, standard_library: DirectDependencySnapshot) -> Self {
        self.standard_library = Some(standard_library);
        self
    }

    pub fn with_workspace_overlays(&self, overlays: impl IntoIterator<Item = SourceFile>) -> Self {
        let mut sources = self.sources.clone();
        for overlay in overlays {
            let source_path = overlay.path().as_str().to_string();
            if let Some(existing) = sources
                .iter_mut()
                .find(|source| source.path().as_str() == source_path)
            {
                *existing = overlay;
            } else {
                sources.push(overlay);
            }
        }
        sources.sort_by(|left, right| left.path().as_str().cmp(right.path().as_str()));
        Self {
            sources,
            direct_dependencies: self.direct_dependencies.clone(),
            standard_library: self.standard_library.clone(),
            navigation_index: OnceLock::new(),
        }
    }

    fn navigation_index(&self) -> Arc<SymbolIndex> {
        self.navigation_index
            .get_or_init(|| {
                Arc::new(SymbolIndex::new(
                    self.sources.clone(),
                    self.direct_dependencies.clone(),
                    self.standard_library.clone(),
                ))
            })
            .clone()
    }

    pub fn resolve_virtual_source(&self, uri: &str) -> Option<&[u8]> {
        self.direct_dependencies
            .iter()
            .find_map(|dependency| dependency.resolve_virtual_source(uri))
            .or_else(|| {
                self.standard_library
                    .as_ref()
                    .and_then(|standard_library| standard_library.resolve_virtual_source(uri))
            })
    }
}

#[derive(Clone, Debug)]
pub struct DirectDependencySnapshot {
    identity: PackageIdentity,
    snapshot: CapturedPackageSnapshot,
    exported_sources: BTreeSet<String>,
    virtual_sources: VirtualSourceCatalog,
    standard_library: bool,
}

impl DirectDependencySnapshot {
    pub fn from_validated_manifest(
        expected_identity: &PackageIdentity,
        snapshot: CapturedPackageSnapshot,
        manifest: ProjectManifest,
    ) -> Result<Self, DirectDependencySnapshotError> {
        let actual_identity = manifest
            .package
            .fields
            .iter()
            .find(|field| field.key == "name")
            .ok_or(DirectDependencySnapshotError::MissingPackageName)?;
        if actual_identity.value != expected_identity.as_str() {
            return Err(DirectDependencySnapshotError::PackageNameMismatch {
                expected: expected_identity.as_str().to_string(),
                actual: actual_identity.value.clone(),
            });
        }
        let exported_sources = manifest
            .lib
            .exports
            .into_iter()
            .map(|export| SourcePath::new(export.path).as_str().to_string())
            .collect();
        let virtual_sources =
            VirtualSourceCatalog::new([(expected_identity.clone(), snapshot.clone())])?;
        Ok(Self {
            identity: expected_identity.clone(),
            snapshot,
            exported_sources,
            virtual_sources,
            standard_library: false,
        })
    }

    pub fn from_validated_standard_library(
        snapshot: CapturedPackageSnapshot,
        manifest: ProjectManifest,
    ) -> Result<Self, DirectDependencySnapshotError> {
        let identity = PackageIdentity::embedded_standard();
        let mut standard_library = Self::from_validated_manifest(&identity, snapshot, manifest)?;
        standard_library.standard_library = true;
        Ok(standard_library)
    }

    fn indexed_sources(
        &self,
    ) -> impl Iterator<Item = (&CapturedPackageSource, &VirtualSourceEntry)> {
        self.snapshot
            .sources()
            .iter()
            .enumerate()
            .map(|(source_index, source)| {
                let entry = self
                    .virtual_sources
                    .entry_for_source(0, source_index)
                    .expect("direct dependency catalog contains every captured source");
                (source, entry)
            })
    }

    fn resolve_virtual_source(&self, uri: &str) -> Option<&[u8]> {
        self.virtual_sources.resolve(uri)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectDependencySnapshotError {
    MissingPackageName,
    PackageNameMismatch { expected: String, actual: String },
    VirtualSourceCatalog(VirtualSourceCatalogError),
}

impl fmt::Display for DirectDependencySnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPackageName => {
                write!(formatter, "direct dependency manifest has no package name")
            }
            Self::PackageNameMismatch { expected, actual } => write!(
                formatter,
                "direct dependency package name `{actual}` does not match requested package `{expected}`"
            ),
            Self::VirtualSourceCatalog(error) => error.fmt(formatter),
        }
    }
}

impl Error for DirectDependencySnapshotError {}

impl From<VirtualSourceCatalogError> for DirectDependencySnapshotError {
    fn from(error: VirtualSourceCatalogError) -> Self {
        Self::VirtualSourceCatalog(error)
    }
}

#[cfg(test)]
mod tests;
