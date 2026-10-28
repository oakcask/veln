fn index_explicit_import_aliases(
    files: &mut [IndexedFile],
    module: &veln_ast::SurfaceModule,
) -> (Vec<ImportAliasSymbol>, Vec<ImportNamespaceBinding>) {
    let aliases = veln_sema::resolved_explicit_import_aliases(module)
        .into_iter()
        .map(|alias| ImportAliasSymbol {
            module: alias.declaring_module,
            name: alias.name,
            declaration: workspace_location(alias.span),
            target_module: alias.target_module,
            target_package: alias.target_package,
        })
        .collect::<Vec<_>>();
    record_explicit_import_alias_construction_work(aliases.len());
    record_explicit_import_alias_retained_entries(aliases.len());
    attach_module_imports(files, module);
    attach_explicit_import_alias_occurrences(files, module);
    let bindings = veln_sema::import_namespace_participants(module)
        .into_iter()
        .map(|binding| ImportNamespaceBinding {
            module: binding.declaring_module,
            qualifier: binding.qualifier,
            declaration: workspace_location(binding.declaration_span),
            selected_alias_declaration: binding.explicit_alias_span,
        })
        .collect::<Vec<_>>();
    record_explicit_import_alias_construction_work(bindings.len());
    record_explicit_import_alias_retained_entries(bindings.len());
    (aliases, bindings)
}

#[derive(Default)]
struct ModuleImports {
    local_modules: BTreeSet<String>,
    external_modules: BTreeSet<(String, String)>,
    local_aliases: BTreeMap<String, String>,
    external_aliases: BTreeMap<String, (String, String)>,
}

fn attach_module_imports(files: &mut [IndexedFile], module: &veln_ast::SurfaceModule) {
    let mut file_counts = BTreeMap::<String, usize>::new();
    for file in files.iter() {
        *file_counts.entry(file.module.clone()).or_default() += 1;
    }
    let resolved_imports = veln_sema::resolved_navigation_imports(module);
    let mut shared_modules = file_counts
        .iter()
        .filter_map(|(module, count)| (*count > 1).then_some(module.clone()))
        .collect::<BTreeSet<_>>();
    shared_modules.extend(
        resolved_imports
            .iter()
            .filter(|import| import.explicit_alias)
            .map(|import| import.declaring_module.clone()),
    );
    let mut by_module = collect_shared_file_imports(files, &shared_modules);
    merge_resolved_imports(&mut by_module, &shared_modules, resolved_imports);
    let shared = freeze_shared_imports(by_module);
    assign_shared_imports(files, &shared);
}

fn collect_shared_file_imports(
    files: &[IndexedFile],
    shared_modules: &BTreeSet<String>,
) -> BTreeMap<String, ModuleImports> {
    let mut by_module = BTreeMap::<String, ModuleImports>::new();
    for file in files
        .iter()
        .filter(|file| shared_modules.contains(&file.module))
    {
        let imports = by_module.entry(file.module.clone()).or_default();
        imports.local_modules.extend(file.uses.iter().cloned());
        imports
            .external_modules
            .extend(file.external_uses.iter().cloned());
        imports
            .local_aliases
            .extend(file.import_aliases.iter().map(|(name, target)| {
                (name.clone(), target.clone())
            }));
        imports.external_aliases.extend(
            file.external_import_aliases
                .iter()
                .map(|(name, target)| (name.clone(), target.clone())),
        );
        record_explicit_import_alias_construction_work(
            file.uses.len()
                + file.external_uses.len()
                + file.import_aliases.len()
                + file.external_import_aliases.len(),
        );
    }
    by_module
}

