use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};
use veln_diagnostics::{Diagnostic, Severity};
use veln_project::{CapturedPackageSnapshot, ProjectManifest, classify_companion_source};
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    ContractClause, ContractKind, FunctionDecl, FunctionKind, ParseDiagnostic, PublicAliasDecl,
    PublicAliasKind, SchemaDecl, SyntaxItem, TypeDecl, TypeVariantDecl, Visibility,
    canonical_type_text, parse,
};

use crate::{NavigationLocation, NavigationSource};

const DOC_DOMAIN: &[u8] = b"veln-package-doc-catalog/v1\0";
const MODULE_ID_DOMAIN: &[u8] = b"veln-package-doc-module-id/v1\0";
const DECLARATION_ID_DOMAIN: &[u8] = b"veln-package-doc-declaration-id/v1\0";
const URI_PREFIX: &str = "veln-doc:///package/";
const SCHEMA_VERSION: &str = "veln-package-doc-catalog/v1";

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
        identity: &str,
        snapshot: &CapturedPackageSnapshot,
        manifest: &ProjectManifest,
        generator_contract: PackageDocGeneratorContract,
    ) -> Self {
        PackageDocBuilder::new(identity, snapshot, manifest, generator_contract).generate()
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
        let source_uri = match &location.source {
            NavigationSource::Package { uri } => uri.clone(),
            NavigationSource::Workspace => source_uri(
                self.identity(),
                self.snapshot_digest(),
                location.span.file.as_str(),
            ),
        };
        self.declaration_locations
            .get(&PackageDocLocationKey::new(&source_uri, &location.span))
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
    pub expected_output: Vec<String>,
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

impl<'a> PackageDocBuilder<'a> {
    fn new(
        identity: &'a str,
        snapshot: &'a CapturedPackageSnapshot,
        manifest: &'a ProjectManifest,
        generator_contract: PackageDocGeneratorContract,
    ) -> Self {
        Self {
            identity,
            snapshot,
            manifest,
            generator_contract,
            diagnostics: Vec::new(),
            #[cfg(test)]
            forced_declaration_id: None,
        }
    }

    #[cfg(test)]
    fn with_forced_declaration_id(mut self, id: impl Into<String>) -> Self {
        self.forced_declaration_id = Some(id.into());
        self
    }

    fn generate(mut self) -> PackageDocResult {
        self.validate_manifest_gate();
        let metadata = self.metadata();
        let parsed_sources = self.parse_sources();
        self.validate_manifest_exports(&parsed_sources);
        self.validate_doctests(&parsed_sources);
        let schema_targets = self.public_schema_targets(&parsed_sources);
        self.validate_doc_references(&parsed_sources, &schema_targets);

        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        let mut semantic_identities = BTreeMap::new();
        let mut declaration_locations = BTreeMap::new();
        let mut modules = Vec::new();
        for source in parsed_sources.iter().filter(|source| source.exported) {
            let module_id = digest_hex(MODULE_ID_DOMAIN, &[source.module_name.as_bytes()]);
            let declarations = self.declarations(
                source,
                &mut semantic_identities,
                &mut declaration_locations,
                &schema_targets,
            );
            modules.push(PackageDocModule {
                uri: self.module_uri(&module_id, ""),
                id: module_id,
                name: source.module_name.clone(),
                source_path: source.source.path().as_str().to_string(),
                doc: doc_block_before(&source.source, 1),
                declarations,
            });
        }

        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        let mut declaration_ids = BTreeSet::new();
        for declaration in modules.iter().flat_map(|module| module.declarations.iter()) {
            if !declaration_ids.insert(declaration.id.clone()) {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "identity".to_string(),
                    code: "package_doc.declaration_id_collision".to_string(),
                    message: format!(
                        "declaration documentation identifier collision `{}`",
                        declaration.id
                    ),
                    span: None,
                });
            }
        }
        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        let mut module_ids = BTreeSet::new();
        for module in &modules {
            if !module_ids.insert(module.id.clone()) {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "identity".to_string(),
                    code: "package_doc.module_id_collision".to_string(),
                    message: format!("module documentation identifier collision `{}`", module.id),
                    span: None,
                });
            }
        }
        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        let mut catalog = PackageDocCatalog {
            schema_version: SCHEMA_VERSION.to_string(),
            generator_contract: self.generator_contract.version().to_string(),
            package_identity: self.identity.to_string(),
            snapshot_digest: self.snapshot.digest().to_string(),
            metadata,
            index_uri: String::new(),
            status_uri: String::new(),
            modules,
            status: PackageDocGenerationStatus {
                state: PackageDocGeneration::Complete,
                diagnostics: Vec::new(),
            },
        };
        let canonical_bytes = catalog_to_json(&catalog).into_bytes();
        let final_doc_digest = doc_digest(&canonical_bytes);
        catalog.index_uri = self.index_uri(&final_doc_digest);
        catalog.status_uri = self.status_uri(&final_doc_digest);
        for module in &mut catalog.modules {
            module.uri = self.module_uri(&module.id, &final_doc_digest);
            for declaration in &mut module.declarations {
                declaration.uri = self.declaration_uri(&declaration.id, &final_doc_digest);
                for reference in &mut declaration.references {
                    reference.target_uri =
                        self.declaration_uri(&reference.target_declaration_id, &final_doc_digest);
                }
            }
        }
        for declaration_uri in declaration_locations.values_mut() {
            *declaration_uri = self.declaration_uri(declaration_uri, &final_doc_digest);
        }
        PackageDocResult {
            identity: self.identity.to_string(),
            snapshot_digest: self.snapshot.digest().to_string(),
            status_uri: self.status_uri(&final_doc_digest),
            doc_digest: final_doc_digest,
            canonical_bytes,
            kind: PackageDocResultKind::Catalog(Box::new(catalog)),
            declaration_locations,
        }
    }

    fn metadata(&self) -> PackageDocMetadata {
        let mut metadata = PackageDocMetadata {
            identity: self.identity.to_string(),
            package_name: manifest_field(&self.manifest.package.fields, "name"),
            version: manifest_field(&self.manifest.package.fields, "version"),
            description: manifest_field(&self.manifest.package.fields, "description"),
            license: manifest_field(&self.manifest.package.fields, "license"),
            authors: manifest_list_field(&self.manifest.package.fields, "authors"),
            keywords: manifest_list_field(&self.manifest.package.fields, "keywords"),
            exported_modules: Vec::new(),
        };
        metadata.exported_modules = self
            .manifest
            .lib
            .exports
            .iter()
            .map(|export| module_name_from_path(&export.path).unwrap_or_default())
            .collect();
        metadata
    }

    fn parse_sources(&mut self) -> Vec<ParsedPackageSource> {
        let exported_paths = self
            .manifest
            .lib
            .exports
            .iter()
            .map(|export| SourcePath::new(export.path.clone()).as_str().to_string())
            .collect::<BTreeSet<_>>();
        let mut parsed = Vec::new();
        for source in self.snapshot.sources() {
            let text = std::str::from_utf8(source.bytes())
                .expect("captured package source text is valid UTF-8");
            let source_file = SourceFile::new(source.path(), text);
            let source_uri = source_uri(self.identity, self.snapshot.digest(), source.path());
            let output = parse(&source_file);
            for diagnostic in output.diagnostics {
                self.diagnostics.push(parse_diagnostic(
                    "parse",
                    diagnostic,
                    self.identity,
                    self.snapshot.digest(),
                ));
            }
            let module_name = explicit_module_name(text)
                .or_else(|| module_name_from_path(source.path()))
                .unwrap_or_default();
            parsed.push(ParsedPackageSource {
                source: source_file,
                tree: output.tree,
                module_name,
                exported: exported_paths.contains(source.path()),
                source_uri,
            });
        }
        parsed
    }

    fn validate_manifest_exports(&mut self, parsed_sources: &[ParsedPackageSource]) {
        let available = parsed_sources
            .iter()
            .map(|source| source.source.path().as_str().to_string())
            .collect::<BTreeSet<_>>();
        let mut exports = BTreeSet::new();
        let mut exported_modules = BTreeMap::new();
        for export in self.manifest.lib.exports.clone() {
            let Some((path, module_name)) = self.validated_manifest_export(&export) else {
                continue;
            };
            if !exports.insert(path.clone()) {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "export".to_string(),
                    code: "package_doc.duplicate_export".to_string(),
                    message: format!("duplicate documentation export `{path}`"),
                    span: Some(PackageDocDiagnosticSpan::from_span(
                        &source_uri(
                            self.identity,
                            self.snapshot.digest(),
                            self.manifest.path.as_str(),
                        ),
                        &export.path_span,
                    )),
                });
                continue;
            }
            if let Some(first_span) =
                exported_modules.insert(module_name.clone(), export.path_span.clone())
            {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "manifest".to_string(),
                    code: "package_doc.duplicate_exported_module".to_string(),
                    message: format!(
                        "manifest export `{path}` duplicates module export `{module_name}`"
                    ),
                    span: Some(PackageDocDiagnosticSpan::from_span(
                        &source_uri(
                            self.identity,
                            self.snapshot.digest(),
                            self.manifest.path.as_str(),
                        ),
                        &first_span,
                    )),
                });
                continue;
            }
            if !available.contains(&path) {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "export".to_string(),
                    code: "package_doc.missing_export".to_string(),
                    message: format!(
                        "documentation export `{path}` is not in the package snapshot"
                    ),
                    span: Some(PackageDocDiagnosticSpan::from_span(
                        &source_uri(
                            self.identity,
                            self.snapshot.digest(),
                            self.manifest.path.as_str(),
                        ),
                        &export.path_span,
                    )),
                });
            }
        }
    }

    fn validate_manifest_gate(&mut self) {
        for section in &self.manifest.unsupported_sections {
            self.diagnostics.push(PackageDocDiagnostic {
                gate: "manifest".to_string(),
                code: "package_doc.unsupported_manifest_section".to_string(),
                message: format!(
                    "manifest section `[{}]` is not supported by package documentation generation",
                    section.name
                ),
                span: Some(PackageDocDiagnosticSpan::from_span(
                    &source_uri(
                        self.identity,
                        self.snapshot.digest(),
                        self.manifest.path.as_str(),
                    ),
                    &section.span,
                )),
            });
        }

        for dependency in &self.manifest.dependencies {
            if dependency.git.is_some() && dependency.selectors.is_empty() {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "manifest".to_string(),
                    code: "package_doc.missing_git_selector".to_string(),
                    message: format!(
                        "git dependency `{}` must specify exactly one selector: `rev`, `tag`, or `branch`",
                        dependency.package
                    ),
                    span: Some(PackageDocDiagnosticSpan::from_span(
                        &source_uri(
                            self.identity,
                            self.snapshot.digest(),
                            self.manifest.path.as_str(),
                        ),
                        &dependency.package_span,
                    )),
                });
            }
            for selector in dependency.selectors.iter().skip(1) {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "manifest".to_string(),
                    code: "package_doc.multiple_git_selectors".to_string(),
                    message: format!(
                        "git dependency `{}` specifies multiple selectors; use exactly one of `rev`, `tag`, or `branch`",
                        dependency.package
                    ),
                    span: Some(PackageDocDiagnosticSpan::from_span(
                        &source_uri(
                            self.identity,
                            self.snapshot.digest(),
                            self.manifest.path.as_str(),
                        ),
                        &selector.field.key_span,
                    )),
                });
            }
        }
    }

    fn validated_manifest_export(
        &mut self,
        export: &veln_project::ManifestExport,
    ) -> Option<(String, String)> {
        if export.path.contains("::") {
            self.invalid_manifest_export(
                export,
                "module paths are not valid manifest exports; use a package-relative source file path",
            );
            return None;
        }
        let path = SourcePath::new(export.path.clone());
        if !is_package_relative_path(path.as_str()) {
            self.invalid_manifest_export(export, "manifest exports must stay inside the package");
            return None;
        }
        if !path.as_str().ends_with(".veln") {
            self.invalid_manifest_export(export, "manifest exports must name `.veln` source files");
            return None;
        }
        if classify_companion_source(path.as_str()).is_some() {
            self.invalid_manifest_export(export, "export names a test companion");
            return None;
        }
        let Some(module_name) = module_name_from_path(path.as_str()) else {
            self.invalid_manifest_export(
                export,
                "manifest export path does not derive a valid module path",
            );
            return None;
        };
        Some((path.as_str().to_string(), module_name))
    }

    fn invalid_manifest_export(&mut self, export: &veln_project::ManifestExport, reason: &str) {
        self.diagnostics.push(PackageDocDiagnostic {
            gate: "manifest".to_string(),
            code: "package_doc.invalid_export".to_string(),
            message: format!("manifest export `{}` is invalid: {reason}", export.path),
            span: Some(PackageDocDiagnosticSpan::from_span(
                &source_uri(
                    self.identity,
                    self.snapshot.digest(),
                    self.manifest.path.as_str(),
                ),
                &export.path_span,
            )),
        });
    }

    fn validate_doctests(&mut self, parsed_sources: &[ParsedPackageSource]) {
        let exported_sources = parsed_sources
            .iter()
            .filter(|source| source.exported)
            .map(|source| source.source.clone())
            .collect::<Vec<_>>();
        if exported_sources.is_empty() {
            return;
        }
        let doctests = veln_test::doctest_sources(&exported_sources);
        for diagnostic in doctests.diagnostics {
            if diagnostic.severity == Severity::Error {
                self.diagnostics
                    .push(self.project_diagnostic("doctest", diagnostic));
            }
        }
        for source in parsed_sources.iter().filter(|source| source.exported) {
            for target_line in public_declaration_lines(&source.tree) {
                for fence in doctest_fences(&source.source, target_line) {
                    match fence {
                        Ok(doctest) => self.validate_doctest(&source.source, &doctest),
                        Err(diagnostic) => self.diagnostics.push(diagnostic),
                    }
                }
            }
        }
    }

    fn public_schema_targets(
        &self,
        parsed_sources: &[ParsedPackageSource],
    ) -> BTreeMap<String, PublicSchemaDocTarget> {
        let mut targets = BTreeMap::new();
        for source in parsed_sources.iter().filter(|source| source.exported) {
            for item in &source.tree.items {
                let SyntaxItem::Schema(schema) = item else {
                    continue;
                };
                if schema.visibility != Visibility::Public {
                    continue;
                }
                let name = schema.name.clone().unwrap_or_default();
                let identity = format!(
                    "schema:{}::{name}:{}",
                    source.module_name,
                    schema_signature(schema)
                );
                let declaration_id = self.declaration_id("schema", &identity);
                let target_uri = self.declaration_uri(&declaration_id, "");
                let target = PublicSchemaDocTarget {
                    declaration_id,
                    target_uri,
                };
                targets.insert(format!("{}::{name}", source.module_name), target.clone());
                targets.insert(name, target);
            }
        }
        targets
    }

    fn validate_doc_references(
        &mut self,
        parsed_sources: &[ParsedPackageSource],
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) {
        for source in parsed_sources.iter().filter(|source| source.exported) {
            for reference in doc_schema_references(&source.source) {
                if schema_reference_target(&reference.target, &source.module_name, schema_targets)
                    .is_none()
                {
                    self.diagnostics.push(PackageDocDiagnostic {
                        gate: "documentation_reference".to_string(),
                        code: "package_doc.unresolved_schema_reference".to_string(),
                        message: format!(
                            "documentation schema reference `{}` is not a public exported schema",
                            reference.target
                        ),
                        span: Some(PackageDocDiagnosticSpan::from_span(
                            &source.source_uri,
                            &reference.span,
                        )),
                    });
                }
            }
        }
    }

    fn declarations(
        &mut self,
        source: &ParsedPackageSource,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) -> Vec<PackageDocDeclaration> {
        let mut declarations = Vec::new();
        for item in &source.tree.items {
            match item {
                SyntaxItem::Type(type_decl) if type_decl.visibility == Visibility::Public => {
                    declarations.push(self.type_declaration(
                        source,
                        type_decl,
                        semantic_identities,
                        declaration_locations,
                        schema_targets,
                    ));
                }
                SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => {
                    declarations.push(self.schema_declaration(
                        source,
                        schema,
                        semantic_identities,
                        declaration_locations,
                        schema_targets,
                    ));
                }
                SyntaxItem::Function(function)
                    if function.kind == FunctionKind::Function
                        && function.visibility == Visibility::Public =>
                {
                    declarations.push(self.function_declaration(
                        source,
                        function,
                        semantic_identities,
                        declaration_locations,
                        schema_targets,
                    ));
                }
                SyntaxItem::PublicAlias(alias) => {
                    declarations.push(self.alias_declaration(
                        source,
                        alias,
                        semantic_identities,
                        declaration_locations,
                        schema_targets,
                    ));
                }
                _ => {}
            }
        }
        declarations
    }

    fn type_declaration(
        &mut self,
        source: &ParsedPackageSource,
        type_decl: &TypeDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) -> PackageDocDeclaration {
        let name = type_decl.name.clone().unwrap_or_default();
        let identity = format!(
            "type:{}::{name}:{}",
            source.module_name,
            type_signature(type_decl)
        );
        self.record_semantic_identity(&identity, &type_decl.span, semantic_identities);
        let declaration_id = self.declaration_id("type", &identity);
        declaration_locations.insert(
            PackageDocLocationKey::new(&source.source_uri, &type_decl.span),
            declaration_id.clone(),
        );
        for variant in &type_decl.variants {
            declaration_locations.insert(
                PackageDocLocationKey::new(&source.source_uri, &variant.span),
                declaration_id.clone(),
            );
        }
        PackageDocDeclaration {
            id: declaration_id,
            kind: "type".to_string(),
            name,
            signature: type_signature(type_decl),
            uri: String::new(),
            doc: doc_block_before(&source.source, type_decl.span.start.line),
            contracts: Vec::new(),
            constructors: type_decl
                .variants
                .iter()
                .filter(|variant| variant.visibility == Visibility::Public)
                .map(type_constructor)
                .collect(),
            alias: None,
            doctests: self.doctests_for(&source.source, type_decl.span.start.line),
            references: self.references_for(
                &source.source,
                type_decl.span.start.line,
                schema_targets,
            ),
        }
    }

    fn schema_declaration(
        &mut self,
        source: &ParsedPackageSource,
        schema: &SchemaDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) -> PackageDocDeclaration {
        let name = schema.name.clone().unwrap_or_default();
        let identity = format!(
            "schema:{}::{name}:{}",
            source.module_name,
            schema_signature(schema)
        );
        self.record_semantic_identity(&identity, &schema.span, semantic_identities);
        let declaration_id = self.declaration_id("schema", &identity);
        declaration_locations.insert(
            PackageDocLocationKey::new(&source.source_uri, &schema.span),
            declaration_id.clone(),
        );
        PackageDocDeclaration {
            id: declaration_id,
            kind: "schema".to_string(),
            name,
            signature: schema_signature(schema),
            uri: String::new(),
            doc: doc_block_before(&source.source, schema.span.start.line),
            contracts: Vec::new(),
            constructors: Vec::new(),
            alias: None,
            doctests: self.doctests_for(&source.source, schema.span.start.line),
            references: self.references_for(&source.source, schema.span.start.line, schema_targets),
        }
    }

    fn function_declaration(
        &mut self,
        source: &ParsedPackageSource,
        function: &FunctionDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) -> PackageDocDeclaration {
        let name = function.name.clone().unwrap_or_default();
        let signature = function_signature(function);
        let identity = format!("function:{}::{name}:{signature}", source.module_name);
        self.record_semantic_identity(&identity, &function.span, semantic_identities);
        let declaration_id = self.declaration_id("function", &identity);
        declaration_locations.insert(
            PackageDocLocationKey::new(&source.source_uri, &function.span),
            declaration_id.clone(),
        );
        PackageDocDeclaration {
            id: declaration_id,
            kind: "function".to_string(),
            name,
            signature,
            uri: String::new(),
            doc: doc_block_before(&source.source, function.span.start.line),
            contracts: function.contracts.iter().map(function_contract).collect(),
            constructors: Vec::new(),
            alias: None,
            doctests: self.doctests_for(&source.source, function.span.start.line),
            references: self.references_for(
                &source.source,
                function.span.start.line,
                schema_targets,
            ),
        }
    }

    fn alias_declaration(
        &mut self,
        source: &ParsedPackageSource,
        alias: &PublicAliasDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) -> PackageDocDeclaration {
        let name = alias.name.clone().unwrap_or_default();
        let signature = alias_signature(alias);
        let kind = alias_kind(alias.kind).to_string();
        let identity = format!("alias:{kind}:{}::{name}:{signature}", source.module_name);
        self.record_semantic_identity(&identity, &alias.span, semantic_identities);
        let declaration_id = self.declaration_id("alias", &identity);
        declaration_locations.insert(
            PackageDocLocationKey::new(&source.source_uri, &alias.span),
            declaration_id.clone(),
        );
        PackageDocDeclaration {
            id: declaration_id,
            kind: "alias".to_string(),
            name,
            signature,
            uri: String::new(),
            doc: doc_block_before(&source.source, alias.span.start.line),
            contracts: Vec::new(),
            constructors: Vec::new(),
            alias: Some(PackageDocAlias {
                kind,
                target: alias.target.clone(),
            }),
            doctests: self.doctests_for(&source.source, alias.span.start.line),
            references: self.references_for(&source.source, alias.span.start.line, schema_targets),
        }
    }

    fn record_semantic_identity(
        &mut self,
        identity: &str,
        span: &SourceSpan,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
    ) {
        if let Some(first) = semantic_identities.insert(identity.to_string(), span.clone()) {
            self.diagnostics.push(PackageDocDiagnostic {
                gate: "identity".to_string(),
                code: "package_doc.duplicate_semantic_identity".to_string(),
                message: format!("duplicate documentation semantic identity `{identity}`"),
                span: Some(PackageDocDiagnosticSpan::from_span(
                    &source_uri(self.identity, self.snapshot.digest(), first.file.as_str()),
                    span,
                )),
            });
        }
    }

    fn doctests_for(&mut self, source: &SourceFile, target_line: usize) -> Vec<PackageDocDoctest> {
        let mut doctests = Vec::new();
        for fence in doctest_fences(source, target_line) {
            match fence {
                Ok(doctest) => {
                    doctests.push(doctest);
                }
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        doctests
    }

    fn validate_doctest(&mut self, source: &SourceFile, doctest: &PackageDocDoctest) {
        if doctest.kind != "veln" {
            return;
        }
        let doctest_source = SourceFile::new(
            format!("{}#package-doc-doctest.veln", source.path().as_str()),
            doctest.code.clone(),
        );
        let diagnostics = parse(&doctest_source).diagnostics;
        if doctest.should_fail {
            if diagnostics.is_empty() {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "doctest".to_string(),
                    code: "doctest.expected_failure_missing".to_string(),
                    message: "negative doctest produced no error diagnostics".to_string(),
                    span: None,
                });
            }
        } else if let Some(diagnostic) = diagnostics.into_iter().next() {
            self.diagnostics.push(parse_diagnostic(
                "doctest",
                diagnostic,
                self.identity,
                self.snapshot.digest(),
            ));
        }
    }

    fn references_for(
        &self,
        source: &SourceFile,
        target_line: usize,
        schema_targets: &BTreeMap<String, PublicSchemaDocTarget>,
    ) -> Vec<PackageDocReference> {
        let current_module = explicit_module_name(source.text())
            .or_else(|| module_name_from_path(source.path().as_str()))
            .unwrap_or_default();
        doc_schema_references_before(source, target_line)
            .into_iter()
            .filter_map(|reference| {
                schema_reference_target(&reference.target, &current_module, schema_targets).map(
                    |target| PackageDocReference {
                        kind: "schema".to_string(),
                        marker: reference.target,
                        target_declaration_id: target.declaration_id.clone(),
                        target_uri: target.target_uri.clone(),
                    },
                )
            })
            .collect()
    }

    fn project_diagnostic(&self, gate: &str, diagnostic: Diagnostic) -> PackageDocDiagnostic {
        PackageDocDiagnostic {
            gate: gate.to_string(),
            code: diagnostic.id,
            message: diagnostic.message,
            span: diagnostic.span.as_ref().map(|span| {
                PackageDocDiagnosticSpan::from_span(
                    &source_uri(self.identity, self.snapshot.digest(), span.file.as_str()),
                    span,
                )
            }),
        }
    }

    fn failed_result(mut self) -> PackageDocResult {
        self.sort_diagnostics();
        let diagnostics = std::mem::take(&mut self.diagnostics);
        let status = PackageDocGenerationStatus {
            state: PackageDocGeneration::Failed,
            diagnostics,
        };
        let canonical_bytes = status_to_json(
            self.identity,
            self.snapshot.digest(),
            self.generator_contract.version(),
            &status,
        )
        .into_bytes();
        let doc_digest = doc_digest(&canonical_bytes);
        PackageDocResult {
            identity: self.identity.to_string(),
            snapshot_digest: self.snapshot.digest().to_string(),
            status_uri: self.status_uri(&doc_digest),
            doc_digest,
            canonical_bytes,
            kind: PackageDocResultKind::Status(status),
            declaration_locations: BTreeMap::new(),
        }
    }

    fn sort_diagnostics(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            left.span
                .cmp(&right.span)
                .then(left.code.cmp(&right.code))
                .then(left.message.cmp(&right.message))
        });
    }

    fn index_uri(&self, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/index",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    fn status_uri(&self, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/status",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    fn module_uri(&self, module_id: &str, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/module/{module_id}",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    fn declaration_uri(&self, declaration_id: &str, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/declaration/{declaration_id}",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    fn declaration_id(&self, kind: &str, identity: &str) -> String {
        #[cfg(test)]
        if let Some(id) = &self.forced_declaration_id {
            return id.clone();
        }

        declaration_id(kind, identity)
    }
}

impl PackageDocDiagnosticSpan {
    fn from_span(source_uri: &str, span: &SourceSpan) -> Self {
        Self {
            source_uri: source_uri.to_string(),
            line: span.start.line,
            column: span.start.column,
            offset: span.start.offset,
        }
    }
}

fn parse_diagnostic(
    gate: &str,
    diagnostic: ParseDiagnostic,
    identity: &str,
    snapshot_digest: &str,
) -> PackageDocDiagnostic {
    PackageDocDiagnostic {
        gate: gate.to_string(),
        code: diagnostic.id.to_string(),
        message: diagnostic.message,
        span: diagnostic.span.as_ref().map(|span| {
            PackageDocDiagnosticSpan::from_span(
                &source_uri(identity, snapshot_digest, span.file.as_str()),
                span,
            )
        }),
    }
}

fn function_signature(function: &FunctionDecl) -> String {
    let mut signature = String::from("fn ");
    signature.push_str(function.name.as_deref().unwrap_or("<anonymous>"));
    signature.push('(');
    signature.push_str(
        &function
            .params
            .iter()
            .map(|param| match &param.ty {
                Some(ty) if param.is_variadic => {
                    format!("{}: ...{}", param.name, canonical_type_text(ty))
                }
                Some(ty) => format!("{}: {}", param.name, canonical_type_text(ty)),
                None => param.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    signature.push(')');
    if let Some(return_type) = &function.return_type {
        signature.push_str(" -> ");
        if let Some(binding) = &function.return_binding {
            signature.push_str(&binding.name);
            signature.push_str(": ");
        }
        signature.push_str(&canonical_type_text(return_type));
    }
    if let Some(effects) = &function.effects {
        signature.push_str(" effects [");
        signature.push_str(&effects.join(", "));
        signature.push(']');
    }
    signature
}

fn public_declaration_lines(tree: &veln_syntax::SyntaxTree) -> Vec<usize> {
    tree.items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) if type_decl.visibility == Visibility::Public => {
                Some(type_decl.span.start.line)
            }
            SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => {
                Some(schema.span.start.line)
            }
            SyntaxItem::Function(function)
                if function.kind == FunctionKind::Function
                    && function.visibility == Visibility::Public =>
            {
                Some(function.span.start.line)
            }
            SyntaxItem::PublicAlias(alias) => Some(alias.span.start.line),
            _ => None,
        })
        .collect()
}

fn type_signature(type_decl: &TypeDecl) -> String {
    let mut signature = String::from("type ");
    signature.push_str(type_decl.name.as_deref().unwrap_or("<anonymous>"));
    if !type_decl.params.is_empty() {
        signature.push('<');
        signature.push_str(&type_decl.params.join(", "));
        signature.push('>');
    }
    signature
}

fn schema_signature(schema: &SchemaDecl) -> String {
    format!("schema {}", schema.name.as_deref().unwrap_or("<anonymous>"))
}

fn alias_signature(alias: &PublicAliasDecl) -> String {
    format!(
        "{} {} = {}",
        alias_kind(alias.kind),
        alias.name.as_deref().unwrap_or("<anonymous>"),
        alias.target.join("::")
    )
}

fn alias_kind(kind: PublicAliasKind) -> &'static str {
    match kind {
        PublicAliasKind::Function => "function",
        PublicAliasKind::Type => "type",
        PublicAliasKind::Schema => "schema",
    }
}

