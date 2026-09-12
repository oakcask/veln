fn function_declarations(file: &IndexedFile) -> Vec<FunctionSymbol> {
    let mut functions = Vec::new();
    let tokens = &file.tokens;
    for (index, token) in tokens.iter().enumerate() {
        if let Some(function) = function_declaration_at(file, index, token) {
            functions.push(function);
        }
    }
    functions
}

fn function_declaration_at(
    file: &IndexedFile,
    index: usize,
    token: &Token,
) -> Option<FunctionSymbol> {
    if !matches!(token.kind, TokenKind::Fn | TokenKind::Test) {
        return None;
    }
    let tokens = &file.tokens;
    let name_index = next_non_layout_index(tokens, index)?;
    let name = tokens.get(name_index)?;
    if !is_identifier(&name.text) {
        return None;
    }
    let span = file.source.span(name.range);
    if is_invalid_declaration_name(file, &span) {
        return None;
    }
    let public = previous_non_layout_token(tokens, index)
        .is_some_and(|previous| previous.kind == TokenKind::Pub);
    let is_public_alias = function_declaration_is_alias(tokens, name_index, name);
    let (declaration, package, package_origin, standard_prelude) =
        function_declaration_location(file, span, public)?;
    Some(FunctionSymbol {
        module: file.module.clone(),
        name: name.text.clone(),
        declaration,
        package,
        package_origin,
        public,
        standard_prelude,
        declaration_kind: function_declaration_kind(is_public_alias),
        alias_target_module: is_public_alias
            .then(|| function_alias_target_module(file, name_index))
            .flatten(),
        alias_target_name: is_public_alias
            .then(|| function_alias_target_name(tokens, name_index))
            .flatten(),
    })
}

fn function_declaration_is_alias(tokens: &[Token], name_index: usize, name: &Token) -> bool {
    next_non_layout_token(tokens, name_index)
        .filter(|token| token.range.start >= name.range.end)
        .is_some_and(|token| token.kind == TokenKind::Equal)
}

fn function_declaration_kind(is_public_alias: bool) -> SymbolDeclarationKind {
    if is_public_alias {
        SymbolDeclarationKind::PublicAlias
    } else {
        SymbolDeclarationKind::Declaration
    }
}

fn function_declaration_location(
    file: &IndexedFile,
    span: SourceSpan,
    public: bool,
) -> Option<(
    NavigationLocation,
    Option<String>,
    Option<PackageOrigin>,
    bool,
)> {
    match &file.origin {
        IndexedOrigin::Workspace => Some((workspace_location(span), None, None, false)),
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
                Some(if *standard_library {
                    PackageOrigin::StandardLibrary
                } else {
                    PackageOrigin::DirectDependency
                }),
                *standard_library && is_standard_prelude_module(&file.module),
            ))
        }
    }
}

fn function_alias_declarations(file: &IndexedFile) -> Vec<FunctionAliasSymbol> {
    let mut aliases = Vec::new();
    let tokens = &file.tokens;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.kind, TokenKind::Fn | TokenKind::Test)
            && let Some(name_index) = next_non_layout_index(tokens, index)
            && let Some(name) = tokens.get(name_index)
            && is_identifier(&name.text)
            && next_non_layout_token(tokens, name_index)
                .filter(|token| token.range.start >= name.range.end)
                .is_some_and(|token| token.kind == TokenKind::Equal)
        {
            let package = match &file.origin {
                IndexedOrigin::Workspace => None,
                IndexedOrigin::Package { identity, .. } => Some(identity.clone()),
            };
            aliases.push(FunctionAliasSymbol {
                module: file.module.clone(),
                name: name.text.clone(),
                package,
                target_module: function_alias_target_module(file, name_index),
                target_name: function_alias_target_name(tokens, name_index),
                import_aliases: file.import_aliases.clone(),
            });
        }
    }
    aliases
}

fn function_alias_target_index(tokens: &[Token], name_index: usize) -> Option<usize> {
    let equal_index = next_non_layout_index(tokens, name_index)
        .filter(|index| tokens[*index].kind == TokenKind::Equal)?;
    let first_index = tokens[equal_index + 1..]
        .iter()
        .enumerate()
        .find(|(_, token)| token.kind == TokenKind::Ident)
        .map(|(index, _)| equal_index + 1 + index)?;
    let mut leaf_index = first_index;
    while let Some(next_index) = next_path_segment_index(tokens, leaf_index) {
        leaf_index = next_index;
    }
    Some(leaf_index)
}

fn function_alias_target_module(file: &IndexedFile, name_index: usize) -> Option<String> {
    let target_index = function_alias_target_index(&file.tokens, name_index)?;
    qualifier_for_token(&file.tokens, target_index)
}

fn function_alias_target_name(tokens: &[Token], name_index: usize) -> Option<String> {
    function_alias_target_index(tokens, name_index).map(|index| tokens[index].text.clone())
}