fn merge_resolved_imports(
    by_module: &mut BTreeMap<String, ModuleImports>,
    shared_modules: &BTreeSet<String>,
    resolved_imports: Vec<veln_sema::ResolvedNavigationImport>,
) {
    for import in resolved_imports {
        if !shared_modules.contains(&import.declaring_module) {
            continue;
        }
        record_explicit_import_alias_construction_work(1);
        let imports = by_module.entry(import.declaring_module).or_default();
        match import.target_package {
            Some(package) => {
                imports
                    .external_modules
                    .insert((import.target_module.clone(), package.clone()));
                if !import.qualifier.is_empty() {
                    imports
                        .external_aliases
                        .insert(import.qualifier, (import.target_module, package));
                }
            }
            None => {
                imports.local_modules.insert(import.target_module.clone());
                if !import.qualifier.is_empty() {
                    imports
                        .local_aliases
                        .insert(import.qualifier, import.target_module);
                }
            }
        }
    }
}

type SharedModuleImports = (
    Arc<BTreeSet<String>>,
    Arc<BTreeSet<(String, String)>>,
    Arc<BTreeMap<String, String>>,
    Arc<BTreeMap<String, (String, String)>>,
);

fn freeze_shared_imports(
    by_module: BTreeMap<String, ModuleImports>,
) -> BTreeMap<String, SharedModuleImports> {
    by_module
        .into_iter()
        .map(|(module, imports)| {
            record_explicit_import_alias_retained_entries(
                imports.local_modules.len()
                    + imports.external_modules.len()
                    + imports.local_aliases.len()
                    + imports.external_aliases.len(),
            );
            (
                module,
                (
                    Arc::new(imports.local_modules),
                    Arc::new(imports.external_modules),
                    Arc::new(imports.local_aliases),
                    Arc::new(imports.external_aliases),
                ),
            )
        })
        .collect()
}

fn assign_shared_imports(
    files: &mut [IndexedFile],
    shared: &BTreeMap<String, SharedModuleImports>,
) {
    for file in files {
        record_explicit_import_alias_construction_work(1);
        if let Some((uses, external_uses, aliases, external_aliases)) = shared.get(&file.module) {
            file.uses = Arc::clone(uses);
            file.external_uses = Arc::clone(external_uses);
            file.import_aliases = Arc::clone(aliases);
            file.external_import_aliases = Arc::clone(external_aliases);
        }
    }
}

fn attach_explicit_import_alias_occurrences(
    files: &mut [IndexedFile],
    module: &veln_ast::SurfaceModule,
) {
    let imports = veln_sema::ResolvedImportPathIndex::new(module);
    let mut shared_targets = BTreeMap::new();
    for file in files {
        attach_explicit_import_alias_occurrences_in_file(file, &imports, &mut shared_targets);
    }
}

fn attach_explicit_import_alias_occurrences_in_file(
    file: &mut IndexedFile,
    imports: &veln_sema::ResolvedImportPathIndex<'_>,
    shared_targets: &mut BTreeMap<veln_sema::ResolvedNavigationImportIdentity, Arc<SharedQualifiedImportTarget>>,
) {
    let tokens = &file.tokens;
    let mut in_use_line = false;
    for token_index in 0..tokens.len() {
        record_explicit_import_alias_construction_work(1);
        record_explicit_import_alias_token_visit();
        if update_use_line(tokens[token_index].kind, &mut in_use_line) {
            continue;
        }
        let Some(qualifier_indices) = alias_qualifier_indices(tokens, token_index, in_use_line)
        else {
            continue;
        };
        record_explicit_import_alias_construction_work(qualifier_indices.len());
        let qualifier_segments = qualifier_indices
            .iter()
            .map(|index| tokens[*index].text.as_str())
            .collect::<Vec<_>>();
        let range = tokens[token_index].range;
        attach_qualifier_resolution(
            &mut file.qualified_import_resolutions,
            &mut file.explicit_import_alias_occurrences,
            &file.module,
            imports,
            shared_targets,
            &qualifier_segments,
            (range.start, range.end),
        );
    }
}

