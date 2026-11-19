fn eligible_workspace_effect_indices(
    effects: &[NeutralSymbol],
    files: &[IndexedFile],
) -> BTreeMap<(String, String), usize> {
    let recovered_by_file = recovered_effect_declaration_ranges_by_file(files);
    effect_candidates_by_identity(effects)
        .into_iter()
        .filter_map(|(identity, candidates)| {
            let [index] = candidates.as_slice() else {
                return None;
            };
            let symbol = &effects[*index];
            let ranges = recovered_by_file.get(symbol.declaration.span.file.as_str())?;
            let insertion = ranges.partition_point(|(start, _)| {
                *start <= symbol.declaration.span.start.offset
            });
            let recovered = insertion.checked_sub(1).is_some_and(|range_index| {
                let (start, end) = ranges[range_index];
                start <= symbol.declaration.span.start.offset
                    && symbol.declaration.span.end.offset <= end
            });
            (!recovered).then_some((identity, *index))
        })
        .collect()
}

fn recovered_effect_declaration_ranges_by_file(
    files: &[IndexedFile],
) -> BTreeMap<&str, Vec<(usize, usize)>> {
    files
        .iter()
        .filter(|file| workspace_navigation_file(file))
        .map(|file| {
            let mut ranges = file
                .recovered_effect_declarations
                .iter()
                .map(|span| (span.start.offset, span.end.offset))
                .collect::<Vec<_>>();
            ranges.sort_unstable();
            (file.source.path().as_str(), ranges)
        })
        .collect()
}

fn effect_candidates_by_identity(
    effects: &[NeutralSymbol],
) -> BTreeMap<(String, String), Vec<usize>> {
    let mut candidates = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in effects.iter().enumerate() {
        #[cfg(test)]
        record_effect_declaration_index_visit();
        if symbol.package.is_none() {
            candidates
                .entry((symbol.module.clone(), symbol.name.clone()))
                .or_default()
                .push(index);
        }
    }
    candidates
}

fn eligible_navigable_handler_indices(
    handlers: &[NeutralSymbol],
    files: &[IndexedFile],
) -> BTreeMap<ModuleMemberIdentity, usize> {
    let eligibility_by_source = handler_declaration_eligibility_by_source(files);
    let mut candidates = BTreeMap::<ModuleMemberIdentity, Vec<usize>>::new();
    for (index, symbol) in handlers.iter().enumerate() {
        if symbol.package_origin != Some(PackageOrigin::StandardLibrary)
            && (symbol.package.is_none()
                || symbol
                    .name
                    .chars()
                    .next()
                    .is_some_and(|initial| initial.is_ascii_lowercase()))
        {
            candidates
                .entry((
                    symbol.package.clone(),
                    symbol.package_origin,
                    symbol.module.clone(),
                    symbol.name.clone(),
                ))
                .or_default()
                .push(index);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(identity, candidates)| {
            let [index] = candidates.as_slice() else {
                return None;
            };
            let symbol = &handlers[*index];
            #[cfg(test)]
            record_handler_eligibility_source_lookup();
            let source = handler_declaration_source_identity(&symbol.declaration);
            let ineligible = eligibility_by_source
                .get(&source)
                .is_none_or(|eligibility| eligibility.rejects(&symbol.declaration.span))
                || (symbol.package.is_some() && !symbol.public);
            (!ineligible).then_some((identity, *index))
        })
        .collect()
}

type HandlerDeclarationSourceIdentity = (Option<String>, String);

struct HandlerDeclarationEligibility {
    navigable: bool,
    recovered_ranges: Vec<(usize, usize)>,
    invalid_name_ranges: BTreeSet<(usize, usize)>,
}

impl HandlerDeclarationEligibility {
    fn rejects(&self, declaration: &SourceSpan) -> bool {
        let insertion = self
            .recovered_ranges
            .partition_point(|(start, _)| *start <= declaration.start.offset);
        let recovered = insertion.checked_sub(1).is_some_and(|range_index| {
            let (start, end) = self.recovered_ranges[range_index];
            start <= declaration.start.offset && declaration.end.offset <= end
        });
        !self.navigable
            || recovered
            || self
                .invalid_name_ranges
                .contains(&(declaration.start.offset, declaration.end.offset))
    }
}

fn handler_declaration_eligibility_by_source(
    files: &[IndexedFile],
) -> BTreeMap<HandlerDeclarationSourceIdentity, HandlerDeclarationEligibility> {
    files
        .iter()
        .map(|file| {
            #[cfg(test)]
            record_handler_eligibility_source_index_entry();
            let mut recovered_ranges = file
                .recovered_handler_declarations
                .iter()
                .map(|span| (span.start.offset, span.end.offset))
                .collect::<Vec<_>>();
            recovered_ranges.sort_unstable();
            let invalid_name_ranges = file
                .invalid_declaration_names
                .iter()
                .map(|span| (span.start.offset, span.end.offset))
                .collect();
            (
                handler_file_source_identity(file),
                HandlerDeclarationEligibility {
                    navigable: matches!(
                        &file.origin,
                        IndexedOrigin::Workspace
                            | IndexedOrigin::Package { exported: true, .. }
                    ),
                    recovered_ranges,
                    invalid_name_ranges,
                },
            )
        })
        .collect()
}

fn handler_file_source_identity(file: &IndexedFile) -> HandlerDeclarationSourceIdentity {
    let owner = match &file.origin {
        IndexedOrigin::Workspace => None,
        IndexedOrigin::Package { uri, .. } => Some(uri.clone()),
    };
    (owner, file.source.path().as_str().to_string())
}

fn handler_declaration_source_identity(
    declaration: &NavigationLocation,
) -> HandlerDeclarationSourceIdentity {
    let owner = match &declaration.source {
        NavigationSource::Workspace => None,
        NavigationSource::Package { uri } => Some(uri.clone()),
    };
    (owner, declaration.span.file.as_str().to_string())
}
