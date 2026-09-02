fn same_constructor(left: &ConstructorSymbol, right: &ConstructorSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.type_name == right.type_name
        && left.name == right.name
        && left.declaration == right.declaration
}

fn same_span(left: &SourceSpan, right: &SourceSpan) -> bool {
    left.file == right.file
        && left.start.offset == right.start.offset
        && left.end.offset == right.end.offset
}

fn same_function(left: &FunctionSymbol, right: &FunctionSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.name == right.name
        && left.standard_prelude == right.standard_prelude
        && left.declaration == right.declaration
}

fn same_type(left: &TypeSymbol, right: &TypeSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.name == right.name
        && left.standard_prelude == right.standard_prelude
        && left.declaration == right.declaration
}

fn index_workspace_source(source: SourceFile) -> (IndexedFile, FileDeclarations) {
    let path = source.path().as_str().to_string();
    let companion_target_module = classify_companion_source(&path)
        .and_then(|companion| module_name_from_path(&companion.target_path));
    let module = explicit_module_name(source.text())
        .or_else(|| module_name_from_path(&path))
        .unwrap_or_default();
    let (uses, external_uses, import_aliases, external_import_aliases) = use_modules(source.text());
    let parsed = parse(&source);
    let invalid_declaration_names = invalid_declaration_names(&parsed);
    let tokens = lex(&source).tokens;
    let recovery_symbols =
        recovery_symbols_for_workspace_source(&source, &tokens, &parsed.tree, &invalid_declaration_names);
    let file = IndexedFile {
        source,
        tokens,
        module,
        companion_target_module,
        uses,
        external_uses,
        import_aliases,
        external_import_aliases,
        invalid_declaration_names: invalid_declaration_names
            .iter()
            .map(|invalid| invalid.span.clone())
            .collect(),
        recovery_symbols,
        classified_path_segments: Vec::new(),
        type_reference_locations: OnceLock::new(),
        origin: IndexedOrigin::Workspace,
    };
    let declarations = file_declarations(&file, &parsed.tree);
    (file, declarations)
}

fn index_dependency_sources(
    files: &mut Vec<IndexedFile>,
    declarations: &mut FileDeclarations,
    dependency: DirectDependencySnapshot,
) {
    for (source, entry) in dependency.indexed_sources() {
        let (file, parsed) = indexed_dependency_source(&dependency, source, entry.uri());
        declarations.extend(file_declarations(&file, &parsed.tree));
        files.push(file);
    }
}

fn indexed_dependency_source(
    dependency: &DirectDependencySnapshot,
    source: &veln_project::CapturedPackageSource,
    uri: &str,
) -> (IndexedFile, ParseOutput) {
    let text =
        std::str::from_utf8(source.bytes()).expect("captured package source text is valid UTF-8");
    let source_file = SourceFile::new(source.path(), text);
    let module = explicit_module_name(text)
        .or_else(|| module_name_from_path(source.path()))
        .unwrap_or_default();
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
        invalid_declaration_names: invalid_declaration_names
            .iter()
            .map(|invalid| invalid.span.clone())
            .collect(),
        recovery_symbols: Vec::new(),
        classified_path_segments: Vec::new(),
        type_reference_locations: OnceLock::new(),
        origin: IndexedOrigin::Package {
            identity: dependency.identity.as_str().to_string(),
            uri: uri.to_string(),
            exported: dependency.exported_sources.contains(source.path()),
            standard_library: dependency.standard_library,
        },
    };
    (file, parsed)
}

fn attach_classified_path_segments(files: &mut [IndexedFile]) {
    let module = merged_surface_module(files);
    let segments = veln_sema::classified_project_qualified_path_segments(&module);
    for file in files {
        file.classified_path_segments = segments
            .iter()
            .filter(|segment| segment.span.file == *file.source.path())
            .cloned()
            .collect();
    }
}

