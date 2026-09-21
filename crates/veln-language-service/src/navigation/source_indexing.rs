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
    let schema_operation_leaf_ranges = valid_schema_operation_leaf_spans(&parsed.tree)
        .into_iter()
        .map(|span| (span.start.offset, span.end.offset))
        .collect();
    let schema_composition_leaf_spans =
        valid_schema_composition_leaf_spans(&source, &tokens, &parsed);
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
        schema_operation_leaf_ranges,
        schema_composition_leaf_spans,
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
    let schema_operation_leaf_ranges = valid_schema_operation_leaf_spans(&parsed.tree)
        .into_iter()
        .map(|span| (span.start.offset, span.end.offset))
        .collect();
    let schema_composition_leaf_spans =
        valid_schema_composition_leaf_spans(&source_file, &tokens, &parsed);
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
        schema_operation_leaf_ranges,
        schema_composition_leaf_spans,
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

fn valid_schema_composition_leaf_spans(
    source: &SourceFile,
    tokens: &[Token],
    parsed: &ParseOutput,
) -> Vec<SourceSpan> {
    if parsed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.parser_context == "schema_field" && diagnostic.span.is_none())
    {
        return Vec::new();
    }
    let mut recovery_ranges = parsed
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.parser_context == "schema_field")
        .filter_map(|diagnostic| diagnostic.span.as_ref())
        .filter(|span| span.file == *source.path())
        .map(|span| (span.start.offset, span.end.offset))
        .collect::<Vec<_>>();
    recovery_ranges.sort_unstable();

    let mut spans = Vec::new();
    let mut token_cursor = 0usize;
    let mut recovery_cursor = 0usize;
    let mut recovery_end = 0usize;
    for field in parsed
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Schema(schema) => Some(schema.fields.as_slice()),
            _ => None,
        })
        .flatten()
    {
        while token_cursor < tokens.len()
            && tokens[token_cursor].range.end <= field.span.start.offset
        {
            token_cursor += 1;
        }
        let field_start = token_cursor;
        while token_cursor < tokens.len()
            && tokens[token_cursor].range.start < field.span.end.offset
        {
            token_cursor += 1;
        }
        #[cfg(test)]
        record_schema_composition_field_token_visits(token_cursor - field_start);
        let Some(leaf_index) =
            schema_composition_path_leaf_in_field(tokens, field_start, token_cursor)
        else {
            continue;
        };
        let token = &tokens[leaf_index];
        while recovery_cursor < recovery_ranges.len()
            && recovery_ranges[recovery_cursor].0 <= token.range.start
        {
            recovery_end = recovery_end.max(recovery_ranges[recovery_cursor].1);
            recovery_cursor += 1;
        }
        if recovery_end >= token.range.end {
            continue;
        }
        spans.push(source.span(token.range));
    }
    spans
}

