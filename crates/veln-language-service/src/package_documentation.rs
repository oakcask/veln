use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};
use veln_project::{CapturedPackageSnapshot, ProjectManifest};
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    ContractClause, ContractKind, FunctionDecl, FunctionKind, ParseDiagnostic, PublicAliasDecl,
    PublicAliasKind, SchemaDecl, SyntaxItem, TypeDecl, TypeVariantDecl, Visibility,
    canonical_type_text, parse,
};

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageDocResultKind {
    Catalog(PackageDocCatalog),
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDocTypeConstructor {
    pub name: String,
    pub signature: String,
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
}

#[derive(Clone)]
struct ParsedPackageSource {
    source: SourceFile,
    tree: veln_syntax::SyntaxTree,
    module_name: String,
    exported: bool,
    source_uri: String,
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
        }
    }

    fn generate(mut self) -> PackageDocResult {
        let metadata = self.metadata();
        let parsed_sources = self.parse_sources();
        self.validate_manifest_exports(&parsed_sources);
        self.validate_doc_references(&parsed_sources);

        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        let mut semantic_identities = BTreeMap::new();
        let mut modules = Vec::new();
        for source in parsed_sources.iter().filter(|source| source.exported) {
            let module_id = digest_hex(MODULE_ID_DOMAIN, &[source.module_name.as_bytes()]);
            let declarations = self.declarations(source, &mut semantic_identities);
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
            }
        }
        PackageDocResult {
            identity: self.identity.to_string(),
            snapshot_digest: self.snapshot.digest().to_string(),
            status_uri: self.status_uri(&final_doc_digest),
            doc_digest: final_doc_digest,
            canonical_bytes,
            kind: PackageDocResultKind::Catalog(catalog),
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
                self.diagnostics.push(parse_diagnostic("parse", diagnostic));
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
        for export in &self.manifest.lib.exports {
            let path = SourcePath::new(export.path.clone()).as_str().to_string();
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
            } else if !available.contains(&path) {
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

    fn validate_doc_references(&mut self, parsed_sources: &[ParsedPackageSource]) {
        let exported_modules = parsed_sources
            .iter()
            .filter(|source| source.exported)
            .map(|source| source.module_name.clone())
            .collect::<BTreeSet<_>>();
        let public_schemas = parsed_sources
            .iter()
            .filter(|source| source.exported)
            .flat_map(|source| {
                source.tree.items.iter().filter_map(move |item| match item {
                    SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => schema
                        .name
                        .as_ref()
                        .map(|name| (source.module_name.clone(), name.clone())),
                    _ => None,
                })
            })
            .collect::<BTreeSet<_>>();
        for source in parsed_sources.iter().filter(|source| source.exported) {
            for reference in doc_schema_references(&source.source) {
                if !schema_reference_is_public(
                    &reference.target,
                    &source.module_name,
                    &exported_modules,
                    &public_schemas,
                ) {
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
    ) -> Vec<PackageDocDeclaration> {
        let mut declarations = Vec::new();
        for item in &source.tree.items {
            match item {
                SyntaxItem::Type(type_decl) if type_decl.visibility == Visibility::Public => {
                    declarations.push(self.type_declaration(
                        source,
                        type_decl,
                        semantic_identities,
                    ));
                }
                SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => {
                    declarations.push(self.schema_declaration(source, schema, semantic_identities));
                }
                SyntaxItem::Function(function)
                    if function.kind == FunctionKind::Function
                        && function.visibility == Visibility::Public =>
                {
                    declarations.push(self.function_declaration(
                        source,
                        function,
                        semantic_identities,
                    ));
                }
                SyntaxItem::PublicAlias(alias) => {
                    declarations.push(self.alias_declaration(source, alias, semantic_identities));
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
    ) -> PackageDocDeclaration {
        let name = type_decl.name.clone().unwrap_or_default();
        let identity = format!(
            "type:{}::{name}:{}",
            source.module_name,
            type_signature(type_decl)
        );
        self.record_semantic_identity(&identity, &type_decl.span, semantic_identities);
        PackageDocDeclaration {
            id: declaration_id("type", &identity),
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
        }
    }

    fn schema_declaration(
        &mut self,
        source: &ParsedPackageSource,
        schema: &SchemaDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
    ) -> PackageDocDeclaration {
        let name = schema.name.clone().unwrap_or_default();
        let identity = format!(
            "schema:{}::{name}:{}",
            source.module_name,
            schema_signature(schema)
        );
        self.record_semantic_identity(&identity, &schema.span, semantic_identities);
        PackageDocDeclaration {
            id: declaration_id("schema", &identity),
            kind: "schema".to_string(),
            name,
            signature: schema_signature(schema),
            uri: String::new(),
            doc: doc_block_before(&source.source, schema.span.start.line),
            contracts: Vec::new(),
            constructors: Vec::new(),
            alias: None,
            doctests: self.doctests_for(&source.source, schema.span.start.line),
        }
    }

    fn function_declaration(
        &mut self,
        source: &ParsedPackageSource,
        function: &FunctionDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
    ) -> PackageDocDeclaration {
        let name = function.name.clone().unwrap_or_default();
        let signature = function_signature(function);
        let identity = format!("function:{}::{name}:{signature}", source.module_name);
        self.record_semantic_identity(&identity, &function.span, semantic_identities);
        PackageDocDeclaration {
            id: declaration_id("function", &identity),
            kind: "function".to_string(),
            name,
            signature,
            uri: String::new(),
            doc: doc_block_before(&source.source, function.span.start.line),
            contracts: function.contracts.iter().map(function_contract).collect(),
            constructors: Vec::new(),
            alias: None,
            doctests: self.doctests_for(&source.source, function.span.start.line),
        }
    }

    fn alias_declaration(
        &mut self,
        source: &ParsedPackageSource,
        alias: &PublicAliasDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
    ) -> PackageDocDeclaration {
        let name = alias.name.clone().unwrap_or_default();
        let signature = alias_signature(alias);
        let kind = alias_kind(alias.kind).to_string();
        let identity = format!("alias:{kind}:{}::{name}:{signature}", source.module_name);
        self.record_semantic_identity(&identity, &alias.span, semantic_identities);
        PackageDocDeclaration {
            id: declaration_id("alias", &identity),
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
                    self.validate_doctest(source, &doctest);
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
        if doctest.expected_error.is_some() {
            if diagnostics.is_empty() {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "doctest".to_string(),
                    code: "package_doc.expected_failure_missing".to_string(),
                    message: "negative documentation doctest produced no parse diagnostics"
                        .to_string(),
                    span: None,
                });
            }
        } else if let Some(diagnostic) = diagnostics.into_iter().next() {
            self.diagnostics
                .push(parse_diagnostic("doctest", diagnostic));
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

fn parse_diagnostic(gate: &str, diagnostic: ParseDiagnostic) -> PackageDocDiagnostic {
    PackageDocDiagnostic {
        gate: gate.to_string(),
        code: diagnostic.id.to_string(),
        message: diagnostic.message,
        span: diagnostic.span.as_ref().map(|span| {
            PackageDocDiagnosticSpan::from_span(&source_uri("", "", span.file.as_str()), span)
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

fn schema_reference_is_public(
    target: &str,
    current_module: &str,
    exported_modules: &BTreeSet<String>,
    public_schemas: &BTreeSet<(String, String)>,
) -> bool {
    let segments = target.split("::").collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => public_schemas.contains(&(current_module.to_string(), (*name).to_string())),
        [module @ .., name] => {
            let module = module.join("::");
            exported_modules.contains(&module)
                && public_schemas.contains(&(module, (*name).to_string()))
        }
        _ => false,
    }
}

fn doctest_fences(
    source: &SourceFile,
    target_line: usize,
) -> Vec<Result<PackageDocDoctest, PackageDocDiagnostic>> {
    let mut result = Vec::new();
    let docs = doc_block_before(source, target_line);
    let mut active: Option<(String, Option<String>, Vec<String>)> = None;
    let mut last_doctest: Option<usize> = None;
    for line in docs {
        let trimmed = line.trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            if let Some((kind, expected_error, lines)) = active.take() {
                if kind == "veln" {
                    result.push(Ok(PackageDocDoctest {
                        kind,
                        code: lines
                            .into_iter()
                            .filter(|line| !line.starts_with("> "))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        expected_error,
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
            let mut failed = None;
            for part in parts {
                if let Some(error) = part.strip_prefix("error=") {
                    if error.is_empty() {
                        failed = Some("empty doctest error attribute".to_string());
                    } else {
                        expected_error = Some(error.to_string());
                    }
                } else {
                    failed = Some(format!("unknown doctest attribute `{part}`"));
                }
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
            active = Some((kind.to_string(), expected_error, Vec::new()));
        } else if let Some((_, _, lines)) = &mut active {
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
        string_array_field(out, "expected_output", &doctest.expected_output);
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
    use veln_project::{
        PackageSnapshotSource, capture_embedded_package_snapshot, parse_manifest_text,
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
                        "## ```veln-output\n",
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
    fn parse_and_doctest_gates_are_package_atomic() {
        let result = generate(
            "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
            &[(
                "main.veln",
                concat!(
                    "## ```veln\n",
                    "## bad\n",
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