fn merged_surface_module(files: &[IndexedFile]) -> veln_ast::SurfaceModule {
    let mut merged = veln_ast::SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for file in files {
        let parsed = parse(&file.source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let mut module = veln_ast::lower_surface_ast(&parsed.tree);
        assign_module_name(&mut module, &file.module);
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.codecs.extend(module.codecs);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
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
    for codec in &mut module.codecs {
        codec.module_name = Some(name.to_string());
    }
    for function in &mut module.functions {
        function.module_name = Some(name.to_string());
    }
}

impl FileDeclarations {
    fn extend(&mut self, other: Self) {
        self.schemas.extend(other.schemas);
        self.effects.extend(other.effects);
        self.handlers.extend(other.handlers);
        self.operations.extend(other.operations);
        self.functions.extend(other.functions);
        self.types.extend(other.types);
        self.constructors.extend(other.constructors);
        self.type_aliases.extend(other.type_aliases);
    }
}

fn file_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> FileDeclarations {
    FileDeclarations {
        schemas: schema_declarations(file, syntax),
        effects: effect_declarations(file, syntax),
        handlers: handler_declarations(file, syntax),
        operations: effect_operation_declarations(file, syntax),
        functions: function_declarations(file),
        types: type_declarations(file, syntax),
        constructors: constructor_declarations(file, syntax),
        type_aliases: type_alias_declarations(file, syntax),
    }
}

fn schema_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<NeutralSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Schema(schema) => {
                let name = schema.name.as_ref()?;
                let span = declaration_name_after_keyword(file, TokenKind::Schema, &schema.span)?;
                neutral_declaration(file, name, span, schema.visibility)
            }
            _ => None,
        })
        .collect()
}

fn effect_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<NeutralSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Effect(effect) => {
                let name = effect.name.as_ref()?;
                let span = declaration_name_after_keyword(file, TokenKind::Effect, &effect.span)?;
                neutral_declaration(file, name, span, effect.visibility)
            }
            _ => None,
        })
        .collect()
}

fn handler_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<NeutralSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Handler(handler) => {
                let name = handler.name.as_ref()?;
                let span = declaration_name_after_keyword(file, TokenKind::Handler, &handler.span)?;
                neutral_declaration(file, name, span, handler.visibility)
            }
            _ => None,
        })
        .collect()
}

fn effect_operation_declarations(
    file: &IndexedFile,
    syntax: &SyntaxTree,
) -> Vec<EffectOperationSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Effect(effect) => Some(effect),
            _ => None,
        })
        .flat_map(|effect| {
            let effect_name = effect.name.clone().unwrap_or_default();
            let public = effect.visibility == Visibility::Public;
            effect.operations.iter().filter_map(move |operation| {
                let name = operation.name.as_ref()?;
                let span = operation.name_span.clone();
                let (declaration, package) = neutral_navigation_origin(file, span, public)?;
                Some(EffectOperationSymbol {
                    module: file.module.clone(),
                    effect_name: effect_name.clone(),
                    name: name.clone(),
                    declaration,
                    package,
                })
            })
        })
        .collect()
}

fn neutral_declaration(
    file: &IndexedFile,
    name: &str,
    span: SourceSpan,
    visibility: Visibility,
) -> Option<NeutralSymbol> {
    let public = visibility == Visibility::Public;
    let (declaration, package) = neutral_navigation_origin(file, span, public)?;
    Some(NeutralSymbol {
        module: file.module.clone(),
        name: name.to_string(),
        declaration,
        package,
    })
}

fn neutral_navigation_origin(
    file: &IndexedFile,
    span: SourceSpan,
    public: bool,
) -> Option<(NavigationLocation, Option<String>)> {
    match &file.origin {
        IndexedOrigin::Workspace => Some((workspace_location(span), None)),
        IndexedOrigin::Package {
            identity,
            uri,
            exported,
            ..
        } => {
            if !exported || !public {
                return None;
            }
            Some((
                NavigationLocation {
                    source: NavigationSource::Package { uri: uri.clone() },
                    span,
                },
                Some(identity.clone()),
            ))
        }
    }
}