fn type_constructor(variant: &TypeVariantDecl) -> PackageDocTypeConstructor {
    PackageDocTypeConstructor {
        name: variant.name.clone().unwrap_or_default(),
        signature: variant_signature(variant),
    }
}

fn variant_signature(variant: &TypeVariantDecl) -> String {
    let name = variant.name.as_deref().unwrap_or("<anonymous>");
    if variant.fields.is_empty() {
        return name.to_string();
    }
    if variant.fields.iter().all(|field| !field.name.is_empty()) {
        let fields = variant
            .fields
            .iter()
            .map(|field| format!("{}: {}", field.name, canonical_type_text(&field.ty)))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{name} {{ {fields} }}");
    }
    let fields = variant
        .fields
        .iter()
        .map(|field| canonical_type_text(&field.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{name}({fields})")
}

fn function_contract(contract: &ContractClause) -> PackageDocFunctionContract {
    PackageDocFunctionContract {
        kind: match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
            ContractKind::Invariant => "invariant",
        }
        .to_string(),
        text: contract.text.clone(),
    }
}

fn doc_block_before(source: &SourceFile, target_line: usize) -> Vec<String> {
    if target_line <= 1 {
        return Vec::new();
    }
    let lines = source.text().lines().collect::<Vec<_>>();
    let mut index = target_line - 2;
    let mut docs = Vec::new();
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim_start();
        if let Some(content) = trimmed.strip_prefix("##") {
            docs.push(content.trim_start().to_string());
        } else {
            break;
        }
        if index == 0 {
            break;
        }
        index -= 1;
    }
    docs.reverse();
    if docs
        .iter()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| matches!(line.trim(), "@adr" | "@adr-lite"))
    {
        return Vec::new();
    }
    rendered_doc_lines(docs)
}

