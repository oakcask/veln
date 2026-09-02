use super::*;

impl<'a> PackageDocBuilder<'a> {
    pub(super) fn new(
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
    pub(super) fn with_forced_declaration_id(mut self, id: impl Into<String>) -> Self {
        self.forced_declaration_id = Some(id.into());
        self
    }

    pub(super) fn generate(mut self) -> PackageDocResult {
        self.validate_manifest_snapshot_binding();
        self.validate_manifest_gate();
        let parsed_sources = self.parse_sources();
        self.validate_manifest_exports(&parsed_sources);
        let metadata = self.metadata(&parsed_sources);
        self.validate_doctests(&parsed_sources);
        let schema_targets = self.public_schema_targets(&parsed_sources);
        self.validate_doc_references(&parsed_sources, &schema_targets);

        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        let (modules, declaration_locations) = self.build_modules(&parsed_sources, &schema_targets);
        self.validate_unique_declaration_ids(&modules);
        self.validate_unique_module_ids(&modules);
        if !self.diagnostics.is_empty() {
            return self.failed_result();
        }

        self.complete_catalog_result(metadata, modules, declaration_locations)
    }

    pub(super) fn build_modules(
        &mut self,
        parsed_sources: &[ParsedPackageSource],
        schema_targets: &SchemaDocResolver<'_>,
    ) -> (
        Vec<PackageDocModule>,
        BTreeMap<PackageDocLocationKey, String>,
    ) {
        let mut semantic_identities = BTreeMap::new();
        let mut declaration_locations = BTreeMap::new();
        let mut modules = Vec::new();
        for source in parsed_sources.iter().filter(|source| source.exported) {
            let module_id = module_id(source.source.path().as_str());
            let declarations = self.declarations(
                source,
                &mut semantic_identities,
                &mut declaration_locations,
                schema_targets,
            );
            modules.push(PackageDocModule {
                uri: self.module_uri(&module_id, ""),
                id: module_id,
                name: source.module_name.clone(),
                source_path: source.source.path().as_str().to_string(),
                doc: module_doc(&source.source, &source.tree),
                doctests: self.module_doctests(source),
                references: self.module_references(source, schema_targets),
                declarations,
            });
        }
        (modules, declaration_locations)
    }

    pub(super) fn validate_unique_declaration_ids(&mut self, modules: &[PackageDocModule]) {
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
    }

    pub(super) fn validate_unique_module_ids(&mut self, modules: &[PackageDocModule]) {
        let mut module_ids = BTreeSet::new();
        for module in modules {
            if !module_ids.insert(module.id.clone()) {
                self.diagnostics.push(PackageDocDiagnostic {
                    gate: "identity".to_string(),
                    code: "package_doc.module_id_collision".to_string(),
                    message: format!("module documentation identifier collision `{}`", module.id),
                    span: None,
                });
            }
        }
    }

    pub(super) fn complete_catalog_result(
        &self,
        metadata: PackageDocMetadata,
        modules: Vec<PackageDocModule>,
        mut declaration_locations: BTreeMap<PackageDocLocationKey, String>,
    ) -> PackageDocResult {
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
            for reference in &mut module.references {
                reference.target_uri =
                    self.declaration_uri(&reference.target_declaration_id, &final_doc_digest);
            }
            for declaration in &mut module.declarations {
                declaration.uri = self.declaration_uri(&declaration.id, &final_doc_digest);
                for constructor in &mut declaration.constructors {
                    for reference in &mut constructor.references {
                        reference.target_uri = self
                            .declaration_uri(&reference.target_declaration_id, &final_doc_digest);
                    }
                }
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

    pub(super) fn metadata(&self, parsed_sources: &[ParsedPackageSource]) -> PackageDocMetadata {
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
            .validated_exported_modules(parsed_sources)
            .iter()
            .map(|(_, module_name)| module_name.clone())
            .collect();
        metadata
    }

    pub(super) fn parse_sources(&mut self) -> Vec<ParsedPackageSource> {
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
            let exported = exported_paths.contains(source.path());
            let module_name = match if exported {
                derive_export_source_module_path(&source_file).map_err(|diagnostics| diagnostics)
            } else {
                derive_source_module_path(&source_file).map_err(|diagnostic| vec![*diagnostic])
            } {
                Ok(module_name) => module_name,
                Err(diagnostics) => {
                    for diagnostic in diagnostics {
                        self.diagnostics.push(module_diagnostic(
                            "manifest",
                            diagnostic,
                            self.identity,
                            self.snapshot.digest(),
                        ));
                    }
                    String::new()
                }
            };
            parsed.push(ParsedPackageSource {
                source: source_file,
                tree: output.tree,
                module_name,
                exported,
                source_uri,
            });
        }
        parsed
    }

    pub(super) fn validate_manifest_exports(&mut self, parsed_sources: &[ParsedPackageSource]) {
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
                self.push_manifest_export_diagnostic(
                    "export",
                    "package_doc.duplicate_export",
                    format!("duplicate documentation export `{path}`"),
                    &export.path_span,
                );
                continue;
            }
            if !module_name.is_empty()
                && let Some(first_span) =
                    exported_modules.insert(module_name.clone(), export.path_span.clone())
            {
                self.push_manifest_export_diagnostic(
                    "manifest",
                    "package_doc.duplicate_exported_module",
                    format!("manifest export `{path}` duplicates module export `{module_name}`"),
                    &first_span,
                );
                continue;
            }
            if !available.contains(&path) {
                self.push_manifest_export_diagnostic(
                    "export",
                    "package_doc.missing_export",
                    format!("documentation export `{path}` is not in the package snapshot"),
                    &export.path_span,
                );
            }
        }
    }

