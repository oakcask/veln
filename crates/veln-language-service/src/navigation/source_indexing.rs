fn index_workspace_source(source: SourceFile) -> (IndexedFile, FileDeclarations, ParseOutput) {
    let path = source.path().as_str().to_string();
    let companion_target_module = classify_companion_source(&path)
        .and_then(|companion| module_name_from_path(&companion.target_path));
    let path_module = module_name_from_path(&path);
    let navigation_isolated = path_module_invalid_for_navigation(path_module.as_deref());
    let module = explicit_module_name(source.text())
        .or(path_module)
        .unwrap_or_default();
    let (uses, external_uses, import_aliases, external_import_aliases) = use_modules(source.text());
    #[cfg(test)]
    record_workspace_source_parse();
    let parsed = parse(&source);
    let invalid_declaration_names = invalid_declaration_names(&parsed);
    let tokens = lex(&source).tokens;
    let recovery_symbols = workspace_recovery_symbols(
        navigation_isolated,
        &source,
        &tokens,
        &parsed.tree,
        &invalid_declaration_names,
    );
    let file = IndexedFile {
        source,
        tokens,
        module,
        companion_target_module,
        uses,
        external_uses,
        import_aliases,
        external_import_aliases,
        invalid_declaration_names: invalid_name_spans(&invalid_declaration_names),
        recovery_symbols,
        classified_path_segments: Vec::new(),
        type_reference_locations: OnceLock::new(),
        navigation_isolated,
        origin: IndexedOrigin::Workspace,
    };
    let declarations = workspace_file_declarations(&file, &parsed.tree);
    (file, declarations, parsed)
}

fn workspace_recovery_symbols(
    navigation_isolated: bool,
    source: &SourceFile,
    tokens: &[Token],
    syntax: &SyntaxTree,
    invalid_names: &[InvalidName],
) -> Vec<RecoverySymbol> {
    if navigation_isolated {
        Vec::new()
    } else {
        recovery_symbols_for_workspace_source(source, tokens, syntax, invalid_names)
    }
}

fn workspace_file_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> FileDeclarations {
    if file.navigation_isolated {
        FileDeclarations::default()
    } else {
        file_declarations(file, syntax)
    }
}

fn path_module_invalid_for_navigation(path_module: Option<&str>) -> bool {
    path_module.is_some_and(module_identity_has_invalid_casing)
}

fn invalid_name_spans(invalid_names: &[InvalidName]) -> Vec<SourceSpan> {
    invalid_names
        .iter()
        .map(|invalid| invalid.span.clone())
        .collect()
}

fn module_identity_has_invalid_casing(module: &str) -> bool {
    module
        .split("::")
        .any(|segment| !segment.starts_with(|ch: char| ch.is_ascii_lowercase()))
}

fn index_dependency_sources(
    files: &mut Vec<IndexedFile>,
    declarations: &mut FileDeclarations,
    module: &mut veln_ast::SurfaceModule,
    dependency: DirectDependencySnapshot,
) {
    for (source, entry) in dependency.indexed_sources() {
        let (file, parsed) = indexed_dependency_source(&dependency, source, entry.uri());
        if !file.navigation_isolated && parsed.diagnostics.is_empty() {
            declarations.extend(file_declarations(&file, &parsed.tree));
            let mut source_module = veln_ast::lower_surface_ast(&parsed.tree);
            assign_module_name(&mut source_module, &file.module);
            append_surface_module(module, source_module);
        }
        files.push(file);
    }
}

fn indexed_dependency_source(
    dependency: &DirectDependencySnapshot,
    source: &veln_project::CapturedPackageSource,
    uri: &str,
) -> (IndexedFile, ParseOutput) {
    #[cfg(test)]
    record_dependency_source_index();
    #[cfg(test)]
    record_dependency_source_parse();

    let text =
        std::str::from_utf8(source.bytes()).expect("captured package source text is valid UTF-8");
    let source_file = SourceFile::new(source.path(), text);
    let path_module = module_name_from_path(source.path());
    let navigation_isolated = path_module_invalid_for_navigation(path_module.as_deref());
    let module = explicit_module_name(text).or(path_module).unwrap_or_default();
    let (uses, external_uses, import_aliases, external_import_aliases) = use_modules(text);
    let parsed = parse(&source_file);
    let invalid_declaration_names = invalid_declaration_names(&parsed);
    let tokens = lex(&source_file).tokens;
    let file = IndexedFile {
        source: source_file,
        tokens,
        module,
        companion_target_module: None,
        uses,
        external_uses,
        import_aliases,
        external_import_aliases,
        invalid_declaration_names: invalid_name_spans(&invalid_declaration_names),
        recovery_symbols: Vec::new(),
        classified_path_segments: Vec::new(),
        type_reference_locations: OnceLock::new(),
        navigation_isolated,
        origin: IndexedOrigin::Package {
            identity: dependency.identity.as_str().to_string(),
            uri: uri.to_string(),
            exported: dependency.exported_sources.contains(source.path()),
            standard_library: dependency.standard_library,
        },
    };
    (file, parsed)
}