fn rendered_doc_lines(lines: Vec<String>) -> Vec<String> {
    let mut rendered = Vec::new();
    let mut in_veln_fence = false;
    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_veln_fence =
                trimmed.starts_with("```veln") && !trimmed.starts_with("```veln-output");
            rendered.push(line);
            continue;
        }
        if in_veln_fence && line.starts_with("> ") {
            continue;
        }
        rendered.push(line);
    }
    rendered
}

#[derive(Clone)]
struct DocSchemaReference {
    target: String,
    span: SourceSpan,
}

fn doc_schema_references(source: &SourceFile) -> Vec<DocSchemaReference> {
    let mut references = Vec::new();
    let mut line_start = 0;
    for line in source.text().split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();
        if let Some(content) = trimmed.strip_prefix("##") {
            let content_start = line_start + indent_len + "##".len();
            references.extend(extract_doc_schema_references(
                source,
                content,
                content_start,
            ));
        }
        line_start += line.len();
    }
    references
}

fn doc_schema_references_before(
    source: &SourceFile,
    target_line: usize,
) -> Vec<DocSchemaReference> {
    if target_line <= 1 {
        return Vec::new();
    }
    let lines = source.text().split_inclusive('\n').collect::<Vec<_>>();
    let mut index = target_line - 2;
    let mut docs = Vec::new();
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim_start();
        if trimmed.strip_prefix("##").is_none() {
            break;
        }
        docs.push((index, *line));
        if index == 0 {
            break;
        }
        index -= 1;
    }
    docs.reverse();

    let mut references = Vec::new();
    let mut line_start = 0;
    for (line_index, line) in lines.iter().enumerate() {
        if docs.iter().any(|(index, _)| *index == line_index) {
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();
            if let Some(content) = trimmed.strip_prefix("##") {
                let content_start = line_start + indent_len + "##".len();
                references.extend(extract_doc_schema_references(
                    source,
                    content,
                    content_start,
                ));
            }
        }
        line_start += line.len();
    }
    references
}

