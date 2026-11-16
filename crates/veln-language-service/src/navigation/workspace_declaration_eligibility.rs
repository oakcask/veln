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

fn eligible_workspace_handler_indices(
    handlers: &[NeutralSymbol],
    files: &[IndexedFile],
) -> BTreeMap<(String, String), usize> {
    let recovered_by_file = recovered_handler_declaration_ranges_by_file(files);
    let mut candidates = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in handlers.iter().enumerate() {
        if symbol.package.is_none() {
            candidates
                .entry((symbol.module.clone(), symbol.name.clone()))
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

fn recovered_handler_declaration_ranges_by_file(
    files: &[IndexedFile],
) -> BTreeMap<&str, Vec<(usize, usize)>> {
    files
        .iter()
        .filter(|file| workspace_navigation_file(file))
        .map(|file| {
            let mut ranges = file
                .recovered_handler_declarations
                .iter()
                .map(|span| (span.start.offset, span.end.offset))
                .collect::<Vec<_>>();
            ranges.sort_unstable();
            (file.source.path().as_str(), ranges)
        })
        .collect()
}
