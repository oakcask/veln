fn same_constructor(left: &ConstructorSymbol, right: &ConstructorSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.type_name == right.type_name
        && left.name == right.name
        && left.package_origin == right.package_origin
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
        && left.package_origin == right.package_origin
        && left.declaration_kind == right.declaration_kind
        && left.standard_prelude == right.standard_prelude
        && left.alias_target_module == right.alias_target_module
        && left.alias_target_name == right.alias_target_name
        && left.invalid_declaration_name == right.invalid_declaration_name
        && left.declaration == right.declaration
}

fn same_type(left: &TypeSymbol, right: &TypeSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.name == right.name
        && left.package_origin == right.package_origin
        && left.standard_prelude == right.standard_prelude
        && left.declaration == right.declaration
}

fn same_type_alias(left: &TypeAliasSymbol, right: &TypeAliasSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.name == right.name
        && left.target_module == right.target_module
        && left.target_name == right.target_name
        && left.package_origin == right.package_origin
        && left.standard_prelude == right.standard_prelude
        && left.declaration == right.declaration
}

impl FileDeclarations {
    fn extend(&mut self, other: Self) {
        self.schemas.extend(other.schemas);
        self.schema_aliases.extend(other.schema_aliases);
        self.effects.extend(other.effects);
        self.handlers.extend(other.handlers);
        self.operations.extend(other.operations);
        self.functions.extend(other.functions);
        self.package_function_targets
            .extend(other.package_function_targets);
        self.package_type_targets.extend(other.package_type_targets);
        self.package_constructor_targets
            .extend(other.package_constructor_targets);
        self.types.extend(other.types);
        self.constructors.extend(other.constructors);
        self.type_aliases.extend(other.type_aliases);
    }
}

fn file_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> FileDeclarations {
    FileDeclarations {
        schemas: schema_declarations(file, syntax),
        schema_aliases: schema_alias_declarations(file, syntax),
        effects: effect_declarations(file, syntax),
        handlers: handler_declarations(file, syntax),
        operations: effect_operation_declarations(file, syntax),
        functions: function_declarations(file),
        package_function_targets: package_function_targets(file, syntax),
        package_type_targets: package_type_targets(file, syntax),
        package_constructor_targets: package_constructor_targets(file, syntax),
        types: type_declarations(file, syntax),
        constructors: constructor_declarations(file, syntax),
        type_aliases: type_alias_declarations(file, syntax),
    }
}

fn schema_alias_declarations(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<NeutralSymbol> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Schema => {
                let name = alias.name.as_ref()?;
                let span = alias.name_span.clone()?;
                if is_invalid_declaration_name(file, &span) {
                    return None;
                }
                neutral_declaration(file, name, span, Visibility::Public)
            }
            _ => None,
        })
        .collect()
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
        public,
    })
}

fn same_schema(left: &NeutralSymbol, right: &NeutralSymbol) -> bool {
    left.package == right.package
        && left.module == right.module
        && left.name == right.name
        && left.public == right.public
        && left.declaration == right.declaration
}

fn neutral_navigation_origin(
    file: &IndexedFile,
    span: SourceSpan,
    public: bool,
) -> Option<(NavigationLocation, Option<String>)> {
    let origin = declaration_origin(file, span, public)?;
    Some((origin.declaration, origin.package))
}

struct DeclarationOrigin {
    declaration: NavigationLocation,
    package: Option<String>,
    package_origin: Option<PackageOrigin>,
    standard_prelude: bool,
}

fn declaration_origin(
    file: &IndexedFile,
    span: SourceSpan,
    public: bool,
) -> Option<DeclarationOrigin> {
    match &file.origin {
        IndexedOrigin::Workspace => Some(DeclarationOrigin {
            declaration: workspace_location(span),
            package: None,
            package_origin: None,
            standard_prelude: false,
        }),
        IndexedOrigin::Package {
            identity,
            uri,
            exported,
            standard_library,
        } if *exported && public => Some(DeclarationOrigin {
            declaration: NavigationLocation {
                source: NavigationSource::Package { uri: uri.clone() },
                span,
            },
            package: Some(identity.clone()),
            package_origin: Some(package_origin(*standard_library)),
            standard_prelude: *standard_library && file.module == "prelude",
        }),
        IndexedOrigin::Package { .. } => None,
    }
}

#[derive(Clone, Copy)]
struct PackageContext<'a> {
    identity: &'a str,
    origin: PackageOrigin,
}

fn package_context(file: &IndexedFile) -> Option<PackageContext<'_>> {
    let IndexedOrigin::Package {
        identity,
        standard_library,
        ..
    } = &file.origin
    else {
        return None;
    };
    Some(PackageContext {
        identity,
        origin: package_origin(*standard_library),
    })
}