fn extract_doc_schema_references(
    source: &SourceFile,
    text: &str,
    text_start: usize,
) -> Vec<DocSchemaReference> {
    let mut references = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("{@schema") {
        let marker_start = cursor + relative_start;
        let after_marker = marker_start + "{@schema".len();
        let Some(next) = text[after_marker..].chars().next() else {
            break;
        };
        if !next.is_whitespace() {
            cursor = after_marker;
            continue;
        }
        let after_space = after_marker
            + text[after_marker..]
                .char_indices()
                .find(|(_, ch)| !ch.is_whitespace())
                .map_or(0, |(index, _)| index);
        let Some(relative_end) = text[after_space..].find('}') else {
            break;
        };
        let marker_end = after_space + relative_end;
        let target_text = &text[after_space..marker_end];
        let leading_trim = target_text.len() - target_text.trim_start().len();
        let trailing_trim = target_text.trim_end().len();
        let target = target_text.trim().to_string();
        if !target.is_empty() {
            references.push(DocSchemaReference {
                target,
                span: source.span(TextRange::new(
                    text_start + after_space + leading_trim,
                    text_start + after_space + trailing_trim,
                )),
            });
        }
        cursor = marker_end + 1;
    }
    references
}

fn schema_reference_target<'a>(
    target: &str,
    current_module: &str,
    public_schemas: &'a BTreeMap<String, PublicSchemaDocTarget>,
) -> Option<&'a PublicSchemaDocTarget> {
    let segments = target.split("::").collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => public_schemas
            .get(&format!("{current_module}::{name}"))
            .or_else(|| public_schemas.get(*name)),
        [module @ .., name] => {
            let module = module.join("::");
            public_schemas.get(&format!("{module}::{name}"))
        }
        _ => None,
    }
}

