use super::*;

type PublicationSpanKey = (String, usize, usize);

impl LoweredSurfaceModule {
    pub fn module_value_publication_diagnostic_indices_for_identities(
        &self,
        identities: &[ModuleValuePublicationIdentity],
    ) -> (usize, Vec<usize>, ModuleValuePublicationWork) {
        let mut work = self.retained_publication_work(identities.len());
        let mut pending = identities
            .iter()
            .filter_map(|identity| {
                self.module_value_publication
                    .failures_by_identity
                    .get(identity)
                    .copied()
            })
            .collect::<Vec<_>>();
        let matched_roots = pending.len();
        let diagnostic_indices = self
            .collect_publication_causes(&mut pending, &mut work)
            .into_iter()
            .map(|(_, index)| index)
            .collect();
        (matched_roots, diagnostic_indices, work)
    }

    pub fn module_value_publication_diagnostics_for_identities(
        &self,
        identities: &[ModuleValuePublicationIdentity],
    ) -> (
        Vec<ModuleValuePublicationDiagnostic>,
        ModuleValuePublicationWork,
    ) {
        let mut work = self.retained_publication_work(identities.len());
        let mut pending = identities
            .iter()
            .filter_map(|identity| {
                self.module_value_publication
                    .failures_by_identity
                    .get(identity)
                    .copied()
            })
            .collect::<Vec<_>>();
        let causes = self.collect_publication_causes(&mut pending, &mut work);
        (
            causes
                .into_iter()
                .map(|(package_name, index)| ModuleValuePublicationDiagnostic {
                    package_name,
                    diagnostic: self.diagnostics[index].clone(),
                })
                .collect(),
            work,
        )
    }

    pub fn module_value_publication_diagnostic_indices(
        &self,
        span: &veln_source::SourceSpan,
    ) -> Option<Vec<usize>> {
        let (matched_roots, diagnostic_indices, work) =
            self.module_value_publication_diagnostic_indices_for_spans(std::slice::from_ref(span));
        record_module_value_publication_work(|recorded| *recorded = work);
        let diagnostic_indices = diagnostic_indices
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        (matched_roots != 0).then_some(diagnostic_indices)
    }

    pub fn module_value_publication_diagnostic_indices_for_spans(
        &self,
        spans: &[veln_source::SourceSpan],
    ) -> (usize, Vec<usize>, ModuleValuePublicationWork) {
        let mut work = self.retained_publication_work(spans.len());
        let mut pending = self.publication_roots(spans);
        let matched_roots = pending.len();
        let diagnostic_indices = self.collect_publication_diagnostics(&mut pending, &mut work);
        (matched_roots, diagnostic_indices, work)
    }

    fn retained_publication_work(
        &self,
        exported_root_lookups: usize,
    ) -> ModuleValuePublicationWork {
        ModuleValuePublicationWork {
            retained_values: self.module_value_publication.values.len(),
            retained_edges: self.module_value_publication.retained_edges,
            retained_direct_causes: self.module_value_publication.retained_direct_causes,
            visited_initializations: self.module_value_publication.values.len(),
            exported_root_lookups,
            ..ModuleValuePublicationWork::default()
        }
    }

    fn publication_roots(&self, spans: &[veln_source::SourceSpan]) -> Vec<usize> {
        spans
            .iter()
            .filter_map(|span| {
                self.module_value_publication
                    .failures_by_file_and_range
                    .get(span.file.as_str())
                    .and_then(|ranges| ranges.get(&(span.start.offset, span.end.offset)))
                    .copied()
            })
            .collect()
    }

    fn collect_publication_diagnostics(
        &self,
        pending: &mut Vec<usize>,
        work: &mut ModuleValuePublicationWork,
    ) -> Vec<usize> {
        self.collect_publication_causes(pending, work)
            .into_iter()
            .map(|(_, index)| index)
            .collect()
    }