fn declaration_name_after_keyword(
    file: &IndexedFile,
    keyword: TokenKind,
    span: &SourceSpan,
) -> Option<SourceSpan> {
    file.tokens
        .iter()
        .enumerate()
        .find(|(_, token)| {
            token.kind == keyword
                && token.range.start >= span.start.offset
                && token.range.end <= span.end.offset
        })
        .and_then(|(index, _)| next_non_layout_token(&file.tokens, index))
        .filter(|token| token.kind == TokenKind::Ident)
        .map(|token| file.source.span(token.range))
}

fn visible_workspace_constructor_from(file: &IndexedFile, symbol: &ConstructorSymbol) -> bool {
    symbol.public || symbol.module == file.module
}

fn constructor_qualifier_matches(symbol: &ConstructorSymbol, qualifier: &str) -> bool {
    qualifier == symbol.module || qualifier == format!("{}::{}", symbol.module, symbol.type_name)
}

fn type_alias_targets_constructor(alias: &TypeAliasSymbol, symbol: &ConstructorSymbol) -> bool {
    if alias.standard_prelude != symbol.standard_prelude {
        return false;
    }
    if alias.target_name != symbol.type_name {
        return false;
    }
    match &alias.target_module {
        Some(module) => module == &symbol.module,
        None => alias.module == symbol.module,
    }
}

fn function_declarations(file: &IndexedFile) -> Vec<FunctionSymbol> {
    let mut functions = Vec::new();
    let tokens = &file.tokens;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.kind, TokenKind::Fn | TokenKind::Test)
            && let Some(name) = next_non_layout_token(tokens, index)
            && is_identifier(&name.text)
        {
            let span = file.source.span(name.range);
            if is_invalid_declaration_name(file, &span) {
                continue;
            }
            let public = previous_non_layout_token(tokens, index)
                .is_some_and(|previous| previous.kind == TokenKind::Pub);
            let (declaration, package, standard_prelude) = match &file.origin {
                IndexedOrigin::Workspace => (workspace_location(span), None, false),
                IndexedOrigin::Package {
                    identity,
                    uri,
                    exported,
                    standard_library,
                } => {
                    if !exported || !public {
                        continue;
                    }
                    (
                        NavigationLocation {
                            source: NavigationSource::Package { uri: uri.clone() },
                            span,
                        },
                        Some(identity.clone()),
                        *standard_library && file.module == "prelude",
                    )
                }
            };
            functions.push(FunctionSymbol {
                module: file.module.clone(),
                name: name.text.clone(),
                declaration,
                package,
                public,
                standard_prelude,
            });
        }
    }
    functions
}

fn type_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<TypeSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) => {
                let name = type_decl.name.as_ref()?;
                let span = type_decl.name_span.clone()?;
                if is_invalid_declaration_name(file, &span) {
                    return None;
                }
                let public = type_decl.visibility == Visibility::Public;
                let (declaration, package, standard_prelude) = match &file.origin {
                    IndexedOrigin::Workspace => (workspace_location(span), None, false),
                    IndexedOrigin::Package {
                        identity,
                        uri,
                        exported,
                        standard_library,
                        ..
                    } => {
                        if !exported || !public {
                            return None;
                        }
                        (
                            NavigationLocation {
                                source: NavigationSource::Package { uri: uri.clone() },
                                span,
                            },
                            Some(identity.clone()),
                            *standard_library && file.module == "prelude",
                        )
                    }
                };
                Some(TypeSymbol {
                    module: file.module.clone(),
                    name: name.clone(),
                    declaration,
                    package,
                    public,
                    standard_prelude,
                })
            }
            _ => None,
        })
        .collect()
}

