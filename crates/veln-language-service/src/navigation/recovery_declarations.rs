fn recovery_declaration_public(
    source: &SourceFile,
    tokens: &[Token],
    syntax: &SyntaxTree,
    span: &SourceSpan,
) -> bool {
    let Some(token_index) = tokens.iter().position(|token| {
        token.range.start == span.start.offset && token.range.end == span.end.offset
    }) else {
        return false;
    };
    if is_function_declaration_name(tokens, token_index) {
        return previous_non_layout_index(tokens, token_index)
            .and_then(|function_index| previous_non_layout_token(tokens, function_index))
            .is_some_and(|previous| previous.kind == TokenKind::Pub)
    }
    syntax.items.iter().any(|item| match item {
        SyntaxItem::Type(type_decl) => {
            type_decl.name_span.as_ref().is_some_and(|name_span| same_span(name_span, span))
                && type_decl.visibility == Visibility::Public
                || type_decl.variants.iter().any(|variant| {
                    recovery_variant_matches(source, tokens, variant, span)
                        && type_decl.visibility == Visibility::Public
                        && variant.visibility == Visibility::Public
                })
        }
        SyntaxItem::PublicAlias(alias) => alias
            .name_span
            .as_ref()
            .is_some_and(|name_span| same_span(name_span, span)),
        _ => false,
    })
}

fn recovery_variant_matches(
    source: &SourceFile,
    tokens: &[Token],
    variant: &TypeVariantDecl,
    span: &SourceSpan,
) -> bool {
    if let Some(name_span) = &variant.name_span {
        return same_span(name_span, span);
    }
    let name = source
        .text()
        .get(span.start.offset..span.end.offset)
        .unwrap_or_default();
    tokens.iter().any(|token| {
        token.kind == TokenKind::Ident
            && token.text == name
            && token.range.start == span.start.offset
            && token.range.end == span.end.offset
            && token.range.start >= variant.span.start.offset
            && token.range.end <= variant.span.end.offset
    })
}

fn scoped_binding_matches(binding: &ScopedBinding, name: &str, span: &SourceSpan) -> bool {
    binding.name == name
        && binding.declaration_start == span.start.offset
        && binding.declaration_end == span.end.offset
}

fn local_binding_matches(binding: &LocalBinding, name: &str, span: &SourceSpan) -> bool {
    binding.name == name
        && binding.declaration_start == span.start.offset
        && binding.declaration_end == span.end.offset
}

fn symbol_kind_for_name_class(class: NameClass) -> Option<SymbolKind> {
    match class {
        NameClass::Type => Some(SymbolKind::Type),
        NameClass::Constructor => Some(SymbolKind::Constructor),
        NameClass::Function => Some(SymbolKind::Function),
        NameClass::ValueBinding => Some(SymbolKind::ValueBinding),
        NameClass::Module => None,
    }
}

fn is_invalid_declaration_name(file: &IndexedFile, span: &SourceSpan) -> bool {
    file.invalid_declaration_names.iter().any(|invalid| {
        invalid.file == span.file
            && invalid.start.offset == span.start.offset
            && invalid.end.offset == span.end.offset
    })
}