fn doctest_fences(
    source: &SourceFile,
    target_line: usize,
) -> Vec<Result<PackageDocDoctest, PackageDocDiagnostic>> {
    let mut result = Vec::new();
    let docs = doc_block_before(source, target_line);
    let mut active: Option<(String, Option<String>, bool, bool, Vec<String>)> = None;
    let mut last_doctest: Option<usize> = None;
    for line in docs {
        let trimmed = line.trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            if let Some((kind, expected_error, ignored, should_fail, lines)) = active.take() {
                if kind == "veln" && !ignored {
                    result.push(Ok(PackageDocDoctest {
                        kind,
                        code: lines
                            .into_iter()
                            .filter(|line| !line.starts_with("> "))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        expected_error,
                        should_fail,
                        expected_output: Vec::new(),
                    }));
                    last_doctest = Some(result.len() - 1);
                } else if kind == "veln-output"
                    && let Some(index) = last_doctest
                    && let Some(Ok(doctest)) = result.get_mut(index)
                {
                    doctest.expected_output = lines;
                }
                continue;
            }
            let mut parts = info.split_whitespace();
            let Some(kind) = parts.next() else {
                continue;
            };
            if !matches!(kind, "veln" | "veln-output") {
                continue;
            }
            let mut expected_error = None;
            let mut ignored = false;
            let mut should_fail = false;
            let mut has_output_stream = kind != "veln-output";
            let mut failed = None;
            for part in parts {
                if let Some(error) = part.strip_prefix("error=") {
                    if error.is_empty() {
                        failed = Some("empty doctest error attribute".to_string());
                    } else {
                        expected_error = Some(error.to_string());
                    }
                } else if kind == "veln" && matches!(part, "ignore" | "fail") {
                    ignored = part == "ignore";
                    should_fail = part == "fail";
                } else if kind == "veln"
                    && (part.starts_with("runtime=")
                        || part.starts_with("clause=")
                        || part.starts_with("predicate=")
                        || part.starts_with("function=")
                        || part.starts_with("blame=")
                        || part.starts_with("value="))
                {
                    continue;
                } else if kind == "veln-output"
                    && let Some(stream) = part.strip_prefix("stream=")
                {
                    if matches!(stream, "stdout" | "stderr") {
                        has_output_stream = true;
                    } else {
                        failed = Some(format!("unknown doctest output stream `{stream}`"));
                    }
                } else {
                    failed = Some(format!("unknown doctest attribute `{part}`"));
                }
            }
            if kind == "veln-output" && !has_output_stream {
                failed = Some("missing doctest output stream".to_string());
            }
            if let Some(message) = failed {
                result.push(Err(PackageDocDiagnostic {
                    gate: "doctest".to_string(),
                    code: "package_doc.invalid_doctest_metadata".to_string(),
                    message,
                    span: None,
                }));
                continue;
            }
            active = Some((
                kind.to_string(),
                expected_error,
                ignored,
                should_fail,
                Vec::new(),
            ));
        } else if let Some((_, _, _, _, lines)) = &mut active {
            lines.push(line);
        }
    }
    result
}

fn catalog_to_json(catalog: &PackageDocCatalog) -> String {
    let mut out = String::new();
    out.push('{');
    field(&mut out, "schema_version", &catalog.schema_version, false);
    field(
        &mut out,
        "generator_contract",
        &catalog.generator_contract,
        true,
    );
    field(
        &mut out,
        "package_identity",
        &catalog.package_identity,
        true,
    );
    field(&mut out, "snapshot_digest", &catalog.snapshot_digest, true);
    out.push_str(",\"metadata\":");
    metadata_json(&mut out, &catalog.metadata);
    out.push_str(",\"modules\":[");
    for (index, module) in catalog.modules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        module_json(&mut out, module);
    }
    out.push_str("],\"status\":");
    status_json(&mut out, &catalog.status);
    out.push('}');
    out
}

fn status_to_json(
    identity: &str,
    snapshot_digest: &str,
    generator_contract: &str,
    status: &PackageDocGenerationStatus,
) -> String {
    let mut out = String::new();
    out.push('{');
    field(&mut out, "schema_version", SCHEMA_VERSION, false);
    field(&mut out, "generator_contract", generator_contract, true);
    field(&mut out, "package_identity", identity, true);
    field(&mut out, "snapshot_digest", snapshot_digest, true);
    out.push_str(",\"status\":");
    status_json(&mut out, status);
    out.push('}');
    out
}

