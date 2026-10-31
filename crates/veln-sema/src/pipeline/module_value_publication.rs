use super::*;

type PublicationSpanKey = (String, usize, usize);

impl LoweredSurfaceModule {
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
        let mut visited = vec![false; self.module_value_publication.values.len()];
        let mut diagnostic_indices = HashSet::new();
        while let Some(index) = pending.pop() {
            if std::mem::replace(&mut visited[index], true) {
                continue;
            }
            let value = &self.module_value_publication.values[index];
            self.record_publication_value(value, &mut diagnostic_indices, work);
            pending.extend(value.dependencies.iter().copied());
        }
        diagnostic_indices.into_iter().collect()
    }

    fn record_publication_value(
        &self,
        value: &IndexedModuleValuePublicationValue,
        diagnostic_indices: &mut HashSet<usize>,
        work: &mut ModuleValuePublicationWork,
    ) {
        work.traversal_values += 1;
        work.traversal_edges += retained_value_edges(value);
        Self::record_diagnostic_indices(&value.direct_diagnostic_indices, diagnostic_indices, work);
        if let Some(identity) = value.identity.as_ref() {
            self.record_collision_diagnostics(identity, diagnostic_indices, work);
        }
        for identity in &value.collision_dependencies {
            self.record_collision_diagnostics(identity, diagnostic_indices, work);
        }
    }

    fn record_collision_diagnostics(
        &self,
        identity: &crate::types::ModuleValueIdentity,
        diagnostic_indices: &mut HashSet<usize>,
        work: &mut ModuleValuePublicationWork,
    ) {
        if let Some(indices) = self
            .module_value_publication
            .collision_diagnostic_indices
            .get(identity)
        {
            Self::record_diagnostic_indices(indices, diagnostic_indices, work);
        }
    }

    fn record_diagnostic_indices(
        indices: &[usize],
        diagnostic_indices: &mut HashSet<usize>,
        work: &mut ModuleValuePublicationWork,
    ) {
        work.cause_insertions += indices.len();
        diagnostic_indices.extend(indices.iter().copied());
    }
}

pub(super) fn index_module_value_publication(
    module: &SurfaceModule,
    diagnostics: &[Diagnostic],
    provenance: ModuleValuePublicationProvenance,
) -> IndexedModuleValuePublicationProvenance {
    let owners_by_file = owners_by_file(&provenance);
    let direct_diagnostic_indices =
        direct_diagnostic_indices(diagnostics, &owners_by_file, provenance.values.len());
    let callable_identities = callable_identities(module);
    let collision_diagnostic_indices =
        collision_diagnostic_indices(diagnostics, &callable_identities);
    let failures_by_file_and_range = failures_by_file_and_range(&provenance);
    let values = indexed_values(provenance, direct_diagnostic_indices);
    let retained_edges = values.iter().map(retained_value_edges).sum();
    let retained_direct_causes = values
        .iter()
        .map(|value| value.direct_diagnostic_indices.len())
        .sum();
    IndexedModuleValuePublicationProvenance {
        failures_by_file_and_range,
        values,
        collision_diagnostic_indices,
        retained_edges,
        retained_direct_causes,
    }
}

fn owners_by_file(
    provenance: &ModuleValuePublicationProvenance,
) -> HashMap<&str, Vec<IndexedModuleValuePublicationOwner>> {
    let mut owners_by_file = HashMap::<&str, Vec<IndexedModuleValuePublicationOwner>>::new();
    for (index, value) in provenance.values.iter().enumerate() {
        owners_by_file
            .entry(value.span.file.as_str())
            .or_default()
            .push(IndexedModuleValuePublicationOwner {
                span: value.span.clone(),
                value_index: index,
            });
    }
    for owners in owners_by_file.values_mut() {
        identifier_casing::radix_sort_by_span(owners, |owner| &owner.span, |_| {});
    }
    owners_by_file
}