fn schema_composition_path_leaf_in_field(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Option<usize> {
    let mut significant = (start..end)
        .filter(|index| {
            !matches!(
                tokens[*index].kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline
            )
        })
        .collect::<Vec<_>>();
    if let Some(where_position) = significant
        .iter()
        .position(|index| tokens[*index].kind == TokenKind::Where)
    {
        significant.truncate(where_position);
    }
    let colon_position = significant
        .iter()
        .position(|index| tokens[*index].kind == TokenKind::Colon)?;
    let field_type = &significant[colon_position + 1..];
    schema_path_leaf_in(field_type, tokens)
        .or_else(|| repeat_schema_path_leaf(field_type, tokens))
        .or_else(|| array_schema_path_leaf(field_type, tokens))
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

struct WorkspaceSchemaCompositionDeclarations<'a> {
    schemas: &'a [NeutralSymbol],
    schema_aliases: &'a [NeutralSymbol],
    types: &'a [TypeSymbol],
    type_aliases: &'a [TypeAliasSymbol],
}

fn package_schema_composition_references(
    files: &[IndexedFile],
    workspace: WorkspaceSchemaCompositionDeclarations<'_>,
    schema_index: &BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    alias_index: &BTreeMap<(String, String, String), Vec<NeutralSymbol>>,
    schema_aliases: &[NeutralSymbol],
    module_imports: &BTreeMap<String, SchemaAliasModuleImports>,
) -> Vec<SchemaCompositionReference> {
    let prelude_alias_index = standard_prelude_schema_alias_index(schema_aliases);
    let workspace_schema_blockers = workspace_schema_blocker_index(workspace.schemas);
    let workspace_schema_alias_blockers =
        workspace_schema_alias_blocker_index(workspace.schema_aliases);
    let workspace_type_blockers =
        workspace_type_blocker_index(workspace.types, workspace.type_aliases);
    let context = SchemaCompositionNavigationContext {
        schema_index,
        alias_index,
        prelude_alias_index: &prelude_alias_index,
        workspace_schema_blockers: &workspace_schema_blockers,
        workspace_schema_alias_blockers: &workspace_schema_alias_blockers,
        workspace_type_blockers: &workspace_type_blockers,
        module_imports,
    };
    files
        .iter()
        .filter(|file| workspace_navigation_file(file))
        .flat_map(|file| {
            let mut token_cursor = 0usize;
            file.schema_composition_leaf_spans
                .iter()
                .filter_map(|span| {
                    context.direct_dependency_schema_composition_reference(
                        file,
                        span,
                        &mut token_cursor,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn workspace_schema_blocker_index(
    schemas: &[NeutralSymbol],
) -> BTreeSet<(String, String)> {
    schemas
        .iter()
        .filter(|schema| schema.package.is_none())
        .map(|schema| (schema.module.clone(), schema.name.clone()))
        .collect()
}

fn workspace_schema_alias_blocker_index(
    aliases: &[NeutralSymbol],
) -> BTreeSet<(String, String)> {
    aliases
        .iter()
        .filter(|alias| alias.package.is_none())
        .map(|alias| (alias.module.clone(), alias.name.clone()))
        .collect()
}

fn workspace_type_blocker_index(
    types: &[TypeSymbol],
    type_aliases: &[TypeAliasSymbol],
) -> BTreeSet<(String, String)> {
    types
        .iter()
        .filter(|symbol| symbol.package.is_none())
        .map(|symbol| (symbol.module.clone(), symbol.name.clone()))
        .chain(
            type_aliases
                .iter()
                .filter(|symbol| symbol.package.is_none())
                .map(|symbol| (symbol.module.clone(), symbol.name.clone())),
        )
        .collect()
}

fn standard_prelude_schema_alias_index(
    schema_aliases: &[NeutralSymbol],
) -> BTreeMap<String, Vec<NeutralSymbol>> {
    let mut aliases = BTreeMap::new();
    for alias in schema_aliases.iter().filter(|alias| alias.standard_prelude) {
        aliases
            .entry(alias.name.clone())
            .or_insert_with(Vec::new)
            .push(alias.clone());
    }
    aliases
}

fn bare_schema_alias_index(
    schemas: &[NeutralSymbol],
    eligible_aliases: &[NeutralSymbol],
    alias_declarations: &[NeutralSymbol],
) -> BareSchemaAliasIndex {
    let mut workspace_aliases = BTreeMap::new();
    for alias in eligible_aliases
        .iter()
        .filter(|alias| alias.package.is_none())
    {
        workspace_aliases
            .entry((alias.module.clone(), alias.name.clone()))
            .or_insert_with(|| alias.clone());
    }
    BareSchemaAliasIndex {
        workspace_aliases,
        workspace_schemas: workspace_schema_blocker_index(schemas),
        workspace_alias_declarations: workspace_schema_alias_blocker_index(alias_declarations),
        standard_prelude_aliases: standard_prelude_schema_alias_index(eligible_aliases),
        standard_prelude_alias_declarations: alias_declarations
            .iter()
            .filter(|alias| alias.standard_prelude)
            .map(|alias| alias.name.clone())
            .collect(),
    }
}

fn schema_operation_lookup_index(
    schemas: &[NeutralSymbol],
    schema_aliases: &[NeutralSymbol],
    alias_declarations: &[PackageSchemaAliasDeclaration],
) -> SchemaOperationLookupIndex {
    let mut index = SchemaOperationLookupIndex::default();
    for schema in schemas {
        if let Some(package) = &schema.package {
            if matches!(
                schema.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            ) {
                index
                    .package_schemas
                    .entry((package.clone(), schema.module.clone(), schema.name.clone()))
                    .or_default()
                    .push(schema.clone());
            }
        } else {
            index
                .workspace_schemas
                .entry((schema.module.clone(), schema.name.clone()))
                .or_default()
                .push(schema.clone());
        }
    }
    for alias in schema_aliases {
        if let Some(package) = &alias.package {
            if matches!(
                alias.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            ) {
                index
                    .package_aliases
                    .entry((package.clone(), alias.module.clone(), alias.name.clone()))
                    .or_default()
                    .push(alias.clone());
            }
        } else {
            index
                .workspace_aliases
                .entry((alias.module.clone(), alias.name.clone()))
                .or_default()
                .push(alias.clone());
        }
    }
    index.package_alias_declarations = alias_declarations
        .iter()
        .filter(|alias| {
            matches!(
                alias.package_origin,
                PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary
            )
        })
        .map(|alias| (alias.package.clone(), alias.module.clone(), alias.name.clone()))
        .collect();
    index
}

struct SchemaCompositionNavigationContext<'a> {
    schema_index: &'a BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    alias_index: &'a BTreeMap<(String, String, String), Vec<NeutralSymbol>>,
    prelude_alias_index: &'a BTreeMap<String, Vec<NeutralSymbol>>,
    workspace_schema_blockers: &'a BTreeSet<(String, String)>,
    workspace_schema_alias_blockers: &'a BTreeSet<(String, String)>,
    workspace_type_blockers: &'a BTreeSet<(String, String)>,
    module_imports: &'a BTreeMap<String, SchemaAliasModuleImports>,
}

impl SchemaCompositionNavigationContext<'_> {
    fn direct_dependency_schema_composition_reference(
        &self,
        file: &IndexedFile,
        span: &SourceSpan,
        token_cursor: &mut usize,
    ) -> Option<SchemaCompositionReference> {
        let token = schema_composition_token(file, span, token_cursor)?;
        let Some(qualifier) = qualifier_for_token(&file.tokens, *token_cursor) else {
            return self.bare_prelude_schema_composition_reference(file, span, &token.text);
        };
        match schema_qualified_workspace_module(file, &qualifier, self.module_imports) {
            QualifiedWorkspaceModule::Unresolved if qualifier == "prelude" => {
                self.prelude_schema_composition_reference(span, &token.text)
            }
            QualifiedWorkspaceModule::External => {
                self.imported_schema_composition_reference(file, span, &qualifier, &token.text)
            }
            QualifiedWorkspaceModule::Workspace(_)
            | QualifiedWorkspaceModule::Ambiguous
            | QualifiedWorkspaceModule::Unresolved => None,
        }
    }

    fn bare_prelude_schema_composition_reference(
        &self,
        file: &IndexedFile,
        span: &SourceSpan,
        name: &str,
    ) -> Option<SchemaCompositionReference> {
        let blocker = (file.module.clone(), name.to_string());
        #[cfg(test)]
        record_schema_composition_blocker_lookup();
        if self.workspace_schema_blockers.contains(&blocker)
            || self.workspace_schema_alias_blockers.contains(&blocker)
            || self.workspace_type_blockers.contains(&blocker)
        {
            return None;
        }
        self.prelude_schema_composition_reference(span, name)
    }

    fn prelude_schema_composition_reference(
        &self,
        span: &SourceSpan,
        name: &str,
    ) -> Option<SchemaCompositionReference> {
        #[cfg(test)]
        record_schema_composition_prelude_lookup();
        let [alias] = self.prelude_alias_index.get(name)?.as_slice() else {
            return None;
        };
        Some(SchemaCompositionReference {
            span: span.clone(),
            target: SchemaReferenceTarget::Alias(alias.clone()),
        })
    }

    fn imported_schema_composition_reference(
        &self,
        file: &IndexedFile,
        span: &SourceSpan,
        qualifier: &str,
        name: &str,
    ) -> Option<SchemaCompositionReference> {
        let (module, package) = self
            .module_imports
            .get(&file.module)?
            .valid_external_route(qualifier)?;
        let package_origin = if package == "std" {
            PackageOrigin::StandardLibrary
        } else {
            PackageOrigin::DirectDependency
        };
        let alias_key = (package.clone(), module.clone(), name.to_string());
        let key = (package_origin, package, module, name.to_string());
        let target = match self.alias_index.get(&alias_key) {
            Some(candidates) if candidates.len() == 1 => {
                SchemaReferenceTarget::Alias(candidates[0].clone())
            }
            Some(_) => return None,
            None => SchemaReferenceTarget::Schema(
                package_schema_target(self.schema_index, &key)?.clone(),
            ),
        };
        Some(SchemaCompositionReference {
            span: span.clone(),
            target,
        })
    }
}

fn schema_composition_token<'a>(
    file: &'a IndexedFile,
    span: &SourceSpan,
    token_cursor: &mut usize,
) -> Option<&'a Token> {
    while *token_cursor < file.tokens.len()
        && file.tokens[*token_cursor].range.end <= span.start.offset
    {
        *token_cursor += 1;
    }
    let token = file.tokens.get(*token_cursor)?;
    (token.range.start == span.start.offset
        && token.range.end == span.end.offset
        && token
            .text
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_uppercase()))
    .then_some(token)
}

fn package_schema_target<'a>(
    schema_index: &'a BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    key: &(PackageOrigin, String, String, String),
) -> Option<&'a NeutralSymbol> {
    #[cfg(test)]
    {
        SCHEMA_COMPOSITION_TARGET_LOOKUPS.set(SCHEMA_COMPOSITION_TARGET_LOOKUPS.get() + 1);
    }
    schema_index.get(key)
}