fn metadata_json(out: &mut String, metadata: &PackageDocMetadata) {
    out.push('{');
    field(out, "identity", &metadata.identity, false);
    optional_field(out, "package_name", metadata.package_name.as_deref());
    optional_field(out, "version", metadata.version.as_deref());
    optional_field(out, "description", metadata.description.as_deref());
    optional_field(out, "license", metadata.license.as_deref());
    string_array_field(out, "authors", &metadata.authors);
    string_array_field(out, "keywords", &metadata.keywords);
    string_array_field(out, "exported_modules", &metadata.exported_modules);
    out.push('}');
}

fn module_json(out: &mut String, module: &PackageDocModule) {
    out.push('{');
    field(out, "id", &module.id, false);
    field(out, "name", &module.name, true);
    field(out, "source_path", &module.source_path, true);
    string_array_field(out, "doc", &module.doc);
    out.push_str(",\"declarations\":[");
    for (index, declaration) in module.declarations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        declaration_json(out, declaration);
    }
    out.push_str("]}");
}

fn declaration_json(out: &mut String, declaration: &PackageDocDeclaration) {
    out.push('{');
    field(out, "id", &declaration.id, false);
    field(out, "kind", &declaration.kind, true);
    field(out, "name", &declaration.name, true);
    field(out, "signature", &declaration.signature, true);
    string_array_field(out, "doc", &declaration.doc);
    out.push_str(",\"contracts\":[");
    for (index, contract) in declaration.contracts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "kind", &contract.kind, false);
        field(out, "text", &contract.text, true);
        out.push('}');
    }
    out.push_str("],\"constructors\":[");
    for (index, constructor) in declaration.constructors.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "name", &constructor.name, false);
        field(out, "signature", &constructor.signature, true);
        out.push('}');
    }
    out.push_str("],\"alias\":");
    if let Some(alias) = &declaration.alias {
        out.push('{');
        field(out, "kind", &alias.kind, false);
        string_array_field(out, "target", &alias.target);
        out.push('}');
    } else {
        out.push_str("null");
    }
    out.push_str(",\"doctests\":[");
    for (index, doctest) in declaration.doctests.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "kind", &doctest.kind, false);
        field(out, "code", &doctest.code, true);
        optional_field(out, "expected_error", doctest.expected_error.as_deref());
        bool_field(out, "should_fail", doctest.should_fail);
        string_array_field(out, "expected_output", &doctest.expected_output);
        out.push('}');
    }
    out.push_str("],\"references\":[");
    for (index, reference) in declaration.references.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "kind", &reference.kind, false);
        field(out, "marker", &reference.marker, true);
        field(
            out,
            "target_declaration_id",
            &reference.target_declaration_id,
            true,
        );
        out.push('}');
    }
    out.push_str("]}");
}

fn status_json(out: &mut String, status: &PackageDocGenerationStatus) {
    out.push('{');
    field(
        out,
        "state",
        match status.state {
            PackageDocGeneration::Complete => "complete",
            PackageDocGeneration::Failed => "failed",
        },
        false,
    );
    out.push_str(",\"diagnostics\":[");
    for (index, diagnostic) in status.diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "gate", &diagnostic.gate, false);
        field(out, "code", &diagnostic.code, true);
        field(out, "message", &diagnostic.message, true);
        out.push_str(",\"span\":");
        if let Some(span) = &diagnostic.span {
            out.push('{');
            field(out, "source_uri", &span.source_uri, false);
            number_field(out, "line", span.line);
            number_field(out, "column", span.column);
            number_field(out, "offset", span.offset);
            out.push('}');
        } else {
            out.push_str("null");
        }
        out.push('}');
    }
    out.push_str("]}");
}

fn field(out: &mut String, key: &str, value: &str, comma: bool) {
    if comma {
        out.push(',');
    }
    string(out, key);
    out.push(':');
    string(out, value);
}

fn optional_field(out: &mut String, key: &str, value: Option<&str>) {
    out.push(',');
    string(out, key);
    out.push(':');
    if let Some(value) = value {
        string(out, value);
    } else {
        out.push_str("null");
    }
}

fn string_array_field(out: &mut String, key: &str, values: &[String]) {
    out.push(',');
    string(out, key);
    out.push_str(":[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        string(out, value);
    }
    out.push(']');
}

fn number_field(out: &mut String, key: &str, value: usize) {
    out.push(',');
    string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn bool_field(out: &mut String, key: &str, value: bool) {
    out.push(',');
    string(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn doc_digest(canonical_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DOC_DOMAIN);
    hasher.update((canonical_bytes.len() as u64).to_be_bytes());
    hasher.update(canonical_bytes);
    lower_hex(hasher.finalize().as_slice())
}

fn declaration_id(kind: &str, identity: &str) -> String {
    digest_hex(
        DECLARATION_ID_DOMAIN,
        &[kind.as_bytes(), identity.as_bytes()],
    )
}

fn digest_hex(domain: &[u8], parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((*part).len().to_be_bytes());
        hasher.update(part);
    }
    lower_hex(hasher.finalize().as_slice())
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn source_uri(identity: &str, digest: &str, source_path: &str) -> String {
    if identity.is_empty() || digest.is_empty() {
        return source_path.to_string();
    }
    let mut uri = String::from("veln-pkg:///");
    uri.push_str(&encoded_segment(identity));
    uri.push_str("/snapshot/");
    uri.push_str(digest);
    uri.push('/');
    for (index, segment) in source_path.split('/').enumerate() {
        if index > 0 {
            uri.push('/');
        }
        uri.push_str(&encoded_segment(segment));
    }
    uri
}

fn encoded_segment(segment: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::new();
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push('%');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    output
}

fn explicit_module_name(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("mod ")?;
        leading_module_path(rest).map(str::to_string)
    })
}

fn module_name_from_path(path: &str) -> Option<String> {
    Some(path.strip_suffix(".veln")?.replace('/', "::"))
}

fn is_package_relative_path(path: &str) -> bool {
    let path = std::path::Path::new(path);
    !path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

fn leading_module_path(input: &str) -> Option<&str> {
    let end = input
        .char_indices()
        .take_while(|(_, ch)| *ch == '_' || ch.is_ascii_alphanumeric() || *ch == ':')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&input[..end])
}

fn manifest_field(fields: &[veln_project::ManifestField], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.clone())
}