fn direct_diagnostic_indices(
    diagnostics: &[Diagnostic],
    owners_by_file: &HashMap<&str, Vec<IndexedModuleValuePublicationOwner>>,
    value_count: usize,
) -> Vec<Vec<usize>> {
    let mut indices = vec![Vec::new(); value_count];
    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        let Some(owner) = diagnostic_owner(diagnostic, owners_by_file) else {
            continue;
        };
        indices[owner.value_index].push(diagnostic_index);
    }
    indices
}

fn diagnostic_owner<'a>(
    diagnostic: &Diagnostic,
    owners_by_file: &'a HashMap<&str, Vec<IndexedModuleValuePublicationOwner>>,
) -> Option<&'a IndexedModuleValuePublicationOwner> {
    if diagnostic.id == "module.value_not_executable" {
        return None;
    }
    let span = diagnostic.span.as_ref()?;
    let owners = owners_by_file.get(span.file.as_str())?;
    let insertion = owners.partition_point(|owner| owner.span.start.offset <= span.start.offset);
    let owner = insertion.checked_sub(1).map(|index| &owners[index])?;
    (span.start.offset >= owner.span.start.offset && span.end.offset <= owner.span.end.offset)
        .then_some(owner)
}

fn callable_identities(
    module: &SurfaceModule,
) -> HashMap<PublicationSpanKey, crate::types::ModuleValueIdentity> {
    let mut identities = HashMap::new();
    insert_function_identities(module, &mut identities);
    insert_alias_identities(module, &mut identities);
    insert_value_identities(module, &mut identities);
    identities
}

fn insert_function_identities(
    module: &SurfaceModule,
    identities: &mut HashMap<PublicationSpanKey, crate::types::ModuleValueIdentity>,
) {
    for function in &module.functions {
        if let Some(name) = function.name.as_ref() {
            identities.insert(
                publication_span_key(&function.span),
                (
                    function.package_name.clone(),
                    function.module_name.clone(),
                    name.clone(),
                ),
            );
        }
    }
}

fn insert_alias_identities(
    module: &SurfaceModule,
    identities: &mut HashMap<PublicationSpanKey, crate::types::ModuleValueIdentity>,
) {
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        if let Some(name) = alias.name.as_ref() {
            identities.insert(
                publication_span_key(&alias.span),
                (
                    alias.package_name.clone(),
                    alias.module_name.clone(),
                    name.clone(),
                ),
            );
        }
    }
}

fn insert_value_identities(
    module: &SurfaceModule,
    identities: &mut HashMap<PublicationSpanKey, crate::types::ModuleValueIdentity>,
) {
    for value in &module.module_values {
        if let Some(name) = value.name.as_ref() {
            identities.insert(
                publication_span_key(&value.span),
                (
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                ),
            );
        }
    }
}

fn collision_diagnostic_indices(
    diagnostics: &[Diagnostic],
    callable_identities: &HashMap<PublicationSpanKey, crate::types::ModuleValueIdentity>,
) -> HashMap<crate::types::ModuleValueIdentity, Vec<usize>> {
    let mut indices = HashMap::<crate::types::ModuleValueIdentity, Vec<usize>>::new();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic.id != "name.duplicate" {
            continue;
        }
        let Some(identity) = diagnostic
            .span
            .as_ref()
            .and_then(|span| callable_identities.get(&publication_span_key(span)))
        else {
            continue;
        };
        indices.entry(identity.clone()).or_default().push(index);
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

fn indexed_values(
    provenance: ModuleValuePublicationProvenance,
    mut direct_diagnostic_indices: Vec<Vec<usize>>,
) -> Vec<IndexedModuleValuePublicationValue> {
    provenance
        .values
        .into_iter()
        .enumerate()
        .map(|(index, value)| IndexedModuleValuePublicationValue {
            identity: value.identity,
            dependencies: value.dependencies,
            collision_dependencies: value.collision_dependencies,
            direct_diagnostic_indices: std::mem::take(&mut direct_diagnostic_indices[index]),
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
