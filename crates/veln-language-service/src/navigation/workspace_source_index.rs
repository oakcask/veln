fn index_workspace_source(source: SourceFile) -> (IndexedFile, FileDeclarations, ParseOutput) {
    #[cfg(test)]
    record_workspace_source_parse();
    let parsed = parse(&source);
    let identity = WorkspaceSourceIdentity::new(&source);
    let imports = WorkspaceImports::new(source.text(), &parsed);
    let syntax = WorkspaceSyntaxIndex::new(identity.source.navigation_isolated, &source, &parsed);
    let file = indexed_workspace_file(source, identity, imports, syntax);
    let declarations = workspace_file_declarations(&file, &parsed.tree);
    (file, declarations, parsed)
}

struct WorkspaceSourceIdentity {
    source: SourceIdentity,
    companion_target_module: Option<String>,
}

impl WorkspaceSourceIdentity {
    fn new(source: &SourceFile) -> Self {
        let path = source.path().as_str().to_string();
        let companion_target_module = classify_companion_source(&path)
            .and_then(|companion| module_name_from_path(&companion.target_path));
        Self {
            source: SourceIdentity::new(&path, source.text()),
            companion_target_module,
        }
    }
}

struct WorkspaceImports {
    modules: UseModuleIndex,
    schema_alias_external_imports: Vec<ExternalImport>,
    workspace_imports: Vec<WorkspaceImport>,
}

impl WorkspaceImports {
    fn new(source: &str, parsed: &ParseOutput) -> Self {
        let use_diagnostics = UseDeclarationDiagnosticIndex::new(parsed);
        Self {
            modules: UseModuleIndex::new(source),
            schema_alias_external_imports: schema_alias_external_imports(parsed, &use_diagnostics),
            workspace_imports: workspace_imports(parsed, &use_diagnostics),
        }
    }
}

struct WorkspaceSyntaxIndex {
    tokens: Vec<Token>,
    invalid_declaration_names: Vec<SourceSpan>,
    recovery_symbols: Vec<RecoverySymbol>,
    recovered_effect_declarations: Vec<SourceSpan>,
    recovered_handler_declarations: Vec<SourceSpan>,
    handler_reference_ranges: BTreeSet<(usize, usize)>,
    handler_operation_clause_references: Vec<HandlerOperationClauseReference>,
    handler_operation_clause_body_ranges: Vec<(usize, usize)>,
    handler_clause_bindings_by_name: BTreeMap<String, Vec<ClauseBinding>>,
    schema_operation_leaf_ranges: BTreeSet<(usize, usize)>,
    schema_composition_leaf_spans: Vec<SourceSpan>,
    effects: WorkspaceEffectIndex,
    variant_refinement_final_ranges: BTreeSet<(usize, usize)>,
}

impl WorkspaceSyntaxIndex {
    fn new(navigation_isolated: bool, source: &SourceFile, parsed: &ParseOutput) -> Self {
        let invalid_names = invalid_declaration_names(parsed);
        let tokens = lex(source).tokens;
        let schema_operation_leaf_ranges = valid_schema_operation_leaf_spans(&parsed.tree)
            .into_iter()
            .map(|span| (span.start.offset, span.end.offset))
            .collect();
        let schema_composition_leaf_spans =
            valid_schema_composition_leaf_spans(source, &tokens, parsed);
        let effects = WorkspaceEffectIndex::new(&tokens, parsed);
        let variant_refinement_final_ranges = variant_refinement_final_ranges(&parsed.tree);
        let handler_diagnostics = HandlerDiagnosticIndex::new(parsed);
        let recovery_symbols = workspace_recovery_symbols(
            navigation_isolated,
            source,
            &tokens,
            &parsed.tree,
            &invalid_names,
        );
        let recovered_effect_declarations = recovered_effect_declarations(&parsed.tree);
        let recovered_handler_declarations =
            recovered_handler_declarations(parsed, &handler_diagnostics);
        let handler_argument_delimiters = HandlerArgumentDelimiters::new(&tokens);
        let handler_reference_ranges = valid_handler_reference_ranges(
            parsed,
            &tokens,
            &handler_argument_delimiters,
            &handler_diagnostics,
        );
        let handler_operation_clause_references = valid_handler_operation_clause_references(
            parsed,
            &recovered_handler_declarations,
        );
        let handler_operation_clause_body_ranges =
            handler_operation_clause_body_ranges(source, &tokens);
        let handler_clause_bindings_by_name = handler_clause_bindings_by_name(&parsed.tree);
        Self {
            tokens,
            invalid_declaration_names: invalid_name_spans(&invalid_names),
            recovery_symbols,
            recovered_effect_declarations,
            recovered_handler_declarations,
            handler_reference_ranges,
            handler_operation_clause_references,
            handler_operation_clause_body_ranges,
            handler_clause_bindings_by_name,
            schema_operation_leaf_ranges,
            schema_composition_leaf_spans,
            effects,
            variant_refinement_final_ranges,
        }
    }
}

fn indexed_workspace_file(
    source: SourceFile,
    identity: WorkspaceSourceIdentity,
    imports: WorkspaceImports,
    syntax: WorkspaceSyntaxIndex,
) -> IndexedFile {
    IndexedFile {
        source,
        tokens: syntax.tokens,
        module: identity.source.module,
        companion_target_module: identity.companion_target_module,
        uses: imports.modules.local_modules,
        external_uses: imports.modules.external_modules,
        import_aliases: imports.modules.local_aliases,
        external_import_aliases: imports.modules.external_aliases,
        schema_alias_external_imports: imports.schema_alias_external_imports,
        workspace_imports: imports.workspace_imports,
        invalid_declaration_names: syntax.invalid_declaration_names,
        recovery_symbols: syntax.recovery_symbols,
        recovered_effect_declarations: syntax.recovered_effect_declarations,
        recovered_handler_declarations: syntax.recovered_handler_declarations,
        handler_reference_ranges: syntax.handler_reference_ranges,
        handler_operation_clause_references: syntax.handler_operation_clause_references,
        handler_operation_clause_body_ranges: syntax.handler_operation_clause_body_ranges,
        handler_clause_bindings_by_name: syntax.handler_clause_bindings_by_name,
        schema_operation_leaf_ranges: syntax.schema_operation_leaf_ranges,
        schema_composition_leaf_spans: syntax.schema_composition_leaf_spans,
        effect_reference_ranges: syntax.effects.reference_ranges,
        effect_operation_ranges: syntax.effects.operation_ranges,
        generic_effect_binders: syntax.effects.generic_binders,
        variant_refinement_final_ranges: syntax.variant_refinement_final_ranges,
        classified_paths: ClassifiedPathIndex::default(),
        type_reference_locations: OnceLock::new(),
        navigation_isolated: identity.source.navigation_isolated,
        origin: IndexedOrigin::Workspace,
    }
}

struct WorkspaceEffectIndex {
    reference_ranges: BTreeSet<(usize, usize)>,
    operation_ranges: BTreeSet<(usize, usize)>,
    generic_binders: Vec<GenericEffectBinder>,
}

impl WorkspaceEffectIndex {
    fn new(tokens: &[Token], parsed: &ParseOutput) -> Self {
        let list_membership = effect_list_membership(tokens);
        let reference_ranges =
            valid_effect_reference_ranges(tokens, &list_membership, &parsed.tree);
        let operation_ranges = valid_effect_operation_ranges(tokens, &reference_ranges, parsed);
        let generic_binders = generic_effect_binders(&parsed.tree);
        Self {
            reference_ranges,
            operation_ranges,
            generic_binders,
        }
    }
}