fn attach_qualifier_resolution(
    resolutions: &mut QualifiedImportResolutionIndex,
    alias_occurrences: &mut BTreeMap<(usize, usize), SourceSpan>,
    module: &str,
    imports: &veln_sema::ResolvedImportPathIndex<'_>,
    shared_targets: &mut BTreeMap<veln_sema::ResolvedNavigationImportIdentity, Arc<SharedQualifiedImportTarget>>,
    qualifier_segments: &[&str],
    occurrence_range: (usize, usize),
) {
    let prefix_selections = imports.select_prefix_sequence(qualifier_segments, Some(module));
    record_explicit_import_alias_path_work(qualifier_segments.len(), 0);
    let Some(final_selection) = prefix_selections.last().copied() else {
        return;
    };
    let (resolution, final_alias_span) = match final_selection {
        veln_sema::ResolvedImportPathPrefixSelection::Resolved(selection) => {
            let shared = Arc::clone(shared_targets.entry(selection.import_id).or_insert_with(|| {
                let bytes = selection.target_module.len()
                    + selection.target_package.map_or(0, str::len);
                record_explicit_import_alias_path_work(0, bytes);
                record_explicit_import_alias_target_bytes(bytes);
                Arc::new(SharedQualifiedImportTarget {
                    module: Arc::<str>::from(selection.target_module),
                    package: selection.target_package.map(Arc::<str>::from),
                })
            }));
            let target = QualifiedImportTarget {
                shared,
                consumed_segments: selection.consumed_segments,
            };
            (
                QualifiedImportResolution::Resolved(target),
                selection.alias_span,
            )
        },
        veln_sema::ResolvedImportPathPrefixSelection::Blocked => {
            (QualifiedImportResolution::Blocked, None)
        }
        veln_sema::ResolvedImportPathPrefixSelection::Missing => return,
    };
    attach_path_resolution(resolutions, qualifier_segments, resolution);
    if let Some(alias_span) = final_alias_span {
        alias_occurrences.insert(occurrence_range, alias_span.clone());
        record_explicit_import_alias_retained_entries(1);
    }
}

fn attach_path_resolution(
    index: &mut QualifiedImportResolutionIndex,
    qualifier_segments: &[&str],
    resolution: QualifiedImportResolution,
) {
    let mut node_index = 0;
    for segment in qualifier_segments {
        let next_index = if let Some(next_index) = index.nodes[node_index].children.get(*segment) {
            *next_index
        } else {
            let next_index = index.nodes.len();
            index.nodes.push(QualifiedImportResolutionNode::default());
            index.nodes[node_index]
                .children
                .insert((*segment).to_owned(), next_index);
            record_explicit_import_alias_path_work(0, segment.len());
            record_explicit_import_alias_retained_entries(1);
            next_index
        };
        node_index = next_index;
    }
    let retained = index.nodes[node_index]
        .resolution
        .as_ref()
        .is_some_and(|current| same_import_resolution(current, &resolution));
    if !retained {
        index.nodes[node_index].resolution = Some(resolution);
    }
}

fn same_import_resolution(
    left: &QualifiedImportResolution,
    right: &QualifiedImportResolution,
) -> bool {
    match (left, right) {
        (QualifiedImportResolution::Blocked, QualifiedImportResolution::Blocked) => true,
        (
            QualifiedImportResolution::Resolved(left),
            QualifiedImportResolution::Resolved(right),
        ) => {
            Arc::ptr_eq(&left.shared, &right.shared)
                && left.consumed_segments == right.consumed_segments
        }
        _ => false,
    }
}

fn update_use_line(kind: TokenKind, in_use_line: &mut bool) -> bool {
    match kind {
        TokenKind::Newline | TokenKind::Eof => {
            *in_use_line = false;
            true
        }
        TokenKind::Use => {
            *in_use_line = true;
            true
        }
        _ => false,
    }
}

fn alias_qualifier_indices(
    tokens: &[Token],
    token_index: usize,
    in_use_line: bool,
) -> Option<Vec<usize>> {
    if !tokens[token_index].kind.is_contextual_identifier()
        || in_use_line
        || previous_non_layout_token(tokens, token_index)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
        || next_non_layout_token(tokens, token_index)
            .is_none_or(|token| token.kind != TokenKind::DoubleColon)
    {
        return None;
    }
    let mut segment_indices = vec![token_index];
    let mut cursor = token_index;
    while let Some(next) = next_path_segment_index(tokens, cursor) {
        segment_indices.push(next);
        cursor = next;
    }
    segment_indices.pop();
    Some(segment_indices)
}