fn manifest_list_field(fields: &[veln_project::ManifestField], key: &str) -> Vec<String> {
    manifest_field(fields, key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use veln_project::{
        PackageIdentity, PackageSnapshotSource, capture_embedded_package_snapshot,
        parse_manifest_text,
    };

    use super::*;

    fn generate(manifest: &str, sources: &[(&str, &str)]) -> PackageDocResult {
        let snapshot = capture_embedded_package_snapshot(
            manifest.as_bytes(),
            sources
                .iter()
                .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
        )
        .unwrap();
        let manifest = parse_manifest_text("veln.toml", manifest);
        PackageDocResult::generate(
            "owner/package",
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-a"),
        )
    }

    fn generate_fixture(name: &str) -> PackageDocResult {
        let root = example_fixture_root(name);
        let manifest_text = fs::read_to_string(root.join("veln.toml")).unwrap();
        let mut source_texts = Vec::new();
        let mut source_paths = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "veln")
            })
            .collect::<Vec<_>>();
        source_paths.sort();
        for path in source_paths {
            let source_name = path.file_name().unwrap().to_string_lossy().to_string();
            source_texts.push((source_name, fs::read(path).unwrap()));
        }
        let snapshot = capture_embedded_package_snapshot(
            manifest_text.as_bytes(),
            source_texts
                .iter()
                .map(|(path, bytes)| PackageSnapshotSource::new(path, bytes.as_slice())),
        )
        .unwrap();
        let manifest = parse_manifest_text("veln.toml", &manifest_text);
        PackageDocResult::generate(
            "owner/package",
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-a"),
        )
    }

    fn example_fixture_root(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/specification/doc")
            .join(name)
    }

    fn generate_with_forced_declaration_id(
        manifest: &str,
        sources: &[(&str, &str)],
        id: &str,
    ) -> PackageDocResult {
        let snapshot = capture_embedded_package_snapshot(
            manifest.as_bytes(),
            sources
                .iter()
                .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
        )
        .unwrap();
        let manifest = parse_manifest_text("veln.toml", manifest);
        PackageDocBuilder::new(
            "owner/package",
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-a"),
        )
        .with_forced_declaration_id(id)
        .generate()
    }

    #[test]
    fn successful_catalog_contains_only_exported_public_api_and_allowed_metadata() {
        let result = generate(
            concat!(
                "[package]\n",
                "name = \"demo\"\n",
                "version = \"1.2.3\"\n",
                "description = \"Public docs.\"\n",
                "license = \"MIT\"\n",
                "authors = \"Ada, Bea\"\n",
                "keywords = \"docs, api\"\n",
                "repository = \"hidden\"\n",
                "[lib]\n",
                "exports = [\"main.veln\"]\n",
                "[dependencies.other]\n",
                "path = \"../other\"\n",
                "[tool.secret]\n",
                "token = \"hidden\"\n",
            ),
            &[
                (
                    "main.veln",
                    concat!(
                        "## Module docs.\n",
                        "\n",
                        "## Public type docs.\n",
                        "pub type ResultBox<A>\n",
                        "\tpub Ready(value: A)\n",
                        "\tHidden(reason: String)\n",
                        "end\n",
                        "\n",
                        "## Public schema docs.\n",
                        "pub schema Packet\n",
                        "\tformat binary\n",
                        "\tvalue: UInt8\n",
                        "end\n",
                        "\n",
                        "## Function docs mention {@schema Packet}.\n",
                        "## ```veln\n",
                        "## fn sample() -> Int\n",
                        "## \t1\n",
                        "## end\n",
                        "## ```\n",
                        "## ```veln-output stream=stdout\n",
                        "## 1\n",
                        "## ```\n",
                        "pub fn value(input: Int) -> output: Int\n",
                        "\trequire input >= 0\n",
                        "\tensure output >= input\n",
                        "\tinput\n",
                        "end\n",
                        "\n",
                        "pub type PacketAlias = Packet\n",
                        "\n",
                        "fn private_helper() -> Int\n",
                        "\t0\n",
                        "end\n",
                    ),
                ),
                (
                    "hidden.veln",
                    "## Hidden module.\npub fn hidden_public() -> Int\n\t1\nend\n",
                ),
            ],
        );

        let catalog = result.catalog().expect("successful catalog");
        assert_eq!(catalog.metadata.package_name.as_deref(), Some("demo"));
        assert_eq!(catalog.metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(catalog.metadata.authors, ["Ada", "Bea"]);
        assert_eq!(catalog.metadata.keywords, ["docs", "api"]);
        assert_eq!(catalog.modules.len(), 1);
        assert_eq!(catalog.modules[0].name, "main");
        assert_eq!(
            catalog.modules[0]
                .declarations
                .iter()
                .map(|declaration| (declaration.kind.as_str(), declaration.name.as_str()))
                .collect::<Vec<_>>(),
            [
                ("type", "ResultBox"),
                ("schema", "Packet"),
                ("function", "value"),
                ("alias", "PacketAlias"),
            ]
        );
        assert_eq!(catalog.modules[0].declarations[0].constructors.len(), 1);
        assert_eq!(catalog.modules[0].declarations[2].contracts.len(), 2);
        assert_eq!(catalog.modules[0].declarations[2].doctests.len(), 1);
        assert_eq!(
            catalog.modules[0].declarations[2].doctests[0].expected_output,
            ["1"]
        );
        let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
        assert!(bytes.contains("\"doc\":[\"Function docs mention {@schema Packet}.\""));
        assert!(!bytes.contains("hidden_public"));
        assert!(!bytes.contains("private_helper"));
        assert!(!bytes.contains("repository"));
        assert!(!bytes.contains("token"));
    }

    #[test]
    fn catalog_identity_digest_and_uris_are_deterministic() {
        let first = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
        );
        let second = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
        );

        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.doc_digest(), second.doc_digest());
        assert_eq!(first.doc_digest(), doc_digest(first.canonical_bytes()));
        assert_eq!(first.status_uri(), second.status_uri());
        let catalog = first.catalog().unwrap();
        assert!(
            catalog
                .index_uri
                .starts_with("veln-doc:///package/owner%2Fpackage/")
        );
        assert_eq!(catalog.modules[0].id.len(), 64);
        assert_eq!(catalog.modules[0].declarations[0].id.len(), 64);
        assert_eq!(
            first.declaration_uri_for("main", "function", "value"),
            Some(catalog.modules[0].declarations[0].uri.as_str())
        );
    }

    #[test]
    fn resolved_schema_references_keep_target_identity_and_uri() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\", \"wire.veln\"]\n",
            &[
                (
                    "main.veln",
                    concat!(
                        "## Uses {@schema wire::Packet}.\n",
                        "pub fn value() -> Int\n",
                        "\t1\n",
                        "end\n",
                    ),
                ),
                (
                    "wire.veln",
                    concat!(
                        "pub schema Packet\n",
                        "\tformat binary\n",
                        "\tvalue: UInt8\n",
                        "end\n",
                    ),
                ),
            ],
        );

        let catalog = result.catalog().expect("successful catalog");
        let function = &catalog.modules[0].declarations[0];
        let schema = &catalog.modules[1].declarations[0];
        assert_eq!(function.references.len(), 1);
        assert_eq!(function.references[0].marker, "wire::Packet");
        assert_eq!(function.references[0].target_declaration_id, schema.id);
        assert_eq!(function.references[0].target_uri, schema.uri);
        assert!(
            std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("\"references\":[{\"kind\":\"schema\"")
        );
    }

    #[test]
    fn declaration_uri_lookup_accepts_declaration_and_constructor_locations() {
        let source_text = concat!(
            "pub type Choice\n",
            "\tpub Some(value: Int)\n",
            "end\n",
            "\n",
            "pub fn value() -> Int\n",
            "\t1\n",
            "end\n",
        );
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[("main.veln", source_text)],
        );
        let source = SourceFile::new("main.veln", source_text);
        let parsed = parse(&source).tree;
        let SyntaxItem::Type(type_decl) = &parsed.items[0] else {
            panic!("expected type declaration");
        };
        let SyntaxItem::Function(function) = &parsed.items[1] else {
            panic!("expected function declaration");
        };
        let catalog = result.catalog().expect("successful catalog");
        let type_uri = catalog.modules[0].declarations[0].uri.as_str();
        let function_uri = catalog.modules[0].declarations[1].uri.as_str();
        let source_uri = source_uri("owner/package", result.snapshot_digest(), "main.veln");

        assert_eq!(
            result.declaration_uri_for_location(&NavigationLocation {
                source: NavigationSource::Package {
                    uri: source_uri.clone()
                },
                span: type_decl.span.clone(),
            }),
            Some(type_uri)
        );
        assert_eq!(
            result.declaration_uri_for_location(&NavigationLocation {
                source: NavigationSource::Package { uri: source_uri },
                span: type_decl.variants[0].span.clone(),
            }),
            Some(type_uri)
        );
        assert_eq!(
            result.declaration_uri_for_location(&NavigationLocation {
                source: NavigationSource::Workspace,
                span: function.span.clone(),
            }),
            Some(function_uri)
        );
    }

    #[test]
    fn package_bytes_and_generator_contract_change_document_identity() {
        let base_manifest = "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n";
        let first = generate(
            base_manifest,
            &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
        );
        let changed_source = generate(
            base_manifest,
            &[("main.veln", "pub fn value() -> Int\n\t2\nend\n")],
        );
        let snapshot = capture_embedded_package_snapshot(
            base_manifest.as_bytes(),
            [PackageSnapshotSource::new(
                "main.veln",
                b"pub fn value() -> Int\n\t1\nend\n",
            )],
        )
        .unwrap();
        let manifest = parse_manifest_text("veln.toml", base_manifest);
        let changed_contract = PackageDocResult::generate(
            "owner/package",
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-b"),
        );

        assert_ne!(first.snapshot_digest(), changed_source.snapshot_digest());
        assert_ne!(first.doc_digest(), changed_source.doc_digest());
        assert_eq!(first.snapshot_digest(), changed_contract.snapshot_digest());
        assert_ne!(first.doc_digest(), changed_contract.doc_digest());
    }

    #[test]
    fn renderer_only_stability_follows_canonical_result_bytes() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
        );
        let same_renderer_bytes = result.canonical_bytes().to_vec();
        assert_eq!(result.doc_digest(), doc_digest(&same_renderer_bytes));
    }

    #[test]
    fn generation_failure_returns_status_without_partial_catalog() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\", \"missing.veln\"]\n",
            &[(
                "main.veln",
                "## Bad reference {@schema Missing}.\npub fn value() -> Int\n\t1\nend\n",
            )],
        );

        assert!(result.catalog().is_none());
        assert!(matches!(
            result.kind(),
            PackageDocResultKind::Status(PackageDocGenerationStatus {
                state: PackageDocGeneration::Failed,
                ..
            })
        ));
        let status = result.status();
        assert_eq!(status.diagnostics.len(), 2);
        assert_eq!(status.diagnostics[0].gate, "documentation_reference");
        assert_eq!(status.diagnostics[1].gate, "export");
        let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
        assert!(bytes.contains("\"state\":\"failed\""));
        assert!(!bytes.contains("\"modules\""));
    }

    #[test]
    fn manifest_gate_rejects_unvalidated_manifest_without_partial_catalog() {
        let result = generate(
            concat!(
                "[package]\n",
                "name = \"demo\"\n",
                "[modules]\n",
                "\"main.veln\" = \"main\"\n",
                "[lib]\n",
                "exports = [\"main.test.veln\", \"../escape.veln\"]\n",
                "[dependencies.\"example\"]\n",
                "git = \"https://example.invalid/repo.git\"\n",
            ),
            &[
                ("main.veln", "pub fn value() -> Int\n\t1\nend\n"),
                ("main.test.veln", "pub fn helper() -> Int\n\t1\nend\n"),
            ],
        );

        assert!(result.catalog().is_none());
        let diagnostics = result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>();
        assert!(diagnostics.contains(&("manifest", "package_doc.unsupported_manifest_section")));
        assert!(diagnostics.contains(&("manifest", "package_doc.invalid_export")));
        assert!(diagnostics.contains(&("manifest", "package_doc.missing_git_selector")));
        assert!(
            !std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("\"modules\"")
        );
    }

    #[test]
    fn executable_specification_fixture_observes_successful_catalog() {
        let result = generate_fixture("package-catalog-api-success");
        let catalog = result.catalog().expect("successful catalog");

        assert_eq!(catalog.modules.len(), 2);
        assert_eq!(catalog.modules[0].name, "main");
        assert!(result.doc_digest().len() == 64);
        assert!(
            catalog.modules[0]
                .declarations
                .iter()
                .any(|declaration| declaration.name == "visible")
        );
        assert!(
            !std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("hidden_helper")
        );
    }

    #[test]
    fn executable_specification_fixture_observes_manifest_gate_failure() {
        let result = generate_fixture("package-catalog-manifest-gate");

        assert!(result.catalog().is_none());
        assert_eq!(result.status().diagnostics[0].gate, "manifest");
        assert_eq!(
            result.status().diagnostics[0].code,
            "package_doc.invalid_export"
        );
        assert!(
            !std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("\"modules\"")
        );
    }

    #[test]
    fn parse_and_doctest_gates_are_package_atomic() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "## ```veln\n",
                    "## @\n",
                    "## ```\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            )],
        );

        assert!(result.catalog().is_none());
        assert_eq!(result.status().diagnostics[0].gate, "doctest");
    }

    #[test]
    fn duplicate_semantic_identity_fails_the_package() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "pub fn value() -> Int\n\t1\nend\n",
                    "pub fn value() -> Int\n\t2\nend\n",
                ),
            )],
        );

        assert!(result.catalog().is_none());
        assert!(
            result
                .status()
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "package_doc.duplicate_semantic_identity" })
        );
    }

    #[test]
    fn declaration_id_collision_fails_the_package() {
        let result = generate_with_forced_declaration_id(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "pub fn first() -> Int\n\t1\nend\n",
                    "pub fn second() -> Int\n\t2\nend\n",
                ),
            )],
            "forced-declaration-id",
        );

        assert!(result.catalog().is_none());
        assert!(
            result
                .status()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "package_doc.declaration_id_collision")
        );
        assert!(
            !std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("\"modules\"")
        );
    }

    #[test]
    fn fail_doctest_is_published_when_generated_parse_diagnostic_matches() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "## ```veln fail\n",
                    "## @\n",
                    "## ```\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            )],
        );

        let catalog = result.catalog().expect("successful catalog");
        let doctest = &catalog.modules[0].declarations[0].doctests[0];
        assert_eq!(doctest.expected_error.as_deref(), None);
        assert!(doctest.should_fail);
        assert_eq!(doctest.code, "@");
        assert!(result.status().diagnostics.is_empty());
        assert!(
            std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("\"should_fail\":true")
        );
    }

    #[test]
    fn fail_doctest_must_produce_generated_parse_diagnostic() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "## ```veln fail\n",
                    "## fn sample() -> Int\n",
                    "## \t1\n",
                    "## end\n",
                    "## ```\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            )],
        );

        assert!(result.catalog().is_none());
        assert!(
            result
                .status()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "doctest.expected_failure_missing")
        );
    }

    #[test]
    fn exported_parse_failure_keeps_virtual_source_catalog_resolvable() {
        let manifest_text =
            "[package]\nname = \"owner/package\"\n[lib]\nexports = [\"main.veln\"]\n";
        let source_text = "pub fn broken() -> ()\n  @\nend\n";
        let snapshot = capture_embedded_package_snapshot(
            manifest_text.as_bytes(),
            [PackageSnapshotSource::new(
                "main.veln",
                source_text.as_bytes(),
            )],
        )
        .unwrap();
        let manifest = parse_manifest_text("veln.toml", manifest_text);
        let result = PackageDocResult::generate(
            "owner/package",
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-a"),
        );
        let virtual_sources = crate::VirtualSourceCatalog::new([(
            PackageIdentity::new("owner/package").unwrap(),
            snapshot,
        )])
        .unwrap();
        let entry = virtual_sources.entries().next().unwrap();

        assert!(result.catalog().is_none());
        assert_eq!(result.status().diagnostics[0].gate, "parse");
        assert!(
            result.status().diagnostics[0]
                .span
                .as_ref()
                .unwrap()
                .source_uri
                .starts_with("veln-pkg:///owner%2Fpackage/snapshot/")
        );
        assert!(
            !std::str::from_utf8(result.canonical_bytes())
                .unwrap()
                .contains("\"modules\"")
        );
        assert_eq!(
            virtual_sources.resolve(entry.uri()),
            Some(source_text.as_bytes())
        );
    }

    #[test]
    fn hidden_doctest_setup_and_adr_lite_are_not_published() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "## @adr\n",
                    "## id: hidden\n",
                    "## status: accepted\n",
                    "## scope: docs\n",
                    "## context: hidden\n",
                    "## decision: hidden\n",
                    "## consequences: hidden\n",
                    "\n",
                    "## ```veln\n",
                    "## > let setup: Int = 1\n",
                    "## fn sample() -> Int\n",
                    "## \t1\n",
                    "## end\n",
                    "## ```\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            )],
        );

        let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
        assert!(!bytes.contains("@adr"));
        assert!(!bytes.contains("let setup"));
        assert!(bytes.contains("fn sample()"));
    }
}