fn package_schema_index(
    schemas: &[NeutralSymbol],
    declarations: &PackageSchemaDeclarations<'_>,
) -> BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol> {
    let mut candidates = BTreeMap::new();
    for schema in schemas
        .iter()
        .filter(|schema| {
            matches!(
                schema.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            )
        })
    {
        #[cfg(test)]
        record_schema_composition_declaration_visit();
        let Some(package) = schema.package.as_ref() else {
            continue;
        };
        candidates
            .entry((
                schema.package_origin.expect("package schema has an origin"),
                package.clone(),
                schema.module.clone(),
                schema.name.clone(),
            ))
            .or_insert_with(Vec::new)
            .push(schema);
    }

    candidates
        .into_iter()
        .filter_map(|(identity, candidates)| {
            (candidates.len() == 1
                && declarations.contains_schema(&(
                    identity.0,
                    &identity.1,
                    &identity.2,
                    &identity.3,
                )))
            .then(|| (identity, candidates[0].clone()))
        })
        .collect()
}

fn eligible_schema_aliases(
    aliases: Vec<NeutralSymbol>,
    declarations: &PackageSchemaDeclarations<'_>,
    resolved_package_aliases: &[ResolvedPackageSchemaAlias],
    resolved: Vec<veln_sema::ResolvedSchemaAlias>,
) -> Vec<NeutralSymbol> {
    let package_eligibility =
        PackageSchemaAliasEligibility::new(declarations, resolved_package_aliases);
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
            Some(PackageOrigin::DirectDependency) => package_eligibility.contains(alias),
            Some(PackageOrigin::StandardLibrary) => package_eligibility.contains(alias),
        })
        .cloned()
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
