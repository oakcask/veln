use super::*;

type ProjectDiagnosticKey = (String, String, usize, usize);

impl PackageDocBuilder<'_> {
    pub(super) fn parse_sources(&mut self) -> Vec<ParsedPackageSource> {
        let exported_paths = self
            .manifest
            .lib
            .exports
            .iter()
            .map(|export| SourcePath::new(export.path.clone()).as_str().to_string())
            .collect::<BTreeSet<_>>();
        self.snapshot
            .sources()
            .iter()
            .map(|source| self.parse_source(source, &exported_paths))
            .collect()
    }

    fn parse_source(
        &mut self,
        source: &veln_project::CapturedPackageSource,
        exported_paths: &BTreeSet<String>,
    ) -> ParsedPackageSource {
        let text = std::str::from_utf8(source.bytes())
            .expect("captured package source text is valid UTF-8");
        let source_file = SourceFile::new(source.path(), text);
        let source_uri = source_uri(self.identity, self.snapshot.digest(), source.path());
        let output = parse(&source_file);
        let public_module_value_name_spans = Self::public_module_value_name_spans(&output.tree);
        self.push_parse_diagnostics(output.diagnostics, &public_module_value_name_spans);
        let exported = exported_paths.contains(source.path());
        let module_name = self.module_name(&source_file, exported);
        ParsedPackageSource {
            source: source_file,
            tree: output.tree,
            module_name,
            exported,
            source_uri,
        }
    }

    fn module_name(&mut self, source: &SourceFile, exported: bool) -> String {
        let derived = if exported {
            derive_export_source_module_path(source)
        } else {
            derive_source_module_path(source).map_err(|diagnostic| vec![*diagnostic])
        };
        match derived {
            Ok(module_name) => module_name,
            Err(diagnostics) => {
                self.diagnostics
                    .extend(diagnostics.into_iter().map(|diagnostic| {
                        module_diagnostic(
                            "manifest",
                            diagnostic,
                            self.identity,
                            self.snapshot.digest(),
                        )
                    }));
                String::new()
            }
        }
    }

    fn public_module_value_name_spans(tree: &veln_syntax::SyntaxTree) -> HashSet<(usize, usize)> {
        tree.items
            .iter()
            .filter_map(|item| match item {
                SyntaxItem::ModuleValue(value) if value.visibility == Visibility::Public => {
                    value.name_span.as_ref()
                }
                _ => None,
            })
            .map(|span| (span.start.offset, span.end.offset))
            .collect()
    }

    fn push_parse_diagnostics(
        &mut self,
        diagnostics: Vec<ParseDiagnostic>,
        public_module_value_name_spans: &HashSet<(usize, usize)>,
    ) {
        for mut diagnostic in diagnostics {
            let gate = if Self::is_public_module_value_casing_diagnostic(
                &diagnostic,
                public_module_value_name_spans,
            ) {
                diagnostic.id = "name.invalid_case";
                "module_value"
            } else {
                "parse"
            };
            self.diagnostics.push(parse_diagnostic(
                gate,
                diagnostic,
                self.identity,
                self.snapshot.digest(),
            ));
        }
    }

    fn is_public_module_value_casing_diagnostic(
        diagnostic: &ParseDiagnostic,
        public_module_value_name_spans: &HashSet<(usize, usize)>,
    ) -> bool {
        diagnostic.id == "parse.module_value_declaration"
            && diagnostic.message == "module value name must start with an ASCII lowercase letter"
            && diagnostic.span.as_ref().is_some_and(|span| {
                record_package_doc_semantic_classification_work(|work| {
                    work.casing_range_comparisons += 1;
                });
                public_module_value_name_spans.contains(&(span.start.offset, span.end.offset))
            })
    }

    pub(super) fn validate_public_declaration_semantics(
        &mut self,
        parsed_sources: &[ParsedPackageSource],
    ) {
        let exported_paths = parsed_sources
            .iter()
            .filter(|source| source.exported)
            .map(|source| source.source.path().as_str())
            .collect::<BTreeSet<_>>();
        let analysis = analyze_project_with_captured_dependencies(
            Project {
                root: self.project_root.to_path_buf(),
                files: parsed_sources
                    .iter()
                    .map(|source| source.source.clone())
                    .collect(),
                manifest: Some(self.manifest.clone()),
            },
            DoctestMode::Exclude,
            self.dependencies,
        );
        let diagnostics = analysis.checked_diagnostics();
        let mut gates = vec![None; diagnostics.len()];
        let diagnostic_indices_by_key = Self::diagnostic_indices_by_key(&diagnostics);
        self.mark_publication_gates(
            parsed_sources,
            &analysis,
            &exported_paths,
            &diagnostic_indices_by_key,
            &mut gates,
        );
        self.retain_gated_project_diagnostics(&diagnostics, &gates);
    }

    fn mark_publication_gates(
        &mut self,
        parsed_sources: &[ParsedPackageSource],
        analysis: &ProjectAnalysis,
        exported_paths: &BTreeSet<&str>,
        diagnostic_indices_by_key: &HashMap<ProjectDiagnosticKey, Vec<usize>>,
        gates: &mut [Option<&'static str>],
    ) {
        let exported_module_value_identities =
            Self::exported_module_value_identities(parsed_sources);
        let (causes, publication_work) = analysis
            .module_value_publication_diagnostics_for_identities(&exported_module_value_identities);
        Self::record_publication_work(publication_work);
        self.retain_module_value_causes(causes);
        Self::mark_semantic_visibility_gates(
            analysis,
            exported_paths,
            diagnostic_indices_by_key,
            gates,
        );
    }

    fn diagnostic_indices_by_key(
        diagnostics: &[Diagnostic],
    ) -> HashMap<ProjectDiagnosticKey, Vec<usize>> {
        diagnostics
            .iter()
            .enumerate()
            .filter_map(|(index, diagnostic)| {
                record_package_doc_semantic_classification_work(|work| {
                    work.diagnostic_lookups += 1;
                });
                diagnostic.span.as_ref().map(|span| {
                    (
                        (
                            diagnostic.id.clone(),
                            span.file.as_str().to_owned(),
                            span.start.offset,
                            span.end.offset,
                        ),
                        index,
                    )
                })
            })
            .fold(HashMap::new(), |mut indices, (key, index)| {
                indices.entry(key).or_insert_with(Vec::new).push(index);
                indices
            })
    }

    fn exported_module_value_identities(
        parsed_sources: &[ParsedPackageSource],
    ) -> Vec<veln_sema::ModuleValuePublicationIdentity> {
        parsed_sources
            .iter()
            .filter(|source| source.exported)
            .flat_map(|source| {
                source.tree.items.iter().filter_map(|item| match item {
                    SyntaxItem::ModuleValue(value) if value.visibility == Visibility::Public => {
                        Some((None, Some(source.module_name.clone()), value.name.clone()?))
                    }
                    _ => None,
                })
            })
            .collect()
    }

    fn record_publication_work(publication_work: veln_sema::ModuleValuePublicationWork) {
        record_package_doc_semantic_classification_work(|work| {
            work.exported_module_value_lookups += publication_work.exported_root_lookups;
            work.retained_values += publication_work.retained_values;
            work.retained_edges += publication_work.retained_edges;
            work.retained_direct_causes += publication_work.retained_direct_causes;
            work.visited_initializations += publication_work.visited_initializations;
            work.traversal_values += publication_work.traversal_values;
            work.traversal_edges += publication_work.traversal_edges;
            work.cause_processing += publication_work.cause_insertions;
        });
    }

    fn retain_module_value_causes(
        &mut self,
        causes: Vec<veln_sema::ModuleValuePublicationDiagnostic>,
    ) {
        for cause in causes {
            record_package_doc_semantic_classification_work(|work| {
                work.cause_processing += 1;
            });
            record_package_doc_semantic_classification_work(|work| {
                work.cause_index_insertions += 1;
            });
            self.diagnostics.push(self.project_diagnostic_for_package(
                "module_value",
                cause.package_name.as_deref(),
                cause.diagnostic,
            ));
        }
    }

    fn mark_semantic_visibility_gates(
        analysis: &ProjectAnalysis,
        exported_paths: &BTreeSet<&str>,
        diagnostic_indices_by_key: &HashMap<ProjectDiagnosticKey, Vec<usize>>,
        gates: &mut [Option<&'static str>],
    ) {
        for cause in analysis
            .unpublished_declarations()
            .iter()
            .filter(|declaration| exported_paths.contains(declaration.span.file.as_str()))
            .flat_map(|declaration| declaration.causal_diagnostics.iter())
            .filter(|cause| cause.diagnostic_id == "type.variant_refinement_private")
        {
            record_package_doc_semantic_classification_work(|work| {
                work.cause_processing += 1;
            });
            let key = (
                cause.diagnostic_id.clone(),
                cause.span.file.as_str().to_owned(),
                cause.span.start.offset,
                cause.span.end.offset,
            );
            if let Some(index) = diagnostic_indices_by_key
                .get(&key)
                .and_then(|indices| indices.first())
            {
                record_package_doc_semantic_classification_work(|work| {
                    work.cause_index_insertions += 1;
                });
                gates[*index].get_or_insert("semantic_visibility");
            }
        }
    }

    fn retain_gated_project_diagnostics(
        &mut self,
        diagnostics: &[Diagnostic],
        gates: &[Option<&'static str>],
    ) {
        for (index, diagnostic) in diagnostics.iter().enumerate() {
            record_package_doc_semantic_classification_work(|work| {
                work.diagnostic_lookups += 1;
            });
            if diagnostic.severity == Severity::Error
                && diagnostic.id != "module.value_not_executable"
                && let Some(gate) = gates[index]
            {
                self.diagnostics
                    .push(self.project_diagnostic(gate, diagnostic.clone()));
            }
        }
    }
}
