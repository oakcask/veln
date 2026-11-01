use super::*;

type PublicationSpanKey = (
    String,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    Option<String>,
    Option<(usize, usize, usize)>,
    Option<(usize, usize, usize)>,
);
impl LoweredSurfaceModule {
    pub fn module_value_publication_diagnostic_indices_for_identities(
        &self,
        identities: &[ModuleValuePublicationIdentity],
    ) -> (usize, Vec<usize>, ModuleValuePublicationWork) {
        let mut work = self.retained_publication_work(identities.len());
        let mut pending = self.publication_roots_for_identities(identities, &mut work);
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
        let mut pending = self.publication_roots_for_identities(identities, &mut work);
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

    fn retained_publication_work(
        &self,
        exported_root_lookups: usize,
    ) -> ModuleValuePublicationWork {
        ModuleValuePublicationWork {
            retained_values: self.module_value_publication.values.len(),
            retained_edges: self.module_value_publication.retained_edges,
            retained_direct_causes: self.module_value_publication.retained_direct_causes,
            collision_identity_insertions: self
                .module_value_publication
                .collision_identity_insertions,
            collision_diagnostic_lookups: self
                .module_value_publication
                .collision_diagnostic_lookups,
            collision_association_iterations: self
                .module_value_publication
                .collision_association_iterations,
            visited_initializations: self.module_value_publication.values.len(),
            exported_root_lookups,
            ..ModuleValuePublicationWork::default()
        }
    }

    fn publication_roots_for_identities(
        &self,
        identities: &[ModuleValuePublicationIdentity],
        work: &mut ModuleValuePublicationWork,
    ) -> Vec<usize> {
        let mut seen = HashSet::with_capacity(identities.len());
        let mut pending = Vec::new();
        for identity in identities {
            if !seen.insert(identity) {
                continue;
            }
            if let Some(indices) = self
                .module_value_publication
                .failures_by_identity
                .get(identity)
            {
                work.identity_root_expansions += indices.len();
                pending.extend(indices.iter().copied());
            }
        }
        pending
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
    let collision_identity_insertions = module
        .functions
        .iter()
        .filter(|function| function.name.is_some())
        .count()
        + module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Function && alias.name.is_some())
            .count()
        + module
            .module_values
            .iter()
            .filter(|value| value.name.is_some())
            .count();
    let (
        collision_diagnostic_indices,
        collision_diagnostic_lookups,
        collision_association_iterations,
    ) = collision_diagnostic_indices(diagnostics, &provenance.callable_collisions);
    let failures_by_identity = failures_by_identity(&provenance);
    let values = indexed_values(provenance, direct_diagnostic_indices);
    let retained_edges = values.iter().map(retained_value_edges).sum();
    let retained_direct_causes = values.iter().map(|value| value.direct_causes.len()).sum();
    IndexedModuleValuePublicationProvenance {
        failures_by_identity,
        values,
        collision_diagnostic_indices,
        collision_identity_insertions,
        collision_diagnostic_lookups,
        collision_association_iterations,
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
            Some((
                (
                    diagnostic.id.as_str(),
                    publication_span_key(span),
                    diagnostic.message.as_str(),
                ),
                index,
            ))
        })
        .collect::<HashMap<_, _>>();
    let diagnostics_by_span = diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            let span = diagnostic.span.as_ref()?;
            Some(((diagnostic.id.as_str(), publication_span_key(span)), index))
        })
        .collect::<HashMap<_, _>>();
    for (value_index, value) in provenance.values.iter().enumerate() {
        for cause in &value.direct_causes {
            if let Some(index) = cause.diagnostic_index.filter(|index| {
                diagnostics
                    .get(*index)
                    .is_some_and(|diagnostic| diagnostic_matches_cause(diagnostic, cause))
            }) {
                indices[value_index].push(index);
                continue;
            }
            let index = if let Some(message) = cause.diagnostic_message.as_deref() {
                diagnostics_by_key.get(&(
                    cause.diagnostic_id.as_str(),
                    publication_span_key(&cause.span),
                    message,
                ))
            } else {
                diagnostics_by_span.get(&(
                    cause.diagnostic_id.as_str(),
                    publication_span_key(&cause.span),
                ))
            };
            if let Some(index) = index {
                indices[value_index].push(*index);
            }
        }
        indices[value_index].sort_unstable();
        indices[value_index].dedup();
    }
    indices
}

fn diagnostic_matches_cause(
    diagnostic: &Diagnostic,
    cause: &crate::analysis::ModuleValuePublicationCause,
) -> bool {
    diagnostic.id == cause.diagnostic_id
        && diagnostic.span.as_ref() == Some(&cause.span)
        && cause
            .diagnostic_message
            .as_deref()
            .is_none_or(|message| diagnostic.message == message)
}

fn collision_diagnostic_indices(
    diagnostics: &[Diagnostic],
    collision_causes: &[(crate::types::ModuleValueIdentity, Diagnostic)],
) -> (
    HashMap<crate::types::ModuleValueIdentity, Vec<usize>>,
    usize,
    usize,
) {
    let mut indices = HashMap::<crate::types::ModuleValueIdentity, Vec<usize>>::new();
    let diagnostics_by_key = diagnostics
        .iter()
        .enumerate()
        .filter(|(_, diagnostic)| diagnostic.id == "name.duplicate")
        .filter_map(|(index, diagnostic)| {
            Some((
                (
                    diagnostic.id.clone(),
                    publication_span_key(diagnostic.span.as_ref()?),
                    diagnostic.message.as_str().to_owned(),
                ),
                index,
            ))
        })
        .collect::<HashMap<_, _>>();
    let mut lookups = 0;
    let mut association_iterations = 0;
    for (identity, diagnostic) in collision_causes {
        lookups += 1;
        association_iterations += 1;
        let Some(span) = diagnostic.span.as_ref() else {
            continue;
        };
        let key = (
            diagnostic.id.clone(),
            publication_span_key(span),
            diagnostic.message.as_str().to_owned(),
        );
        if let Some(index) = diagnostics_by_key.get(&key) {
            indices.entry(identity.clone()).or_default().push(*index);
        }
    }
    (indices, lookups, association_iterations)
}

fn failures_by_identity(
    provenance: &ModuleValuePublicationProvenance,
) -> HashMap<ModuleValuePublicationIdentity, Vec<usize>> {
    provenance
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            (value.invalid).then(|| value.identity.clone().map(|identity| (identity, index)))?
        })
        .fold(HashMap::new(), |mut failures, (identity, index)| {
            failures
                .entry(identity)
                .or_insert_with(Vec::new)
                .push(index);
            failures
        })
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
        span.start.line,
        span.start.column,
        span.start.offset,
        span.end.line,
        span.end.column,
        span.end.offset,
        span.original_path().map(|path| path.as_str().to_owned()),
        span.original_start()
            .map(|position| (position.line, position.column, position.offset)),
        span.original_end()
            .map(|position| (position.line, position.column, position.offset)),
    )
}
