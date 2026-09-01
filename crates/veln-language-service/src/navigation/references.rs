fn call_references(file: &IndexedFile, name: &str) -> Vec<SourceSpan> {
    let tokens = lex(&file.source).tokens;
    let scopes = function_scopes(&tokens);
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_identifier(&token.text)
                && previous_non_layout_token(&tokens, *index)
                    .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                && !is_field_name(&tokens, *index)
                && !is_function_declaration_name(&tokens, *index)
                && !is_parameter_name(&tokens, *index)
                && !is_local_binding_name(&tokens, *index)
                && !is_handler_operation_clause_operation_name(&tokens, *index)
                && (token_scope(&scopes, token.range.start)
                    .is_some_and(|scope| !scope.shadows(name, &tokens, *index))
                    || handler_function_reference_is_unshadowed(file, &tokens, *index, name)
                    || is_function_alias_target_reference(&tokens, *index, name)
                    || is_codec_implementation_function_reference(&tokens, *index, name))
        })
        .map(|(_, token)| file.source.span(token.range))
        .collect()
}

fn type_reference_spans(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
) -> Vec<(usize, SourceSpan)> {
    let parsed = parse(source);
    let mut spans = Vec::new();
    for item in &parsed.tree.items {
        match item {
            SyntaxItem::Function(function) => {
                spans.extend(type_references_in_params(
                    source,
                    tokens,
                    name,
                    &function.params,
                ));
                if let Some(span) = &function.return_type_span {
                    spans.extend(type_reference_tokens_in_span(source, tokens, name, span));
                }
                spans.extend(type_references_in_body_lines(
                    source,
                    tokens,
                    name,
                    &function.body,
                ));
            }
            SyntaxItem::Handler(handler) => {
                spans.extend(type_references_in_params(
                    source,
                    tokens,
                    name,
                    &handler.params,
                ));
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    spans.extend(type_references_in_params(
                        source,
                        tokens,
                        name,
                        &operation.params,
                    ));
                    spans.extend(type_references_after_token_in_span(
                        source,
                        tokens,
                        name,
                        &operation.span,
                        TokenKind::Arrow,
                    ));
                }
            }
            SyntaxItem::Type(type_decl) => {
                for variant in &type_decl.variants {
                    for field in &variant.fields {
                        spans.extend(type_references_in_variant_field(
                            source,
                            tokens,
                            name,
                            &field.span,
                        ));
                    }
                }
            }
            SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Type => {
                spans.extend(
                    alias
                        .target_spans
                        .iter()
                        .flat_map(|span| type_reference_tokens_in_span(source, tokens, name, span)),
                );
            }
            _ => {}
        }
    }
    sort_type_reference_locations(&mut spans);
    spans
}

fn sort_type_reference_locations(locations: &mut Vec<(usize, SourceSpan)>) {
    locations.sort_by(|left, right| {
        left.1
            .file
            .as_str()
            .cmp(right.1.file.as_str())
            .then(left.1.start.offset.cmp(&right.1.start.offset))
            .then(left.1.end.offset.cmp(&right.1.end.offset))
    });
    locations.dedup_by(|left, right| {
        left.1.file == right.1.file
            && left.1.start.offset == right.1.start.offset
            && left.1.end.offset == right.1.end.offset
    });
}

fn is_type_reference_token(source: &SourceFile, name: &str, selection: &SourceSpan) -> bool {
    let tokens = lex(source).tokens;
    type_reference_spans(source, &tokens, name)
        .into_iter()
        .any(|(_, span)| {
            span.file == selection.file
                && span.start.offset == selection.start.offset
                && span.end.offset == selection.end.offset
        })
}

fn type_references_in_params(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    params: &[veln_syntax::Param],
) -> Vec<(usize, SourceSpan)> {
    params
        .iter()
        .filter_map(|param| param.ty_span.as_ref())
        .flat_map(|span| type_reference_tokens_in_span(source, tokens, name, span))
        .collect()
}

fn type_references_in_body_lines(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    body: &[BodyLine],
) -> Vec<(usize, SourceSpan)> {
    body.iter()
        .flat_map(|line| match line {
            BodyLine::Let {
                annotation: Some(_),
                span,
                ..
            } => type_references_after_token_until_token_in_span(
                source,
                tokens,
                name,
                span,
                TokenKind::Colon,
                TokenKind::Equal,
            ),
            _ => Vec::new(),
        })
        .collect()
}

fn type_references_in_variant_field(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    field_span: &SourceSpan,
) -> Vec<(usize, SourceSpan)> {
    let colon_offset = tokens
        .iter()
        .find(|token| {
            token.kind == TokenKind::Colon
                && token.range.start >= field_span.start.offset
                && token.range.end <= field_span.end.offset
        })
        .map(|token| token.range.end)
        .unwrap_or(field_span.start.offset);
    tokens
        .iter()
        .enumerate()
        .filter(|token| {
            token.1.range.start >= colon_offset
                && token.1.range.end <= field_span.end.offset
                && is_type_reference_token_text(token.1, name)
        })
        .map(|(index, token)| (index, source.span(token.range)))
        .collect()
}

fn type_references_after_token_in_span(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    span: &SourceSpan,
    start_kind: TokenKind,
) -> Vec<(usize, SourceSpan)> {
    let start_offset = tokens
        .iter()
        .find(|token| {
            token.kind == start_kind
                && token.range.start >= span.start.offset
                && token.range.end <= span.end.offset
        })
        .map(|token| token.range.end)
        .unwrap_or(span.end.offset);
    type_reference_tokens_in_range(source, tokens, name, start_offset, span.end.offset)
}

fn type_references_after_token_until_token_in_span(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    span: &SourceSpan,
    start_kind: TokenKind,
    end_kind: TokenKind,
) -> Vec<(usize, SourceSpan)> {
    let Some(start_index) = tokens.iter().position(|token| {
        token.kind == start_kind
            && token.range.start >= span.start.offset
            && token.range.end <= span.end.offset
    }) else {
        return Vec::new();
    };
    let start_offset = tokens[start_index].range.end;
    let end_offset = tokens[start_index + 1..]
        .iter()
        .find(|token| token.kind == end_kind && token.range.end <= span.end.offset)
        .map(|token| token.range.start)
        .unwrap_or(span.end.offset);
    type_reference_tokens_in_range(source, tokens, name, start_offset, end_offset)
}

fn type_reference_tokens_in_span(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    span: &SourceSpan,
) -> Vec<(usize, SourceSpan)> {
    type_reference_tokens_in_range(source, tokens, name, span.start.offset, span.end.offset)
}

fn type_reference_tokens_in_range(
    source: &SourceFile,
    tokens: &[Token],
    name: &str,
    start_offset: usize,
    end_offset: usize,
) -> Vec<(usize, SourceSpan)> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            token.range.start >= start_offset
                && token.range.end <= end_offset
                && is_type_reference_token_text(token, name)
        })
        .map(|(index, token)| (index, source.span(token.range)))
        .collect()
}

fn is_type_reference_token_text(token: &Token, name: &str) -> bool {
    token.kind == TokenKind::Ident
        && token.text == name
        && token
            .text
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_uppercase())
}
