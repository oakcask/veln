struct FunctionAliasDeclaration {
    declaration_kind: SymbolDeclarationKind,
    target_module: Option<String>,
    target_name: Option<String>,
}

fn function_alias_declaration(
    tokens: &[Token],
    alias_name_index: usize,
    alias_name_end: usize,
) -> FunctionAliasDeclaration {
    if !has_function_alias_equal(tokens, alias_name_index, alias_name_end) {
        return FunctionAliasDeclaration {
            declaration_kind: SymbolDeclarationKind::Declaration,
            target_module: None,
            target_name: None,
        };
    }
    let (target_module, target_name) = function_alias_target(tokens, alias_name_index)
        .map(|(module, name)| (module, Some(name)))
        .unwrap_or((None, None));
    FunctionAliasDeclaration {
        declaration_kind: SymbolDeclarationKind::PublicAlias,
        target_module,
        target_name,
    }
}

fn has_function_alias_equal(
    tokens: &[Token],
    alias_name_index: usize,
    alias_name_end: usize,
) -> bool {
    next_non_layout_token(tokens, alias_name_index)
        .filter(|token| token.range.start >= alias_name_end)
        .is_some_and(|token| token.kind == TokenKind::Equal)
}

fn function_alias_target(
    tokens: &[Token],
    alias_name_index: usize,
) -> Option<(Option<String>, String)> {
    let equal = next_non_layout_token(tokens, alias_name_index)?;
    if equal.kind != TokenKind::Equal {
        return None;
    }
    let mut index = tokens
        .iter()
        .position(|token| token.range.start == equal.range.start)?;
    let mut segments = Vec::new();
    loop {
        index = next_non_layout_index(tokens, index)?;
        let token = tokens.get(index)?;
        if token.kind != TokenKind::Ident {
            break;
        }
        segments.push(token.text.clone());
        let Some(next) = next_non_layout_index(tokens, index) else {
            break;
        };
        if tokens.get(next).is_none_or(|token| token.kind != TokenKind::DoubleColon) {
            break;
        }
        index = next;
    }
    let name = segments.pop()?;
    let module = (!segments.is_empty()).then(|| segments.join("::"));
    Some((module, name))
}
