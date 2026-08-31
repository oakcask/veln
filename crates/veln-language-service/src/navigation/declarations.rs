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
    let invalid_declaration_names = invalid_declaration_name_spans(&parsed);
    let tokens = lex(&source).tokens;
    let file = IndexedFile {
        source,
        tokens,
        module,
        companion_target_module,
        uses,
        external_uses,
        import_aliases,
        external_import_aliases,
        invalid_declaration_names,
        classified_path_segments: Vec::new(),
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
        let text = std::str::from_utf8(source.bytes())
            .expect("captured package source text is valid UTF-8");
        let source_file = SourceFile::new(source.path(), text);
        let module = explicit_module_name(text)
            .or_else(|| module_name_from_path(source.path()))
            .unwrap_or_default();
        let (uses, external_uses, import_aliases, external_import_aliases) = use_modules(text);
        let parsed = parse(&source_file);
        let invalid_declaration_names = invalid_declaration_name_spans(&parsed);
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
            invalid_declaration_names,
            classified_path_segments: Vec::new(),
            origin: IndexedOrigin::Package {
                identity: dependency.identity.as_str().to_string(),
                uri: entry.uri().to_string(),
                exported: dependency.exported_sources.contains(source.path()),
                standard_library: dependency.standard_library,
            },
        };
        declarations.extend(file_declarations(&file, &parsed.tree));
        files.push(file);
    }
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
        self.functions.extend(other.functions);
        self.types.extend(other.types);
        self.constructors.extend(other.constructors);
        self.type_aliases.extend(other.type_aliases);
    }
}

fn file_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> FileDeclarations {
    FileDeclarations {
        functions: function_declarations(file),
        types: type_declarations(file, syntax),
        constructors: constructor_declarations(file, syntax),
        type_aliases: type_alias_declarations(file, syntax),
    }
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
        if token.kind == TokenKind::Fn
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

fn invalid_declaration_name_spans(parsed: &ParseOutput) -> Vec<SourceSpan> {
    if !parsed.diagnostics.is_empty() {
        return Vec::new();
    }
    veln_ast::lower_surface_ast(&parsed.tree)
        .invalid_names
        .into_iter()
        .filter(|invalid| {
            matches!(
                invalid.occurrence,
                veln_ast::NameOccurrence::Declaration | veln_ast::NameOccurrence::Binding
            )
        })
        .map(|invalid| invalid.span)
        .collect()
}

fn is_invalid_declaration_name(file: &IndexedFile, span: &SourceSpan) -> bool {
    file.invalid_declaration_names.iter().any(|invalid| {
        invalid.file == span.file
            && invalid.start.offset == span.start.offset
            && invalid.end.offset == span.end.offset
    })
}