fn attach_classified_path_segments(
    files: &mut [IndexedFile],
    module: &veln_ast::SurfaceModule,
    project: &veln_ast::SurfaceModule,
) {
    #[cfg(test)]
    record_dependency_path_classifications(
        files
            .iter()
            .filter(|file| matches!(file.origin, IndexedOrigin::Package { .. }))
            .count(),
    );
    let segments =
        veln_sema::classified_project_qualified_path_segments_with_context(module, project);
    for file in files {
        file.classified_path_segments = segments
            .iter()
            .filter(|segment| segment.span.file == *file.source.path())
            .cloned()
            .collect();
    }
}

fn append_parsed_surface_module(
    merged: &mut veln_ast::SurfaceModule,
    file: &IndexedFile,
    parsed: &ParseOutput,
) {
    if file.navigation_isolated || !parsed.diagnostics.is_empty() {
        return;
    }
    let mut module = veln_ast::lower_surface_ast(&parsed.tree);
    assign_module_name(&mut module, &file.module);
    append_surface_module(merged, module);
}

fn workspace_schema_composition_references(
    files: &[IndexedFile],
    schemas: &[NeutralSymbol],
    schema_aliases: &[NeutralSymbol],
    references: Vec<veln_sema::ResolvedSchemaCompositionReference>,
) -> Vec<SchemaCompositionReference> {
    references
        .into_iter()
        .filter_map(|reference| {
            let schema_target = schemas.iter().find(|schema| {
                schema.package.is_none()
                    && schema.name == reference.target_name
                    && Some(schema.module.as_str()) == reference.target_module.as_deref()
                    && schema.declaration.span.file == reference.target_span.file
            })?;
            let alias_target = reference.alias_span.as_ref().and_then(|alias_span| {
                schema_aliases.iter().find(|alias| {
                    alias.package.is_none()
                        && Some(alias.name.as_str()) == reference.alias_name.as_deref()
                        && Some(alias.module.as_str()) == reference.alias_module.as_deref()
                        && alias.declaration.span.file == alias_span.file
                        && alias.declaration.span.start.offset >= alias_span.start.offset
                        && alias.declaration.span.end.offset <= alias_span.end.offset
                })
            });
            let leaf = reference.path.last()?;
            let file = files
                .iter()
                .find(|file| file.source.path() == &reference.field_span.file)?;
            let (_, token) = file.tokens.iter().enumerate().find(|(index, token)| {
                token.range.start >= reference.field_span.start.offset
                    && token.range.end <= reference.field_span.end.offset
                    && token.text == *leaf
                    && is_schema_composition_path_leaf_token(&file.tokens, *index)
            })?;
            Some(SchemaCompositionReference {
                span: file.source.span(token.range),
                target: alias_target.map_or_else(
                    || SchemaReferenceTarget::Schema(schema_target.clone()),
                    |alias| SchemaReferenceTarget::Alias(alias.clone()),
                ),
            })
        })
        .collect()
}

fn eligible_workspace_schema_aliases(
    aliases: Vec<NeutralSymbol>,
    resolved: Vec<veln_sema::ResolvedSchemaAlias>,
) -> Vec<NeutralSymbol> {
    aliases
        .into_iter()
        .filter(|alias| {
            alias.package.is_none()
                && resolved.iter().any(|candidate| {
                    candidate.alias_name == alias.name
                        && candidate.alias_module.as_deref() == Some(alias.module.as_str())
                        && candidate.alias_span.file == alias.declaration.span.file
                        && alias.declaration.span.start.offset >= candidate.alias_span.start.offset
                        && alias.declaration.span.end.offset <= candidate.alias_span.end.offset
                })
        })
        .collect()
}

fn empty_surface_module() -> veln_ast::SurfaceModule {
    veln_ast::SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    }
}

fn append_surface_module(merged: &mut veln_ast::SurfaceModule, module: veln_ast::SurfaceModule) {
    merged.uses.extend(module.uses);
    merged.aliases.extend(module.aliases);
    merged.effects.extend(module.effects);
    merged.handlers.extend(module.handlers);
    merged.schemas.extend(module.schemas);
    merged.types.extend(module.types);
    merged.functions.extend(module.functions);
    merged.invalid_names.extend(module.invalid_names);
}

fn assign_module_name(module: &mut veln_ast::SurfaceModule, name: &str) {
    for use_decl in &mut module.uses {
        use_decl.module_name = Some(name.to_string());
    }
    for alias in &mut module.aliases {
        alias.module_name = Some(name.to_string());
    }
    for effect in &mut module.effects {
        effect.module_name = Some(name.to_string());
    }
    for handler in &mut module.handlers {
        handler.module_name = Some(name.to_string());
    }
    for type_decl in &mut module.types {
        type_decl.module_name = Some(name.to_string());
    }
    for schema in &mut module.schemas {
        schema.module_name = Some(name.to_string());
    }
    for function in &mut module.functions {
        function.module_name = Some(name.to_string());
    }
}
