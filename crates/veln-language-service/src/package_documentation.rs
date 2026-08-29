use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};
use veln_analysis::{DoctestMode, analyze_project, derive_source_module_path};
use veln_diagnostics::{Diagnostic, DiagnosticKind, Severity};
use veln_project::{
    CapturedPackageSnapshot, PackageIdentity, Project, ProjectManifest, classify_companion_source,
    parse_manifest_text,
};
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    ContractClause, ContractKind, FunctionDecl, FunctionKind, ParseDiagnostic, PublicAliasDecl,
    PublicAliasKind, SchemaDecl, SyntaxItem, TokenKind, TypeDecl, TypeVariantDecl, Visibility,
    canonical_type_text, lex, parse,
};
use veln_test::{ExpectedOutput, doctest_sources, visible_doctests};

use crate::{NavigationLocation, NavigationSource};

const DOC_DOMAIN: &[u8] = b"veln-package-doc-catalog/v1\0";
const MODULE_ID_DOMAIN: &[u8] = b"veln-package-doc-module-id/v1\0";
const DECLARATION_ID_DOMAIN: &[u8] = b"veln-package-doc-declaration-id/v1\0";
const URI_PREFIX: &str = "veln-doc:///package/";
const SCHEMA_VERSION: &str = "veln-package-doc-catalog/v1";
const SNAPSHOT_MANIFEST_PATH: &str = "veln.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocGeneratorContract {
    version: String,
}

