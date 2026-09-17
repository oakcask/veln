fn index_workspace_source(source: SourceFile) -> (IndexedFile, FileDeclarations, ParseOutput) {
    let path = source.path().as_str().to_string();
    let companion_target_module = classify_companion_source(&path)
        .and_then(|companion| module_name_from_path(&companion.target_path));
    let path_module = module_name_from_path(&path);
    let navigation_isolated = path_module_invalid_for_navigation(path_module.as_deref());
    let module = explicit_module_name(source.text())
        .or(path_module)
        .unwrap_or_default();
    #[cfg(test)]
    record_workspace_source_parse();
    let parsed = parse(&source);
    let (uses, external_uses, import_aliases, external_import_aliases) = use_modules(source.text());
    let schema_alias_external_imports = schema_alias_external_imports(&parsed);
    let invalid_declaration_names = invalid_declaration_names(&parsed);
    let tokens = lex(&source).tokens;
    let schema_operation_leaf_spans = valid_schema_operation_leaf_spans(&parsed.tree);
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
        schema_alias_external_imports,
        invalid_declaration_names: invalid_name_spans(&invalid_declaration_names),
        recovery_symbols,
        schema_operation_leaf_spans,
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
            if parsed.diagnostics.is_empty() {
                declarations.extend(file_declarations(&file, &parsed.tree));
                let mut source_module = veln_ast::lower_surface_ast(&parsed.tree);
                assign_module_name(&mut source_module, &file.module);
                append_surface_module(&mut dependency_module, source_module.clone());
                append_surface_module(module, source_module);
            } else {
                declarations
                    .schema_alias_blockers
                    .extend(schema_alias_declarations(&file, &parsed.tree));
                declarations
                    .recovered_package_schema_targets
                    .extend(package_schema_targets(&file, &parsed.tree));
            }
        }
        files.push(file);
    }
    declarations.resolved_package_schema_aliases.extend(
        veln_sema::resolved_schema_aliases(&dependency_module)
            .into_iter()
            .map(|resolved| ResolvedPackageSchemaAlias {
                package: package.clone(),
                package_origin,
                alias_module: resolved.alias_module,
                alias_name: resolved.alias_name,
                alias_span: resolved.alias_span,
                target_module: resolved.target_module,
                target_name: resolved.target_name,
            }),
    );
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
    let schema_operation_leaf_spans = valid_schema_operation_leaf_spans(&parsed.tree);
    let file = IndexedFile {
        source: source_file,
        tokens,
        module,
        companion_target_module: None,
        uses,
        external_uses,
        import_aliases,
        external_import_aliases,
        schema_alias_external_imports: Vec::new(),
        invalid_declaration_names: invalid_name_spans(&invalid_declaration_names),
        recovery_symbols: Vec::new(),
        schema_operation_leaf_spans,
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

fn valid_schema_operation_leaf_spans(syntax: &SyntaxTree) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    for item in &syntax.items {
        match item {
            SyntaxItem::Function(function) => {
                for line in &function.body {
                    let expr = match line {
                        BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => expr,
                    };
                    collect_valid_schema_operation_leaf_spans(expr, &mut spans);
                }
            }
            SyntaxItem::Handler(handler) => {
                for clause in &handler.operation_clauses {
                    collect_valid_schema_operation_leaf_spans(&clause.body, &mut spans);
                }
            }
            _ => {}
        }
    }
    spans
}