    pub(super) fn push_manifest_export_diagnostic(
        &mut self,
        gate: &str,
        code: &str,
        message: String,
        span: &SourceSpan,
    ) {
        self.diagnostics.push(PackageDocDiagnostic {
            gate: gate.to_string(),
            code: code.to_string(),
            message,
            span: Some(PackageDocDiagnosticSpan::from_span(
                &source_uri(
                    self.identity,
                    self.snapshot.digest(),
                    SNAPSHOT_MANIFEST_PATH,
                ),
                span,
            )),
        });
    }

    pub(super) fn validate_manifest_gate(&mut self) {
        match manifest_field_with_span(&self.manifest.package.fields, "name") {
            Some(name) if name.value == self.identity => {}
            Some(name) => self.diagnostics.push(PackageDocDiagnostic {
                gate: "manifest".to_string(),
                code: "package_doc.package_identity_mismatch".to_string(),
                message: format!(
                    "manifest package name `{}` does not match package identity `{}`",
                    name.value, self.identity
                ),
                span: Some(PackageDocDiagnosticSpan::from_span(
                    &source_uri(
                        self.identity,
                        self.snapshot.digest(),
                        SNAPSHOT_MANIFEST_PATH,
                    ),
                    &name.value_span,
                )),
            }),
            None => self.diagnostics.push(PackageDocDiagnostic {
                gate: "manifest".to_string(),
                code: "package_doc.missing_package_name".to_string(),
                message: "manifest package name is required for package documentation generation"
                    .to_string(),
                span: Some(PackageDocDiagnosticSpan {
                    source_uri: source_uri(
                        self.identity,
                        self.snapshot.digest(),
                        SNAPSHOT_MANIFEST_PATH,
                    ),
                    line: 1,
                    column: 1,
                    offset: 0,
                }),
            }),
        }

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
                        SNAPSHOT_MANIFEST_PATH,
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
                            SNAPSHOT_MANIFEST_PATH,
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
                            SNAPSHOT_MANIFEST_PATH,
                        ),
                        &selector.field.key_span,
                    )),
                });
            }
        }
    }

    pub(super) fn validate_manifest_snapshot_binding(&mut self) {
        if self.manifest.source_bytes == self.snapshot.manifest_bytes() {
            return;
        }
        self.diagnostics.push(PackageDocDiagnostic {
            gate: "manifest".to_string(),
            code: "package_doc.manifest_snapshot_mismatch".to_string(),
            message: "validated manifest bytes do not match the captured package snapshot manifest"
                .to_string(),
            span: Some(PackageDocDiagnosticSpan {
                source_uri: source_uri(
                    self.identity,
                    self.snapshot.digest(),
                    SNAPSHOT_MANIFEST_PATH,
                ),
                line: 1,
                column: 1,
                offset: 0,
            }),
        });
    }

    pub(super) fn validated_manifest_export(
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
        if is_test_source_path(path.as_str()) {
            self.invalid_manifest_export(export, "export names a test source");
            return None;
        }
        let source = SourceFile::new(path.as_str(), "");
        let module_name = match derive_export_source_module_path(&source) {
            Ok(module_name) => module_name,
            Err(diagnostics)
                if diagnostics
                    .iter()
                    .all(is_source_path_invalid_case_diagnostic) =>
            {
                String::new()
            }
            Err(_) => {
                self.invalid_manifest_export(
                    export,
                    "manifest export path does not derive a valid module path",
                );
                return None;
            }
        };
        Some((path.as_str().to_string(), module_name))
    }

    pub(super) fn validated_exported_modules(
        &self,
        parsed_sources: &[ParsedPackageSource],
    ) -> Vec<(String, String)> {
        let available = parsed_sources
            .iter()
            .map(|source| source.source.path().as_str().to_string())
            .collect::<BTreeSet<_>>();
        let mut seen_paths = BTreeSet::new();
        let mut seen_modules = BTreeSet::new();
        let mut exports = Vec::new();
        for export in &self.manifest.lib.exports {
            if export.path.contains("::") {
                continue;
            }
            let path = SourcePath::new(export.path.clone());
            if !is_package_relative_path(path.as_str())
                || !path.as_str().ends_with(".veln")
                || is_test_source_path(path.as_str())
            {
                continue;
            }
            let source = SourceFile::new(path.as_str(), "");
            let Ok(module_name) = derive_export_source_module_path(&source) else {
                continue;
            };
            if available.contains(path.as_str())
                && seen_paths.insert(path.as_str().to_string())
                && seen_modules.insert(module_name.clone())
            {
                exports.push((path.as_str().to_string(), module_name));
            }
        }
        exports
    }

    pub(super) fn invalid_manifest_export(
        &mut self,
        export: &veln_project::ManifestExport,
        reason: &str,
    ) {
        self.diagnostics.push(PackageDocDiagnostic {
            gate: "manifest".to_string(),
            code: "package_doc.invalid_export".to_string(),
            message: format!("manifest export `{}` is invalid: {reason}", export.path),
            span: Some(PackageDocDiagnosticSpan::from_span(
                &source_uri(
                    self.identity,
                    self.snapshot.digest(),
                    SNAPSHOT_MANIFEST_PATH,
                ),
                &export.path_span,
            )),
        });
    }

    pub(super) fn validate_doctests(&mut self, parsed_sources: &[ParsedPackageSource]) {
        let public_sources = parsed_sources
            .iter()
            .filter(|source| source.exported)
            .filter_map(public_doctest_source)
            .collect::<Vec<_>>();
        if public_sources.is_empty() {
            return;
        }
        let doctest_source_locations = doctest_source_locations(&public_sources);
        let doctests = doctest_sources(&public_sources);
        self.report_doctest_extraction_diagnostics(
            &doctests.diagnostics,
            &doctest_source_locations,
        );
        let generated_doctests = generated_doctest_static_gate_sources(&doctests.sources);
        let static_gate_locations = doctest_static_gate_locations(&generated_doctests);
        let diagnostics = reconcile_package_expected_doctest_failures(
            analyze_doctest_static_gate(parsed_sources, &generated_doctests),
            &doctests.expected_failures,
        );
        self.report_doctest_static_gate_diagnostics(
            diagnostics,
            &doctest_source_locations,
            &static_gate_locations,
        );
    }

    fn report_doctest_extraction_diagnostics(
        &mut self,
        diagnostics: &[Diagnostic],
        doctest_source_locations: &BTreeMap<String, Vec<SourceSpan>>,
    ) {
        for diagnostic in diagnostics {
            if diagnostic.severity == Severity::Error {
                let diagnostic = remap_doctest_diagnostic(
                    diagnostic.clone(),
                    doctest_source_locations,
                    &BTreeMap::new(),
                );
                self.diagnostics
                    .push(self.project_diagnostic("doctest", diagnostic));
            }
        }
    }

    fn report_doctest_static_gate_diagnostics(
        &mut self,
        diagnostics: Vec<Diagnostic>,
        doctest_source_locations: &BTreeMap<String, Vec<SourceSpan>>,
        static_gate_locations: &BTreeMap<String, BTreeMap<usize, DoctestSourceLineOrigin>>,
    ) {
        for diagnostic in diagnostics {
            if diagnostic.severity == Severity::Error && is_doctest_gate_diagnostic(&diagnostic) {
                let diagnostic = remap_doctest_diagnostic(
                    diagnostic,
                    doctest_source_locations,
                    static_gate_locations,
                );
                self.diagnostics
                    .push(self.project_diagnostic("doctest", diagnostic));
            }
        }
    }
}

fn generated_doctest_static_gate_sources(sources: &[SourceFile]) -> Vec<GeneratedDoctestSource> {
    sources
        .iter()
        .map(generated_doctest_static_gate_source)
        .collect()
}

fn doctest_static_gate_locations(
    sources: &[GeneratedDoctestSource],
) -> BTreeMap<String, BTreeMap<usize, DoctestSourceLineOrigin>> {
    sources
        .iter()
        .map(|source| {
            (
                source.source.path().as_str().to_string(),
                source.line_origins.clone(),
            )
        })
        .collect()
}

fn analyze_doctest_static_gate(
    parsed_sources: &[ParsedPackageSource],
    generated_doctests: &[GeneratedDoctestSource],
) -> Vec<Diagnostic> {
    analyze_project(
        Project {
            root: ".".into(),
            files: generated_doctests
                .iter()
                .map(|source| source.source.clone())
                .chain(
                    parsed_sources
                        .iter()
                        .filter(|source| source.exported)
                        .map(|source| source.source.clone()),
                )
                .collect(),
            manifest: None,
        },
        DoctestMode::Exclude,
    )
    .checked_diagnostics()
}