impl PackageDocGeneratorContract {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocResult {
    identity: String,
    snapshot_digest: String,
    doc_digest: String,
    canonical_bytes: Vec<u8>,
    status_uri: String,
    kind: PackageDocResultKind,
    declaration_locations: BTreeMap<PackageDocLocationKey, String>,
}

impl PackageDocResult {
    pub fn generate(
        identity: &PackageIdentity,
        snapshot: &CapturedPackageSnapshot,
        manifest: &ProjectManifest,
        generator_contract: PackageDocGeneratorContract,
    ) -> Self {
        let manifest = if manifest.source_bytes == snapshot.manifest_bytes() {
            let text = std::str::from_utf8(snapshot.manifest_bytes())
                .expect("captured package manifest text is valid UTF-8");
            parse_manifest_text(SNAPSHOT_MANIFEST_PATH, text)
        } else {
            manifest.clone()
        };
        PackageDocBuilder::new(identity.as_str(), snapshot, &manifest, generator_contract)
            .generate()
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn snapshot_digest(&self) -> &str {
        &self.snapshot_digest
    }

    pub fn doc_digest(&self) -> &str {
        &self.doc_digest
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub fn status_uri(&self) -> &str {
        &self.status_uri
    }

    pub fn kind(&self) -> &PackageDocResultKind {
        &self.kind
    }

    pub fn catalog(&self) -> Option<&PackageDocCatalog> {
        match &self.kind {
            PackageDocResultKind::Catalog(catalog) => Some(catalog),
            PackageDocResultKind::Status(_) => None,
        }
    }

    pub fn status(&self) -> &PackageDocGenerationStatus {
        match &self.kind {
            PackageDocResultKind::Catalog(catalog) => &catalog.status,
            PackageDocResultKind::Status(status) => status,
        }
    }

    pub fn declaration_uri_for(
        &self,
        module_name: &str,
        declaration_kind: &str,
        declaration_name: &str,
    ) -> Option<&str> {
        let catalog = self.catalog()?;
        catalog
            .modules
            .iter()
            .find(|module| module.name == module_name)?
            .declarations
            .iter()
            .find(|declaration| {
                declaration.kind == declaration_kind && declaration.name == declaration_name
            })
            .map(|declaration| declaration.uri.as_str())
    }

    pub fn declaration_uri_for_location(&self, location: &NavigationLocation) -> Option<&str> {
        let NavigationSource::Package { uri: source_uri } = &location.source else {
            return None;
        };
        self.declaration_locations
            .get(&PackageDocLocationKey::new(source_uri, &location.span))
            .map(String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageDocResultKind {
    Catalog(Box<PackageDocCatalog>),
    Status(PackageDocGenerationStatus),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocCatalog {
    pub schema_version: String,
    pub generator_contract: String,
    pub package_identity: String,
    pub snapshot_digest: String,
    pub metadata: PackageDocMetadata,
    pub index_uri: String,
    pub status_uri: String,
    pub modules: Vec<PackageDocModule>,
    pub status: PackageDocGenerationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PackageDocMetadata {
    pub identity: String,
    pub package_name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub authors: Vec<String>,
    pub keywords: Vec<String>,
    pub exported_modules: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocModule {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub uri: String,
    pub doc: Vec<String>,
    pub doctests: Vec<PackageDocDoctest>,
    pub references: Vec<PackageDocReference>,
    pub declarations: Vec<PackageDocDeclaration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocDeclaration {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub uri: String,
    pub doc: Vec<String>,
    pub contracts: Vec<PackageDocFunctionContract>,
    pub constructors: Vec<PackageDocTypeConstructor>,
    pub alias: Option<PackageDocAlias>,
    pub doctests: Vec<PackageDocDoctest>,
    pub references: Vec<PackageDocReference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocTypeConstructor {
    pub name: String,
    pub signature: String,
    pub doc: Vec<String>,
    pub doctests: Vec<PackageDocDoctest>,
    pub references: Vec<PackageDocReference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocReference {
    pub kind: String,
    pub marker: String,
    pub target_declaration_id: String,
    pub target_uri: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocAlias {
    pub kind: String,
    pub target: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocFunctionContract {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocDoctest {
    pub kind: String,
    pub code: String,
    pub expected_error: Option<String>,
    pub should_fail: bool,
    pub expected_output: Vec<PackageDocExpectedOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocExpectedOutput {
    pub stream: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocGenerationStatus {
    pub state: PackageDocGeneration,
    pub diagnostics: Vec<PackageDocDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageDocGeneration {
    Complete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocDiagnostic {
    pub gate: String,
    pub code: String,
    pub message: String,
    pub span: Option<PackageDocDiagnosticSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageDocDiagnosticSpan {
    pub source_uri: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl fmt::Display for PackageDocDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}: {}", self.gate, self.code, self.message)
    }
}

impl Error for PackageDocDiagnostic {}

struct PackageDocBuilder<'a> {
    identity: &'a str,
    snapshot: &'a CapturedPackageSnapshot,
    manifest: &'a ProjectManifest,
    generator_contract: PackageDocGeneratorContract,
    diagnostics: Vec<PackageDocDiagnostic>,
    #[cfg(test)]
    forced_declaration_id: Option<String>,
}

#[derive(Clone)]
struct ParsedPackageSource {
    source: SourceFile,
    tree: veln_syntax::SyntaxTree,
    module_name: String,
    exported: bool,
    source_uri: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PackageDocLocationKey {
    source_uri: String,
    line: usize,
    column: usize,
    offset: usize,
}

impl PackageDocLocationKey {
    fn new(source_uri: &str, span: &SourceSpan) -> Self {
        Self {
            source_uri: source_uri.to_string(),
            line: span.start.line,
            column: span.start.column,
            offset: span.start.offset,
        }
    }
}

#[derive(Clone, Debug)]
struct PublicSchemaDocTarget {
    declaration_id: String,
    target_uri: String,
}

struct SchemaDocResolver<'a> {
    sources: BTreeMap<String, &'a ParsedPackageSource>,
    schemas: BTreeMap<(String, String), PublicSchemaDocTarget>,
    aliases: BTreeMap<(String, String), Vec<String>>,
}

mod builder_declarations;
mod builder_generation;
mod builder_results;
mod diagnostics_and_doctests;
mod doctest_metadata;
mod identity_and_paths;
mod json_output;
mod schema_references;
mod signatures_and_docs;

use diagnostics_and_doctests::*;
use doctest_metadata::*;
use identity_and_paths::*;
use json_output::*;
use signatures_and_docs::*;

#[cfg(test)]
mod tests;
