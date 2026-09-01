fn recovery_roles_compatible(record: NameClass, role: NameClass) -> bool {
    record == role || (record == NameClass::Function && role == NameClass::ValueBinding)
}

fn recovery_matches_name_file_and_roles(
    symbol: &RecoverySymbol,
    file: &IndexedFile,
    name: &str,
    roles: &[NameClass],
) -> bool {
    symbol.name == name
        && symbol.source_file == file.source.path().as_str()
        && symbol.name_class().is_some_and(|class| {
            roles
                .iter()
                .any(|role| recovery_roles_compatible(class, *role))
        })
}

fn recovery_matches_declaration(
    symbol: &RecoverySymbol,
    file: &IndexedFile,
    name: &str,
    roles: &[NameClass],
    selection: &SourceSpan,
) -> bool {
    recovery_matches_name_file_and_roles(symbol, file, name, roles)
        && same_span(&symbol.declaration, selection)
}

fn recovery_visible_to_selected(
    symbol: &RecoverySymbol,
    file: &IndexedFile,
    name: &str,
    roles: &[NameClass],
    selected: &[RecoverySymbol],
) -> bool {
    recovery_matches_name_file_and_roles(symbol, file, name, roles)
        && selected.iter().any(|selected| {
            same_recovery_symbol(selected, symbol) || recovery_scopes_overlap(selected, symbol)
        })
}

fn dedup_recovery_symbols(mut symbols: Vec<RecoverySymbol>) -> Vec<RecoverySymbol> {
    let mut unique = Vec::new();
    for symbol in symbols.drain(..) {
        if !unique
            .iter()
            .any(|existing| same_recovery_symbol(existing, &symbol))
        {
            unique.push(symbol);
        }
    }
    unique
}

fn recovery_scopes_overlap(left: &RecoverySymbol, right: &RecoverySymbol) -> bool {
    left.scope_start < right.scope_end && right.scope_start < left.scope_end
}

fn recovery_roles_for_declaration_token(tokens: &[Token], index: usize) -> Option<Vec<NameClass>> {
    if is_function_declaration_name(tokens, index) {
        return Some(vec![NameClass::Function]);
    }
    if is_type_declaration_name(tokens, index) {
        return Some(vec![NameClass::Type]);
    }
    if is_constructor_declaration_name(tokens, index) {
        return Some(vec![NameClass::Constructor]);
    }
    if is_parameter_name(tokens, index)
        || is_local_binding_name(tokens, index)
        || is_satisfy_candidate_binding_name(tokens, index)
    {
        return Some(vec![NameClass::ValueBinding]);
    }
    None
}
