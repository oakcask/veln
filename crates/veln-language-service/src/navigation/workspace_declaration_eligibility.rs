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

fn eligible_handler_indices(
    handlers: &[NeutralSymbol],
    files: &[IndexedFile],
) -> BTreeMap<ModuleMemberIdentity, usize> {
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
            let ineligible = files.iter().any(|file| {
                file.source.path() == &symbol.declaration.span.file
                    && match (&file.origin, &symbol.package) {
                        (IndexedOrigin::Workspace, None) => true,
                        (IndexedOrigin::Package { identity, .. }, Some(package)) => {
                            identity == package
                        }
                        _ => false,
                    }
                    && (file.recovered_handler_declarations.iter().any(|span| {
                        span.start.offset <= symbol.declaration.span.start.offset
                            && symbol.declaration.span.end.offset <= span.end.offset
                    }) || file
                        .invalid_declaration_names
                        .iter()
                        .any(|span| same_span(span, &symbol.declaration.span)))
            });
            (!ineligible).then_some((identity, *index))
        })
        .collect()
}
