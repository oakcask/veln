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
    let package = dependency.identity.as_str().to_string();
    let package_origin = if dependency.standard_library {
        PackageOrigin::StandardLibrary
    } else {
        PackageOrigin::DirectDependency
    };
    let mut dependency_module = empty_surface_module();
    for (source, entry) in dependency.indexed_sources() {
        let (file, parsed) = indexed_dependency_source(&dependency, source, entry.uri());
        if !file.navigation_isolated {
            declarations
                .package_schema_alias_declarations
                .extend(package_schema_alias_declarations(&file, &parsed.tree));
            let mut source_module = veln_ast::lower_surface_ast(&parsed.tree);
            assign_module_name(&mut source_module, &file.module);
            if parsed.diagnostics.is_empty() {
                declarations.extend(file_declarations(&file, &parsed.tree));
                append_surface_module(&mut dependency_module, source_module.clone());
                append_surface_module(module, source_module);
            } else {
                declarations
                    .schema_alias_blockers
                    .extend(schema_alias_declarations(&file, &parsed.tree));
                declarations
                    .recovered_package_schema_targets
                    .extend(package_schema_targets(&file, &parsed.tree));
                append_recovered_dependency_imports(
                    &mut dependency_module,
                    source_module,
                    &parsed,
                );
            }
        }
        files.push(file);
    }
    declarations.resolved_package_schema_aliases.extend(
        veln_sema::resolved_schema_alias_chains(&dependency_module)
            .into_iter()
            .map(|resolved| {
                let target_exported = dependency
                    .exported_sources
                    .contains(resolved.target_span.file.as_str());
                ResolvedPackageSchemaAlias {
                    package: package.clone(),
                    package_origin,
                    alias_module: resolved.alias_module,
                    alias_name: resolved.alias_name,
                    target_exported,
                    direct_target_module: resolved.direct_target_module,
                    direct_target_name: resolved.direct_target_name,
                    direct_target_is_alias: resolved.direct_target_is_alias,
                }
            }),
    );
}

fn append_recovered_dependency_imports(
    dependency_module: &mut veln_ast::SurfaceModule,
    mut source_module: veln_ast::SurfaceModule,
    parsed: &ParseOutput,
) {
    for use_decl in &source_module.uses {
        let recovered = parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.parser_context == "use_declaration"
                && diagnostic.span.as_ref().is_none_or(|span| {
                    span.file == use_decl.span.file
                        && span.start.offset <= use_decl.span.end.offset
                        && span.end.offset >= use_decl.span.start.offset
                })
        });
        if !recovered {
            continue;
        }
        let span = use_decl
            .name_spans
            .first()
            .cloned()
            .unwrap_or_else(|| use_decl.span.clone());
        source_module.invalid_names.push(veln_ast::InvalidName {
            name: use_decl.name.clone(),
            class: veln_ast::NameClass::Module,
            occurrence: veln_ast::NameOccurrence::PathSegment,
            span,
            enclosing_function_span: None,
            segment_index: None,
        });
    }
    source_module.aliases.clear();
    source_module.effects.clear();
    source_module.handlers.clear();
    source_module.schemas.clear();
    source_module.types.clear();
    source_module.functions.clear();
    append_surface_module(dependency_module, source_module);
}

struct ParsedDependencySource {
    source: SourceFile,
    tokens: Vec<Token>,
    identity: SourceIdentity,
    imports: UseModuleIndex,
    invalid_declaration_names: Vec<SourceSpan>,
    parsed: ParseOutput,
}

