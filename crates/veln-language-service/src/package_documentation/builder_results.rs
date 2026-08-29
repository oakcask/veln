use super::*;

impl<'a> PackageDocBuilder<'a> {
    pub(super) fn references_for(
        &self,
        source: &SourceFile,
        target_line: usize,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> Vec<PackageDocReference> {
        let current_module = derive_source_module_path(source).unwrap_or_default();
        doc_schema_references_before(source, target_line)
            .into_iter()
            .filter_map(|reference| {
                schema_targets
                    .resolve(&reference.target, &current_module)
                    .map(|target| PackageDocReference {
                        kind: "schema".to_string(),
                        marker: reference.target,
                        target_declaration_id: target.declaration_id,
                        target_uri: target.target_uri,
                    })
            })
            .collect()
    }

    pub(super) fn module_references(
        &self,
        source: &ParsedPackageSource,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> Vec<PackageDocReference> {
        source
            .tree
            .module
            .as_ref()
            .map(|module| {
                self.references_for(&source.source, module.span.start.line, schema_targets)
            })
            .unwrap_or_default()
    }

    pub(super) fn type_constructor(
        &mut self,
        source: &SourceFile,
        variant: &TypeVariantDecl,
        schema_targets: &SchemaDocResolver<'_>,
    ) -> PackageDocTypeConstructor {
        PackageDocTypeConstructor {
            name: variant.name.clone().unwrap_or_default(),
            signature: variant_signature(variant),
            doc: doc_block_before(source, variant.span.start.line),
            doctests: self.doctests_for(source, variant.span.start.line),
            references: self.references_for(source, variant.span.start.line, schema_targets),
        }
    }

    pub(super) fn project_diagnostic(
        &self,
        gate: &str,
        diagnostic: Diagnostic,
    ) -> PackageDocDiagnostic {
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

    pub(super) fn failed_result(mut self) -> PackageDocResult {
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

    pub(super) fn sort_diagnostics(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            left.span
                .cmp(&right.span)
                .then(left.code.cmp(&right.code))
                .then(left.message.cmp(&right.message))
        });
    }

    pub(super) fn index_uri(&self, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/index",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    pub(super) fn status_uri(&self, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/status",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    pub(super) fn module_uri(&self, module_id: &str, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/module/{module_id}",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    pub(super) fn declaration_uri(&self, declaration_id: &str, doc_digest: &str) -> String {
        format!(
            "{URI_PREFIX}{}/snapshot/{}/documentation/{doc_digest}/declaration/{declaration_id}",
            encoded_segment(self.identity),
            self.snapshot.digest()
        )
    }

    pub(super) fn declaration_id(&self, kind: &str, identity: &str) -> String {
        #[cfg(test)]
        if let Some(id) = &self.forced_declaration_id {
            return id.clone();
        }

        declaration_id(kind, identity)
    }
}