fn constructor_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<ConstructorSymbol> {
    let tokens = file.tokens.as_slice();
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) => Some(type_decl),
            _ => None,
        })
        .flat_map(|type_decl| {
            let type_public = type_decl.visibility == Visibility::Public;
            type_decl.variants.iter().filter_map(move |variant| {
                let name = variant.name.as_ref()?;
                let public = type_public && variant.visibility == Visibility::Public;
                let span = constructor_variant_name_span(file, tokens, variant, name);
                if is_invalid_declaration_name(file, &span) {
                    return None;
                }
                let (declaration, package, standard_prelude) =
                    constructor_navigation_origin(file, span, public)?;
                Some(ConstructorSymbol {
                    module: file.module.clone(),
                    type_name: type_decl.name.clone().unwrap_or_default(),
                    name: name.clone(),
                    declaration,
                    package,
                    public,
                    standard_prelude,
                })
            })
        })
        .collect()
}

fn constructor_variant_name_span(
    file: &IndexedFile,
    tokens: &[Token],
    variant: &TypeVariantDecl,
    name: &str,
) -> SourceSpan {
    if let Some(span) = &variant.name_span {
        return span.clone();
    }
    tokens
        .iter()
        .find(|token| {
            token.kind == TokenKind::Ident
                && token.text == name
                && token.range.start >= variant.span.start.offset
                && token.range.end <= variant.span.end.offset
        })
        .map_or_else(
            || variant.span.clone(),
            |token| file.source.span(token.range),
        )
}

fn constructor_navigation_origin(
    file: &IndexedFile,
    span: SourceSpan,
    public: bool,
) -> Option<(NavigationLocation, Option<String>, bool)> {
    match &file.origin {
        IndexedOrigin::Workspace => Some((workspace_location(span), None, false)),
        IndexedOrigin::Package {
            identity,
            uri,
            exported,
            standard_library,
        } => {
            if !exported || !public {
                return None;
            }
            Some((
                NavigationLocation {
                    source: NavigationSource::Package { uri: uri.clone() },
                    span,
                },
                Some(identity.clone()),
                *standard_library && file.module == "prelude",
            ))
        }
    }
}

fn type_alias_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<TypeAliasSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Type => {
                let name = alias.name.clone()?;
                let name_span = alias.name_span.as_ref()?;
                if is_invalid_declaration_name(file, name_span) {
                    return None;
                }
                let target_name = alias.target.last()?.clone();
                let target_module = match alias.target.as_slice() {
                    [_] => None,
                    [segments @ .., _] => Some(segments.join("::")),
                    [] => None,
                };
                let (declaration, package, standard_prelude) = match &file.origin {
                    IndexedOrigin::Workspace => (workspace_location(name_span.clone()), None, false),
                    IndexedOrigin::Package {
                        identity,
                        uri,
                        exported,
                        standard_library,
                        ..
                    } => {
                        if !exported {
                            return None;
                        }
                        (
                            NavigationLocation {
                                source: NavigationSource::Package { uri: uri.clone() },
                                span: name_span.clone(),
                            },
                            Some(identity.clone()),
                            *standard_library && file.module == "prelude",
                        )
                    }
                };
                Some(TypeAliasSymbol {
                    module: file.module.clone(),
                    name,
                    declaration,
                    target_module,
                    target_name,
                    package,
                    standard_prelude,
                })
            }
            _ => None,
        })
        .collect()
}

fn invalid_declaration_names(parsed: &ParseOutput) -> Vec<InvalidName> {
    if !parsed.diagnostics.is_empty() {
        return Vec::new();
    }
    veln_ast::lower_surface_ast(&parsed.tree)
        .invalid_names
        .into_iter()
        .filter(|invalid| {
            matches!(
                invalid.occurrence,
                veln_ast::NameOccurrence::Declaration
                    | veln_ast::NameOccurrence::Binding
                    | veln_ast::NameOccurrence::PatternHead
            )
        })
        .collect()
}

