use super::*;

impl<'a> PackageDocBuilder<'a> {
    pub(super) fn public_schema_targets(
        &self,
        parsed_sources: &'a [ParsedPackageSource],
    ) -> SchemaDocResolver<'a> {
        let mut sources = BTreeMap::new();
        let mut schemas = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        for source in parsed_sources.iter().filter(|source| source.exported) {
            sources.insert(source.module_name.clone(), source);
            for item in &source.tree.items {
                match item {
                    SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => {
                        let name = schema.name.clone().unwrap_or_default();
                        let identity = format!(
                            "schema:{}::{name}:{}",
                            source.module_name,
                            schema_signature(schema)
                        );
                        let declaration_id = self.declaration_id("schema", &identity);
                        let target_uri = self.declaration_uri(&declaration_id, "");
                        schemas.insert(
                            (source.module_name.clone(), name),
                            PublicSchemaDocTarget {
                                declaration_id,
                                target_uri,
                            },
                        );
                    }
                    SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Schema => {
                        aliases.insert(
                            (
                                source.module_name.clone(),
                                alias.name.clone().unwrap_or_default(),
                            ),
                            alias.target.clone(),
                        );
                    }
                    _ => {}
                }
            }
        }
        SchemaDocResolver {
            sources,
            schemas,
            aliases,
        }
    }

    pub(super) fn validate_doc_references(
        &mut self,
        parsed_sources: &[ParsedPackageSource],
        schema_targets: &SchemaDocResolver<'_>,
    ) {
        for source in parsed_sources.iter().filter(|source| source.exported) {
            for target_line in public_documentation_lines(&source.tree) {
                if doc_block_before(&source.source, target_line).is_empty() {
                    continue;
                }
                for reference in doc_schema_references_before(&source.source, target_line) {
                    if schema_targets
                        .resolve(&reference.target, &source.module_name)
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
    }

    pub(super) fn declarations(
        &mut self,
        source: &ParsedPackageSource,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &SchemaDocResolver<'_>,
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

    pub(super) fn type_declaration(
        &mut self,
        source: &ParsedPackageSource,
        type_decl: &TypeDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> PackageDocDeclaration {
        let name = type_decl.name.clone().unwrap_or_default();
        let identity = format!(
            "type:{}::{name}:{}",
            source.module_name,
            type_signature(type_decl)
        );
        self.record_semantic_identity(&identity, &type_decl.span, semantic_identities);
        let declaration_id = self.declaration_id("type", &identity);
        record_declaration_location(
            &source.source,
            &source.source_uri,
            declaration_locations,
            &declaration_id,
            &type_decl.span,
            type_decl.name.as_deref(),
        );
        for variant in &type_decl.variants {
            if variant.visibility == Visibility::Public {
                record_declaration_location(
                    &source.source,
                    &source.source_uri,
                    declaration_locations,
                    &declaration_id,
                    &variant.span,
                    variant.name.as_deref(),
                );
            }
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
                .map(|variant| self.type_constructor(&source.source, variant, schema_targets))
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

    pub(super) fn schema_declaration(
        &mut self,
        source: &ParsedPackageSource,
        schema: &SchemaDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> PackageDocDeclaration {
        let name = schema.name.clone().unwrap_or_default();
        let identity = format!(
            "schema:{}::{name}:{}",
            source.module_name,
            schema_signature(schema)
        );
        self.record_semantic_identity(&identity, &schema.span, semantic_identities);
        let declaration_id = self.declaration_id("schema", &identity);
        record_declaration_location(
            &source.source,
            &source.source_uri,
            declaration_locations,
            &declaration_id,
            &schema.span,
            schema.name.as_deref(),
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

    pub(super) fn function_declaration(
        &mut self,
        source: &ParsedPackageSource,
        function: &FunctionDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> PackageDocDeclaration {
        let name = function.name.clone().unwrap_or_default();
        let signature = function_signature(function);
        let identity = format!("function:{}::{name}:{signature}", source.module_name);
        self.record_semantic_identity(&identity, &function.span, semantic_identities);
        let declaration_id = self.declaration_id("function", &identity);
        record_declaration_location(
            &source.source,
            &source.source_uri,
            declaration_locations,
            &declaration_id,
            &function.span,
            function.name.as_deref(),
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

    pub(super) fn alias_declaration(
        &mut self,
        source: &ParsedPackageSource,
        alias: &PublicAliasDecl,
        semantic_identities: &mut BTreeMap<String, SourceSpan>,
        declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> PackageDocDeclaration {
        let name = alias.name.clone().unwrap_or_default();
        let signature = alias_signature(alias);
        let kind = alias_kind(alias.kind).to_string();
        let identity = format!("alias:{kind}:{}::{name}:{signature}", source.module_name);
        self.record_semantic_identity(&identity, &alias.span, semantic_identities);
        let declaration_id = self.declaration_id("alias", &identity);
        record_declaration_location(
            &source.source,
            &source.source_uri,
            declaration_locations,
            &declaration_id,
            &alias.span,
            alias.name.as_deref(),
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

    pub(super) fn record_semantic_identity(
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

    pub(super) fn doctests_for(
        &mut self,
        source: &SourceFile,
        target_line: usize,
    ) -> Vec<PackageDocDoctest> {
        let extracted = visible_doctests_for(source, target_line);
        for diagnostic in extracted.diagnostics {
            self.diagnostics
                .push(self.project_diagnostic("doctest", diagnostic));
        }
        extracted.doctests
    }

    pub(super) fn module_doctests(
        &mut self,
        source: &ParsedPackageSource,
    ) -> Vec<PackageDocDoctest> {
        source
            .tree
            .module
            .as_ref()
            .map(|module| self.doctests_for(&source.source, module.span.start.line))
            .unwrap_or_default()
    }

    pub(super) fn validate_doctest(&mut self, _doctest: &PackageDocDoctest) {
        // The shared analysis pipeline validates visible Veln doctests above.
    }
}