fn package_origin(standard_library: bool) -> PackageOrigin {
    if standard_library {
        PackageOrigin::StandardLibrary
    } else {
        PackageOrigin::DirectDependency
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
            && let Some(name_index) = next_non_layout_index(tokens, index)
            && let Some(name) = tokens.get(name_index)
            && is_identifier(&name.text)
        {
            let span = file.source.span(name.range);
            let public = previous_non_layout_token(tokens, index)
                .is_some_and(|previous| previous.kind == TokenKind::Pub);
            let alias = function_alias_declaration(tokens, name_index, name.range.end);
            let invalid_declaration_name = is_invalid_declaration_name(file, &span);
            if invalid_declaration_name && alias.declaration_kind != SymbolDeclarationKind::PublicAlias
            {
                continue;
            }
            let Some(origin) = declaration_origin(file, span, public) else {
                continue;
            };
            functions.push(FunctionSymbol {
                module: file.module.clone(),
                name: name.text.clone(),
                alias_target_module: alias.target_module,
                alias_target_name: alias.target_name,
                declaration: origin.declaration,
                package: origin.package,
                package_origin: origin.package_origin,
                public,
                standard_prelude: origin.standard_prelude,
                declaration_kind: alias.declaration_kind,
                invalid_declaration_name,
            });
        }
    }
    functions
}

fn package_function_targets(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<PackageFunctionTarget> {
    let Some(package) = package_context(file) else {
        return Vec::new();
    };
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Function(function) if function.kind == veln_syntax::FunctionKind::Function => {
                let name = function.name.as_ref()?;
                let name_span = function.name_span.as_ref()?;
                if is_invalid_declaration_name(file, name_span) {
                    return None;
                }
                Some(PackageFunctionTarget {
                    module: file.module.clone(),
                    name: name.clone(),
                    package: package.identity.to_string(),
                    package_origin: package.origin,
                })
            }
            _ => None,
        })
        .collect()
}

fn package_type_targets(file: &IndexedFile, syntax: &SyntaxTree) -> Vec<PackageTypeTarget> {
    let Some(package) = package_context(file) else {
        return Vec::new();
    };
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) => {
                let name = type_decl.name.as_ref()?;
                let name_span = type_decl.name_span.as_ref()?;
                if is_invalid_declaration_name(file, name_span) {
                    return None;
                }
                Some(PackageTypeTarget {
                    module: file.module.clone(),
                    name: name.clone(),
                    package: package.identity.to_string(),
                    package_origin: package.origin,
                })
            }
            _ => None,
        })
        .collect()
}

fn package_constructor_targets(
    file: &IndexedFile,
    syntax: &SyntaxTree,
) -> Vec<PackageConstructorTarget> {
    let Some(package) = package_context(file) else {
        return Vec::new();
    };
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) if type_decl.end_present => {
                let type_name = type_decl.name.as_ref()?;
                Some((type_name.clone(), type_decl))
            }
            _ => None,
        })
        .flat_map(|(type_name, type_decl)| {
            type_decl.variants.iter().filter_map(move |variant| {
                if variant.visibility != Visibility::Public {
                    return None;
                }
                let name = variant.name.as_ref()?;
                let span = constructor_variant_name_span(file, file.tokens.as_slice(), variant, name);
                if is_invalid_declaration_name(file, &span) {
                    return None;
                }
                Some(PackageConstructorTarget {
                    module: file.module.clone(),
                    type_name: type_name.clone(),
                    name: name.clone(),
                    package: package.identity.to_string(),
                    package_origin: package.origin,
                })
            })
        })
        .collect()
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
                let origin = declaration_origin(file, span, public)?;
                Some(TypeSymbol {
                    module: file.module.clone(),
                    name: name.clone(),
                    declaration: origin.declaration,
                    package: origin.package,
                    package_origin: origin.package_origin,
                    public,
                    standard_prelude: origin.standard_prelude,
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
                let origin = declaration_origin(file, span, public)?;
                Some(ConstructorSymbol {
                    module: file.module.clone(),
                    type_name: type_decl.name.clone().unwrap_or_default(),
                    name: name.clone(),
                    declaration: origin.declaration,
                    package: origin.package,
                    package_origin: origin.package_origin,
                    public,
                    standard_prelude: origin.standard_prelude,
                    declaration_kind: SymbolDeclarationKind::Declaration,
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
                let origin = declaration_origin(file, name_span.clone(), true)?;
                Some(TypeAliasSymbol {
                    module: file.module.clone(),
                    name,
                    declaration: origin.declaration,
                    target_module,
                    target_name,
                    package: origin.package,
                    package_origin: origin.package_origin,
                    standard_prelude: origin.standard_prelude,
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