fn collect_valid_schema_operation_leaf_spans(expr: &Expr, spans: &mut Vec<SourceSpan>) {
    match &expr.kind {
        ExprKind::SchemaDecode {
            schema_spans,
            recovered,
            input,
            base,
            ..
        } => {
            if !recovered && let Some(leaf) = schema_spans.last() {
                spans.push(leaf.clone());
            }
            collect_valid_schema_operation_leaf_spans(input, spans);
            collect_valid_schema_operation_leaf_spans(base, spans);
        }
        ExprKind::SchemaEncode {
            schema_spans,
            recovered,
            value,
            ..
        } => {
            if !recovered && let Some(leaf) = schema_spans.last() {
                spans.push(leaf.clone());
            }
            collect_valid_schema_operation_leaf_spans(value, spans);
        }
        ExprKind::TypeApply { callee, .. }
        | ExprKind::FieldAccess { base: callee, .. }
        | ExprKind::Try(callee)
        | ExprKind::Prefix { expr: callee, .. } => {
            collect_valid_schema_operation_leaf_spans(callee, spans);
        }
        ExprKind::Call { callee, args } => {
            collect_valid_schema_operation_leaf_spans(callee, spans);
            for arg in args {
                collect_valid_schema_operation_leaf_spans(arg, spans);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_valid_schema_operation_leaf_spans(arg, spans);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_valid_schema_operation_leaf_spans(body, spans);
            for arg in args {
                collect_valid_schema_operation_leaf_spans(arg, spans);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_valid_schema_operation_leaf_spans(&field.expr, spans);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_valid_schema_operation_leaf_spans(&entry.key, spans);
                collect_valid_schema_operation_leaf_spans(&entry.value, spans);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_valid_schema_operation_leaf_spans(item, spans);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_valid_schema_operation_leaf_spans(scrutinee, spans);
            for arm in arms {
                collect_valid_schema_operation_leaf_spans(&arm.expr, spans);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_valid_schema_operation_leaf_spans(condition, spans);
            collect_valid_schema_operation_leaf_spans(then_branch, spans);
            for branch in else_if_branches {
                collect_valid_schema_operation_leaf_spans(&branch.condition, spans);
                collect_valid_schema_operation_leaf_spans(&branch.expr, spans);
            }
            collect_valid_schema_operation_leaf_spans(else_branch, spans);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_valid_schema_operation_leaf_spans(left, spans);
            collect_valid_schema_operation_leaf_spans(right, spans);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
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

fn eligible_schema_aliases(
    aliases: Vec<NeutralSymbol>,
    package_aliases: &[PackageSchemaAliasDeclaration],
    package_targets: &[PackageSchemaTarget],
    recovered_package_targets: &[PackageSchemaTarget],
    resolved_package_aliases: &[ResolvedPackageSchemaAlias],
    resolved: Vec<veln_sema::ResolvedSchemaAlias>,
) -> Vec<NeutralSymbol> {
    let resolved_package_aliases = resolved_package_aliases
        .iter()
        .filter(|candidate| candidate.package_origin == PackageOrigin::DirectDependency)
        .filter_map(|candidate| {
            Some((
                (
                    candidate.package.as_str(),
                    candidate.alias_module.as_deref()?,
                    candidate.alias_name.as_str(),
                    candidate.alias_span.file.as_str(),
                ),
                candidate,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    aliases
        .iter()
        .filter(|alias| match alias.package_origin {
            None => resolved.iter().any(|candidate| {
                    candidate.alias_name == alias.name
                        && candidate.alias_module.as_deref() == Some(alias.module.as_str())
                        && candidate.alias_span.file == alias.declaration.span.file
                        && alias.declaration.span.start.offset >= candidate.alias_span.start.offset
                        && alias.declaration.span.end.offset <= candidate.alias_span.end.offset
                }),
            Some(PackageOrigin::DirectDependency) => {
                let Some(package) = alias.package.as_deref() else {
                    return false;
                };
                let Some(resolved_alias) = resolved_package_aliases.get(&(
                    package,
                    alias.module.as_str(),
                    alias.name.as_str(),
                    alias.declaration.span.file.as_str(),
                )) else {
                    return false;
                };
                let Some(target_module) = resolved_alias.target_module.as_deref() else {
                    return false;
                };
                let target_name = resolved_alias.target_name.as_str();
                package_aliases
                    .iter()
                    .filter(|candidate| {
                        candidate.package == package
                            && candidate.module == alias.module
                            && candidate.name == alias.name
                            && candidate.package_origin == PackageOrigin::DirectDependency
                    })
                    .count()
                    == 1
                    && !package_aliases.iter().any(|candidate| {
                        candidate.package == package
                            && candidate.module == *target_module
                            && candidate.name == target_name
                            && candidate.package_origin == PackageOrigin::DirectDependency
                    })
                    && !recovered_package_targets.iter().any(|target| {
                        target.package == package
                            && ((target.module == alias.module && target.name == alias.name)
                                || (target.module == *target_module && target.name == target_name))
                            && target.package_origin == PackageOrigin::DirectDependency
                    })
                    && !package_targets.iter().any(|target| {
                        target.package == package
                            && target.module == alias.module
                            && target.name == alias.name
                            && target.package_origin == PackageOrigin::DirectDependency
                    })
                    && has_unique_public_direct_package_schema_target(
                        package_targets,
                        package,
                        target_module,
                        target_name,
                    )
            }
            Some(PackageOrigin::StandardLibrary) => false,
        })
        .cloned()
        .collect()
}

fn has_unique_public_direct_package_schema_target(
    package_targets: &[PackageSchemaTarget],
    package: &str,
    module: &str,
    name: &str,
) -> bool {
    let mut targets = package_targets.iter().filter(|target| {
        target.package == package
            && target.module == module
            && target.name == name
            && target.package_origin == PackageOrigin::DirectDependency
    });
    matches!((targets.next(), targets.next()), (Some(target), None) if target.public)
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