    fn collect_publication_causes(
        &self,
        pending: &mut Vec<usize>,
        work: &mut ModuleValuePublicationWork,
    ) -> Vec<(Option<String>, usize)> {
        let mut visited = vec![false; self.module_value_publication.values.len()];
        let mut causes = HashSet::new();
        while let Some(index) = pending.pop() {
            if std::mem::replace(&mut visited[index], true) {
                continue;
            }
            let value = &self.module_value_publication.values[index];
            self.record_publication_value(value, &mut causes, work);
            pending.extend(value.dependencies.iter().copied());
        }
        causes.into_iter().collect()
    }

    fn record_publication_value(
        &self,
        value: &IndexedModuleValuePublicationValue,
        causes: &mut HashSet<(Option<String>, usize)>,
        work: &mut ModuleValuePublicationWork,
    ) {
        work.traversal_values += 1;
        work.traversal_edges += retained_value_edges(value);
        work.cause_insertions += value.direct_causes.len();
        causes.extend(
            value
                .direct_causes
                .iter()
                .map(|cause| (cause.package_name.clone(), cause.diagnostic_index)),
        );
        if let Some(identity) = value.identity.as_ref() {
            self.record_collision_diagnostics(identity, causes, work);
        }
        for identity in &value.collision_dependencies {
            self.record_collision_diagnostics(identity, causes, work);
        }
    }

    fn record_collision_diagnostics(
        &self,
        identity: &crate::types::ModuleValueIdentity,
        causes: &mut HashSet<(Option<String>, usize)>,
        work: &mut ModuleValuePublicationWork,
    ) {
        if let Some(indices) = self
            .module_value_publication
            .collision_diagnostic_indices
            .get(identity)
        {
            work.cause_insertions += indices.len();
            causes.extend(indices.iter().map(|index| (identity.0.clone(), *index)));
        }
    }
}

pub(super) fn index_module_value_publication(
    module: &SurfaceModule,
    diagnostics: &[Diagnostic],
    provenance: ModuleValuePublicationProvenance,
) -> IndexedModuleValuePublicationProvenance {
    let direct_diagnostic_indices = direct_diagnostic_indices(diagnostics, &provenance);
    let callable_identities = callable_identities(module);
    let collision_diagnostic_indices =
        collision_diagnostic_indices(diagnostics, &callable_identities);
    let failures_by_file_and_range = failures_by_file_and_range(&provenance);
    let failures_by_identity = failures_by_identity(&provenance);
    let values = indexed_values(provenance, direct_diagnostic_indices);
    let retained_edges = values.iter().map(retained_value_edges).sum();
    let retained_direct_causes = values.iter().map(|value| value.direct_causes.len()).sum();
    IndexedModuleValuePublicationProvenance {
        failures_by_identity,
        failures_by_file_and_range,
        values,
        collision_diagnostic_indices,
        retained_edges,
        retained_direct_causes,
    }
}

fn direct_diagnostic_indices(
    diagnostics: &[Diagnostic],
    provenance: &ModuleValuePublicationProvenance,
) -> Vec<Vec<usize>> {
    let mut indices = vec![Vec::new(); provenance.values.len()];
    let diagnostics_by_key = diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            let span = diagnostic.span.as_ref()?;
            Some(((diagnostic.id.as_str(), publication_span_key(span)), index))
        })
        .collect::<HashMap<_, _>>();
    for (value_index, value) in provenance.values.iter().enumerate() {
        for cause in &value.direct_causes {
            if let Some(index) = cause.diagnostic_index {
                indices[value_index].push(index);
            } else if let Some(index) = diagnostics_by_key.get(&(
                cause.diagnostic_id.as_str(),
                publication_span_key(&cause.span),
            )) {
                indices[value_index].push(*index);
            }
        }
        indices[value_index].sort_unstable();
        indices[value_index].dedup();
    }
    indices
}

fn callable_identities(
    module: &SurfaceModule,
) -> Vec<(PublicationSpanKey, crate::types::ModuleValueIdentity)> {
    let mut identities = Vec::new();
    insert_function_identities(module, &mut identities);
    insert_alias_identities(module, &mut identities);
    insert_value_identities(module, &mut identities);
    identities
}