fn recovery_symbols_for_workspace_source(
    source: &SourceFile,
    tokens: &[Token],
    syntax: &SyntaxTree,
    invalid_names: &[InvalidName],
) -> Vec<RecoverySymbol> {
    let mut symbols = Vec::new();
    for invalid in invalid_names {
        let Some(name) = source
            .text()
            .get(invalid.span.start.offset..invalid.span.end.offset)
        else {
            continue;
        };
        match invalid.occurrence {
            veln_ast::NameOccurrence::Declaration => {
                let Some(kind) = symbol_kind_for_name_class(invalid.class) else {
                    continue;
                };
                symbols.push(RecoverySymbol {
                    name: name.to_string(),
                    declaration: invalid.span.clone(),
                    source_file: source.path().as_str().to_string(),
                    scope_start: 0,
                    scope_end: source.text().len(),
                    declaration_scope_start: 0,
                    declaration_scope_end: source.text().len(),
                    public: recovery_declaration_public(source, tokens, syntax, &invalid.span),
                    kind,
                });
            }
            veln_ast::NameOccurrence::Binding | veln_ast::NameOccurrence::PatternHead
                if invalid.class == NameClass::ValueBinding =>
            {
                symbols.extend(recovery_binding_symbols(source, tokens, name, &invalid.span));
            }
            _ => {}
        }
    }
    symbols
}

fn recovery_binding_symbols(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    span: &SourceSpan,
) -> Vec<RecoverySymbol> {
    let mut symbols = Vec::new();
    for scope in function_scopes(tokens) {
        symbols.extend(recovery_parameter_symbols(source, name, span, &scope));
        symbols.extend(recovery_result_binding_symbol(source, name, span, &scope));
        symbols.extend(recovery_local_binding_symbols(source, name, span, scope));
    }
    symbols.extend(recovery_handler_binding_symbols(source, tokens, name, span));
    symbols
}

fn recovery_parameter_symbols(
    source: &SourceFile,
    name: &str,
    span: &SourceSpan,
    scope: &FunctionScope,
) -> Vec<RecoverySymbol> {
    scope
        .params
        .iter()
        .filter(|binding| scoped_binding_matches(binding, name, span))
        .map(|_| {
            value_binding_recovery_symbol(source, name, span, scope.body_start, scope.end, scope)
        })
        .collect()
}

fn recovery_result_binding_symbol(
    source: &SourceFile,
    name: &str,
    span: &SourceSpan,
    scope: &FunctionScope,
) -> Option<RecoverySymbol> {
    let binding = scope.result_binding.as_ref()?;
    scoped_binding_matches(binding, name, span)
        .then(|| value_binding_recovery_symbol(source, name, span, scope.body_start, scope.end, scope))
}

fn recovery_local_binding_symbols(
    source: &SourceFile,
    name: &str,
    span: &SourceSpan,
    scope: FunctionScope,
) -> Vec<RecoverySymbol> {
    scope
        .local_bindings
        .iter()
        .filter(|binding| local_binding_matches(binding, name, span))
        .map(|binding| value_binding_recovery_symbol(source, name, span, binding.start, binding.end, &scope))
        .collect()
}

fn recovery_handler_binding_symbols(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    span: &SourceSpan,
) -> Vec<RecoverySymbol> {
    handler_operation_clause_bindings_for_source(source, tokens)
        .into_iter()
        .filter(|binding| {
            binding.name == name
                && binding.declaration.start.offset == span.start.offset
                && binding.declaration.end.offset == span.end.offset
        })
        .map(|binding| RecoverySymbol {
            name: name.to_string(),
            declaration: span.clone(),
            source_file: source.path().as_str().to_string(),
            scope_start: binding.start,
            scope_end: binding.end,
            declaration_scope_start: binding.start,
            declaration_scope_end: binding.end,
            public: false,
            kind: binding.kind.symbol_kind(),
        })
        .collect()
}

fn value_binding_recovery_symbol(
    source: &SourceFile,
    name: &str,
    span: &SourceSpan,
    scope_start: usize,
    scope_end: usize,
    declaration_scope: &FunctionScope,
) -> RecoverySymbol {
    RecoverySymbol {
        name: name.to_string(),
        declaration: span.clone(),
        source_file: source.path().as_str().to_string(),
        scope_start,
        scope_end,
        declaration_scope_start: declaration_scope.body_start,
        declaration_scope_end: declaration_scope.end,
        public: false,
        kind: SymbolKind::ValueBinding,
    }
}