fn parse_dependency_source(
    source: &veln_project::CapturedPackageSource,
) -> ParsedDependencySource {
    let text =
        std::str::from_utf8(source.bytes()).expect("captured package source text is valid UTF-8");
    let source_file = SourceFile::new(source.path(), text);
    let parsed = parse(&source_file);
    let invalid_declaration_names = invalid_name_spans(&invalid_declaration_names(&parsed));
    let tokens = lex(&source_file).tokens;

    ParsedDependencySource {
        source: source_file,
        tokens,
        identity: SourceIdentity::new(source.path(), text),
        imports: UseModuleIndex::new(text),
        invalid_declaration_names,
        parsed,
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

    let exported = dependency.exported_sources.contains(source.path());
    let ParsedDependencySource {
        source,
        tokens,
        identity,
        imports,
        invalid_declaration_names,
        parsed,
    } = parse_dependency_source(source);
    let schema_operation_leaf_ranges = valid_schema_operation_leaf_spans(&parsed.tree)
        .into_iter()
        .map(|span| (span.start.offset, span.end.offset))
        .collect();
    let schema_composition_leaf_spans =
        valid_schema_composition_leaf_spans(&source, &tokens, &parsed);
    let handler_operation_clause_body_ranges =
        handler_operation_clause_body_ranges(&source, &tokens);
    let handler_clause_bindings_by_name = handler_clause_bindings_by_name(&parsed.tree);
    let constructor_reference_declaration_ranges =
        constructor_reference_declaration_ranges(&parsed.tree, &tokens);
    let variant_refinement_source_index = variant_refinement_source_index(&parsed.tree);
    let file = IndexedFile {
        source,
        tokens,
        module: identity.module,
        companion_target_module: None,
        uses: imports.local_modules,
        external_uses: imports.external_modules,
        import_aliases: imports.local_aliases,
        external_import_aliases: imports.external_aliases,
        schema_alias_external_imports: Vec::new(),
        workspace_imports: Vec::new(),
        invalid_declaration_names,
        recovery_symbols: Vec::new(),
        recovered_effect_declarations: recovered_effect_declarations(&parsed.tree),
        recovered_handler_declarations: recovered_handler_declarations(
            &parsed,
            &HandlerDiagnosticIndex::new(&parsed),
        ),
        handler_reference_ranges: BTreeSet::new(),
        handler_operation_clause_references: Vec::new(),
        handler_operation_clause_body_ranges,
        handler_clause_bindings_by_name,
        schema_operation_leaf_ranges,
        schema_composition_leaf_spans,
        effect_reference_ranges: BTreeSet::new(),
        effect_operation_ranges: BTreeSet::new(),
        generic_effect_binders: Vec::new(),
        variant_refinement_final_ranges: variant_refinement_source_index.final_ranges,
        variant_refinement_final_range_by_base_range: variant_refinement_source_index
            .final_range_by_base_range,
        variant_refinement_type_argument_count_by_final_range: variant_refinement_source_index
            .type_argument_count_by_final_range,
        variant_refinement_union_group_index_by_final_range: variant_refinement_source_index
            .union_group_index_by_final_range,
        variant_refinement_union_final_range_groups: variant_refinement_source_index
            .union_final_range_groups,
        variant_refinement_type_argument_ranges_by_final_range:
            variant_refinement_source_index.type_argument_ranges_by_final_range,
        variant_refinement_type_parameter_contexts:
            variant_refinement_source_index.type_parameter_contexts,
        variant_refinement_type_parameter_context_index_by_final_range:
            variant_refinement_source_index.type_parameter_context_index_by_final_range,
        canonical_variant_refinement_type_arguments_by_final_range: BTreeMap::new(),
        constructor_reference_declaration_ranges,
        classified_paths: ClassifiedPathIndex::default(),
        type_reference_locations: OnceLock::new(),
        navigation_isolated: identity.navigation_isolated,
        origin: IndexedOrigin::Package {
            identity: dependency.identity.as_str().to_string(),
            uri: uri.to_string(),
            exported,
            standard_library: dependency.standard_library,
        },
    };
    (file, parsed)
}

fn workspace_needs_path_classification(
    files: &[IndexedFile],
    module: &veln_ast::SurfaceModule,
) -> bool {
    // Qualified syntax needs `::`, but invalid imports can contribute
    // single-segment paths to the recovery classifier as well.
    files
        .iter()
        .any(|file| file.tokens.iter().any(|token| token.kind == TokenKind::DoubleColon))
        || module
            .invalid_names
            .iter()
            .any(|name| name.occurrence == veln_ast::NameOccurrence::PathSegment)
}

fn attach_classified_path_segments(
    files: &mut [IndexedFile],
    module: &veln_ast::SurfaceModule,
    project: &veln_ast::SurfaceModule,
) {
    #[cfg(test)]
    PATH_CLASSIFICATION_CONTEXTS.set(PATH_CLASSIFICATION_CONTEXTS.get() + 1);
    #[cfg(test)]
    record_dependency_path_classifications(
        files
            .iter()
            .filter(|file| matches!(file.origin, IndexedOrigin::Package { .. }))
            .count(),
    );
    let segments =
        veln_sema::classified_project_qualified_path_segments_with_context(module, project);
    let mut segments_by_file = BTreeMap::<String, Vec<QualifiedPathSegment>>::new();
    for segment in segments {
        segments_by_file
            .entry(segment.span.file.as_str().to_string())
            .or_default()
            .push(segment);
    }
    for file in files.iter_mut() {
        file.classified_paths.segments = segments_by_file
            .remove(file.source.path().as_str())
            .unwrap_or_default();
        file.classified_paths.by_range = file
            .classified_paths
            .segments
            .iter()
            .map(|segment| {
                (
                    (segment.span.start.offset, segment.span.end.offset),
                    segment.clone(),
                )
            })
            .collect();
    }
    attach_canonical_variant_refinement_type_arguments(files, project);
}

fn attach_canonical_variant_refinement_type_arguments(
    files: &mut [IndexedFile],
    project: &veln_ast::SurfaceModule,
) {
    let mut annotations = Vec::<(&str, Option<&str>, &[String])>::new();
    let mut groups = Vec::<(usize, (usize, usize), usize, usize)>::new();
    for (file_index, file) in files.iter().enumerate() {
        for (range, argument_ranges) in
            &file.variant_refinement_type_argument_ranges_by_final_range
        {
            let start = annotations.len();
            let type_parameters = file
                .variant_refinement_type_parameter_context_index_by_final_range
                .get(range)
                .and_then(|context_index| {
                    file.variant_refinement_type_parameter_contexts
                        .get(*context_index)
                })
                .map_or(&[][..], Vec::as_slice);
            annotations.extend(argument_ranges.iter().map(|(start, end)| {
                (
                    &file.source.text()[*start..*end],
                    Some(file.module.as_str()),
                    type_parameters,
                )
            }));
            groups.push((file_index, *range, start, annotations.len()));
        }
    }
    let resolved =
        veln_sema::canonical_type_annotation_identities_with_context(project, &annotations);
    for (file_index, range, start, end) in groups {
        if let Some(identities) = resolved[start..end].iter().cloned().collect() {
            files[file_index]
                .canonical_variant_refinement_type_arguments_by_final_range
                .insert(range, identities);
        }
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

pub(crate) fn empty_surface_module() -> veln_ast::SurfaceModule {
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