fn insert_function_identities(
    module: &SurfaceModule,
    identities: &mut Vec<(PublicationSpanKey, crate::types::ModuleValueIdentity)>,
) {
    for function in &module.functions {
        if let Some(name) = function.name.as_ref() {
            identities.push((
                publication_span_key(&function.span),
                (
                    function.package_name.clone(),
                    function.module_name.clone(),
                    name.clone(),
                ),
            ));
        }
    }
}

fn insert_alias_identities(
    module: &SurfaceModule,
    identities: &mut Vec<(PublicationSpanKey, crate::types::ModuleValueIdentity)>,
) {
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        if let Some(name) = alias.name.as_ref() {
            identities.push((
                publication_span_key(&alias.span),
                (
                    alias.package_name.clone(),
                    alias.module_name.clone(),
                    name.clone(),
                ),
            ));
        }
    }
}

fn insert_value_identities(
    module: &SurfaceModule,
    identities: &mut Vec<(PublicationSpanKey, crate::types::ModuleValueIdentity)>,
) {
    for value in &module.module_values {
        if let Some(name) = value.name.as_ref() {
            identities.push((
                publication_span_key(&value.span),
                (
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                ),
            ));
        }
    }
}

fn collision_diagnostic_indices(
    diagnostics: &[Diagnostic],
    callable_identities: &[(PublicationSpanKey, crate::types::ModuleValueIdentity)],
) -> HashMap<crate::types::ModuleValueIdentity, Vec<usize>> {
    let mut indices = HashMap::<crate::types::ModuleValueIdentity, Vec<usize>>::new();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic.id != "name.duplicate" {
            continue;
        }
        let Some(span) = diagnostic.span.as_ref() else {
            continue;
        };
        let key = publication_span_key(span);
        for (_, identity) in callable_identities.iter().filter(|(span, _)| *span == key) {
            indices.entry(identity.clone()).or_default().push(index);
        }
    }
    indices
}

fn failures_by_file_and_range(
    provenance: &ModuleValuePublicationProvenance,
) -> HashMap<String, HashMap<(usize, usize), usize>> {
    let mut failures = HashMap::<String, HashMap<(usize, usize), usize>>::new();
    for (value_index, value) in provenance
        .values
        .iter()
        .enumerate()
        .filter(|(_, value)| value.invalid && value.identity.is_some())
    {
        failures
            .entry(value.span.file.as_str().to_owned())
            .or_default()
            .insert(
                (value.span.start.offset, value.span.end.offset),
                value_index,
            );
    }
    failures
}

fn failures_by_identity(
    provenance: &ModuleValuePublicationProvenance,
) -> HashMap<ModuleValuePublicationIdentity, usize> {
    provenance
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            (value.invalid).then(|| value.identity.clone().map(|identity| (identity, index)))?
        })
        .collect()
}

fn indexed_values(
    provenance: ModuleValuePublicationProvenance,
    mut direct_diagnostic_indices: Vec<Vec<usize>>,
) -> Vec<IndexedModuleValuePublicationValue> {
    provenance
        .values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let package_name = value
                .identity
                .as_ref()
                .and_then(|identity| identity.0.clone());
            IndexedModuleValuePublicationValue {
                identity: value.identity,
                dependencies: value.dependencies,
                collision_dependencies: value.collision_dependencies,
                direct_causes: std::mem::take(&mut direct_diagnostic_indices[index])
                    .into_iter()
                    .map(|diagnostic_index| IndexedModuleValuePublicationCause {
                        package_name: package_name.clone(),
                        diagnostic_index,
                    })
                    .collect(),
            }
        })
        .collect()
}

fn retained_value_edges(value: &IndexedModuleValuePublicationValue) -> usize {
    value.dependencies.len() + value.collision_dependencies.len()
}

fn publication_span_key(span: &veln_source::SourceSpan) -> PublicationSpanKey {
    (
        span.file.as_str().to_owned(),
        span.start.offset,
        span.end.offset,
    )
}
