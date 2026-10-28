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
    for file in files {
        attach_explicit_import_alias_occurrences_in_file(file, &imports);
    }
}

fn attach_explicit_import_alias_occurrences_in_file(
    file: &mut IndexedFile,
    imports: &veln_sema::ResolvedImportPathIndex<'_>,
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
        record_explicit_import_alias_path_work(
            qualifier_indices.len(),
            qualifier_indices
                .iter()
                .map(|index| tokens[*index].text.len())
                .sum(),
        );
        record_explicit_import_alias_construction_work(qualifier_indices.len());
        let Some(veln_sema::ResolvedImportPath::ExplicitAlias(alias)) = imports.resolve_prefixes(
            qualifier_indices
                .iter()
                .map(|index| tokens[*index].text.as_str()),
            Some(&file.module),
        ) else {
            continue;
        };
        let range = tokens[token_index].range;
        file.explicit_import_alias_occurrences
            .insert((range.start, range.end), alias.span);
        record_explicit_import_alias_retained_entries(1);
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
